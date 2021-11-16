#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_runtime::{
    traits::{MaybeDisplay, MaybeFromStr},
    DispatchError,
};

type ApiResult<T> = Result<T, DispatchError>;

#[derive(Eq, PartialEq, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BalanceWrapper<Balance> {
    pub amount: Balance,
}

#[cfg(feature = "std")]
impl<T> Serialize for BalanceWrapper<T>
where
    T: MaybeDisplay + MaybeFromStr,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.amount.to_string())
    }
}

#[cfg(feature = "std")]
impl<'de, T> Deserialize<'de> for BalanceWrapper<T>
where
    T: MaybeDisplay + MaybeFromStr,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s
            .parse::<T>()
            .map_err(|_| serde::de::Error::custom("Parse from string failed"))?;

        Ok(s.into())
    }
}

impl<T> From<T> for BalanceWrapper<T>
where
    T: MaybeDisplay + MaybeFromStr,
{
    fn from(amount: T) -> Self {
        BalanceWrapper { amount }
    }
}

impl<T> Into<u32> for BalanceWrapper<T>
where
    T: Into<u32>,
{
    fn into(self) -> u32 {
        self.amount.into()
    }
}

impl<T> Into<u64> for BalanceWrapper<T>
where
    T: Into<u64>,
{
    fn into(self) -> u64 {
        self.amount.into()
    }
}

impl<T> Into<u128> for BalanceWrapper<T>
where
    T: Into<u128>,
{
    fn into(self) -> u128 {
        self.amount.into()
    }
}

sp_api::decl_runtime_apis! {
    pub trait SwapRuntimeApi<AssetId, Balance>
    where
        AssetId: Codec,
        Balance: Codec + MaybeDisplay + MaybeFromStr,
    {
        /// Get dry-run result of add_liquidity
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be involved in the swap
        /// * `max_tokens` - The maximum amount of tokens to be involved in the swap
        ///
        /// # Results
        ///
        /// tuple of (token_id, tokens, lp_token_id, liquidity)
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be involved in the swap
        /// * `lp_token_id` - The Asset ID of the liquidity provider token
        /// * `liquidity` - The amount of liquidity to be minted
        fn dryly_add_liquidity(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
            max_tokens: BalanceWrapper<Balance>,
        ) -> ApiResult<(
            AssetId,
            BalanceWrapper<Balance>,
            AssetId,
            BalanceWrapper<Balance>,
        )>;

        /// Get dry-run result of remove_liquidity
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `liquidity` - The amount of liquidity to be removed
        ///
        /// # Results
        ///
        /// tuple of (token_id, tokens, lp_token_id, currency)
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be returned
        /// * `lp_token_id` - The Asset ID of the liquidity provider token
        /// * `currency` - The currency to be returned
        fn dryly_remove_liquidity(
            token_id: AssetId,
            liquidity: BalanceWrapper<Balance>,
        ) -> ApiResult<(
            AssetId,
            BalanceWrapper<Balance>,
            AssetId,
            BalanceWrapper<Balance>,
        )>;

        /// Get dry-run result of buy_tokens
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be bought
        ///
        /// # Results
        ///
        /// The currency needed
        fn dryly_buy_tokens(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> ApiResult<BalanceWrapper<Balance>>;

        /// Get dry-run result of sell_tokens
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `tokens` - The amount of tokens to be sold
        ///
        /// # Results
        ///
        /// The currency to be gained
        fn dryly_sell_tokens(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> ApiResult<BalanceWrapper<Balance>>;

        /// Get dry-run result of sell_currency
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be sold
        ///
        /// # Results
        ///
        /// The amount of tokens to be gained
        fn dryly_sell_currency(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> ApiResult<BalanceWrapper<Balance>>;

        /// Get dry-run result of buy_currency
        ///
        /// # Arguments
        ///
        /// * `token_id` - The Asset ID
        /// * `currency` - The currency to be bought
        ///
        /// # Results
        ///
        /// The amount of tokens needed
        fn dryly_buy_currency(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> ApiResult<BalanceWrapper<Balance>>;
    }
}
