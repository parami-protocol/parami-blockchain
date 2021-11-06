use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Metadata<A, N, T> {
    pub account: A,
    pub pot: A,
    pub nft: Option<T>,
    pub avatar: Vec<u8>,
    pub nickname: Vec<u8>,
    pub revoked: bool,
    pub created: N,
}
