use super::*;

#[allow(unused)]
use crate::Pallet as Advertiser;
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::{Get, NamedReservableCurrency};
use frame_system::RawOrigin;
use parami_did::Pallet as Did;
use sp_runtime::traits::{Bounded, Saturating};

benchmarks! {
    where_clause {where T: Config + parami_did::Config}

    deposit {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        let id = <T as Config>::PalletId::get();
    }: _(RawOrigin::Signed(caller.clone()), pot)
    verify {
        assert_eq!(T::Currency::reserved_balance_named(&id.0, &caller), pot);
    }

    block {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        Advertiser::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;

        let id = <T as Config>::PalletId::get();

        let did = Did::<T>::did_of(&caller).unwrap();
    }: _(RawOrigin::Root, did)
    verify {
        assert_eq!(T::Currency::reserved_balance_named(&id.0, &caller), 0u32.into());
    }
}

impl_benchmark_test_suite!(Advertiser, crate::mock::new_test_ext(), crate::mock::Test);
