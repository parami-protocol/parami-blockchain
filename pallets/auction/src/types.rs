use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

/// Auction ID
pub type AuctionId = u64;

#[derive(Clone, Copy, Decode, Encode, Eq, Ord, PartialEq, PartialOrd, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum AuctionType {
    Auction,
    BuyNow,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AuctionInfo<AccountId, Balance, BlockNumber> {
    /// Current bidder and bid price.
    pub bid: Option<(AccountId, Balance)>,
    /// Define which block this auction will be started.
    pub start: BlockNumber,
    /// Define which block this auction will be ended.
    pub end: Option<BlockNumber>,
}

#[derive(Clone, Decode, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AuctionItem<AccountId, BlockNumber, Balance, AssetId> {
    pub item_id: ItemId<AssetId>,
    pub recipient: AccountId,
    pub initial_amount: Balance,
    pub amount: Balance,
    pub start_time: BlockNumber,
    pub end_time: BlockNumber,
    pub auction_type: AuctionType,
}

/// Public item id for auction
#[derive(Clone, Copy, Decode, Encode, Eq, Ord, PartialEq, PartialOrd, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ItemId<AssetId> {
    NFT(AssetId),
    Block(u64),
}
