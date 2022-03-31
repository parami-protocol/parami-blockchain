#![cfg_attr(not(feature = "std"), no_std)]

pub use enums::*;

pub mod constants;
pub use constants::*;
mod enums;
pub mod names;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sp_runtime::{
    generic,
    traits::{BlakeTwo256, IdentifyAccount, MaybeDisplay, MaybeFromStr, Verify},
    MultiAddress, MultiSignature, RuntimeDebug,
};

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

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Task<T, H> {
    pub task: T,
    pub deadline: H,
    pub created: H,
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// ID of an asset.
pub type AssetId = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Expressing timestamp.
pub type Moment = u64;

/// DID of an account.
pub type DecentralizedId = sp_core::H160;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
    use constants::CENTS;

    (items as Balance) * 15 * CENTS + (bytes as Balance) * 6 * CENTS
}
