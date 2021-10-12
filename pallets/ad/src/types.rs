use codec::{Decode, Encode};
use parami_primitives::Balance;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{PerU16, RuntimeDebug};
use sp_std::{marker::PhantomData, prelude::*};

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
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

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
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
    pub metadata: Vec<u8>,
}

pub struct TagScoreDefault<T>(PhantomData<T>);

impl<T: crate::pallet::Config> frame_support::traits::Get<(TagScore, T::Moment)>
    for TagScoreDefault<T>
{
    #[cfg(test)]
    fn get() -> (TagScore, T::Moment) {
        (50, Default::default())
    }
    #[cfg(not(test))]
    fn get() -> (TagScore, T::Moment) {
        (0, Default::default())
    }
}

pub type TagType = u8;
pub type TagScore = i8;
pub type TagCoefficient = u8;
pub type GlobalId = u64;
pub type AdvertiserId = GlobalId;
pub type AdId = GlobalId;
