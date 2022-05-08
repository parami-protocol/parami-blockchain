use super::*;

#[allow(unused)]
use crate::Pallet as Linker;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use parami_did::Pallet as Did;
use parami_traits::types::Network;
use sp_runtime::traits::{Bounded, Saturating};

benchmarks! {
    where_clause {
        where
        T: parami_did::Config
    }

    link_sociality {
        let n in 0 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();

        let profile = vec![0u8; n as usize];
    }: _(RawOrigin::Signed(caller), Network::Mastodon, profile)
    verify {
        assert_ne!(<PendingOf<T>>::get(&Network::Mastodon, &did), None);
    }

    link_crypto {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();

        let address = vec![0u8; 256];
        let signature = [0u8; 65];
    }: _(RawOrigin::Signed(caller), Network::Unknown, address.clone(), signature)
    verify {
        assert_eq!(<LinksOf<T>>::get(&did, &Network::Unknown), Some(address));
    }

    deposit {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        let id = <T as Config>::PalletId::get();
    }: _(RawOrigin::Signed(caller.clone()), pot)
    verify {
        assert_eq!(T::Currency::reserved_balance_named(&id.0, &caller), pot);
    }

    force_trust {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();
        Linker::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;
    }: _(RawOrigin::Root, did)
    verify {
        assert_eq!(<Registrar<T>>::get(&did), Some(true));
    }

    force_block {
        let caller: T::AccountId = whitelisted_caller();

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();
        Linker::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;
        Linker::<T>::force_trust(RawOrigin::Root.into(), did)?;
    }: _(RawOrigin::Root, did)
    verify {
        assert_eq!(<Registrar<T>>::get(&did), Some(false));
    }

    force_unlink {
        let caller: T::AccountId = whitelisted_caller();

        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, pot);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();

        let address = vec![0u8; 20];
        let signature = [0u8; 65];

        Linker::<T>::link_crypto(RawOrigin::Signed(caller.clone()).into(), Network::Unknown, address.clone(), signature.clone())?;
    }: _(RawOrigin::Root, did.clone(), Network::Unknown)
    verify {
        assert_eq!(<LinksOf<T>>::get(&did, &Network::Unknown), None);
    }

    submit_link {
        let n in 0 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let applicant: T::AccountId = account("applicant", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();
        Linker::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;
        Linker::<T>::force_trust(RawOrigin::Root.into(), did)?;

        T::Currency::make_free_balance_be(&applicant, pot);
        Did::<T>::register(RawOrigin::Signed(applicant.clone()).into(), None)?;
        let did = Did::<T>::did_of(&applicant).unwrap();

        let profile = vec![0u8; n as usize];

        Linker::<T>::link_sociality(RawOrigin::Signed(applicant.clone()).into(), Network::Mastodon, profile.clone())?;
    }: _(RawOrigin::Signed(caller), did.clone(), Network::Mastodon, profile.clone(), true)
    verify {
        assert_eq!(<LinksOf<T>>::get(&did, &Network::Mastodon), Some(profile));
    }

    submit_score {
        let n in 0 .. 1000;

        let caller: T::AccountId = whitelisted_caller();

        let applicant: T::AccountId = account("applicant", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();
        Linker::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), pot)?;
        Linker::<T>::force_trust(RawOrigin::Root.into(), did)?;

        T::Currency::make_free_balance_be(&applicant, pot);
        Did::<T>::register(RawOrigin::Signed(applicant.clone()).into(), None)?;
        let did = Did::<T>::did_of(&applicant).unwrap();

        let tag = vec![0u8; n as usize];
    }: _(RawOrigin::Signed(caller), did.clone(), tag.clone(), 100)
    verify {
        assert_eq!(T::Tags::get_score(&did, &tag), 100);
    }
}

impl_benchmark_test_suite!(Linker, crate::mock::new_test_ext(), crate::mock::Test);
