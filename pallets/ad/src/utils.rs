use crate::*;
use frame_support::dispatch::DispatchError;
use num_traits::pow::Pow;
use parami_primitives::Signature;
use sp_runtime::{traits::AtLeast32Bit, FixedI64};
use sp_std::convert::{TryFrom, TryInto};

#[macro_export]
macro_rules! s {
    ($e: expr) => {
        sp_runtime::SaturatedConversion::saturated_into($e)
    };
}

pub fn sr25519_signature(sign: &[u8]) -> Result<Signature, DispatchError> {
    if let Ok(signature) = sp_core::sr25519::Signature::try_from(sign) {
        Ok(signature.into())
    } else {
        Err(DispatchError::Other("Not a sr25519 signature."))
    }
}

pub fn saturate_score(score: i64) -> i64 {
    if score < 0 {
        0
    } else if score > 100 {
        100
    } else {
        score
    }
}

/// now is duration since unix epoch in millisecond
pub fn now<T: Config>() -> T::Moment {
    pallet_timestamp::Pallet::<T>::now()
}

pub fn decayed_score<Moment: AtLeast32Bit + Copy + Default>(
    old_s: TagScore,
    old_time: Moment,
    now: Moment,
    coefficient: PerU16,
) -> TagScore {
    let days = now
        .saturating_sub(old_time)
        .checked_div(&s!(DAY_MILLION_SECOND))
        .unwrap_or_default();
    let coefficient = coefficient.pow(s!(days));
    let old_s: u16 = old_s
        .try_into()
        .expect("TagScore must greater than or equal to zero.");
    coefficient.mul_ceil(old_s) as TagScore
}

pub fn calc_reward<T: Config>(
    ad: &AdvertisementOf<T>,
    user_did: &DidMethodSpecId,
    user: &AccountIdOf<T>,
    media: &AccountIdOf<T>,
    tag_score_delta: Option<&[TagScore]>,
) -> ResultPost<(Balance, Balance, Balance)> {
    let mut score: FixedI64 = (0, 1).into();
    let now = now::<T>();
    for (i, &(t, c)) in ad.tag_coefficients.iter().enumerate() {
        let old_s = {
            let (old_s, old_time) = <UserTagScores<T>>::get(user_did, t);
            decayed_score::<T::Moment>(old_s, old_time, now, <TimeDecayCoefficient<T>>::get())
        };

        {
            let c: FixedI64 = (c, TAG_DENOMINATOR).into();
            let s: FixedI64 = (old_s, 1).into();
            score = score.saturating_add(c.saturating_mul(s));
        }

        if let Some(tag_score_delta) = tag_score_delta {
            ensure!(
                tag_score_delta[i] <= MAX_TAG_SCORE_DELTA,
                Error::<T>::TagScoreDeltaOutOfRange
            );
            ensure!(
                tag_score_delta[i] >= MIN_TAG_SCORE_DELTA,
                Error::<T>::TagScoreDeltaOutOfRange
            );
            let old_s: i64 = old_s as i64;
            let delta: i64 = tag_score_delta[i] as i64;
            let s = saturate_score(old_s + delta) as TagScore;
            ensure!(s >= 0, Error::<T>::SomethingTerribleHappened);
            <UserTagScores<T>>::insert(&user_did, t, (s, now));
        }
    }

    let reward: Balance = score.saturating_mul_int(UNIT);
    let reward_media = ad.media_reward_rate.mul_ceil(reward);
    let reward_user = reward.saturating_sub(reward_media);

    let srr = <StakingRewardRate<T>>::get();
    let reward_user = srr
        .mul_ceil(staked_balance_by_controller::<T>(user))
        .saturating_add(reward_user);
    let reward_media = srr
        .mul_ceil(staked_balance_by_controller::<T>(media))
        .saturating_add(reward_media);

    Ok((
        reward_media.saturating_add(reward_user),
        reward_media,
        reward_user,
    ))
}

pub fn free_balance<T: Config>(asset_id: T::AssetId, who: AccountIdOf<T>) -> BalanceOfAsset<T> {
    <pallet_assets::Pallet<T>>::balance(asset_id, who)
    // <T as Config>::Currency::free_balance(who)
}

pub fn staked_balance_by_controller<T: Config>(controller: &AccountIdOf<T>) -> Balance {
    // Map from all (unlocked) "controller" accounts to the info regarding the staking.
    s!(pallet_staking::Ledger::<T>::get(controller)
        .map(|l| l.total)
        .unwrap_or_default())
}

#[cfg(any(test, feature = "runtime-benchmarks"))]
pub mod test_helper {
    use crate::*;
    use sp_core::{sr25519::Pair as SrPair, Pair};
    use sp_std::vec::Vec;
    use std::iter::FromIterator;

    #[macro_export]
    macro_rules! d {
        ($who: expr) => {
            parami_did::Pallet::<Runtime>::lookup_account($who.clone()).unwrap()
        };
    }

    pub fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
        frame_system::Pallet::<T>::assert_last_event(generic_event.into());
    }

    pub fn signer<T: Config>(who: AccountIdOf<T>) -> sp_runtime::MultiSigner
    where
        T: frame_system::Config<AccountId = sp_runtime::AccountId32>,
    {
        sp_runtime::MultiSigner::from(sp_core::sr25519::Public(
            std::convert::TryInto::<[u8; 32]>::try_into(who.as_ref()).unwrap(),
        ))
    }

    pub fn reserved_balance<T: Config>(who: &AccountIdOf<T>) -> BalanceOf<T> {
        <T as Config>::Currency::reserved_balance(who)
    }

    pub fn sign<Runtime: Config>(
        signer_pair: SrPair,
        user: Runtime::AccountId,
        media: Runtime::AccountId,
        advertiser: Runtime::AccountId,
        ad_id: AdId,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decayed_score_should_work() {
        let a = decayed_score::<u64>(
            100,
            DAY_MILLION_SECOND,
            DAY_MILLION_SECOND * 3,
            PerU16::from_percent(50),
        );
        assert_eq!(a, (100.0 * 0.5 * 0.5) as i8);
    }
}
