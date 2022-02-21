use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[derive(Clone, Copy, Decode, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum Releases {
    V0,
    V1,
}

impl Default for Releases {
    fn default() -> Self {
        Releases::V1
    }
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StableAccount<A, N> {
    pub stash_account: A,
    pub controller_account: A,
    pub magic_account: A,
    pub created: N,
}
