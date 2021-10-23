use super::*;

#[allow(unused)]
use crate::Pallet as Did;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
    register {
        let caller: T::AccountId = whitelisted_caller();

        let referer: T::AccountId = account::<T::AccountId>("referer", 1, 1);

        Did::<T>::register(RawOrigin::Signed(referer.clone()).into(), None)?;

        let referer = DidOf::<T>::get(referer);
    }: _(RawOrigin::Signed(caller), referer)
    verify {
        let caller: T::AccountId = whitelisted_caller();
        assert_ne!(DidOf::<T>::get(caller), None);
    }

    transfer {
        let caller: T::AccountId = whitelisted_caller();
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        let receiver: T::AccountId = account::<T::AccountId>("receiver", 1, 1);
    }: _(RawOrigin::Signed(caller), receiver)
    verify {
        let caller: T::AccountId = whitelisted_caller();
        assert_eq!(DidOf::<T>::get(caller), None);

        let receiver: T::AccountId = account::<T::AccountId>("receiver", 1, 1);
        assert_ne!(DidOf::<T>::get(receiver), None);
    }

    revoke {
        let caller: T::AccountId = whitelisted_caller();
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
    }: _(RawOrigin::Signed(caller))
    verify {
        let caller: T::AccountId = whitelisted_caller();
        assert_eq!(DidOf::<T>::get(caller), None);
    }
}

impl_benchmark_test_suite!(Did, crate::mock::new_test_ext(), crate::mock::Test);
