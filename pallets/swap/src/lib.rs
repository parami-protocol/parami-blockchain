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
use sp_core::U512;
use sp_runtime::{
    traits::{AccountIdConversion, AtLeast32BitUnsigned, Bounded, One, Saturating, Zero},
    DispatchError,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    prelude::*,
};

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type SwapOf<T> = types::Swap<AccountOf<T>, BalanceOf<T>, HeightOf<T>, <T as Config>::AssetId>;

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
    pub struct Pallet<T>(_);

    /// Metadata of a swap
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, SwapOf<T>>;

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
            #[pallet::compact] token_id: T::AssetId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let swap_id = Self::new(&who, token_id)?;

            Self::deposit_event(Event::Created(swap_id));

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

        /// Remove Liquidity
        ///
        /// * `token_id` - The Asset ID
        /// * `liquidity` - The amount of liquidity to be removed
        /// * `min_currency` - The minimum currency to be returned
        /// * `min_tokens` - The minimum amount of tokens to be returned
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::remove_liquidity())]
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

        /// Buy tokens
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be bought
        /// * `max_currency` - The maximum currency to be spent
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::buy_tokens())]
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

        /// Sell tokens
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be sold
        /// * `min_currency` - The maximum currency to be gained
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::sell_tokens())]
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

            let (tokens, currency) = Self::token_in(&who, token_id, tokens, min_currency, false)?;

            Self::deposit_event(Event::SwapSell(token_id, who, tokens, currency));

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

        /// Buy currency (sell tokens)
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be bought
        /// * `max_tokens` - The maximum amount of tokens to be spent
        /// * `deadline` - The block number at which the swap should be invalidated
        #[pallet::weight(T::WeightInfo::buy_currency())]
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

            let (currency, tokens) = Self::quote_out(&who, token_id, currency, max_tokens, false)?;

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
        }
    }
}

impl<T: Config> Pallet<T> {
    fn try_into<S, D>(value: S) -> Result<D, Error<T>>
    where
        S: TryInto<u128>,
        D: TryFrom<u128>,
    {
        let value: u128 = value.try_into().map_err(|_| Error::<T>::Overflow)?;

        value.try_into().map_err(|_| Error::<T>::Overflow)
    }

    fn calculate_liquidity(
        token_id: T::AssetId,
        currency: BalanceOf<T>,
        max_tokens: BalanceOf<T>,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>, SwapOf<T>), Error<T>> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let total_liquidity = T::Assets::total_issuance(meta.lp_token_id);

        if total_liquidity <= Zero::zero() {
            return Ok((max_tokens, currency, meta));
        }

        let currency: U512 = Self::try_into(currency)?;
        let total_quote: U512 = Self::try_into(meta.quote)?;
        let total_token: U512 = Self::try_into(meta.token)?;
        let total_liquidity: U512 = Self::try_into(total_liquidity)?;

        let tokens = currency * total_token / total_quote;
        let liquidity = currency * total_liquidity / total_quote;

        let tokens = Self::try_into(tokens)?;
        let liquidity = Self::try_into(liquidity)?;

