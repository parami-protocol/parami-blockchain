use super::*;

#[allow(unused)]
use crate::Pallet as Ad;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use parami_advertiser::Pallet as Advertiser;
use parami_did::Pallet as Did;
use parami_nft::Pallet as Nft;
use parami_tag::Pallet as Tag;
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
        // TODO: add back variables

        let caller: T::AccountId = whitelisted_caller();

        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Tag::<T>::force_create(RawOrigin::Root.into(), vec![1u8; 6])?;
    }: _(RawOrigin::Signed(caller), min, vec![vec![1u8; 6]], vec![0u8; 500], 1, HeightOf::<T>::max_value(), min, min, pot)
    verify {
        assert_ne!(<Metadata<T>>::iter_values().next(), None);
    }

    update_reward_rate {
        let caller: T::AccountId = whitelisted_caller();

        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            min,
            vec![],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
            min,
            min,
            pot,
        )?;

        let ad = <Metadata<T>>::iter_keys().next().unwrap();
    }: _(RawOrigin::Signed(caller), ad, 100)
    verify {
        let ad = <Metadata<T>>::get(&ad).unwrap();
        assert_eq!(ad.reward_rate, 100);
    }

    update_tags {
        Tag::<T>::force_create(RawOrigin::Root.into(), vec![1u8; 6])?;

        let caller: T::AccountId = whitelisted_caller();

        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            min,
            vec![],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
            min,
            min,
            pot,
        )?;

        let ad = <Metadata<T>>::iter_keys().next().unwrap();
    }: _(RawOrigin::Signed(caller), ad, vec![vec![1u8; 6]])
    verify {
        assert_eq!(Tag::<T>::tags_of(&ad).len(), 1);
    }

    add_budget {
        let caller: T::AccountId = whitelisted_caller();

        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            min,
            vec![],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
            min,
            min,
            pot,
        )?;

        let ad = <Metadata<T>>::iter_keys().next().unwrap();
    }: _(RawOrigin::Signed(caller.clone()), ad, pot)
    verify {
        let meta = <Metadata<T>>::get(&ad).unwrap();
        assert_eq!(<T as parami_did::Config>::Currency::free_balance(&meta.pot), pot.saturating_add(min));
    }

    bid {
        let caller: T::AccountId = whitelisted_caller();
        let kol: T::AccountId = account("kol", 1, 1);

        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;

        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            min,
            vec![],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
            min,
            min,
            pot,
        )?;

        let ad = <Metadata<T>>::iter_keys().next().unwrap();

        Nft::<T>::kick(RawOrigin::Signed(kol.clone()).into())?;
        Nft::<T>::back(RawOrigin::Signed(caller.clone()).into(), Zero::zero(), pot)?;
        Nft::<T>::mint(RawOrigin::Signed(kol).into(), Zero::zero(), b"Test Token".to_vec(), b"XTT".to_vec())?;

        Nft::<T>::claim(RawOrigin::Signed(caller.clone()).into(), Zero::zero())?;
    }: _(RawOrigin::Signed(caller.clone()), ad, Zero::zero(), min, None, None)
    verify {
        assert_ne!(<SlotOf<T>>::get(T::AssetId::zero()), None);
    }

    pay {
        Tag::<T>::force_create(RawOrigin::Root.into(), vec![1u8; 6])?;

        let caller: T::AccountId = whitelisted_caller();
        let kol: T::AccountId = account("kol", 1, 1);
        let visitor: T::AccountId = account("visitor", 2, 2);

        let min = <T as parami_did::Config>::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        <T as parami_did::Config>::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(visitor.clone()).into(), None)?;

        let did = Did::<T>::did_of(&visitor).unwrap();

        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        Ad::<T>::create(
            RawOrigin::Signed(caller.clone()).into(),
            min,
            vec![vec![1u8; 6]],
            Default::default(),
            1,
            HeightOf::<T>::max_value(),
            min,
            min,
            pot,
        )?;

        let ad = <Metadata<T>>::iter_keys().next().unwrap();

        Nft::<T>::kick(RawOrigin::Signed(kol.clone()).into())?;

        Nft::<T>::back(RawOrigin::Signed(caller.clone()).into(), Zero::zero(), pot)?;
        Nft::<T>::mint(RawOrigin::Signed(kol).into(), Zero::zero(), b"Test Token".to_vec(), b"XTT".to_vec())?;
        Nft::<T>::claim(RawOrigin::Signed(caller.clone()).into(), Zero::zero())?;

        Ad::<T>::bid(RawOrigin::Signed(caller.clone()).into(), ad, Zero::zero(), min, None, None)?;
    }: _(RawOrigin::Signed(caller.clone()), ad, Zero::zero(), did, vec![(vec![1u8; 6], 5)], None)
    verify {
        use frame_support::traits::tokens::fungibles::Inspect;

        assert_ne!(<T as parami_nft::Config>::Assets::balance(T::AssetId::zero(), &visitor), Zero::zero());
    }
}

impl_benchmark_test_suite!(Tag, crate::mock::new_test_ext(), crate::mock::Test);
