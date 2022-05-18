#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use types::Score;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod migrations;
mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    storage::PrefixIterator,
    traits::{Currency, ExistenceRequirement::KeepAlive, StorageVersion, WithdrawReasons},
    Blake2_256, StorageHasher,
};
#[cfg(not(feature = "std"))]
use num_traits::Float;
use parami_traits::Tags;
use scale_info::TypeInfo;
use sp_runtime::traits::{Hash, MaybeSerializeDeserialize, Member};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AdOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HashOf = <Blake2_256 as StorageHasher>::Output;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<<T as Config>::DecentralizedId, HeightOf<T>>;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait
        type Currency: Currency<AccountOf<Self>>;

        /// The DID type
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

        /// Submission fee to create new tags
        #[pallet::constant]
        type SubmissionFee: Get<BalanceOf<Self>>;

        /// The origin which may do calls
        type CallOrigin: EnsureOrigin<
            Self::Origin,
            Success = (Self::DecentralizedId, AccountOf<Self>),
        >;

        /// The origin which may forcibly create tag or otherwise alter privileged attributes
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Metadata of a tag
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Blake2_256, Vec<u8>, MetaOf<T>>;

    /// Tags of an advertisement
    #[pallet::storage]
    pub(super) type TagsOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        AdOf<T>,
        Blake2_256,
        Vec<u8>, //
        bool,
        ValueQuery,
    >;

    /// Tags and Scores of a DID
    #[pallet::storage]
    pub(super) type PersonasOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::DecentralizedId,
        Blake2_256,
        Vec<u8>,
        types::Score, // (last_output, last_input)
        ValueQuery,
    >;

    /// Tags and Scores of a KOL
    #[pallet::storage]
    pub(super) type InfluencesOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::DecentralizedId,
        Blake2_256,
        Vec<u8>,
        types::Score, // (last_output, last_input)
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Tag created \[hash, creator\]
        Created(HashOf, T::DecentralizedId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            migrations::migrate::<T>()
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        Exists,
        InsufficientBalance,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::create(tag.len() as u32))]
        pub fn create(origin: OriginFor<T>, tag: Vec<u8>) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            ensure!(!<Metadata<T>>::contains_key(&tag), Error::<T>::Exists);

            let fee = T::SubmissionFee::get();

            let imb = T::Currency::burn(fee);

            let res = T::Currency::settle(&who, imb, WithdrawReasons::FEE, KeepAlive);

            ensure!(res.is_ok(), Error::<T>::InsufficientBalance);

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
    pub struct GenesisConfig<T: Config> {
        pub tag: Vec<Vec<u8>>,
        pub tags: Vec<(AdOf<T>, Vec<u8>)>,
        pub personas: Vec<(T::DecentralizedId, Vec<u8>, types::Score)>,
        pub influences: Vec<(T::DecentralizedId, Vec<u8>, types::Score)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                tag: Default::default(),
                tags: Default::default(),
                personas: Default::default(),
                influences: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for tag in &self.tag {
                <Metadata<T>>::insert(
                    tag,
                    types::Metadata {
                        creator: T::DecentralizedId::default(),
                        created: Default::default(),
                    },
                );
            }

            for (ad, tag) in &self.tags {
                <TagsOf<T>>::insert(ad, tag, true);
            }

            for (did, tag, score) in &self.personas {
                <PersonasOf<T>>::insert(
                    did,
                    tag,
                    types::Score {
                        current_score: score.current_score,
                        last_input: score.last_input,
                    },
                );
            }

            for (did, tag, score) in &self.influences {
                <InfluencesOf<T>>::insert(
                    did,
                    tag,
                    types::Score {
                        current_score: score.current_score,
                        last_input: score.last_input,
                    },
                );
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    fn inner_create(creator: T::DecentralizedId, tag: Vec<u8>) -> HashOf {
        let created = <frame_system::Pallet<T>>::block_number();

        <Metadata<T>>::insert(&tag, types::Metadata { creator, created });

        Self::key(&tag)
    }

    pub(crate) fn accrue(score: &types::Score, delta: i32) -> types::Score {
        use core::f32::consts::PI;

        // f[x] := ArcTan[x/50] * 200 / PI

        let last_input = score.last_input + delta;
        let current_score = last_input as f32 / 50.0;
        let current_score = current_score.atan();
        let current_score = current_score * 200.0 / PI;

        let current_score = (current_score.round() * 10.0) as i32 / 10;

        types::Score {
            current_score,
            last_input,
        }
    }

    fn storage_double_map_to_btree_map<TValue, TSource, F: FnMut(TSource) -> TValue>(
        iter: &mut PrefixIterator<TSource>,
        mut f: F,
    ) -> BTreeMap<HashOf, TValue> {
        let mut hashes = BTreeMap::new();
        while let Some(value) = iter.next() {
            let prefix = iter.prefix();
            let raw = iter.last_raw_key();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&raw[prefix.len()..]);

            let value = f(value);
            hashes.insert(hash, value);
        }

        hashes
    }
}

