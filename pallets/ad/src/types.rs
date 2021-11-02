use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Metadata<A, B, D, H, N> {
    pub id: H,
    pub creator: D,
    pub pot: A,
    #[codec(compact)]
    pub budget: B,
    #[codec(compact)]
    pub remain: B,
    pub metadata: Vec<u8>,
    pub reward_rate: u16,
    pub deadline: N,
    pub created: N,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Slot<B, H, N> {
    #[codec(compact)]
    pub budget: B,
    #[codec(compact)]
    pub remain: B,
    pub deadline: N,
    pub ad: H,
}
