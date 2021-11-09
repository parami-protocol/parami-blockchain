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
    traits::{Currency, ExistenceRequirement::KeepAlive, WithdrawReasons},
};
#[cfg(not(feature = "std"))]
use num_traits::Float;
use parami_traits::Tags;
use scale_info::TypeInfo;
use sp_runtime::traits::{Hash, MaybeSerializeDeserialize, Member};
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<<T as Config>::DecentralizedId, HeightOf<T>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait
        type Currency: Currency<Self::AccountId>;

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
            Success = (Self::DecentralizedId, Self::AccountId),
        >;

        /// The origin which may forcibly create tag or otherwise alter privileged attributes
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Blake2_128, Vec<u8>, MetaOf<T>>;

    /// Tags of an advertisement
    #[pallet::storage]
    pub(super) type TagsOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        HashOf<T>,
        Blake2_128,
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
        Blake2_128,
        Vec<u8>, //
        i32,
        ValueQuery,
    >;

    /// Tags and Scores of a KOL
    #[pallet::storage]
    pub(super) type InfluencesOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::DecentralizedId,
        Blake2_128,
        Vec<u8>, //
        i32,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Tag created \[hash, creator\]
        Created(Vec<u8>, T::DecentralizedId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

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
            for tag in &self.tags {
                <Metadata<T>>::insert(
                    tag,
                    types::Metadata {
                        creator: T::DecentralizedId::default(),
                        created: Default::default(),
                    },
                );
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    fn inner_create(creator: T::DecentralizedId, tag: Vec<u8>) -> Vec<u8> {
        let created = <frame_system::Pallet<T>>::block_number();

        <Metadata<T>>::insert(&tag, types::Metadata { creator, created });

        Self::key(&tag)
    }

    fn accrue(score: i32, delta: i32) -> i32 {
        // f[x] := nLog[102, 102-Abs[x]] when n * x >= 0
        // f[x] := nLog[102, Abs[x] + 2] when n * x < 0

        let s = score as f32;
        let d = delta as f32;

        let b = if s.signum() == d.signum() {
            102f32 - s.abs()
        } else {
            s.abs() + 2f32
        };

        let d = b.log(102f32) * d;
        let s = (s + d) * 10f32;

        // MARK: due to rounding, the score won't exceed 100 or -100
        s.round() as i32 / 10
    }
}

impl<T: Config> Tags for Pallet<T> {
    type DecentralizedId = T::DecentralizedId;
    type Hash = HashOf<T>;

    fn key<K: AsRef<Vec<u8>>>(tag: K) -> Vec<u8> {
        use codec::Encode;
        use frame_support::{Blake2_128, StorageHasher};

        tag.as_ref().using_encoded(Blake2_128::hash).to_vec()
    }

    fn exists<K: AsRef<Vec<u8>>>(tag: K) -> bool {
        <Metadata<T>>::contains_key(tag.as_ref())
    }

    fn tags_of(id: &Self::Hash) -> Vec<Vec<u8>> {
        let mut iter = <TagsOf<T>>::iter_prefix_values(id);

        let mut hashes = vec![];
        while let Some(_) = iter.next() {
            let prefix = iter.prefix();
            let raw = iter.last_raw_key();
            let hash = raw[prefix.len()..].to_vec();

            hashes.push(hash);
        }

        hashes
    }

    fn add_tag(id: &Self::Hash, tag: Vec<u8>) -> DispatchResult {
        <TagsOf<T>>::insert(id, &tag, true);

        Ok(())
    }

    fn del_tag<K: AsRef<Vec<u8>>>(id: &Self::Hash, tag: K) -> DispatchResult {
        <TagsOf<T>>::remove(id, tag.as_ref());

        Ok(())
    }

    fn clr_tag(id: &Self::Hash) -> DispatchResult {
        <TagsOf<T>>::remove_prefix(id, None);

        Ok(())
    }

    fn has_tag<K: AsRef<Vec<u8>>>(id: &Self::Hash, tag: K) -> bool {
        <TagsOf<T>>::contains_key(id, tag.as_ref())
    }

    fn personas_of(did: &Self::DecentralizedId) -> Vec<(Vec<u8>, i32)> {
        let mut iter = <PersonasOf<T>>::iter_prefix_values(did);

        let mut tags = vec![];
        while let Some(score) = iter.next() {
            let prefix = iter.prefix();
            let raw = iter.last_raw_key();
            let hash = raw[prefix.len()..].to_vec();

            tags.push((hash, score));
        }

        tags
    }

    fn get_score<K: AsRef<Vec<u8>>>(did: &Self::DecentralizedId, tag: K) -> i32 {
        <PersonasOf<T>>::get(did, tag.as_ref())
    }

    fn influence<K: AsRef<Vec<u8>>>(
        did: &Self::DecentralizedId,
        tag: K,
        delta: i32,
    ) -> DispatchResult {
        <PersonasOf<T>>::mutate(&did, tag.as_ref(), |score| {
            *score = Self::accrue(*score, delta);
        });

        Ok(())
    }

    fn influences_of(kol: &Self::DecentralizedId) -> Vec<(Vec<u8>, i32)> {
        let mut iter = <InfluencesOf<T>>::iter_prefix_values(kol);

        let mut tags = vec![];
        while let Some(score) = iter.next() {
            let prefix = iter.prefix();
            let raw = iter.last_raw_key();
            let hash = raw[prefix.len()..].to_vec();

            tags.push((hash, score));
        }

        tags
    }

    fn get_influence<K: AsRef<Vec<u8>>>(kol: &Self::DecentralizedId, tag: K) -> i32 {
        <InfluencesOf<T>>::get(kol, tag.as_ref())
    }

    fn impact<K: AsRef<Vec<u8>>>(
        kol: &Self::DecentralizedId,
        tag: K,
        delta: i32,
    ) -> DispatchResult {
        <InfluencesOf<T>>::mutate(&kol, tag.as_ref(), |score| {
            *score = Self::accrue(*score, delta);
        });

        Ok(())
    }
}
