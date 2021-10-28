use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Metadata<B, D, M> {
    pub creator: D,
    pub budget: B,
    pub remain: B,
    pub metadata: sp_core::H512,
    pub reward_rate: u16,
    #[codec(compact)]
    pub created: M,
    #[codec(compact)]
    pub deadline: M,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Slot<B, D, M> {
    pub budget: B,
    pub remain: B,
    #[codec(compact)]
    pub deadline: M,
    pub ad: D,
}
