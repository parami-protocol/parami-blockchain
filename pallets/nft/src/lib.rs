#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::fungibles::{
            metadata::Mutate as FungMetaMutate, Create as FungCreate, Mutate as FungMutate,
            Transfer as FungTransfer,
        },
        Currency, EnsureOrigin,
        ExistenceRequirement::KeepAlive,
    },
};
use orml_nft::Pallet as Nft;
use parami_did::{EnsureDid, Pallet as Did};
use parami_swap::Pallet as Swap;
use sp_runtime::traits::{Saturating, Zero};
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as parami_swap::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<AccountOf<T>, HeightOf<T>, <T as parami_swap::Config>::AssetId>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + parami_did::Config
        + parami_swap::Config
        + orml_nft::Config<
            ClassId = Self::AssetId,
            TokenId = Self::AssetId,
            ClassData = (),
            TokenData = (),
        >
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        #[pallet::constant]
        type InitialMintingValue: Get<BalanceOf<Self>>;

        #[pallet::constant]
        type InitialMintingDeposit: Get<BalanceOf<Self>>;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Identity, T::DecentralizedId, MetaOf<T>>;

    /// Total deposit in pot
    #[pallet::storage]
    #[pallet::getter(fn deposit)]
    pub(super) type Deposit<T: Config> = StorageMap<
        _,
        Identity,
        T::DecentralizedId, // KOL
        BalanceOf<T>,
    >;

    /// Deposits by supporter in pot
    #[pallet::storage]
    #[pallet::getter(fn deposits)]
    pub(super) type Deposits<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::DecentralizedId, // KOL
        Identity,
        T::DecentralizedId, // Supporter
        BalanceOf<T>,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// NFT fragments Minted \[did, kol, value\]
        Backed(T::DecentralizedId, T::DecentralizedId, BalanceOf<T>),
        /// NFT fragments Claimed \[did, kol, value\]
        Claimed(T::DecentralizedId, T::DecentralizedId, BalanceOf<T>),
        /// NFT fragments Minted \[kol, class, token, tokens\]
        Minted(T::DecentralizedId, T::ClassId, T::TokenId, BalanceOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        InsufficientBalance,
        Minted,
        NotExists,
        NoTokens,
        YourSelf,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Back (support) the KOL.
        #[pallet::weight(1_000_000_000)]
        pub fn back(
            origin: OriginFor<T>,
            kol: T::DecentralizedId,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(kol != did, Error::<T>::YourSelf);

            ensure!(!<Metadata<T>>::contains_key(&kol), Error::<T>::Minted);

            let meta = Did::<T>::meta(kol).ok_or(Error::<T>::NotExists)?;

            T::Currency::transfer(&who, &meta.pot, value, KeepAlive)?;

            <Deposit<T>>::mutate(&kol, |maybe_deposit| {
                if let Some(deposit) = maybe_deposit {
                    *deposit += value;
                } else {
                    *maybe_deposit = Some(value);
                }
            });

            <Deposits<T>>::mutate(&kol, &did, |maybe_deposit| {
                if let Some(deposit) = maybe_deposit {
                    *deposit += value;
                } else {
                    *maybe_deposit = Some(value);
                }
            });

            Self::deposit_event(Event::Backed(did, kol, value));

            Ok(())
        }

        /// Fragment the NFT and mint token.
        #[pallet::weight(1_000_000_000)]
        pub fn mint(origin: OriginFor<T>, name: Vec<u8>, symbol: Vec<u8>) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            let created = <frame_system::Pallet<T>>::block_number();

            // 1. ensure funded

            let meta = Did::<T>::meta(did).ok_or(Error::<T>::NotExists)?;

            let deposit = T::Currency::free_balance(&meta.pot);
            let minimal = T::Currency::minimum_balance();

            ensure!(
                deposit > T::InitialMintingDeposit::get() + minimal,
                Error::<T>::InsufficientBalance
            );

            // 2. create NFT token

            let raw = T::DecentralizedId::encode(&did);
            let cid = Nft::<T>::create_class(&who, raw, ())?;
            let tid = Nft::<T>::mint(&who, cid, vec![], ())?;

            <Metadata<T>>::insert(
                &did,
                types::Metadata {
                    pot: meta.pot.clone(),
                    cid,
                    created,
                },
            );

            // 3. initial minting

            let initial = T::InitialMintingValue::get();

            T::Assets::create(cid, meta.pot.clone(), false, minimal)?;
            T::Assets::set(cid, &meta.pot, name, symbol, 18)?;
            T::Assets::mint_into(cid, &meta.pot, initial.saturating_mul(3u32.into()))?;

            // 4. transfer third of initial minting to swap

            let origin: T::Origin = frame_system::RawOrigin::Signed(meta.pot).into();
            Swap::<T>::create(origin.clone(), cid)?;
            Swap::<T>::add_liquidity(
                origin,
                cid,
                deposit - minimal,
                Zero::zero(),
                initial,
                Zero::zero(),
            )?;

            Self::deposit_event(Event::Minted(did, cid, tid, initial));

            Ok(())
        }

        /// Claim the fragments.
        #[pallet::weight(1_000_000_000)]
        pub fn claim(origin: OriginFor<T>, kol: T::DecentralizedId) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            // TODO: ensure locked?

            // TODO: KOL claim self-issued tokens

            let meta = <Metadata<T>>::get(&kol).ok_or(Error::<T>::NotExists)?;
            let total = <Deposit<T>>::get(&kol).ok_or(Error::<T>::NotExists)?;

            let deposit = <Deposits<T>>::get(&kol, &did).ok_or(Error::<T>::NoTokens)?;

            let initial = T::InitialMintingValue::get();

            let tokens = initial * deposit / total;

            T::Assets::transfer(meta.cid, &meta.pot, &who, tokens, false)?;

            <Deposits<T>>::remove(&kol, &did);

            Self::deposit_event(Event::Claimed(did, kol, tokens));

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {}
