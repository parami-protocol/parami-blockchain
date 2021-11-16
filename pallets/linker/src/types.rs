use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Decode, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AccountType {
    /// Unknown account type
    Unknown,

    /// BTC Address
    Bitcoin,
    /// ETH Address
    Ethereum,
    /// EOS Address
    Eosio,
    /// SOL Address
    Solana,
    /// Substrate Address on the Kusama (KSM) Network
    Kusama,
    /// Substrate Address on the Polkadot (DOT) Network
    Polkadot,

    /// Telegram Profile
    Telegram,
    /// Discord Profile
    Discord,
    /// Facebook Profile
    Facebook,
    /// Mastodon Profile
    Mastodon,
    /// Twitter Profile
    Twitter,
    /// Github Profile
    Github,
    /// Hacker News Profile
    HackerNews,
    /// Reddit Profile
    Reddit,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Unknown
    }
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Pending<H> {
    pub profile: Vec<u8>,
    pub deadline: H,
}

pub type Signature = [u8; 65];
