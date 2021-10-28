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
    traits::{Currency, ExistenceRequirement::KeepAlive, StoredMap, Time},
    PalletId,
};
use scale_info::TypeInfo;
use sp_runtime::traits::{AccountIdConversion, Hash, MaybeSerializeDeserialize, Member};
use sp_std::prelude::*;

use weights::WeightInfo;

type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type MomentOf<T> = <<T as pallet::Config>::Time as Time>::Moment;
type BalanceOf<T> =
    <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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

        #[pallet::constant]
        type PalletId: Get<PalletId>;

        type TagsStore: StoredMap<Vec<u8>, Vec<u8>> + StoredMap<HashOf<Self>, Vec<Vec<u8>>>;

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

    /// Metadata of an advertisement
    #[pallet::storage]
    pub(super) type Metadata<T: Config> = StorageMap<
        _,
        Identity,
        HashOf<T>,
        types::Metadata<BalanceOf<T>, T::DecentralizedId, MomentOf<T>>,
    >;

    /// Advertisement of an advertiser
    #[pallet::storage]
    #[pallet::getter(fn ads_of)]
    pub(super) type AdsOf<T: Config> = StorageMap<_, Identity, T::DecentralizedId, Vec<HashOf<T>>>;

    /// Slot of a KOL
    #[pallet::storage]
    #[pallet::getter(fn slot_of)]
    pub(super) type SlotOf<T: Config> = StorageMap<
        _,
        Identity,
        T::DecentralizedId,
        types::Slot<BalanceOf<T>, T::DecentralizedId, MomentOf<T>>,
    >;

    /// Slots of an advertisement
    #[pallet::storage]
    #[pallet::getter(fn slots_of)]
    pub(super) type SlotsOf<T: Config> =
        StorageMap<_, Identity, HashOf<T>, Vec<T::DecentralizedId>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Created(HashOf<T>),
        Updated(HashOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        NotExists,
        NotOwned,
        TagNotExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::create(tags.len() as u32))]
        pub fn create(
            origin: OriginFor<T>,
            budget: BalanceOf<T>,
            tags: Vec<Vec<u8>>,
            metadata: sp_core::H512,
            reward_rate: u16,
            deadline: MomentOf<T>,
        ) -> DispatchResult {
            let (creator, who) = T::CallOrigin::ensure_origin(origin)?;

            let mut hashes = vec![];
            for tag in &tags {
                let hash = T::TagsStore::get(tag);
                ensure!(hash.len() > 0, Error::<T>::TagNotExists);
                hashes.push(hash);
            }

            // 1. derive deposit poll account and advertisement ID

            let created = T::Time::now();

            let mut raw = T::AccountId::encode(&who);
            let mut ord = codec::Encode::encode(&created);
            raw.append(&mut ord);

            let pool: T::AccountId = T::PalletId::get().into_sub_account(raw);

            let raw = T::AccountId::encode(&pool);
            let id = T::Hashing::hash(&raw);

            // 2. deposit budget

            T::Currency::transfer(&who, &pool, budget, KeepAlive)?;

            // 3. insert metadata, ads_of, tags_of

            <Metadata<T>>::insert(
                &id,
                types::Metadata {
                    creator,
                    budget,
                    remain: budget,
                    metadata,
                    reward_rate,
                    created,
                    deadline,
                },
            );
            <AdsOf<T>>::mutate(&creator, |maybe| {
                if let Some(ads) = maybe {
                    ads.push(id);
                } else {
                    *maybe = Some(vec![id]);
                }
            });

            let _ = T::TagsStore::mutate(&id, |maybe| *maybe = hashes);

            Self::deposit_event(Event::Created(id));

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::update_reward_rate())]
        pub fn update_reward_rate(
            origin: OriginFor<T>,
            id: HashOf<T>,
            reward_rate: u16,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            let mut meta = Self::ensure_owned(did, id)?;

            meta.reward_rate = reward_rate;

            <Metadata<T>>::insert(&id, meta);

            Self::deposit_event(Event::Updated(id));

            Ok(())
        }

        #[pallet::weight(T::WeightInfo::update_tags(tags.len() as u32))]
        pub fn update_tags(
            origin: OriginFor<T>,
            id: HashOf<T>,
            tags: Vec<Vec<u8>>,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            Self::ensure_owned(did, id)?;

            let mut hashes = vec![];
            for tag in &tags {
                let hash = T::TagsStore::get(tag);
                ensure!(hash.len() > 0, Error::<T>::TagNotExists);
                hashes.push(hash);
            }

            let _ = T::TagsStore::mutate(&id, |maybe| *maybe = hashes);

            Self::deposit_event(Event::Updated(id));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn payouts(
            origin: OriginFor<T>,
            id: HashOf<T>,
            visitor: T::DecentralizedId,
            referer: Option<T::DecentralizedId>,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            Self::ensure_owned(did, id)?;

            todo!()
        }

        #[pallet::weight(1_000_000_000)]
        pub fn punish(
            origin: OriginFor<T>,
            id: HashOf<T>,
            slot: T::DecentralizedId,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            Self::ensure_owned(did, id)?;

            todo!()
        }
    }
}

impl<T: Config> Pallet<T> {
    fn ensure_owned(
        did: T::DecentralizedId,
        id: HashOf<T>,
    ) -> Result<
        types::Metadata<BalanceOf<T>, T::DecentralizedId, MomentOf<T>>,
        sp_runtime::DispatchError,
    > {
        let meta = <Metadata<T>>::get(id).ok_or(Error::<T>::NotExists)?;
        ensure!(meta.creator == did, Error::<T>::NotOwned);

        Ok(meta)
    }
}
