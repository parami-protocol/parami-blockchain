use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Swap<A, B, N, T> {
    #[codec(compact)]
    pub token_id: T,
    #[codec(compact)]
    pub lp_token_id: T,
    pub pot: A,
    pub quote: B,
    pub token: B,
    pub created: N,
}
