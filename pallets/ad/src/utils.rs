use sp_std::convert::TryFrom;
use sp_runtime::{FixedI64, DispatchError};

use crate::*;
use parami_primitives::{Signature};

#[macro_export]
macro_rules! s {
	($e: expr) => {
        sp_runtime::SaturatedConversion::saturated_into($e)
	}
}

pub fn sr25519_signature(sign: &[u8]) -> Result<Signature, DispatchError> {
    if let Ok(signature) = sp_core::sr25519::Signature::try_from(sign) {
        Ok(signature.into())
    } else {
        Err(DispatchError::Other("Not a sr25519 signature."))
    }
}

pub fn saturate_score (score: i64) -> i64 {
    if score < 0 {
        0
    } else if score > 100 {
        100
    } else {
        score
    }
}

pub fn calc_reward<T: Config>(
    ad: &AdvertisementOf<T>,
    user_did: &DidMethodSpecId,
    tag_score_delta: Option<&[TagScore]>,
) -> ResultPost<(Balance, Balance, Balance)> {
    let mut score: FixedI64 = (0, 1).into();
    for (i, &(t, c)) in ad.tag_coefficients.iter().enumerate() {
        let c: FixedI64 = (c, TAG_DENOMINATOR).into();

        let old_s = UserTagScores::<T>::get(user_did, t);
        let s: FixedI64 = (old_s, 1).into();
        score = score.saturating_add(c.saturating_mul(s));

        if let Some(tag_score_delta) = tag_score_delta {
            ensure!(tag_score_delta[i] <= MAX_TAG_SCORE_DELTA, Error::<T>::TagScoreDeltaOutOfRange);
            ensure!(tag_score_delta[i] >= MIN_TAG_SCORE_DELTA, Error::<T>::TagScoreDeltaOutOfRange);
            let old_s: i64 = old_s as i64;
            let delta: i64 = tag_score_delta[i] as i64;
            let s = saturate_score(old_s + delta) as TagScore;
            UserTagScores::<T>::insert(&user_did, t, s);
        }
    }

    let reward: Balance = score.saturating_mul_int(UNIT);
    let reward_media = ad.media_reward_rate.mul_ceil(reward);
    let reward_user = reward.saturating_sub(reward_media);

    Ok((reward, reward_media, reward_user))
}

pub fn free_balance<T: Config>(who: &T::AccountId) -> BalanceOf<T> {
    <T as Config>::Currency::free_balance(who)
}

#[cfg(any(test, feature = "runtime-benchmarks"))]
pub mod test_helper {
    use crate::*;
    use std::iter::FromIterator;
    use sp_std::vec::Vec;
    use sp_core::sr25519::Pair as SrPair;
    use sp_core::Pair;

    #[macro_export]
    macro_rules! d {
        ($who: expr) => {
            parami_did::Pallet::<Runtime>::lookup_account($who.clone()).unwrap()
        }
    }

    pub fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
        frame_system::Pallet::<T>::assert_last_event(generic_event.into());
    }

    pub fn signer<T: Config>(who: T::AccountId) -> sp_runtime::MultiSigner
        where T: frame_system::Config<AccountId = sp_runtime::AccountId32>,
    {
        sp_runtime::MultiSigner::from(
            sp_core::sr25519::Public(
                std::convert::TryInto::<[u8; 32]>::try_into(
                    who.as_ref()
                ).unwrap()))
    }

    pub fn reserved_balance<T: Config>(who: &T::AccountId) -> BalanceOf<T> {
        <T as Config>::Currency::reserved_balance(who)
    }

    pub fn sign<Runtime: Config>(
        signer_pair: SrPair, user: Runtime::AccountId,
        media: Runtime::AccountId, advertiser: Runtime::AccountId, ad_id: AdId,
        now: Runtime::Moment,
    ) -> (Vec<u8>, Vec<u8>) {
        let user_did = d!(user);
        let media_did = d!(media);
        let advertiser_did = d!(advertiser);
        let data = codec::Encode::encode(&(user_did, media_did, advertiser_did, now, ad_id));
        let data_sign = Vec::from_iter(signer_pair.sign(data.as_slice()).0);
        (data, data_sign)
    }
}
