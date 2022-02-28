#![cfg_attr(not(feature = "std"), no_std)]

pub use farming::{FarmingCurve, LinearFarmingCurve};
pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod farming;
mod functions;
mod impl_swaps;
mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::fungibles::{
            InspectMetadata as FungMeta, Mutate as FungMutate, Transfer as FungTransfer,
        },
        Currency, Get,
    },
    PalletId,
};
use parami_traits::Swaps;
use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded, Saturating, Zero};
use sp_std::prelude::*;

use weights::WeightInfo;

type AssetOf<T> = <T as Config>::AssetId;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type SwapOf<T> = types::Swap<HeightOf<T>, BalanceOf<T>>;
type LiquidityOf<T> = types::Liquidity<AccountOf<T>, BalanceOf<T>, HeightOf<T>, AssetOf<T>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Fungible token ID type
        type AssetId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + AtLeast32BitUnsigned
            + Default
            + Bounded
            + Copy;

        /// The assets trait to create, mint, and transfer fungible tokens
        type Assets: FungMeta<AccountOf<Self>, AssetId = AssetOf<Self>>
            + FungMutate<AccountOf<Self>, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
            + FungTransfer<AccountOf<Self>, AssetId = AssetOf<Self>, Balance = BalanceOf<Self>>;

        /// The currency trait
        type Currency: Currency<AccountOf<Self>>;

        /// The curve for seasoned orffering
        type FarmingCurve: FarmingCurve<Self>;

        /// The pallet id, used for deriving liquid accounts
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Metadata of a swap
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Twox64Concat, AssetOf<T>, SwapOf<T>>;

    /// Liquid Provider
    #[pallet::storage]
    pub(super) type Provider<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AssetOf<T>, // Asset ID
        Twox64Concat,
        AccountOf<T>, // Provider Account
        BalanceOf<T>,
        ValueQuery,
    >;

    /// Liquid Provider Token (non-fungible)
    #[pallet::storage]
    pub(super) type Liquidity<T: Config> = StorageMap<
        _,
        Twox64Concat,
        AssetOf<T>, // LP Token ID
        LiquidityOf<T>,
    >;

    /// Liquid Provider Token Account
    #[pallet::storage]
    pub(super) type Account<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        AccountOf<T>, // Provider Account
        Twox64Concat,
        AssetOf<T>,  // LP Token ID
        HeightOf<T>, // Last Claimed
    >;

    /// Next Liquidity Provider Token ID
    #[pallet::storage]
    pub(super) type NextTokenId<T: Config> = StorageValue<_, AssetOf<T>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        /// New swap pair created \[id\]
        Created(AssetOf<T>),
        /// Liquidity add \[id, account, liquidity, currency, tokens\]
        LiquidityAdded(
            AssetOf<T>,
            AccountOf<T>,
            BalanceOf<T>,
            BalanceOf<T>,
            BalanceOf<T>,
        ),
        /// Liquidity removed \[id, account, liquidity, currency, tokens\]
        LiquidityRemoved(
            AssetOf<T>,
            AccountOf<T>,
            BalanceOf<T>,
            BalanceOf<T>,
            BalanceOf<T>,
        ),
        /// Tokens bought \[id, account, tokens, currency\]
        TokenBought(AssetOf<T>, AccountOf<T>, BalanceOf<T>, BalanceOf<T>),
        /// Tokens sold \[id, account, tokens, currency\]
        TokenSold(AssetOf<T>, AccountOf<T>, BalanceOf<T>, BalanceOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        Deadline,
        Exists,
        InsufficientCurrency,
        InsufficientLiquidity,
        InsufficientTokens,
        NoLiquidity,
        NotExists,
        Overflow,
        TooExpensiveCurrency,
        TooExpensiveTokens,
        TooLowCurrency,
        TooLowLiquidity,
        TooLowTokens,
        ZeroCurrency,
        ZeroLiquidity,
        ZeroTokens,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// create new swap pair
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        #[pallet::weight(T::WeightInfo::create())]
        pub fn create(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: AssetOf<T>,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            Self::new(token_id)?;

            Ok(())
        }

        /// Add Liquidity
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be involved in the swap
        /// * `min_liquidity` - The minimum amount of liquidity to be minted
        /// * `max_tokens` - The maximum amount of tokens to be involved in the swap
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::add_liquidity())]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: AssetOf<T>,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] min_liquidity: BalanceOf<T>,
            #[pallet::compact] max_tokens: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let _ = Self::mint(
                who,
                token_id,
                currency,
                min_liquidity,
                max_tokens,
                true, // keep alive
            )?;

            Ok(())
        }

        /// Remove Liquidity
        ///
        /// * `lp_token_id` - The Liquidity Provider Token ID
        /// * `liquidity` - The amount of liquidity to be removed
        /// * `min_currency` - The minimum currency to be returned
        /// * `min_tokens` - The minimum amount of tokens to be returned
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::remove_liquidity())]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            #[pallet::compact] lp_token_id: AssetOf<T>,
            #[pallet::compact] min_currency: BalanceOf<T>,
            #[pallet::compact] min_tokens: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let _ = Self::burn(who, lp_token_id, min_currency, min_tokens)?;

            Ok(())
        }

        /// Buy tokens
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be bought
        /// * `max_currency` - The maximum currency to be spent
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::buy_tokens())]
        pub fn buy_tokens(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: AssetOf<T>,
            #[pallet::compact] tokens: BalanceOf<T>,
            #[pallet::compact] max_currency: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let _ = Self::token_out(who, token_id, tokens, max_currency, true)?;

            Ok(())
        }

        /// Sell tokens
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be sold
        /// * `min_currency` - The maximum currency to be gained
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::sell_tokens())]
        pub fn sell_tokens(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: AssetOf<T>,
            #[pallet::compact] tokens: BalanceOf<T>,
            #[pallet::compact] min_currency: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let _ = Self::token_in(who, token_id, tokens, min_currency, false)?;

            Ok(())
        }

        /// Sell currency
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be sold
        /// * `min_tokens` - The minimum amount of tokens to be gained
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::sell_currency())]
        pub fn sell_currency(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: AssetOf<T>,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] min_tokens: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let _ = Self::quote_in(who, token_id, currency, min_tokens, true)?;

            Ok(())
        }

        /// Buy currency (sell tokens)
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be bought
        /// * `max_tokens` - The maximum amount of tokens to be spent
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::buy_currency())]
        pub fn buy_currency(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: AssetOf<T>,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] max_tokens: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let _ = Self::quote_out(who, token_id, currency, max_tokens, false)?;

            Ok(())
        }

        /// Acquire Liquidity
        ///
        /// * `lp_token_id` - The Liquidity Provider Token ID
        #[pallet::weight(T::WeightInfo::remove_liquidity())]
        pub fn acquire_reward(
            origin: OriginFor<T>,
            #[pallet::compact] lp_token_id: AssetOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (liquidity, reward) = Self::calculate_reward(lp_token_id)?;
            ensure!(liquidity.owner == who, Error::<T>::NotExists);

            T::Assets::mint_into(liquidity.token_id, &who, reward)?;

            let claimed = <frame_system::Pallet<T>>::block_number();
            <Account<T>>::insert(&who, lp_token_id, claimed);

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub liquidities: Vec<(AssetOf<T>, AssetOf<T>, BalanceOf<T>, AccountOf<T>)>,
        pub next_token_id: AssetOf<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                liquidities: Default::default(),
                next_token_id: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <NextTokenId<T>>::put(self.next_token_id);

            for (id, token_id, amount, owner) in &self.liquidities {
                let id = *id;
                let token_id = *token_id;
                let amount = *amount;

                if id >= self.next_token_id {
                    panic!("Liquidity Token ID must be less than next_token_id");
                }

                <Liquidity<T>>::insert(
                    id,
                    types::Liquidity {
                        owner: owner.clone(),
                        token_id,
                        amount,
                        minted: Zero::zero(),
                    },
                );
                <Account<T>>::insert(owner, id, HeightOf::<T>::zero());

                <Provider<T>>::mutate(token_id, owner, |holding| {
                    holding.saturating_accrue(amount);
                });

                <Metadata<T>>::mutate(token_id, |maybe| {
                    if let Some(meta) = maybe {
                        meta.liquidity.saturating_accrue(amount);
                    } else {
                        *maybe = Some(types::Swap {
                            liquidity: amount,
                            ..Default::default()
                        });
                    }
                });
            }
        }
    }
}
