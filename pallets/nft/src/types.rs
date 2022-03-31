//! Various basic types for use in the assets pallet

use super::*;
use codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct External<Did> {
    pub owner: Did,
    pub network: Network,
    pub namespace: Vec<u8>,
    pub token: Vec<u8>,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
pub struct Metadata<Did, AccountId, NftClassId, AssetId, Balance> {
    pub owner: Did,
    pub pot: AccountId,
    pub class_id: NftClassId,
    pub minted: bool,
    pub token_asset_id: AssetId,
    pub swap_init_quote_reservation: Balance, // farming initial quote value
    pub back_up_reservation: Balance,
    pub own_reservation: Balance,
    pub farming_reward_reservation: Balance, // reserved farming reward value
}
