use super::*;

#[allow(unused)]
use crate::Pallet as Nft;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::tokens::fungibles::Inspect;
use frame_system::RawOrigin;
use parami_did::Pallet as Did;
use parami_linker::Pallet as Linker;
use sp_runtime::traits::{Bounded, Saturating, Zero};

benchmarks! {
    where_clause {
        where
        T: parami_did::Config,
        T: parami_linker::Config
    }

    port {
        let caller: T::AccountId = whitelisted_caller();

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        let did = Did::<T>::did_of(&caller).unwrap();

        Linker::<T>::submit_link(RawOrigin::None.into(), did, Network::Ethereum, vec![1u8; 20], true)?;
    }: _(RawOrigin::Signed(caller.clone()), Network::Ethereum, vec![1u8; 20], vec![1u8; 32])
    verify {
        assert_ne!(<Porting<T>>::get((Network::Ethereum, &vec![1u8; 20], &vec![1u8; 32])), None);
    }

    kick {
        let caller: T::AccountId = whitelisted_caller();

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
    }: _(RawOrigin::Signed(caller.clone()))
    verify {
        let did = Did::<T>::did_of(&caller).unwrap();
        assert_eq!(<Preferred<T>>::get(&did), Some(Zero::zero()));
    }

    back {
        let caller: T::AccountId = whitelisted_caller();

        let kol: T::AccountId = account("kol", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        T::Currency::make_free_balance_be(&kol, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;

        Nft::<T>::kick(RawOrigin::Signed(kol.clone()).into())?;

        let did = Did::<T>::did_of(&kol).unwrap();
        let nft = <Preferred<T>>::get(&did).unwrap();
    }: _(RawOrigin::Signed(caller), nft, pot)
    verify {
        let meta = <Metadata<T>>::get(nft).unwrap();
        assert_eq!(T::Currency::free_balance(&meta.pot), pot);
    }

    mint {
        let n in 1 .. 1000 - 4;
        let s in 1 .. 1000 - 4;

        let name = vec![b'x'; n as usize];
        let symbol = vec![b'x'; n as usize];

        let caller: T::AccountId = whitelisted_caller();

        let supporter: T::AccountId = account("supporter", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&supporter, max);
        T::Currency::make_free_balance_be(&caller, pot);

        Did::<T>::register(RawOrigin::Signed(supporter.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;

        Nft::<T>::kick(RawOrigin::Signed(caller.clone()).into())?;

        let did = Did::<T>::did_of(&caller).unwrap();
        let nft = <Preferred<T>>::get(&did).unwrap();

        Nft::<T>::back(RawOrigin::Signed(supporter).into(), nft, pot)?;
    }: _(RawOrigin::Signed(caller), nft, name, symbol)
    verify {
        let meta = <Metadata<T>>::get(nft).unwrap();
        assert!(meta.minted);
    }

    claim {
        let caller: T::AccountId = whitelisted_caller();

        let kol: T::AccountId = account("kol", 1, 1);

        let max = BalanceOf::<T>::max_value();
        let min = T::Currency::minimum_balance();
        let pot = min.saturating_mul(1_000_000_000u32.into());

        T::Currency::make_free_balance_be(&caller, max);
        T::Currency::make_free_balance_be(&kol, pot);

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        Did::<T>::register(RawOrigin::Signed(kol.clone()).into(), None)?;

        Nft::<T>::kick(RawOrigin::Signed(kol.clone()).into())?;

        let did = Did::<T>::did_of(&kol).unwrap();
        let nft = <Preferred<T>>::get(&did).unwrap();

        Nft::<T>::back(RawOrigin::Signed(caller.clone()).into(), nft, pot)?;

        Nft::<T>::mint(RawOrigin::Signed(kol).into(), nft, b"Test Token".to_vec(), b"XTT".to_vec())?;
    }: _(RawOrigin::Signed(caller.clone()), nft)
    verify {
        assert!(T::Assets::balance(nft, &caller) > Zero::zero());
    }

    submit_porting {
        let caller: T::AccountId = whitelisted_caller();

        Did::<T>::register(RawOrigin::Signed(caller.clone()).into(), None)?;
        let did = Did::<T>::did_of(&caller).unwrap();

        Linker::<T>::submit_link(RawOrigin::None.into(), did, Network::Ethereum, vec![1u8; 20], true)?;

        Nft::<T>::port(RawOrigin::Signed(caller).into(), Network::Ethereum, vec![1u8; 20], vec![1u8; 32])?;
    }: _(RawOrigin::None, did, Network::Ethereum, vec![1u8; 20], vec![1u8; 32], true)
    verify {
        assert_eq!(<Porting<T>>::get((Network::Ethereum, &vec![1u8; 20], &vec![1u8; 32])), None);
        assert_ne!(<Ported<T>>::get((Network::Ethereum, &vec![1u8; 20], &vec![1u8; 32])), None);
    }
}

impl_benchmark_test_suite!(Did, crate::mock::new_test_ext(), crate::mock::Test);
