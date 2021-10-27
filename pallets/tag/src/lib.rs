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

pub mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, ExistenceRequirement, StoredMap, Time, WithdrawReasons},
    StorageHasher,
};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{Hash, MaybeSerializeDeserialize, Member},
    DispatchError,
};
use sp_std::prelude::*;

use weights::WeightInfo;

type BalanceOf<T> =
    <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type MomentOf<T> = <<T as pallet::Config>::Time as Time>::Moment;
type MetaOf<T> = types::Metadata<<T as pallet::Config>::DecentralizedId, MomentOf<T>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type Currency: Currency<Self::AccountId>;

        type DecentralizedId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Ord
            + Default
            + Copy
            + sp_std::hash::Hash
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen
            + TypeInfo;

        type Hashing: StorageHasher;

        #[pallet::constant]
        type SubmissionFee: Get<BalanceOf<Self>>;

        type Time: Time;

        type CallOrigin: EnsureOrigin<
            Self::Origin,
            Success = (Self::DecentralizedId, Self::AccountId),
        >;

        type ForceOrigin: EnsureOrigin<Self::Origin>;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    pub type Metadata<T: Config> =
        StorageMap<_, <T as pallet::Config>::Hashing, Vec<u8>, MetaOf<T>>;

    /// Tags of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn tags_of)]
    pub(super) type TagsOf<T: Config> = StorageMap<_, Identity, HashOf<T>, Vec<Vec<u8>>>;

    /// Tags and Scores of a DID
    #[pallet::storage]
    #[pallet::getter(fn personas_of)]
    pub(super) type PersonasOf<T: Config> =
        StorageDoubleMap<_, Identity, T::DecentralizedId, Identity, Vec<u8>, i64>;

    /// Tags and Scores of a KOL
    #[pallet::storage]
    #[pallet::getter(fn influences_of)]
    pub(super) type InfluencesOf<T: Config> =
        StorageDoubleMap<_, Identity, T::DecentralizedId, Identity, Vec<u8>, i64>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Created(Vec<u8>, T::DecentralizedId),
    }

    #[pallet::error]
    pub enum Error<T> {
        Exists,
        InsufficientBalance,
        NotExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::create(tag.len() as u32))]
        pub fn create(origin: OriginFor<T>, tag: Vec<u8>) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            ensure!(!<Metadata<T>>::contains_key(&tag), Error::<T>::Exists);

            let fee = T::SubmissionFee::get();

            ensure!(
                T::Currency::free_balance(&who) >= fee,
                Error::<T>::InsufficientBalance
            );

            let imb = T::Currency::burn(fee);

            let _ = T::Currency::settle(
                &who,
                imb,
                WithdrawReasons::FEE,
                ExistenceRequirement::KeepAlive,
            );

            let hash = Self::inner_create(did, tag);

            Self::deposit_event(Event::Created(hash, did));

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::force_create(tag.len() as u32))]
        pub fn force_create(origin: OriginFor<T>, tag: Vec<u8>) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;

            ensure!(!<Metadata<T>>::contains_key(&tag), Error::<T>::Exists);

            let did = T::DecentralizedId::default();

            let hash = Self::inner_create(did, tag);

            Self::deposit_event(Event::Created(hash, did));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T> {
        pub tags: Vec<Vec<u8>>,
        pub phantom: PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                tags: Default::default(),
                phantom: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let created = T::Time::now();

            for tag in &self.tags {
                <Metadata<T>>::insert(
                    tag,
                    types::Metadata {
                        creator: T::DecentralizedId::default(),
                        created,
                    },
                );
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    fn inner_create(creator: T::DecentralizedId, tag: Vec<u8>) -> Vec<u8> {
        let created = T::Time::now();

        <Metadata<T>>::insert(&tag, types::Metadata { creator, created });

        <Metadata<T>>::hashed_key_for(&tag)
    }

    /// update score of a tag for a DID
    pub fn influence(did: T::DecentralizedId, tag: Vec<u8>, delta: i64) -> DispatchResult {
        ensure!(<Metadata<T>>::contains_key(&tag), Error::<T>::NotExists);

        let hash = <Metadata<T>>::hashed_key_for(&tag);

        <PersonasOf<T>>::mutate(&did, hash, |maybe_score| {
            if let Some(score) = maybe_score {
                *score += delta;
            } else {
                *maybe_score = Some(delta);
            }
        });

        Ok(())
    }

    /// update score of a tag for a KOL
    pub fn impact(did: T::DecentralizedId, tag: Vec<u8>, delta: i64) -> DispatchResult {
        ensure!(<Metadata<T>>::contains_key(&tag), Error::<T>::NotExists);

        let hash = <Metadata<T>>::hashed_key_for(&tag);

        <InfluencesOf<T>>::mutate(&did, hash, |maybe_score| {
            if let Some(score) = maybe_score {
                *score += delta;
            } else {
                *maybe_score = Some(delta);
            }
        });

        Ok(())
    }
}

impl<T: Config> StoredMap<Vec<u8>, Option<MetaOf<T>>> for Pallet<T> {
    fn get(k: &Vec<u8>) -> Option<MetaOf<T>> {
        <Metadata<T>>::get(k)
    }

    fn try_mutate_exists<R, E: From<DispatchError>>(
        k: &Vec<u8>,
        f: impl FnOnce(&mut Option<Option<MetaOf<T>>>) -> Result<R, E>,
    ) -> Result<R, E> {
        let mut some = match <Metadata<T>>::get(k) {
            Some(some) => Some(Some(some)),
            None => None,
        };

        let r = f(&mut some)?;

        <Metadata<T>>::mutate(k, |maybe| {
            *maybe = match some {
                Some(some) => some,
                None => None,
            }
        });

        Ok(r)
    }
}

impl<T: Config> StoredMap<HashOf<T>, Vec<Vec<u8>>> for Pallet<T> {
    fn get(k: &HashOf<T>) -> Vec<Vec<u8>> {
        match <TagsOf<T>>::get(k) {
            Some(tags) => tags,
            None => Default::default(),
        }
    }

    fn try_mutate_exists<R, E: From<DispatchError>>(
        k: &HashOf<T>,
        f: impl FnOnce(&mut Option<Vec<Vec<u8>>>) -> Result<R, E>,
    ) -> Result<R, E> {
        <TagsOf<T>>::mutate(k, f)
    }
}
