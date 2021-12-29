//! Various basic types for use in the assets pallet

use super::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

pub type NftIdOf<T> = <T as parami_did::Config>::AssetId;
pub(super) type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
pub(super) type AccountOf<T> = <T as frame_system::Config>::AccountId;
pub type NftMetaFor<T> = NftMeta<DidOf<T>, AccountOf<T>, NftIdOf<T>, <T as parami_did::Config>::AssetId>;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct NftMeta<Did, AccountId, NftClassId, AssetId> {
    pub(super) owner: Did,
    pub(super) pot: AccountId,
    pub(super) class_id: NftClassId,
    pub minted: bool,
    pub token_asset_id: AssetId,
}