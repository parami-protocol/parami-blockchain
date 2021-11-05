#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

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
use parami_traits::Swaps;
use sp_runtime::{
    traits::{
        AccountIdConversion, AtLeast32BitUnsigned, Bounded, CheckedSub, One, Saturating, Zero,
    },
    DispatchError,
};
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type SwapOf<T> = types::Swap<AccountOf<T>, BalanceOf<T>, HeightOf<T>, <T as Config>::AssetId>;

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

        /// Fungible token ID type
        type AssetId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + Bounded;

        /// The assets trait to create, mint, and transfer fungible tokens
        type Assets: FungCreate<Self::AccountId, AssetId = Self::AssetId>
            + FungMeta<Self::AccountId, AssetId = Self::AssetId>
            + FungMetaMutate<Self::AccountId, AssetId = Self::AssetId>
            + FungMutate<Self::AccountId, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
            + FungTransfer<Self::AccountId, AssetId = Self::AssetId, Balance = BalanceOf<Self>>;

        /// The currency trait
        type Currency: Currency<Self::AccountId>;

        /// The pallet id, used for deriving liquid accounts
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
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
        /// Liquidity add \[id, account, liquidity, currency, tokens\]
        LiquidityAdded(T::AssetId, AccountOf<T>, BalanceOf<T>, BalanceOf<T>),
        /// Liquidity removed \[id, account, currency, tokens\]
        LiquidityRemoved(T::AssetId, AccountOf<T>, BalanceOf<T>, BalanceOf<T>),
        /// Tokens bought \[id, account, tokens, currency\]
        SwapBuy(T::AssetId, AccountOf<T>, BalanceOf<T>, BalanceOf<T>),
        /// Tokens sold \[id, account, tokens, currency\]
        SwapSell(T::AssetId, AccountOf<T>, BalanceOf<T>, BalanceOf<T>),
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
            let who = ensure_signed(origin)?;

            let swap_id = Self::new(&who, token_id)?;

            Self::deposit_event(Event::Created(swap_id));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] min_liquidity: BalanceOf<T>,
            #[pallet::compact] max_tokens: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let (currency, tokens) = Self::mint(
                &who,
                token_id,
                currency,
                min_liquidity,
                max_tokens,
                true, // keep alive
            )?;

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
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let (currency, tokens) = Self::burn(
                &who,
                token_id,
                liquidity, // can burn all
                min_currency,
                min_tokens,
            )?;

            Self::deposit_event(Event::LiquidityRemoved(token_id, who, currency, tokens));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn buy_tokens(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] tokens: BalanceOf<T>,
            #[pallet::compact] max_currency: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let (tokens, currency) = Self::token_out(&who, token_id, tokens, max_currency, true)?;

            Self::deposit_event(Event::SwapBuy(token_id, who, tokens, currency));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn sell_tokens(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] tokens: BalanceOf<T>,
            #[pallet::compact] min_currency: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let (tokens, currency) = Self::token_in(&who, token_id, tokens, min_currency, true)?;

            Self::deposit_event(Event::SwapSell(token_id, who, tokens, currency));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn sell_currency(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] min_tokens: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let (currency, tokens) = Self::quote_in(&who, token_id, currency, min_tokens, true)?;

            Self::deposit_event(Event::SwapBuy(token_id, who, tokens, currency));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn buy_currency(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            #[pallet::compact] currency: BalanceOf<T>,
            #[pallet::compact] max_tokens: BalanceOf<T>,
            deadline: HeightOf<T>,
        ) -> DispatchResult {
            let height = <frame_system::Pallet<T>>::block_number();
            ensure!(deadline > height, Error::<T>::Deadline);

            let who = ensure_signed(origin)?;

            let (currency, tokens) = Self::quote_out(&who, token_id, currency, max_tokens, true)?;

            Self::deposit_event(Event::SwapSell(token_id, who, tokens, currency));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub swaps: Vec<(u32, u32, T::AccountId)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                swaps: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            use sp_runtime::traits::Saturating;

            let length = self.swaps.len();

            for i in 0..length {
                let token_id = self.swaps[i].0.into();
                let lp_token_id = self.swaps[i].1.into();
                let pot = self.swaps[i].2.clone();

                let quote = T::Currency::free_balance(&pot);
                let token = T::Assets::balance(token_id, &pot);

                <Metadata<T>>::insert(
                    token_id,
                    types::Swap {
                        pot,
                        quote,
                        token,
                        token_id,
                        lp_token_id,
                        created: Default::default(),
                    },
                );
            }

            let length = length as u32;
            <NextLpId<T>>::put(T::AssetId::max_value().saturating_sub(length.into()));
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

impl<T: Config> Swaps for Pallet<T> {
    type AccountId = AccountOf<T>;
    type AssetId = T::AssetId;
    type QuoteBalance = BalanceOf<T>;
    type TokenBalance = BalanceOf<T>;

    fn new(
        _who: &Self::AccountId,
        token_id: Self::AssetId,
    ) -> Result<Self::AssetId, DispatchError> {
        ensure!(!<Metadata<T>>::contains_key(&token_id), Error::<T>::Exists);

        let minimum = T::Currency::minimum_balance();

        let mut name = T::Assets::name(&token_id);
        name.extend_from_slice(b"/AD3 LP");

        let mut symbol = T::Assets::symbol(&token_id);
        symbol.extend_from_slice(b"/AD3");

        let lp_token_id = <NextLpId<T>>::try_mutate(|id| -> Result<T::AssetId, DispatchError> {
            let current_id = *id;
            *id = id
                .checked_sub(&One::one())
                .ok_or(Error::<T>::NoAvailableTokenId)?;
            Ok(current_id)
        })?;

        // 1. create pot

        let created = <frame_system::Pallet<T>>::block_number();

        let pot: AccountOf<T> = T::PalletId::get().into_sub_account(token_id);

        // 2. create lp token

        T::Assets::create(lp_token_id, pot.clone(), true, minimum)?;
        T::Assets::set(lp_token_id, &pot, name, symbol, 18)?;

        // 3. insert metadata

        <Metadata<T>>::insert(
            &token_id,
            types::Swap {
                pot,
                quote: Zero::zero(),
                token: Zero::zero(),
                token_id,
                lp_token_id,
                created,
            },
        );

        Ok(token_id)
    }

    fn mint(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_liquidity: Self::TokenBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError> {
        ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);
        ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);

        let mut meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let total_liquidity = T::Assets::total_issuance(meta.lp_token_id);

        let (tokens, liquidity) = if total_liquidity > Zero::zero() {
            ensure!(min_liquidity > Zero::zero(), Error::<T>::ZeroLiquidity);

            let tokens = currency * meta.token / meta.quote;
            let liquidity = currency * total_liquidity / meta.quote;

            (tokens, liquidity)
        } else {
            // Fresh swap with no liquidity
            (max_tokens, currency)
        };

        ensure!(max_tokens >= tokens, Error::<T>::TooManyTokens);
        ensure!(liquidity >= min_liquidity, Error::<T>::TooLowLiquidity);

        T::Currency::transfer(
            &who,
            &meta.pot,
            currency,
            if keep_alive { KeepAlive } else { AllowDeath },
        )?;
        T::Assets::transfer(token_id, &who, &meta.pot, tokens, keep_alive)?;

        T::Assets::mint_into(meta.lp_token_id, &who, liquidity)?;

        meta.quote.saturating_accrue(currency);
        meta.token.saturating_accrue(tokens);

        <Metadata<T>>::insert(token_id, meta);

        Ok((currency, tokens))
    }

    fn burn(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        liquidity: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError> {
        ensure!(liquidity > Zero::zero(), Error::<T>::ZeroLiquidity);

        let mut meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let total_liquidity = T::Assets::total_issuance(meta.lp_token_id);

        ensure!(total_liquidity > Zero::zero(), Error::<T>::NoLiquidity);

        let currency = liquidity * meta.quote / total_liquidity;
        let tokens = liquidity * meta.token / total_liquidity;

        ensure!(currency >= min_currency, Error::<T>::TooLowCurrency);
        ensure!(tokens >= min_tokens, Error::<T>::TooLowTokens);

        T::Assets::slash(meta.lp_token_id, &who, liquidity)?;

        T::Assets::transfer(token_id, &meta.pot, &who, tokens, false)?;
        T::Currency::transfer(&meta.pot, &who, currency, AllowDeath)?;

        meta.quote.saturating_reduce(currency);
        meta.token.saturating_reduce(tokens);

        <Metadata<T>>::insert(token_id, meta);

        Ok((currency, tokens))
    }

    fn token_out_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> Result<(Self::QuoteBalance, Self::AccountId), DispatchError> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let currency_sold = Self::price_buy(tokens, meta.quote, meta.token);

        Ok((currency_sold, meta.pot))
    }

    fn token_out(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        max_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::QuoteBalance), DispatchError> {
        ensure!(tokens > Zero::zero(), Error::<T>::ZeroTokens);
        ensure!(max_currency > Zero::zero(), Error::<T>::ZeroCurrency);

        let (currency_sold, pot) = Self::token_out_dry(token_id, tokens)?;

        ensure!(
            currency_sold <= max_currency,
            Error::<T>::TooExpensiveCurrency
        );

        T::Currency::transfer(
            &who,
            &pot,
            currency_sold,
            if keep_alive { KeepAlive } else { AllowDeath },
        )?;
        T::Assets::transfer(token_id, &pot, &who, tokens, false)?;

        <Metadata<T>>::mutate(&token_id, |maybe_meta| {
            if let Some(meta) = maybe_meta {
                meta.quote.saturating_accrue(currency_sold);
                meta.token.saturating_reduce(tokens);
            }
        });

        Ok((tokens, currency_sold))
    }

    fn token_in_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> Result<(Self::QuoteBalance, Self::AccountId), DispatchError> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let currency_bought = Self::price_sell(tokens, meta.token, meta.quote);

        Ok((currency_bought, meta.pot))
    }

    fn token_in(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::QuoteBalance), DispatchError> {
        ensure!(tokens > Zero::zero(), Error::<T>::ZeroTokens);
        ensure!(min_currency > Zero::zero(), Error::<T>::ZeroCurrency);

        let (currency_bought, pot) = Self::token_in_dry(token_id, tokens)?;

        ensure!(currency_bought >= min_currency, Error::<T>::TooLowCurrency);

        T::Assets::transfer(token_id, &who, &pot, tokens, keep_alive)?;
        T::Currency::transfer(&pot, &who, currency_bought, AllowDeath)?;

        <Metadata<T>>::mutate(&token_id, |maybe_meta| {
            if let Some(meta) = maybe_meta {
                meta.quote.saturating_reduce(currency_bought);
                meta.token.saturating_accrue(tokens);
            }
        });

        Ok((tokens, currency_bought))
    }

    /// dry-run of quote_in
    fn quote_in_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> Result<(Self::TokenBalance, Self::AccountId), DispatchError> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let tokens_bought = Self::price_sell(currency, meta.quote, meta.token);

        Ok((tokens_bought, meta.pot))
    }

    fn quote_in(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError> {
        ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);
        ensure!(min_tokens > Zero::zero(), Error::<T>::ZeroTokens);

        let (tokens_bought, pot) = Self::quote_in_dry(token_id, currency)?;

        ensure!(tokens_bought >= min_tokens, Error::<T>::TooExpensiveTokens);

        T::Currency::transfer(
            &who,
            &pot,
            currency,
            if keep_alive { KeepAlive } else { AllowDeath },
        )?;
        T::Assets::transfer(token_id, &pot, &who, tokens_bought, false)?;

        <Metadata<T>>::mutate(&token_id, |maybe_meta| {
            if let Some(meta) = maybe_meta {
                meta.quote.saturating_accrue(currency);
                meta.token.saturating_reduce(tokens_bought);
            }
        });

        Ok((currency, tokens_bought))
    }

    /// dry-run of quote_out
    fn quote_out_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> Result<(Self::TokenBalance, Self::AccountId), DispatchError> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let tokens_sold = Self::price_buy(currency, meta.token, meta.quote);

        Ok((tokens_sold, meta.pot))
    }

    fn quote_out(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError> {
        ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);
        ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);

        let (tokens_sold, pot) = Self::quote_out_dry(token_id, currency)?;

        ensure!(max_tokens >= tokens_sold, Error::<T>::TooLowTokens);

        T::Assets::transfer(token_id, &who, &pot, tokens_sold, keep_alive)?;
        T::Currency::transfer(&pot, &who, currency, AllowDeath)?;

        <Metadata<T>>::mutate(&token_id, |maybe_meta| {
            if let Some(meta) = maybe_meta {
                meta.quote.saturating_reduce(currency);
                meta.token.saturating_accrue(tokens_sold);
            }
        });

        Ok((currency, tokens_sold))
    }
}
