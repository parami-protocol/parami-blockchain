#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::fungibles::{
            metadata::Mutate as FungMetaMutate, Create as FungCreate, Inspect as FungInspect,
            InspectMetadata as FungMeta, Mutate as FungMutate, Transfer as FungTransfer,
        },
        Currency,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        Get,
    },
    PalletId,
};
use sp_runtime::traits::{
    AccountIdConversion, AtLeast32BitUnsigned, Bounded, CheckedSub, One, Zero,
};
use sp_std::prelude::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type SwapOf<T> = types::Swap<AccountOf<T>, HeightOf<T>, <T as pallet::Config>::AssetId>;

pub struct MaxValue {}
impl<T: Bounded> Get<T> for MaxValue {
    fn get() -> T {
        T::max_value()
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type AssetId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + Bounded;

        type Assets: FungCreate<Self::AccountId, AssetId = Self::AssetId>
            + FungMeta<Self::AccountId, AssetId = Self::AssetId>
            + FungMetaMutate<Self::AccountId, AssetId = Self::AssetId>
            + FungMutate<Self::AccountId, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
            + FungTransfer<Self::AccountId, AssetId = Self::AssetId, Balance = BalanceOf<Self>>;

        type Currency: Currency<Self::AccountId>;

        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Twox128, T::AssetId, SwapOf<T>>;

    #[pallet::storage]
    #[pallet::getter(fn next_class_id)]
    pub type NextLpId<T: Config> = StorageValue<_, T::AssetId, ValueQuery, MaxValue>;

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        /// New swap pair created \[id\]
        Created(T::AssetId),
        /// Liquidity add \[id, account, currency, tokens\]
        LiquidityAdded(T::AssetId, T::AccountId, BalanceOf<T>, BalanceOf<T>),
        /// Liquidity removed \[id, account, currency, tokens\]
        LiquidityRemoved(T::AssetId, T::AccountId, BalanceOf<T>, BalanceOf<T>),
        /// Tokens bought \[id, account, tokens\]
        SwapBuy(T::AssetId, T::AccountId, BalanceOf<T>),
        /// Tokens sold \[id, account, tokens\]
        SwapSell(T::AssetId, T::AccountId, BalanceOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        Deadline,
        Exists,
        NoAvailableTokenId,
        NoLiquidity,
        NotExists,
        TooLowCurrency,
        TooLowLiquidity,
        TooLowTokens,
        TooManyTokens,
        TooExpensiveTokens,
        TooExpensiveCurrency,
        ZeroCurrency,
        ZeroLiquidity,
        ZeroTokens,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000_000_000)]
        pub fn create(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            ensure!(!Metadata::<T>::contains_key(&token_id), Error::<T>::Exists);

            let minimum = T::Currency::minimum_balance();

            let mut name = T::Assets::name(&token_id);
            name.extend_from_slice(b"/AD3 LP");

            let mut symbol = T::Assets::symbol(&token_id);
            symbol.extend_from_slice(b"/AD3");

            let lp_token_id =
                <NextLpId<T>>::try_mutate(|id| -> Result<T::AssetId, DispatchError> {
                    let current_id = *id;
                    *id = id
                        .checked_sub(&One::one())
                        .ok_or(Error::<T>::NoAvailableTokenId)?;
                    Ok(current_id)
                })?;

            // 1. create pot

            let created = <frame_system::Pallet<T>>::block_number();

            let pot: T::AccountId = T::PalletId::get().into_sub_account(token_id);

            // 2. create lp token

            T::Assets::create(lp_token_id, pot.clone(), false, minimum)?;
            T::Assets::set(lp_token_id, &pot, name, symbol, 18)?;

            // 3. insert metadata

            <Metadata<T>>::insert(
                &token_id,
                types::Swap {
                    pot,
                    token_id,
                    lp_token_id,
                    created,
                },
            );

            Self::deposit_event(Event::Created(token_id));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] min_liquidity: BalanceOf<T>,
            #[pallet::compact] max_tokens: BalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            if deadline > Zero::zero() {
                let height = <frame_system::Pallet<T>>::block_number();
                ensure!(deadline > height, Error::<T>::Deadline);
            }

            let who = ensure_signed(origin)?;

            ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);
            ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);

            let meta = Metadata::<T>::get(&token_id).ok_or(Error::<T>::NotExists)?;

            let total_liquidity = T::Assets::total_issuance(meta.lp_token_id);

            let (tokens, liquidity) = if total_liquidity > Zero::zero() {
                ensure!(min_liquidity > Zero::zero(), Error::<T>::ZeroLiquidity);

                let swap_balance = T::Currency::free_balance(&meta.pot);
                let token_reserve = T::Assets::balance(token_id, &meta.pot);

                let tokens = currency * token_reserve / swap_balance;
                let liquidity = currency * total_liquidity / swap_balance;

                (tokens, liquidity)
            } else {
                // Fresh swap with no liquidity
                (max_tokens, currency)
            };

            ensure!(max_tokens >= tokens, Error::<T>::TooManyTokens);
            ensure!(liquidity >= min_liquidity, Error::<T>::TooLowLiquidity);

            T::Currency::transfer(&who, &meta.pot, currency, KeepAlive)?;
            T::Assets::transfer(token_id, &who, &meta.pot, tokens, true)?;

            T::Assets::mint_into(meta.lp_token_id, &who, liquidity)?;

            Self::deposit_event(Event::LiquidityAdded(token_id, who, currency, tokens));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] liquidity: BalanceOf<T>,
            #[pallet::compact] min_currency: BalanceOf<T>,
            #[pallet::compact] min_tokens: BalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            if deadline > Zero::zero() {
                let height = <frame_system::Pallet<T>>::block_number();
                ensure!(deadline > height, Error::<T>::Deadline);
            }

            let who = ensure_signed(origin)?;

            ensure!(liquidity > Zero::zero(), Error::<T>::ZeroLiquidity);

            let meta = Metadata::<T>::get(&token_id).ok_or(Error::<T>::NotExists)?;

            let total_liquidity = T::Assets::total_issuance(meta.lp_token_id);

            ensure!(total_liquidity > Zero::zero(), Error::<T>::NoLiquidity);

            let swap_balance = T::Currency::free_balance(&meta.pot);
            let token_reserve = T::Assets::balance(token_id, &meta.pot);

            let currency = liquidity * swap_balance / total_liquidity;
            let tokens = liquidity * token_reserve / total_liquidity;

            ensure!(currency >= min_currency, Error::<T>::TooLowCurrency);
            ensure!(tokens >= min_tokens, Error::<T>::TooLowTokens);

            T::Assets::slash(meta.lp_token_id, &who, liquidity)?;

            T::Assets::transfer(token_id, &meta.pot, &who, tokens, false)?;
            T::Currency::transfer(&meta.pot, &who, currency, AllowDeath)?;

            Self::deposit_event(Event::LiquidityRemoved(token_id, who, currency, tokens));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn buy_tokens(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] tokens: BalanceOf<T>,
            #[pallet::compact] max_currency: BalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            ensure!(tokens > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(max_currency > Zero::zero(), Error::<T>::ZeroCurrency);

            let meta = Metadata::<T>::get(&token_id).ok_or(Error::<T>::NotExists)?;

            let swap_balance = T::Currency::free_balance(&meta.pot);
            let token_reserve = T::Assets::balance(token_id, &meta.pot);

            let currency_sold = Self::price_buy(tokens, swap_balance, token_reserve);

            ensure!(
                currency_sold <= max_currency,
                Error::<T>::TooExpensiveCurrency
            );

            T::Currency::transfer(&who, &meta.pot, currency_sold, KeepAlive)?;
            T::Assets::transfer(token_id, &meta.pot, &who, tokens, false)?;

            Self::deposit_event(Event::SwapBuy(token_id, who, tokens));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn sell_tokens(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] tokens: BalanceOf<T>,
            #[pallet::compact] min_currency: BalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            ensure!(tokens > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(min_currency > Zero::zero(), Error::<T>::ZeroCurrency);

            let meta = Metadata::<T>::get(&token_id).ok_or(Error::<T>::NotExists)?;

            let swap_balance = T::Currency::free_balance(&meta.pot);
            let token_reserve = T::Assets::balance(token_id, &meta.pot);

            let currency_bought = Self::price_sell(tokens, token_reserve, swap_balance);

            ensure!(currency_bought >= min_currency, Error::<T>::TooLowCurrency);

            T::Assets::transfer(token_id, &who, &meta.pot, tokens, true)?;
            T::Currency::transfer(&meta.pot, &who, currency_bought, AllowDeath)?;

            Self::deposit_event(Event::SwapSell(token_id, who, tokens));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn sell_currency(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] min_tokens: BalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);
            ensure!(min_tokens > Zero::zero(), Error::<T>::ZeroTokens);

            let meta = Metadata::<T>::get(&token_id).ok_or(Error::<T>::NotExists)?;

            let swap_balance = T::Currency::free_balance(&meta.pot);
            let token_reserve = T::Assets::balance(token_id, &meta.pot);

            let tokens_bought = Self::price_sell(currency, swap_balance, token_reserve);

            ensure!(tokens_bought >= min_tokens, Error::<T>::TooExpensiveTokens);

            T::Currency::transfer(&who, &meta.pot, currency, KeepAlive)?;
            T::Assets::transfer(token_id, &meta.pot, &who, tokens_bought, false)?;

            Self::deposit_event(Event::SwapBuy(token_id, who, tokens_bought));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn buy_currency(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] max_tokens: BalanceOf<T>,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);

            let meta = Metadata::<T>::get(&token_id).ok_or(Error::<T>::NotExists)?;

            let swap_balance = T::Currency::free_balance(&meta.pot);
            let token_reserve = T::Assets::balance(token_id, &meta.pot);

            let tokens_sold = Self::price_buy(currency, token_reserve, swap_balance);

            ensure!(max_tokens >= tokens_sold, Error::<T>::TooLowTokens);

            T::Assets::transfer(token_id, &who, &meta.pot, tokens_sold, true)?;
            T::Currency::transfer(&meta.pot, &who, currency, AllowDeath)?;

            Self::deposit_event(Event::SwapSell(token_id, who, tokens_sold));

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn price_buy(
        output_amount: BalanceOf<T>,
        input_reserve: BalanceOf<T>,
        output_reserve: BalanceOf<T>,
    ) -> BalanceOf<T> {
        let numerator = input_reserve * output_amount * 1000u32.into();
        let denominator = (output_reserve - output_amount) * 997u32.into();
        numerator / denominator + 1u32.into()
    }

    fn price_sell(
        input_amount: BalanceOf<T>,
        input_reserve: BalanceOf<T>,
        output_reserve: BalanceOf<T>,
    ) -> BalanceOf<T> {
        let input_amount_with_fee = input_amount * 997u32.into();
        let numerator = input_amount_with_fee * output_reserve;
        let denominator = (input_reserve * 1000u32.into()) + input_amount_with_fee;
        numerator / denominator
    }
}
