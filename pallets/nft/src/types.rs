//! Various basic types for use in the assets pallet

use super::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct NftMeta<Did, AccountId, NftClassId, AssetId> {
    pub(super) owner: Did,
    pub(super) pot: AccountId,
    pub(super) class_id: NftClassId,
    pub minted: bool,
    pub token_asset_id: AssetId,
}
