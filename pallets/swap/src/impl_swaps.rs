use crate::{
    types, Account, AccountOf, AssetOf, BalanceOf, Config, Error, Event, HeightOf, Liquidity,
    Metadata, NextTokenId, Pallet, Provider,
};

use frame_support::{
    ensure,
    traits::{
        tokens::fungibles::{Inspect as FungInspect, Transfer as FungTransfer},
        Currency,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        Get,
    },
};
use parami_traits::Swaps;
use sp_runtime::traits::{AccountIdConversion, CheckedAdd, One, Saturating, Zero};
use sp_std::boxed::Box;

type DispatchResult<T> = Result<T, sp_runtime::DispatchError>;

impl<T: Config> Swaps<AccountOf<T>> for Pallet<T> {
    type AssetId = AssetOf<T>;
    type QuoteBalance = BalanceOf<T>;
    type TokenBalance = BalanceOf<T>;

    fn iter() -> Box<dyn Iterator<Item = Self::AssetId>> {
        Box::new(<Metadata<T>>::iter_keys())
    }

    fn iter_providers(token_id: Self::AssetId) -> Box<dyn Iterator<Item = AccountOf<T>>> {
        Box::new(<Provider<T>>::iter_key_prefix(token_id))
    }

    fn total_liquidity(token_id: Self::AssetId) -> Self::TokenBalance {
        <Metadata<T>>::get(token_id)
            .map(|meta| meta.liquidity)
            .unwrap_or_default()
    }

    fn liquidity(token_id: Self::AssetId, who: &AccountOf<T>) -> Self::TokenBalance {
        <Provider<T>>::get(token_id, who)
    }

    fn new(token_id: Self::AssetId) -> DispatchResult<()> {
        ensure!(!<Metadata<T>>::contains_key(token_id), Error::<T>::Exists);

        let created = <frame_system::Pallet<T>>::block_number();

        <Metadata<T>>::insert(
            token_id,
            types::Swap {
                created,
                liquidity: Zero::zero(),
            },
        );

        Self::deposit_event(Event::Created(token_id));

        Ok(())
    }

    fn get_pool_account(token_id: Self::AssetId) -> AccountOf<T> {
        T::PalletId::get().into_sub_account_truncating(token_id)
    }

