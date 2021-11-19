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
    pub created: H,
}

#[derive(Clone, Decode, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct RawImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl RawImage {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            data,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel(&self, x: u32, y: u32) -> u8 {
        let pos = (y * self.width + x) as usize;

        return self.data[pos];
    }
}

pub type Signature = [u8; 65];
