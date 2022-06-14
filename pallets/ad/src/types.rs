use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Metadata<B, D, H, N> {
    pub id: H,
    pub creator: D,
    pub metadata: Vec<u8>,
    pub reward_rate: u16,
    pub created: N,
    pub payout_base: B,
    pub payout_min: B,
    pub payout_max: B,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Slot<Hash, Height, NftId, TokenId, AccountId> {
    pub ad_id: Hash,
    pub nft_id: NftId,
    pub fraction_id: TokenId,
    pub fungible_id: Option<TokenId>,
    // budget pot is specifically for locking budget.
    pub budget_pot: AccountId,
    pub created: Height,
}
