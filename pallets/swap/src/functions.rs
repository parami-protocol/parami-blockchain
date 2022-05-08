use crate::{AssetOf, BalanceOf, Config, Error, Metadata, Pallet, SwapOf};

use frame_support::{
    ensure,
    traits::{tokens::fungibles::Inspect as FungInspect, Currency},
};
use parami_traits::Swaps;
use sp_core::U512;
use sp_runtime::{traits::Zero, DispatchError};

impl<T: Config> Pallet<T> {
    pub(super) fn try_into<S, D>(value: S) -> Result<D, DispatchError>
    where
        S: TryInto<u128>,
        D: TryFrom<u128>,
    {
        let value: u128 = value.try_into().map_err(|_| Error::<T>::Overflow)?;

        let value: D = value.try_into().map_err(|_| Error::<T>::Overflow)?;

        Ok(value)
    }

    /// Calculate how many tokens should be involved
    /// and how many the liquidity should be minted
    pub(super) fn calculate_liquidity(
        token_id: AssetOf<T>,
        currency: BalanceOf<T>,
        max_tokens: BalanceOf<T>,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>, SwapOf<T>), DispatchError> {
        let meta = <Metadata<T>>::get(token_id).ok_or(Error::<T>::NotExists)?;

        let total_liquidity = meta.liquidity;

        if total_liquidity <= Zero::zero() {
            return Ok((max_tokens, currency, meta));
        }

        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let currency: U512 = Self::try_into(currency)?;
        let total_quote: U512 = Self::try_into(total_quote)?;
        let total_token: U512 = Self::try_into(total_token)?;
        let total_liquidity: U512 = Self::try_into(total_liquidity)?;

        let tokens = currency * total_token / total_quote;
        let liquidity = currency * total_liquidity / total_quote;

        let tokens = Self::try_into(tokens)?;
        let liquidity = Self::try_into(liquidity)?;

        Ok((tokens, liquidity, meta))
    }

    /// Calculate how many tokens and currency should be returned
    pub(super) fn calculate_solidness(
        token_id: AssetOf<T>,
        liquidity: BalanceOf<T>,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>, SwapOf<T>), DispatchError> {
        let meta = <Metadata<T>>::get(token_id).ok_or(Error::<T>::NotExists)?;

        let total_liquidity = meta.liquidity;

        ensure!(total_liquidity > Zero::zero(), Error::<T>::NoLiquidity);

        let pot = Self::get_pool_account(token_id);

        let total_quote = T::Currency::free_balance(&pot);
        let total_token = T::Assets::balance(token_id, &pot);

        let liquidity: U512 = Self::try_into(liquidity)?;
        let total_quote: U512 = Self::try_into(total_quote)?;
        let total_token: U512 = Self::try_into(total_token)?;
        let total_liquidity: U512 = Self::try_into(total_liquidity)?;

        let currency = liquidity * total_quote / total_liquidity;
        let tokens = liquidity * total_token / total_liquidity;

        let currency = Self::try_into(currency)?;
        let tokens = Self::try_into(tokens)?;

        Ok((tokens, currency, meta))
    }

    /// Calculate buy price in U512
    pub(self) fn calculate_price_buy(
        output_amount: U512,
        input_reserve: U512,
        output_reserve: U512,
    ) -> U512 {
        let ten_percent = output_reserve / 10;

        if output_amount > ten_percent {
            let d = Self::calculate_price_buy(ten_percent, input_reserve, output_reserve);

            d + Self::calculate_price_buy(
                output_amount - ten_percent,
                input_reserve + d,
                output_reserve - ten_percent,
            )
        } else {
            let numerator = input_reserve * output_amount * U512::from(1000);
            let denominator = (output_reserve - output_amount) * U512::from(997);
            let result = numerator / denominator + U512::from(1);

            result
        }
    }

    /// Calculate sell price in U512
    pub(self) fn calculate_price_sell(
        input_amount: U512,
        input_reserve: U512,
        output_reserve: U512,
    ) -> U512 {
        let ten_percent = input_reserve / 10;

        if input_amount > ten_percent {
            let d = Self::calculate_price_sell(ten_percent, input_reserve, output_reserve);

            d + Self::calculate_price_sell(
                input_amount - ten_percent,
                input_reserve + ten_percent,
                output_reserve - d,
            )
        } else {
            let input_amount_with_fee = input_amount * U512::from(997);
            let numerator = input_amount_with_fee * output_reserve;
            let denominator = (input_reserve * U512::from(1000)) + input_amount_with_fee;
            let result = numerator / denominator;

            result
        }
    }

    /// Calculate buy price with assertion
    pub(super) fn price_buy(
        output_amount: BalanceOf<T>,
        input_reserve: BalanceOf<T>,
        output_reserve: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        ensure!(
            output_reserve > output_amount,
            Error::<T>::InsufficientLiquidity
        );

        let output_amount = Self::try_into(output_amount)?;
        let input_reserve = Self::try_into(input_reserve)?;
        let output_reserve = Self::try_into(output_reserve)?;

        let result = Self::calculate_price_buy(output_amount, input_reserve, output_reserve);

        let result = Self::try_into(result)?;

        Ok(result)
    }

    /// Calculate sell price with assertion
    pub(super) fn price_sell(
        input_amount: BalanceOf<T>,
        input_reserve: BalanceOf<T>,
        output_reserve: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let input_amount = Self::try_into(input_amount)?;
        let input_reserve = Self::try_into(input_reserve)?;
        let output_reserve = Self::try_into(output_reserve)?;

        let result = Self::calculate_price_sell(input_amount, input_reserve, output_reserve);

        ensure!(output_reserve > result, Error::<T>::InsufficientLiquidity);

        let result = Self::try_into(result)?;

        Ok(result)
    }
}
