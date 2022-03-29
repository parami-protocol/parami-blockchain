use super::*;

#[allow(unused)]
use crate::Pallet as Linker;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use parami_did::Pallet as Did;
use sp_runtime::traits::{Bounded, Saturating};
use super::*;


benchmarks! {
    submit_proof {
        let n in 0 .. 500;
        let caller: T::AccountId = whitelisted_caller();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();
        let ipfs=b"https://ipfs.parami.io/ipfs/QmWFBFLb55z6FV4BRbbRnQkF4s68bMs6XdVU72NscoxgxD?filename=test.json".to_vec();
        let ipfs = vec![0u8; n as usize];
    }: _(RawOrigin::Signed(caller),ipfs.clone())
    verify {
        // assert_ne!(<PendingOf<T>>::get(&types::AccountType::Mastodon, &did), None);
    }
}
