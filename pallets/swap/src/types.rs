use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Swap<N, B> {
    pub created: N,
    pub liquidity: B,
    pub initial_quote: B,
    pub farming_reward_quote: B,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Liquidity<A, B, N, T> {
    pub owner: A,
    pub token_id: T,
    #[codec(compact)]
    pub amount: B,
    pub minted: N,
}