        Ok((tokens, liquidity, meta))
    }

    fn calculate_solidness(
        token_id: T::AssetId,
        liquidity: BalanceOf<T>,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>, SwapOf<T>), Error<T>> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let total_liquidity = T::Assets::total_issuance(meta.lp_token_id);

        ensure!(total_liquidity > Zero::zero(), Error::<T>::NoLiquidity);

        let liquidity: U512 = Self::try_into(liquidity)?;
        let total_quote: U512 = Self::try_into(meta.quote)?;
        let total_token: U512 = Self::try_into(meta.token)?;
        let total_liquidity: U512 = Self::try_into(total_liquidity)?;

        let currency = liquidity * total_quote / total_liquidity;
        let tokens = liquidity * total_token / total_liquidity;

        let currency = Self::try_into(currency)?;
        let tokens = Self::try_into(tokens)?;

        Ok((currency, tokens, meta))
    }

    fn price_buy(
        output_amount: BalanceOf<T>,
        input_reserve: BalanceOf<T>,
        output_reserve: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, Error<T>> {
        ensure!(
            output_reserve > output_amount,
            Error::<T>::InsufficientLiquidity
        );

        let output_amount: U512 = Self::try_into(output_amount)?;
        let input_reserve: U512 = Self::try_into(input_reserve)?;
        let output_reserve: U512 = Self::try_into(output_reserve)?;

        let numerator = input_reserve * output_amount * U512::from(1000);
        let denominator = (output_reserve - output_amount) * U512::from(997);
        let result = numerator / denominator + U512::from(1);

        let result = Self::try_into(result)?;

        Ok(result)
    }

    fn price_sell(
        input_amount: BalanceOf<T>,
        input_reserve: BalanceOf<T>,
        output_reserve: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, Error<T>> {
        ensure!(
            input_reserve > input_amount,
            Error::<T>::InsufficientLiquidity
        );

        let input_amount: U512 = Self::try_into(input_amount)?;
        let input_reserve: U512 = Self::try_into(input_reserve)?;
        let output_reserve: U512 = Self::try_into(output_reserve)?;

        let input_amount_with_fee = input_amount * U512::from(997);
        let numerator = input_amount_with_fee * output_reserve;
        let denominator = (input_reserve * U512::from(1000)) + input_amount_with_fee;
        let result = numerator / denominator;

        let result = Self::try_into(result)?;

        Ok(result)
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

        let mut name = T::Assets::name(&token_id);
        name.extend_from_slice(b" LP*");

        let mut symbol = T::Assets::symbol(&token_id);
        symbol.extend_from_slice(b"/AD3");

        let lp_token_id = T::AssetId::max_value() - token_id;

        // 1. create pot

        let created = <frame_system::Pallet<T>>::block_number();

        let pot: AccountOf<T> = T::PalletId::get().into_sub_account(token_id);

        // 2. create lp token

        T::Assets::create(lp_token_id, pot.clone(), true, One::one())?;
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

    fn mint_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
    ) -> Result<
        (
            Self::AssetId,
            Self::TokenBalance,
            Self::AssetId,
            Self::TokenBalance,
        ),
        DispatchError,
    > {
        let (tokens, lp, meta) = Self::calculate_liquidity(token_id, currency, max_tokens)?;

        Ok((token_id, tokens, meta.lp_token_id, lp))
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
        ensure!(min_liquidity > Zero::zero(), Error::<T>::ZeroLiquidity);
        ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);

        let (tokens, lp, mut meta) = Self::calculate_liquidity(token_id, currency, max_tokens)?;

        ensure!(max_tokens >= tokens, Error::<T>::TooExpensiveCurrency);
        ensure!(lp >= min_liquidity, Error::<T>::TooLowLiquidity);

        if keep_alive {
            ensure!(
                T::Currency::free_balance(&who) - T::Currency::minimum_balance() >= currency,
                Error::<T>::InsufficientCurrency
            );
        } else {
            ensure!(
                T::Currency::free_balance(&who) >= currency,
                Error::<T>::InsufficientCurrency
            );
        }
        ensure!(
            T::Assets::balance(token_id, &who) >= tokens,
            Error::<T>::InsufficientTokens
        );

        T::Currency::transfer(
            &who,
            &meta.pot,
            currency,
            if keep_alive { KeepAlive } else { AllowDeath },
        )?;
        T::Assets::transfer(token_id, &who, &meta.pot, tokens, false)?;

        T::Assets::mint_into(meta.lp_token_id, &who, lp)?;

        meta.quote.saturating_accrue(currency);
        meta.token.saturating_accrue(tokens);

        <Metadata<T>>::insert(&token_id, meta);

        Ok((currency, tokens))
    }

    fn burn_dry(
        token_id: Self::AssetId,
        liquidity: Self::TokenBalance,
    ) -> Result<
        (
            Self::AssetId,
            Self::TokenBalance,
            Self::AssetId,
            Self::QuoteBalance,
        ),
        DispatchError,
    > {
        let (currency, tokens, meta) = Self::calculate_solidness(token_id, liquidity)?;

        Ok((token_id, tokens, meta.lp_token_id, currency))
    }

    fn burn(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        liquidity: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError> {
        ensure!(liquidity > Zero::zero(), Error::<T>::ZeroLiquidity);

        let (currency, tokens, mut meta) = Self::calculate_solidness(token_id, liquidity)?;

        ensure!(currency >= min_currency, Error::<T>::TooLowCurrency);
        ensure!(tokens >= min_tokens, Error::<T>::TooLowTokens);

        T::Assets::slash(meta.lp_token_id, &who, liquidity)?;

        T::Assets::transfer(token_id, &meta.pot, &who, tokens, false)?;
        T::Currency::transfer(&meta.pot, &who, currency, AllowDeath)?;

        meta.quote.saturating_reduce(currency);
        meta.token.saturating_reduce(tokens);

        <Metadata<T>>::insert(&token_id, meta);

        Ok((currency, tokens))
    }

    fn token_out_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> Result<Self::QuoteBalance, DispatchError> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let currency_sold = Self::price_buy(tokens, meta.quote, meta.token)?;

        Ok(currency_sold)
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

        let mut meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let currency_sold = Self::price_buy(tokens, meta.quote, meta.token)?;

        ensure!(
            currency_sold <= max_currency,
            Error::<T>::TooExpensiveCurrency
        );

        T::Currency::transfer(
            &who,
            &meta.pot,
            currency_sold,
            if keep_alive { KeepAlive } else { AllowDeath },
        )?;
        T::Assets::transfer(token_id, &meta.pot, &who, tokens, false)?;

        meta.quote.saturating_accrue(currency_sold);
        meta.token.saturating_reduce(tokens);

        <Metadata<T>>::insert(&token_id, meta);

        Ok((tokens, currency_sold))
    }

    fn token_in_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> Result<Self::QuoteBalance, DispatchError> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let currency_bought = Self::price_sell(tokens, meta.token, meta.quote)?;

        Ok(currency_bought)
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

        let mut meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let currency_bought = Self::price_sell(tokens, meta.token, meta.quote)?;

        ensure!(currency_bought >= min_currency, Error::<T>::TooLowCurrency);

        T::Assets::transfer(token_id, &who, &meta.pot, tokens, keep_alive)?;
        T::Currency::transfer(&meta.pot, &who, currency_bought, AllowDeath)?;

        meta.quote.saturating_reduce(currency_bought);
        meta.token.saturating_accrue(tokens);

        <Metadata<T>>::insert(&token_id, meta);

        Ok((tokens, currency_bought))
    }

    fn quote_in_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> Result<Self::TokenBalance, DispatchError> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let tokens_bought = Self::price_sell(currency, meta.quote, meta.token)?;

        Ok(tokens_bought)
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

        let mut meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let tokens_bought = Self::price_sell(currency, meta.quote, meta.token)?;

        ensure!(tokens_bought >= min_tokens, Error::<T>::TooExpensiveTokens);

        T::Currency::transfer(
            &who,
            &meta.pot,
            currency,
            if keep_alive { KeepAlive } else { AllowDeath },
        )?;
        T::Assets::transfer(token_id, &meta.pot, &who, tokens_bought, false)?;

        meta.quote.saturating_accrue(currency);
        meta.token.saturating_reduce(tokens_bought);

        <Metadata<T>>::insert(&token_id, meta);

        Ok((currency, tokens_bought))
    }

    fn quote_out_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> Result<Self::TokenBalance, DispatchError> {
        let meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let tokens_sold = Self::price_buy(currency, meta.token, meta.quote)?;

        Ok(tokens_sold)
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

        let mut meta = <Metadata<T>>::get(&token_id).ok_or(Error::<T>::NotExists)?;

        let tokens_sold = Self::price_buy(currency, meta.token, meta.quote)?;

        ensure!(max_tokens >= tokens_sold, Error::<T>::TooLowTokens);

        T::Assets::transfer(token_id, &who, &meta.pot, tokens_sold, keep_alive)?;
        T::Currency::transfer(&meta.pot, &who, currency, AllowDeath)?;

        meta.quote.saturating_reduce(currency);
        meta.token.saturating_accrue(tokens_sold);

        <Metadata<T>>::insert(&token_id, meta);

        Ok((currency, tokens_sold))
    }
}
