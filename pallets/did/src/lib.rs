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

mod migrations;
mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{EnsureOrigin, NamedReservableCurrency, StorageVersion},
};
use parami_did_utils::derive_storage_key;
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{
        Hash, LookupError, MaybeDisplay, MaybeMallocSizeOf, MaybeSerializeDeserialize, Member,
        SimpleBitOps, StaticLookup,
    },
    DispatchError, MultiAddress,
};
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<AccountOf<T>, HeightOf<T>>;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The reservable currency trait
        type Currency: NamedReservableCurrency<AccountOf<Self>, ReserveIdentifier = [u8; 8]>;

        /// The DID type, should be 20 bytes length
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

        /// The hashing algorithm being used to create DID
        type Hashing: Hash + TypeInfo;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// The metadata of a DID.
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Identity, T::DecentralizedId, MetaOf<T>>;

    /// The DID of an account id.
    #[pallet::storage]
    #[pallet::getter(fn did_of)]
    pub(super) type DidOf<T: Config> = StorageMap<_, Blake2_256, AccountOf<T>, T::DecentralizedId>;

    /// The inviter's DID of a DID.
    #[pallet::storage]
    #[pallet::getter(fn referrer_of)]
    pub(super) type ReferrerOf<T: Config> = StorageMap<
        _,
        Identity,
        T::DecentralizedId,
        T::DecentralizedId, // inviter's DID
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New DID assigned \[did, account, inviter\]
        Assigned(T::DecentralizedId, AccountOf<T>, Option<T::DecentralizedId>),
        /// DID was revoked \[did\]
        Revoked(T::DecentralizedId),
        /// DID transferred \[did, from, to\]
        Transferred(T::DecentralizedId, AccountOf<T>, AccountOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            migrations::migrate::<T>()
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        DidExists,
        DidNotExists,
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

            Self::create(who, referrer)?;

            Ok(())
        }

        /// Transfer a new DID.
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn transfer(origin: OriginFor<T>, account: AccountOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let did = <DidOf<T>>::get(&who).ok_or(Error::<T>::DidNotExists)?;

            Self::assign(&did, &account)?;

            Ok(())
        }

        /// Revoke a new DID.
        #[pallet::weight(T::WeightInfo::revoke())]
        pub fn revoke(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let did = <DidOf<T>>::get(&who).ok_or(Error::<T>::DidNotExists)?;

            let meta = <Metadata<T>>::get(&did).ok_or(Error::<T>::DidNotExists)?;

            <Metadata<T>>::insert(
                &did,
                types::Metadata {
                    account: meta.account.clone(),
                    revoked: true,
                    ..Default::default()
                },
            );

            <DidOf<T>>::remove(&who);

            Self::deposit_event(Event::<T>::Revoked(did));

            Ok(())
        }

        /// Set metadata of a DID.
        #[pallet::weight(T::WeightInfo::set_metadata(
            key.len() as u32,
            value.len() as u32
        ))]
        pub fn set_metadata(origin: OriginFor<T>, key: Vec<u8>, value: Vec<u8>) -> DispatchResult {
            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;

            let key = derive_storage_key(&key, &did);
            sp_io::offchain_index::set(&key, &value);

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// \[ account, did\]
        pub ids: Vec<(AccountOf<T>, T::DecentralizedId)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                ids: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (id, did) in &self.ids {
                <Metadata<T>>::insert(
                    did,
                    types::Metadata {
                        account: id.clone(),
                        revoked: false,
                        ..Default::default()
                    },
                );
                <DidOf<T>>::insert(id, did);
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn create(
        account: AccountOf<T>,
        referrer: Option<T::DecentralizedId>,
    ) -> Result<T::DecentralizedId, DispatchError> {
        use codec::Encode;

        ensure!(!<DidOf<T>>::contains_key(&account), Error::<T>::DidExists);

        if let Some(r) = referrer.as_ref() {
            ensure!(
                <Metadata<T>>::contains_key(r),
                Error::<T>::ReferrerNotExists
            );
        }

        // 1. generate DID

        let created = <frame_system::Pallet<T>>::block_number();

        // TODO: use a HMAC-based algorithm.
        let mut raw = <AccountOf<T>>::encode(&account);
        let mut ord = T::BlockNumber::encode(&created);
        raw.append(&mut ord);

        let did = <T as Config>::Hashing::hash(&raw);
        let did: T::DecentralizedId = Self::truncate(&did);

        // 2. store metadata

        <Metadata<T>>::insert(
            &did,
            types::Metadata {
                account: account.clone(),
                revoked: false,
                created,
                ..Default::default()
            },
        );
        <DidOf<T>>::insert(&account, did);
        if let Some(referrer) = referrer {
            <ReferrerOf<T>>::insert(&did, referrer);
        }

        Self::deposit_event(Event::<T>::Assigned(did.clone(), account, referrer));

        Ok(did)
    }

    pub fn assign(did: &T::DecentralizedId, dest: &AccountOf<T>) -> DispatchResult {
        ensure!(!<DidOf<T>>::contains_key(dest), Error::<T>::DidExists);

        let mut meta = <Metadata<T>>::get(did).ok_or(Error::<T>::DidNotExists)?;

        let source = meta.account.clone();

        meta.account = dest.clone();
        meta.created = <frame_system::Pallet<T>>::block_number();

        <Metadata<T>>::insert(did, meta);

        <DidOf<T>>::remove(&source);
        <DidOf<T>>::insert(dest, did);

        Self::deposit_event(Event::<T>::Transferred(did.clone(), source, dest.clone()));

        Ok(())
    }

    pub fn lookup_address(a: MultiAddress<AccountOf<T>, ()>) -> Option<AccountOf<T>> {
        match a {
            MultiAddress::Id(i) => Some(i),
            MultiAddress::Address20(a) => Self::lookup_did(a.into()),
            MultiAddress::Raw(r) => match r.len() {
                20 => Self::lookup_did(Self::truncate(&r)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn lookup_did(did: T::DecentralizedId) -> Option<AccountOf<T>> {
        <Metadata<T>>::get(&did).map(|x| x.account)
    }

    pub fn set_meta(did: &T::DecentralizedId, meta: MetaOf<T>) {
        <Metadata<T>>::insert(did, meta)
    }

    fn truncate<H1: Default + AsMut<[u8]>, H2: AsRef<[u8]>>(src: &H2) -> H1 {
        let src = src.as_ref();
        let mut dest = H1::default();
        let len = dest.as_mut().len();
        assert!(len <= src.len());
        dest.as_mut().copy_from_slice(&src[(src.len() - len)..]);
        dest
    }
}

impl<T: Config> StaticLookup for Pallet<T> {
    type Source = MultiAddress<AccountOf<T>, ()>;
    type Target = AccountOf<T>;

    fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
        Self::lookup_address(a).ok_or(LookupError)
    }

    fn unlookup(i: Self::Target) -> Self::Source {
        MultiAddress::Id(i)
    }
}

pub struct EnsureDid<T>(sp_std::marker::PhantomData<T>);
impl<T: pallet::Config> EnsureOrigin<T::Origin> for EnsureDid<T> {
    type Success = (T::DecentralizedId, AccountOf<T>);

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
