//! Various basic types for use in the assets pallet

use super::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

pub(super) type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
pub(super) type AccountOf<T> = <T as frame_system::Config>::AccountId;
pub(super) type NftInstanceId<T> = <T as parami_did::Config>::AssetId;
pub(super) type NftClassIdOf<T> = <T as parami_did::Config>::AssetId;
pub(super) type NftMetaFor<T> = NftMeta<DidOf<T>, AccountOf<T>, NftClassIdOf<T>>;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub(super) struct NftMeta<Did, AccountId, NftClassId> {
    pub(super) owner: Did,
    pub(super) pot: AccountId,
    pub(super) class_id: NftClassId,
    pub(super) minted: bool,
}