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
    traits::{EnsureOrigin, Time},
};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{
        Hash, LookupError, MaybeDisplay, MaybeMallocSizeOf, MaybeSerializeDeserialize, Member,
        SimpleBitOps, StaticLookup,
    },
    MultiAddress,
};
use sp_std::prelude::*;

use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type DecentralizedId: Parameter
            + Member
            + MaybeSerializeDeserialize
            // + Debug
            + MaybeDisplay
            + SimpleBitOps
            + Ord
            + Default
            + Copy
            // + CheckEqual
            + sp_std::hash::Hash
            + AsRef<[u8]>
            + AsMut<[u8]>
            + Into<[u8; 20]>
            + From<[u8; 20]>
            + MaybeMallocSizeOf
            + MaxEncodedLen
            + TypeInfo;

        type Hashing: Hash<Output = Self::Hash> + TypeInfo;

        type Time: Time;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// The metadata of a did.
    #[pallet::storage]
    pub(super) type Metadata<T: Config> = StorageMap<
        _,
        Identity,
        T::DecentralizedId,
        types::Metadata<T::AccountId, <T::Time as Time>::Moment>,
    >;

    /// The did of an account id.
    #[pallet::storage]
    #[pallet::getter(fn did_of)]
    pub(super) type DidOf<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, T::DecentralizedId>;

    /// The inviter did of a did.
    #[pallet::storage]
    #[pallet::getter(fn referrer_of)]
    pub(super) type ReferrerOf<T: Config> =
        StorageMap<_, Identity, T::DecentralizedId, T::DecentralizedId>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New DID assigned to AccountId, invited by DID. \[did, who, referrer\]
        Assigned(T::DecentralizedId, T::AccountId, Option<T::DecentralizedId>),
        /// Existed DID revoked. \[did\]
        Revoked(T::DecentralizedId),
        /// Existed DID transferred from one AccountId to another AccountId. \[did, from, to\]
        Transferred(T::DecentralizedId, T::AccountId, T::AccountId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        /// DID already exists
        Exists,
        /// DID does not exist
        NotExists,
        /// Referrer DID does not exist
        ReferrerNotExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new DID.
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            referrer: Option<T::DecentralizedId>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(!<DidOf<T>>::contains_key(&who), Error::<T>::Exists);

            if let Some(r) = referrer.as_ref() {
                ensure!(
                    <Metadata<T>>::contains_key(r),
                    Error::<T>::ReferrerNotExists
                );
            }

            let created = T::Time::now();
            let height = <frame_system::Pallet<T>>::block_number();

            // TODO: use a HMAC-based algorithm.
            let mut raw = T::AccountId::encode(&who);
            let mut ord = T::BlockNumber::encode(&height);
            raw.append(&mut ord);

            let did = <T as pallet::Config>::Hashing::hash(&raw);
            let did = Self::truncate(&did);

            <Metadata<T>>::insert(
                did,
                types::Metadata {
                    account: who.clone(),
                    created,
                    revoked: false,
                },
            );
            <DidOf<T>>::insert(who.clone(), did);
            if let Some(referrer) = referrer {
                <ReferrerOf<T>>::insert(did, referrer);
            }

            Self::deposit_event(Event::<T>::Assigned(did, who, referrer));

            Ok(())
        }

        /// Transfer a new DID.
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn transfer(origin: OriginFor<T>, account: T::AccountId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(!<DidOf<T>>::contains_key(&account), Error::<T>::Exists);

            let did = <DidOf<T>>::get(&who).ok_or(Error::<T>::NotExists)?;

            <Metadata<T>>::mutate(did, |maybe| {
                if let Some(metadata) = maybe {
                    *metadata = types::Metadata {
                        account: account.clone(),
                        ..*metadata
                    };
                }
            });

            <DidOf<T>>::remove(who.clone());
            <DidOf<T>>::insert(account.clone(), did);

            Self::deposit_event(Event::<T>::Transferred(did, who, account));

            Ok(())
        }

        /// Revoke a new DID.
        #[pallet::weight(T::WeightInfo::revoke())]
        pub fn revoke(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let did = <DidOf<T>>::get(&who).ok_or(Error::<T>::NotExists)?;

            <Metadata<T>>::mutate(did, |maybe| {
                if let Some(metadata) = maybe {
                    *metadata = types::Metadata {
                        account: metadata.account.clone(),
                        created: Default::default(),
                        revoked: true,
                    };
                }
            });

            <DidOf<T>>::remove(who.clone());

            Self::deposit_event(Event::<T>::Revoked(did));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub ids: Vec<(T::AccountId, T::DecentralizedId)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                ids: Default::default(),
            }
        }
    }

    #[cfg(feature = "std")]
    impl<T: Config> GenesisConfig<T> {
        pub fn build_storage(&self) -> Result<sp_runtime::Storage, String> {
            <Self as GenesisBuild<T>>::build_storage(self)
        }

        pub fn assimilate_storage(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
            <Self as GenesisBuild<T>>::assimilate_storage(self, storage)
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let created = T::Time::now();

            for (id, did) in &self.ids {
                <Metadata<T>>::insert(
                    did,
                    types::Metadata {
                        account: id.clone(),
                        created,
                        revoked: false,
                    },
                );
                <DidOf<T>>::insert(id.clone(), did);
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    fn truncate<H1: Default + AsMut<[u8]>, H2: AsRef<[u8]>>(src: &H2) -> H1 {
        let src = src.as_ref();
        let mut dest = H1::default();
        let len = dest.as_mut().len();
        assert!(len <= src.len());
        dest.as_mut().copy_from_slice(&src[(src.len() - len)..]);
        dest
    }

    fn lookup_index(did: T::DecentralizedId) -> Option<T::AccountId> {
        <Metadata<T>>::get(did).map(|x| x.account)
    }

    fn lookup_address(a: MultiAddress<T::AccountId, ()>) -> Option<T::AccountId> {
        match a {
            MultiAddress::Id(i) => Some(i),
            MultiAddress::Address20(a) => Self::lookup_index(a.into()),
            _ => None,
        }
    }
}

impl<T: Config> StaticLookup for Pallet<T> {
    type Source = MultiAddress<T::AccountId, ()>;
    type Target = T::AccountId;

    fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
        Self::lookup_address(a).ok_or(LookupError)
    }

    fn unlookup(i: Self::Target) -> Self::Source {
        MultiAddress::Id(i)
    }
}

pub struct EnsureDid<AccountId>(sp_std::marker::PhantomData<AccountId>);
impl<T: pallet::Config> EnsureOrigin<T::Origin> for EnsureDid<T> {
    type Success = (T::DecentralizedId, T::AccountId);

    fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
        use frame_support::traits::OriginTrait;
        use frame_system::RawOrigin;

        o.into().and_then(|o| match o {
            RawOrigin::Signed(who) => {
                let did = <DidOf<T>>::get(&who).ok_or(T::Origin::none())?;

                Ok((did, who))
            }
            r => Err(T::Origin::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> T::Origin {
        use frame_system::RawOrigin;

        T::Origin::from(RawOrigin::Root)
    }
}