impl<T: Config> Tags<HashOf, AdOf<T>, T::DecentralizedId> for Pallet<T> {
    fn key<K: AsRef<Vec<u8>>>(tag: K) -> HashOf {
        use codec::Encode;

        tag.as_ref().using_encoded(Blake2_256::hash)
    }

    fn exists<K: AsRef<Vec<u8>>>(tag: K) -> bool {
        <Metadata<T>>::contains_key(tag.as_ref())
    }

    fn tags_of(id: &AdOf<T>) -> BTreeMap<HashOf, bool> {
        Self::storage_double_map_to_btree_map(&mut <TagsOf<T>>::iter_prefix_values(id), |v| v)
    }

    fn add_tag(id: &AdOf<T>, tag: Vec<u8>) -> DispatchResult {
        <TagsOf<T>>::insert(id, &tag, true);

        Ok(())
    }

    fn del_tag<K: AsRef<Vec<u8>>>(id: &AdOf<T>, tag: K) -> DispatchResult {
        <TagsOf<T>>::remove(id, tag.as_ref());

        Ok(())
    }

    fn clr_tag(id: &AdOf<T>) -> DispatchResult {
        <TagsOf<T>>::remove_prefix(id, None);

        Ok(())
    }

    fn has_tag<K: AsRef<Vec<u8>>>(id: &AdOf<T>, tag: K) -> bool {
        <TagsOf<T>>::contains_key(id, tag.as_ref())
    }

    fn personas_of(did: &T::DecentralizedId) -> BTreeMap<HashOf, i32> {
        Self::storage_double_map_to_btree_map(&mut <PersonasOf<T>>::iter_prefix_values(did), |v| {
            v.current_score
        })
    }

    fn get_score<K: AsRef<Vec<u8>>>(did: &T::DecentralizedId, tag: K) -> i32 {
        <PersonasOf<T>>::get(did, tag.as_ref()).current_score
    }

    fn influence<K: AsRef<Vec<u8>>>(
        did: &T::DecentralizedId,
        tag: K,
        delta: i32,
    ) -> DispatchResult {
        <PersonasOf<T>>::mutate(&did, tag.as_ref(), |score| {
            *score = Self::accrue(score, delta);
        });

        Ok(())
    }

    fn influences_of(kol: &T::DecentralizedId) -> BTreeMap<HashOf, i32> {
        Self::storage_double_map_to_btree_map(
            &mut <InfluencesOf<T>>::iter_prefix_values(kol),
            |v| v.current_score,
        )
    }

    fn get_influence<K: AsRef<Vec<u8>>>(kol: &T::DecentralizedId, tag: K) -> i32 {
        <InfluencesOf<T>>::get(kol, tag.as_ref()).current_score
    }

    fn impact<K: AsRef<Vec<u8>>>(kol: &T::DecentralizedId, tag: K, delta: i32) -> DispatchResult {
        <InfluencesOf<T>>::mutate(&kol, tag.as_ref(), |score| {
            *score = Self::accrue(score, delta);
        });

        Ok(())
    }
}
