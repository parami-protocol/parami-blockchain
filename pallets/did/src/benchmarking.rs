use super::*;

#[allow(unused)]
use crate::Pallet as Did;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
    register {
        // referrer setting
        let public: T::Public = sr25519::Public([1; 32]).into();
        let caller: T::AccountId = public.clone().into_account();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        whitelist_account!(caller);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), public, None)?;
        let ref_id = Did::<T>::did_of(caller).unwrap();

        // caller setting
        let public: T::Public = sr25519::Public([2; 32]).into();
        let caller: T::AccountId = public.clone().into_account();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        whitelist_account!(caller);
    }: register(RawOrigin::Signed(caller.clone()), public, Some(ref_id))
    verify {
        assert_eq!(Did::<T>::total_dids(), Some(2), "should create did");
    }

    register_for {
        use frame_support::traits::Get;

        let min_deposit = <T as Did::Config>::Deposit::get();
        // referrer setting
        let public: T::Public = sr25519::Public([1; 32]).into();
        let caller: T::AccountId = public.clone().into_account();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        whitelist_account!(caller);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), public, None)?;
        Did::<T>::lock(RawOrigin::Signed(caller.clone()).into(), min_deposit)?;

        let public: T::Public = sr25519::Public([2; 32]).into();
    }: register_for(RawOrigin::Signed(caller.clone()), public)
    verify {
        assert_eq!(Did::<T>::total_dids(), Some(2), "should create did");
    }

    lock {
        use frame_support::traits::Get;

        let min_deposit = <T as Did::Config>::Deposit::get();
        // referrer setting
        let public: T::Public = sr25519::Public([1; 32]).into();
        let caller: T::AccountId = public.clone().into_account();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        whitelist_account!(caller);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), public, None)?;
    }: lock(RawOrigin::Signed(caller.clone()), min_deposit)
    verify {
        assert_eq!(Did::<T>::total_dids(), Some(1), "should create did");
    }

    revoke {
        use frame_support::traits::Get;

        let min_deposit = <T as Did::Config>::Deposit::get();
        // referrer setting
        let public: T::Public = sr25519::Public([1; 32]).into();
        let caller: T::AccountId = public.clone().into_account();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        whitelist_account!(caller);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), public, None)?;
        Did::<T>::lock(RawOrigin::Signed(caller.clone()).into(), min_deposit)?;
    }: revoke(RawOrigin::Signed(caller.clone()))
    verify {
        assert_eq!(Did::<T>::total_dids(), Some(0), "should revoke did");
    }
}

impl_benchmark_test_suite!(Did, crate::mock::new_test_ext(), crate::mock::Test);
