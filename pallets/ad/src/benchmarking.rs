use super::*;

#[allow(unused)]
use crate::Pallet as Ad;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use sp_runtime::traits::{Bounded, Saturating};

benchmarks! {
    where_clause {
        where
        T: parami_advertiser::Config,
        T: parami_did::Config,
        T: parami_tag::Config
    }

    create {
        let n in 1 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = BalanceOf::<T>::min_value();

        let pot = <T as parami_advertiser::Config>::Currency::minimum_balance().saturating_mul(1_000_000u32.into());

        <T as crate::Config>::Currency::make_free_balance_be(&caller, max);
        parami_did::Pallet::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        parami_advertiser::Pallet::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        let mut rng = SmallRng::from_seed(Default::default());
        let mut tags = vec![];
        for i in 0..n {
            let name: Vec<u8> = (0..512).map(|_| { rng.gen() }).collect();
            parami_tag::Pallet::<T>::create(RawOrigin::Signed(caller.clone()).into(), name.clone())?;
            tags.push(name);
        }
    }: _(RawOrigin::Signed(caller), min, tags, Default::default(), 1, <MomentOf<T>>::max_value())
    verify {
        assert_ne!(<Metadata<T>>::iter_values().next(), None);
    }

    update_reward_rate {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = BalanceOf::<T>::min_value();

        let pot = <T as parami_advertiser::Config>::Currency::minimum_balance().saturating_mul(1_000_000u32.into());

        <T as crate::Config>::Currency::make_free_balance_be(&caller, max);
        parami_did::Pallet::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        parami_advertiser::Pallet::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(RawOrigin::Signed(caller.clone()).into(), min, vec![], Default::default(), 1, <MomentOf<T>>::max_value())?;

        let ad = <Metadata<T>>::iter_keys().next().unwrap();
    }: _(RawOrigin::Signed(caller.clone()), ad, 100)
    verify {
        let ad = <Metadata<T>>::get(&ad).unwrap();
        assert_eq!(ad.reward_rate, 100);
    }

    update_tags {
        let n in 1 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = BalanceOf::<T>::min_value();

        let pot = <T as parami_advertiser::Config>::Currency::minimum_balance().saturating_mul(1_000_000u32.into());

        <T as crate::Config>::Currency::make_free_balance_be(&caller, max);
        parami_did::Pallet::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        parami_advertiser::Pallet::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(RawOrigin::Signed(caller.clone()).into(), min, vec![], Default::default(), 1, <MomentOf<T>>::max_value())?;

        let ad = <Metadata<T>>::iter_keys().next().unwrap();

        let mut rng = SmallRng::from_seed(Default::default());
        let mut tags = vec![];
        for i in 0..n {
            let name: Vec<u8> = (0..512).map(|_| { rng.gen() }).collect();
            parami_tag::Pallet::<T>::create(RawOrigin::Signed(caller.clone()).into(), name.clone())?;
            tags.push(name);
        }
    }: _(RawOrigin::Signed(caller.clone()), ad, tags)
    verify {
        assert_ne!(T::TagsStore::get(&ad), Vec::<Vec<u8>>::default());
    }
}

impl_benchmark_test_suite!(Tag, crate::mock::new_test_ext(), crate::mock::Test);
