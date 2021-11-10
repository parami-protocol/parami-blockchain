use super::*;

#[allow(unused)]
use crate::Pallet as Magic;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Saturating;
use sp_std::prelude::*;

benchmarks! {
    create_stable_account {
        let caller: T::AccountId = whitelisted_caller();

        let magic: T::AccountId = account::<T::AccountId>("magic", 1, 1);

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);
        T::Currency::make_free_balance_be(&magic, min);
    }: _(RawOrigin::Signed(caller.clone()), magic, min)
    verify {
        assert_ne!(StableAccountOf::<T>::get(&caller), None);
    }

    change_controller {
        let old: T::AccountId = account::<T::AccountId>("old", 1, 1);
        let alt: T::AccountId = account::<T::AccountId>("alt", 1, 1);

        let magic: T::AccountId = account::<T::AccountId>("magic", 1, 1);

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&old, pot);
        T::Currency::make_free_balance_be(&alt, min);
        T::Currency::make_free_balance_be(&magic, min);

        Magic::<T>::create_stable_account(RawOrigin::Signed(old.clone()).into(), magic.clone(), min)?;
    }: _(RawOrigin::Signed(magic), alt.clone())
    verify {
        assert_eq!(StableAccountOf::<T>::get(&old), None);
        assert_ne!(StableAccountOf::<T>::get(&alt), None);
    }

    codo {
        let caller: T::AccountId = whitelisted_caller();

        let magic: T::AccountId = account::<T::AccountId>("magic", 1, 1);

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);
        T::Currency::make_free_balance_be(&magic, min);

        Magic::<T>::create_stable_account(RawOrigin::Signed(caller.clone()).into(), magic, min)?;

        let call: <T as Config>::Call = frame_system::Call::<T>::remark { remark: vec![] }.into();
    }: _(RawOrigin::Signed(caller), Box::new(call))
    verify {
        let event:<T as Config>::Event = Event::Codo(Ok(())).into();
        frame_system::Pallet::<T>::assert_last_event(event.into());
    }
}

impl_benchmark_test_suite!(Tag, crate::mock::new_test_ext(), crate::mock::Test);
