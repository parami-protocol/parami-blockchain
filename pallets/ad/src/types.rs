use crate::*;

#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
use parami_primitives::{Balance};
use sp_std::vec::Vec;
use sp_runtime::PerU16;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Advertiser<Moment, AccountId> {
    /// creation time.
    #[codec(compact)]
    pub created_time: Moment,
    /// advertiser id
    #[codec(compact)]
    pub advertiser_id: AdvertiserId,
    /// The minimum balances to create an advertiser account.
    #[codec(compact)]
    pub deposit: Balance,
    /// an account to keep the deposit of an advertiser.
    pub deposit_account: AccountId,
    /// an account to keep the reward pool balances of an advertiser.
    pub reward_pool_account: AccountId,
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Advertisement<Moment, AccountId> {
    /// creation time.
    #[codec(compact)]
    pub created_time: Moment,
    /// The minimum balances to create an advertiser account.
    #[codec(compact)]
    pub deposit: Balance,
    /// coefficients for calculating ad rewards.
    pub tag_coefficients: Vec<(TagType, TagCoefficient)>,
    /// should be used to sign an ad.
    pub signer: AccountId,
    /// a part of ad reward will be sent to media DID.
    #[codec(compact)]
    pub media_reward_rate: PerU16,
}

pub struct TagScoreDefault;
impl frame_support::traits::Get<TagScore> for TagScoreDefault {
    fn get() -> TagScore {
        50
    }
}

pub type AdvertiserOf<T> = Advertiser<<T as pallet_timestamp::Config>::Moment, <T as frame_system::Config>::AccountId>;
pub type AdvertisementOf<T> = Advertisement<<T as pallet_timestamp::Config>::Moment, <T as frame_system::Config>::AccountId>;
pub type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type ResultPost<T> = sp_std::result::Result<T, DispatchErrorWithPostInfo<PostDispatchInfo>>;
pub type TagType = u8;
pub type TagScore = i8;
pub type TagCoefficient = u8;
pub type GlobalId = u64;
pub type AdvertiserId = GlobalId;
pub type AdId = GlobalId;