    fn mint_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
    ) -> DispatchResult<(Self::TokenBalance, Self::TokenBalance)> {
        let (tokens, liquidity, _) = Self::calculate_liquidity(token_id, currency, max_tokens)?;

        Ok((tokens, liquidity))
    }

    fn mint(
        who: AccountOf<T>,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_liquidity: Self::TokenBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> DispatchResult<(Self::TokenBalance, Self::TokenBalance)> {
        ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);
        ensure!(min_liquidity > Zero::zero(), Error::<T>::ZeroLiquidity);
        ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);

        let (tokens, liquidity, mut meta) =
            Self::calculate_liquidity(token_id, currency, max_tokens)?;

        ensure!(max_tokens >= tokens, Error::<T>::TooExpensiveCurrency);
        ensure!(liquidity >= min_liquidity, Error::<T>::TooLowLiquidity);

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

        let pot = Self::get_pool_account(token_id);

        T::Currency::transfer(
            &who,
            &pot,
            currency,
            if keep_alive { KeepAlive } else { AllowDeath },
        )?;
        T::Assets::transfer(token_id, &who, &pot, tokens, false)?;

        let minted = <frame_system::Pallet<T>>::block_number();

        let lp_token_id = <NextTokenId<T>>::try_mutate(|id| -> DispatchResult<AssetOf<T>> {
            let current_id = *id;
            *id = id.checked_add(&One::one()).ok_or(Error::<T>::Overflow)?;
            Ok(current_id)
        })?;
        <Liquidity<T>>::insert(
            lp_token_id,
            types::Liquidity {
                owner: who.clone(),
                token_id,
                amount: liquidity,
                minted,
            },
        );
        <Account<T>>::insert(&who, lp_token_id, HeightOf::<T>::zero());

        <Provider<T>>::mutate(token_id, &who, |holding| {
            holding.saturating_accrue(liquidity);
        });

        meta.liquidity.saturating_accrue(liquidity);
        <Metadata<T>>::insert(token_id, meta);

        Self::deposit_event(Event::LiquidityAdded(
            token_id, who, liquidity, currency, tokens,
        ));

        Ok((tokens, liquidity))
    }

    fn burn_dry(
        lp_token_id: Self::AssetId,
    ) -> DispatchResult<(
        Self::AssetId,
        Self::TokenBalance,
        Self::TokenBalance,
        Self::QuoteBalance,
    )> {
        let liquidity = <Liquidity<T>>::get(lp_token_id).ok_or(Error::<T>::NotExists)?;

        let (tokens, currency, _) =
            Self::calculate_solidness(liquidity.token_id, liquidity.amount)?;

        Ok((liquidity.token_id, liquidity.amount, tokens, currency))
    }

    fn burn(
        who: AccountOf<T>,
        lp_token_id: Self::AssetId,
        min_currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
    ) -> DispatchResult<(
        Self::AssetId,
        Self::TokenBalance,
        Self::TokenBalance,
        Self::QuoteBalance,
    )> {
        let liquidity = <Liquidity<T>>::get(lp_token_id).ok_or(Error::<T>::NotExists)?;

        let (tokens, currency, mut meta) =
            Self::calculate_solidness(liquidity.token_id, liquidity.amount)?;

        ensure!(currency >= min_currency, Error::<T>::TooLowCurrency);
        ensure!(tokens >= min_tokens, Error::<T>::TooLowTokens);

        <Liquidity<T>>::remove(lp_token_id);
        <Account<T>>::remove(&who, lp_token_id);

        let mut holding = <Provider<T>>::get(liquidity.token_id, &who);
        if holding <= liquidity.amount {
            <Provider<T>>::remove(liquidity.token_id, &who);
        } else {
            holding.saturating_reduce(liquidity.amount);

            <Provider<T>>::insert(liquidity.token_id, &who, holding);
        }

        meta.liquidity.saturating_reduce(liquidity.amount);
        <Metadata<T>>::insert(liquidity.token_id, meta);

        let pot = Self::get_pool_account(liquidity.token_id);

        T::Assets::transfer(liquidity.token_id, &pot, &who, tokens, false)?;
        T::Currency::transfer(&pot, &who, currency, AllowDeath)?;

        Self::deposit_event(Event::LiquidityRemoved(
            liquidity.token_id,
            who,
            liquidity.amount,
            currency,
            tokens,
        ));

        Ok((liquidity.token_id, liquidity.amount, tokens, currency))
    }

    fn token_out_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> DispatchResult<Self::QuoteBalance> {
        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let currency_sold = Self::price_buy(tokens, total_quote, total_token)?;

        Ok(currency_sold)
    }

    fn token_out(
        who: AccountOf<T>,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        max_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> DispatchResult<Self::QuoteBalance> {
        ensure!(tokens > Zero::zero(), Error::<T>::ZeroTokens);
        ensure!(max_currency > Zero::zero(), Error::<T>::ZeroCurrency);

        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let currency_sold = Self::price_buy(tokens, total_quote, total_token)?;

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

        Self::deposit_event(Event::TokenBought(token_id, who, tokens, currency_sold));

        Ok(currency_sold)
    }

    fn token_in_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> DispatchResult<Self::QuoteBalance> {
        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let currency_bought = Self::price_sell(tokens, total_token, total_quote)?;

        Ok(currency_bought)
    }

    fn token_in(
        who: AccountOf<T>,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> DispatchResult<Self::QuoteBalance> {
        ensure!(tokens > Zero::zero(), Error::<T>::ZeroTokens);
        ensure!(min_currency > Zero::zero(), Error::<T>::ZeroCurrency);

        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let currency_bought = Self::price_sell(tokens, total_token, total_quote)?;

        ensure!(currency_bought >= min_currency, Error::<T>::TooLowCurrency);

        T::Assets::transfer(token_id, &who, &pot, tokens, keep_alive)?;
        T::Currency::transfer(&pot, &who, currency_bought, AllowDeath)?;

        Self::deposit_event(Event::TokenSold(token_id, who, tokens, currency_bought));

        Ok(currency_bought)
    }

    fn quote_in_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> DispatchResult<Self::TokenBalance> {
        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let tokens_bought = Self::price_sell(currency, total_quote, total_token)?;

        Ok(tokens_bought)
    }

    fn quote_in(
        who: AccountOf<T>,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> DispatchResult<Self::TokenBalance> {
        ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);
        ensure!(min_tokens > Zero::zero(), Error::<T>::ZeroTokens);

        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let tokens_bought = Self::price_sell(currency, total_quote, total_token)?;

        ensure!(tokens_bought >= min_tokens, Error::<T>::TooExpensiveTokens);

        T::Currency::transfer(
            &who,
            &pot,
            currency,
            if keep_alive { KeepAlive } else { AllowDeath },
        )?;
        T::Assets::transfer(token_id, &pot, &who, tokens_bought, false)?;

        Self::deposit_event(Event::TokenBought(token_id, who, tokens_bought, currency));

        Ok(tokens_bought)
    }

    fn quote_out_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> DispatchResult<Self::TokenBalance> {
        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let tokens_sold = Self::price_buy(currency, total_token, total_quote)?;

        Ok(tokens_sold)
    }

    fn quote_out(
        who: AccountOf<T>,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> DispatchResult<Self::TokenBalance> {
        ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);
        ensure!(currency > Zero::zero(), Error::<T>::ZeroCurrency);

        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let tokens_sold = Self::price_buy(currency, total_token, total_quote)?;

        ensure!(max_tokens >= tokens_sold, Error::<T>::TooLowTokens);

        T::Assets::transfer(token_id, &who, &pot, tokens_sold, keep_alive)?;
        T::Currency::transfer(&pot, &who, currency, AllowDeath)?;

        Self::deposit_event(Event::TokenSold(token_id, who, tokens_sold, currency));

        Ok(tokens_sold)
    }
}
