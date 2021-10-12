use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StableAccount<Moment, AccountId> {
    #[codec(compact)]
    pub created_time: Moment,
    pub stash_account: AccountId,
    pub controller_account: AccountId,
    pub magic_account: AccountId,
    pub new_controller_account: Option<AccountId>,
}
