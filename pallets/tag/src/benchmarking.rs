use super::*;

#[allow(unused)]
use crate::Pallet as Tag;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;

benchmarks! {
    where_clause { where T: parami_did::Config }

    create {
        let n in 0 .. 255;

        let caller: T::AccountId = whitelisted_caller();

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        parami_did::Pallet::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        let name = vec![0u8; n as usize];
    }: _(RawOrigin::Signed(caller), name.clone())
    verify {
        assert_ne!(Metadata::<T>::get(&name), None);
    }

    force_create {
        let n in 0 .. 255;

        let name = vec![0u8; n as usize];
    }: _(RawOrigin::Root, name.clone())
    verify {
        assert_ne!(Metadata::<T>::get(&name), None);
    }
}

impl_benchmark_test_suite!(Tag, crate::mock::new_test_ext(), crate::mock::Test);
