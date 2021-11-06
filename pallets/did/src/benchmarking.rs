use super::*;

#[allow(unused)]
use crate::Pallet as Did;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Saturating;

benchmarks! {
    register {
        let caller: T::AccountId = whitelisted_caller();

        let referer: T::AccountId = account::<T::AccountId>("referer", 1, 1);

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);
        T::Currency::make_free_balance_be(&referer, pot);

        Did::<T>::register(RawOrigin::Signed(referer.clone()).into(), None)?;

        let referer = DidOf::<T>::get(referer);
    }: _(RawOrigin::Signed(caller), referer)
    verify {
        let caller: T::AccountId = whitelisted_caller();
        assert_ne!(DidOf::<T>::get(caller), None);
    }

    transfer {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);

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

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
    }: _(RawOrigin::Signed(caller))
    verify {
        let caller: T::AccountId = whitelisted_caller();
        assert_eq!(DidOf::<T>::get(caller), None);
    }

    set_avatar {
        let n in 0 .. 1000;

        let avatar = vec![0u8; n as usize];

        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
    }: _(RawOrigin::Signed(caller), avatar.clone())
    verify {
        let caller: T::AccountId = whitelisted_caller();
        let did = DidOf::<T>::get(caller).unwrap();
        let meta = Metadata::<T>::get(did).unwrap();
        assert_eq!(meta.avatar, avatar);
    }

    set_nickname {
        let n in 0 .. 1000;

        let nickname = vec![0u8; n as usize];

        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
    }: _(RawOrigin::Signed(caller), nickname.clone())
    verify {
        let caller: T::AccountId = whitelisted_caller();
        let did = DidOf::<T>::get(caller).unwrap();
        let meta = Metadata::<T>::get(did).unwrap();
        assert_eq!(meta.nickname, nickname);
    }
}

impl_benchmark_test_suite!(Did, crate::mock::new_test_ext(), crate::mock::Test);
