use super::*;

#[allow(unused)]
use crate::Pallet as Swap;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::fungibles::{Create, Inspect, Mutate};
use frame_system::RawOrigin;
use sp_runtime::traits::{Saturating, Zero};

benchmarks! {
    where_clause {
        where
        T::Assets: Create<T::AccountId>
    }

    create {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();

        let id = T::AssetId::min_value();

        T::Assets::create(id, caller.clone(), true, min)?;
    }: _(RawOrigin::Signed(caller), id, 1_000_000u32.into(), 7_000_000u32.into())
    verify {
        let meta = <Metadata<T>>::get(id);
        assert_ne!(meta, None);
    }

    add_liquidity {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();

        let pot = min.saturating_mul(1_000_000u32.into());

        let id = T::AssetId::min_value();
        let deadline = HeightOf::<T>::max_value();

        T::Assets::create(id, caller.clone(), true, min)?;

        T::Currency::make_free_balance_be(&caller, pot.saturating_mul(5u32.into()));
        T::Assets::mint_into(id, &caller, pot.saturating_mul(5u32.into()))?;

        Swap::<T>::create(RawOrigin::Signed(caller.clone()).into(), id, 1_000_000u32.into(), 7_000_000u32.into())?;

        Swap::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            id,
            pot,
            min,
            pot.saturating_mul(2u32.into()),
            deadline,
        )?;
    }: _(RawOrigin::Signed(caller.clone()), id, pot, min, max, deadline)
    verify {
        let meta = <Metadata<T>>::get(id).unwrap();
        assert_eq!(meta.liquidity, pot.saturating_mul(2u32.into()));
    }

    remove_liquidity {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();

        let pot = min.saturating_mul(1_000_000u32.into());

        let id = T::AssetId::min_value();
        let deadline = HeightOf::<T>::max_value();

        T::Assets::create(id, caller.clone(), true, min)?;

        T::Currency::make_free_balance_be(&caller, pot.saturating_mul(5u32.into()));
        T::Assets::mint_into(id, &caller, pot.saturating_mul(5u32.into()))?;

        Swap::<T>::create(RawOrigin::Signed(caller.clone()).into(), id, 1_000_000u32.into(), 7_000_000u32.into())?;

        Swap::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            id,
            pot,
            min,
            pot.saturating_mul(2u32.into()),
            deadline,
        )?;
    }: _(RawOrigin::Signed(caller.clone()), id, min, min, deadline)
    verify {
        let meta = <Metadata<T>>::get(id).unwrap();
        assert_eq!(meta.liquidity, Zero::zero());
    }

    buy_tokens {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();

        let pot = min.saturating_mul(1_000_000u32.into());

        let id = T::AssetId::min_value();
        let deadline = HeightOf::<T>::max_value();

        T::Assets::create(id, caller.clone(), true, min)?;

        T::Currency::make_free_balance_be(&caller, pot.saturating_mul(5u32.into()));
        T::Assets::mint_into(id, &caller, pot.saturating_mul(5u32.into()))?;

        Swap::<T>::create(RawOrigin::Signed(caller.clone()).into(), id, 1_000_000u32.into(), 7_000_000u32.into())?;

        Swap::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            id,
            pot,
            min,
            pot.saturating_mul(2u32.into()),
            deadline,
        )?;
    }: _(RawOrigin::Signed(caller.clone()), id, pot, max, deadline)
    verify {
        assert_eq!(T::Assets::balance(id, &caller), pot.saturating_mul(4u32.into()));
    }

    sell_tokens {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();

        let pot = min.saturating_mul(1_000_000u32.into());

        let id = T::AssetId::min_value();
        let deadline = HeightOf::<T>::max_value();

        T::Assets::create(id, caller.clone(), true, min)?;

        T::Currency::make_free_balance_be(&caller, pot.saturating_mul(5u32.into()));
        T::Assets::mint_into(id, &caller, pot.saturating_mul(5u32.into()))?;

        Swap::<T>::create(RawOrigin::Signed(caller.clone()).into(), id, 1_000_000u32.into(), 7_000_000u32.into())?;

        Swap::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            id,
            pot,
            min,
            pot.saturating_mul(2u32.into()),
            deadline,
        )?;
    }: _(RawOrigin::Signed(caller.clone()), id, pot, min, deadline)
    verify {
        assert_eq!(T::Assets::balance(id, &caller), pot.saturating_mul(2u32.into()));
    }

    sell_currency {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();

        let pot = min.saturating_mul(1_000_000u32.into());

        let id = T::AssetId::min_value();
        let deadline = HeightOf::<T>::max_value();

        T::Assets::create(id, caller.clone(), true, min)?;

        T::Currency::make_free_balance_be(&caller, pot.saturating_mul(5u32.into()));
        T::Assets::mint_into(id, &caller, pot.saturating_mul(5u32.into()))?;

        Swap::<T>::create(RawOrigin::Signed(caller.clone()).into(), id, 1_000_000u32.into(), 7_000_000u32.into())?;

        Swap::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            id,
            pot.saturating_mul(2u32.into()),
            min,
            pot,
            deadline,
        )?;
    }: _(RawOrigin::Signed(caller.clone()), id, pot, min, deadline)
    verify {
        assert_eq!(T::Currency::free_balance(&caller), pot.saturating_mul(2u32.into()));
    }

    buy_currency {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();

        let pot = min.saturating_mul(1_000_000u32.into());

        let id = T::AssetId::min_value();
        let deadline = HeightOf::<T>::max_value();

        T::Assets::create(id, caller.clone(), true, min)?;

        T::Currency::make_free_balance_be(&caller, pot.saturating_mul(5u32.into()));
        T::Assets::mint_into(id, &caller, pot.saturating_mul(5u32.into()))?;

        Swap::<T>::create(RawOrigin::Signed(caller.clone()).into(), id, 1_000_000u32.into(), 7_000_000u32.into())?;

        Swap::<T>::add_liquidity(
            RawOrigin::Signed(caller.clone()).into(),
            id,
            pot.saturating_mul(2u32.into()),
            min,
            pot,
            deadline,
        )?;
    }: _(RawOrigin::Signed(caller.clone()), id, pot, max, deadline)
    verify {
        assert_eq!(T::Currency::free_balance(&caller), pot.saturating_mul(4u32.into()));
    }
}

impl_benchmark_test_suite!(Swap, crate::mock::new_test_ext(), crate::mock::Test);
