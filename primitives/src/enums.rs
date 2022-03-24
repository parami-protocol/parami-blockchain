use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[derive(Clone, Copy, Decode, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Network {
    Unknown = 0,

    // crypto networks id start from 0x10
    /// BSC
    Binance = 0x10,
    /// BTC
    Bitcoin = 0x11,
    /// EOS
    Eosio = 0x12,
    /// ETH
    Ethereum = 0x13,
    /// KSM
    Kusama = 0x14,
    /// DOT
    Polkadot = 0x15,
    /// SOL
    Solana = 0x16,
    /// TRX
    Tron = 0x17,
    /// NEAR
    Near = 0x18,

    // social networks id start from 0x80
    /// Discord
    Discord = 0x80,
    /// Facebook
    Facebook = 0x81,
    /// Github
    Github = 0x82,
    /// Hacker News
    HackerNews = 0x83,
    /// Mastodon
    Mastodon = 0x84,
    /// Reddit
    Reddit = 0x85,
    /// Telegram
    Telegram = 0x86,
    /// Twitter
    Twitter = 0x87,
}

impl Default for Network {
    fn default() -> Self {
        Network::Unknown
    }
}
