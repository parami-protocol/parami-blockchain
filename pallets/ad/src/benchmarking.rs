use super::*;

#[allow(unused)]
use crate::Pallet as Ad;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use parami_advertiser::Pallet as Advertiser;
use parami_did::Pallet as Did;
use parami_nft::Pallet as Nft;
use parami_tag::Pallet as Tag;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use sp_runtime::traits::{Bounded, Saturating};

benchmarks! {
    where_clause {
        where
        T: parami_advertiser::Config,
        T: parami_did::Config,
        T: parami_nft::Config,
        T: parami_tag::Config
    }

    create {
        let n in 0 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = <T as parami_did::Config>::Currency::minimum_balance();

        let pot = <T as parami_did::Config>::Currency::minimum_balance().saturating_mul(1_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, max);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        let mut tags = vec![];
        if n > 0 {
            let mut rng = SmallRng::from_seed(Default::default());

            for i in 0..n {
                let name: Vec<u8> = (0..50).map(|_| rng.gen()).collect();
                Tag::<T>::create(RawOrigin::Signed(caller.clone()).into(), name.clone())?;
                tags.push(name);
            }
        }
    }: _(RawOrigin::Signed(caller), min, tags, Default::default(), 1, HeightOf::<T>::max_value())
    verify {
        assert_ne!(Metadata::<T>::iter_values().next(), None);
    }

    update_reward_rate {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = <T as parami_did::Config>::Currency::minimum_balance();

        let pot = <T as parami_did::Config>::Currency::minimum_balance().saturating_mul(1_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, max);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            min,
            vec![],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
        )?;

        let ad = Metadata::<T>::iter_keys().next().unwrap();
    }: _(RawOrigin::Signed(caller), ad, 100)
    verify {
        let ad = Metadata::<T>::get(&ad).unwrap();
        assert_eq!(ad.reward_rate, 100);
    }

    update_tags {
        let n in 0 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = <T as parami_did::Config>::Currency::minimum_balance();

        let pot = <T as parami_did::Config>::Currency::minimum_balance().saturating_mul(1_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, max);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        let mut tags = vec![];
        if n > 0 {
            let mut rng = SmallRng::from_seed(Default::default());

            for i in 0..n {
                let name: Vec<u8> = (0..50).map(|_| rng.gen()).collect();
                Tag::<T>::create(RawOrigin::Signed(caller.clone()).into(), name.clone())?;
                tags.push(name);
            }
        }

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            min,
            vec![],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
        )?;

        let ad = Metadata::<T>::iter_keys().next().unwrap();
    }: _(RawOrigin::Signed(caller), ad, tags.clone())
    verify {
        assert_eq!(Tag::<T>::tags_of(&ad).len(), tags.len());
    }

    bid {
        let caller: T::AccountId = whitelisted_caller();

        let kol: T::AccountId = account::<T::AccountId>("kol", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, max);
        <T as parami_did::Config>::Currency::make_free_balance_be(&kol, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;
        let did = Did::<T>::did_of(&kol).unwrap();

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            pot,
            vec![],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
        )?;
        let ad = Metadata::<T>::iter_keys().next().unwrap();

        Nft::<T>::back(
            RawOrigin::Signed(caller.clone()).into(),
            did,
            pot.saturating_mul(2u32.into()),
        )?;

        Nft::<T>::mint(
            RawOrigin::Signed(kol).into(),
            b"Test Token".to_vec(),
            b"XTT".to_vec(),
        )?;
    }: _(RawOrigin::Signed(caller.clone()), ad, did, pot)
    verify {
        assert_ne!(SlotOf::<T>::get(&did), None);
    }

    add_budget {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = <T as parami_did::Config>::Currency::minimum_balance();

        let pot = <T as parami_did::Config>::Currency::minimum_balance().saturating_mul(1_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, max);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            pot,
            vec![],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
        )?;

        let ad = Metadata::<T>::iter_keys().next().unwrap();
    }: _(RawOrigin::Signed(caller.clone()), ad, pot)
    verify {
        let meta = Metadata::<T>::get(&ad).unwrap();
        assert_eq!(<T as parami_did::Config>::Currency::free_balance(&meta.pot), pot.saturating_mul(2u32.into()));
    }

    pay {
        let n in 1 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let kol: T::AccountId = account::<T::AccountId>("kol", 1, 1);

        let visitor: T::AccountId = account::<T::AccountId>("visitor", 2, 2);

        let max = BalanceOf::<T>::max_value();
        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, max);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        <T as parami_did::Config>::Currency::make_free_balance_be(&kol, pot);
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;
        let slot = Did::<T>::did_of(&kol).unwrap();

        <T as parami_did::Config>::Currency::make_free_balance_be(&visitor, pot);
        Did::<T>::register(RawOrigin::Signed(visitor.clone()).into(), None)?;
        let visitor = Did::<T>::did_of(&visitor).unwrap();

        let mut tags = vec![];
        let mut scores = vec![];
        if n > 0 {
            let mut rng = SmallRng::from_seed(Default::default());

            for i in 0..n {
                let name: Vec<u8> = (0..50).map(|_| rng.gen()).collect();
                Tag::<T>::create(RawOrigin::Signed(caller.clone()).into(), name.clone())?;
                tags.push(name.clone());
                scores.push((name, 5))
            }
        }

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            pot,
            tags,
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
        )?;
        let ad = Metadata::<T>::iter_keys().next().unwrap();

        Nft::<T>::back(
            RawOrigin::Signed(caller.clone()).into(),
            slot,
            pot.saturating_mul(2u32.into()),
        )?;

        Nft::<T>::mint(
            RawOrigin::Signed(kol).into(),
            b"Test Token".to_vec(),
            b"XTT".to_vec(),
        )?;

        Ad::<T>::bid(RawOrigin::Signed(caller.clone()).into(), ad, slot, pot)?;
    }: _(RawOrigin::Signed(caller.clone()), ad, slot, visitor, scores, None)
    verify {
        use frame_support::traits::fungibles::Inspect;

        let slot = Did::<T>::meta(&slot).unwrap();
        let nft = slot.nft.unwrap();

        let visitor: T::AccountId = account::<T::AccountId>("visitor", 2, 2);

        assert!(<T as parami_nft::Config>::Assets::balance(nft, &visitor) > min.into());
    }
}

impl_benchmark_test_suite!(Tag, crate::mock::new_test_ext(), crate::mock::Test);
