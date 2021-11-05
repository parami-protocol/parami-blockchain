#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_runtime::traits::{MaybeDisplay, MaybeFromStr};

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
        fn dryly_mint(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
            tokens: BalanceWrapper<Balance>,
        ) -> Option<(
            AssetId,
            BalanceWrapper<Balance>,
            AssetId,
            BalanceWrapper<Balance>,
        )>;

        fn dryly_burn(
            token_id: AssetId,
            liquidity: BalanceWrapper<Balance>,
        ) -> Option<(
            AssetId,
            BalanceWrapper<Balance>,
            AssetId,
            BalanceWrapper<Balance>,
        )>;

        fn dryly_token_out(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> Option<BalanceWrapper<Balance>>;

        fn dryly_token_in(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> Option<BalanceWrapper<Balance>>;

        fn dryly_quote_in(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> Option<BalanceWrapper<Balance>>;

        fn dryly_quote_out(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> Option<BalanceWrapper<Balance>>;
    }
}
