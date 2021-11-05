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
    traits::{Currency, EnsureOrigin, NamedReservableCurrency},
    PalletId,
};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{
        AccountIdConversion, AtLeast32BitUnsigned, Bounded, Hash, LookupError, MaybeDisplay,
        MaybeMallocSizeOf, MaybeSerializeDeserialize, Member, SimpleBitOps, StaticLookup,
    },
    MultiAddress,
};
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<AccountOf<T>, HeightOf<T>, <T as Config>::AssetId>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Non-Fungible Token and fragments (fungible token) ID type used to store NFT Class ID in metadata
        type AssetId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + Bounded;

        /// Deposit to create a new DID
        #[pallet::constant]
        type CreationDeposit: Get<BalanceOf<Self>>;

        /// The reservable currency trait
        type Currency: NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]>;

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

        /// The pallet id, used for deriving "pot" accounts to receive donation
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// The metadata of a did.
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Identity, T::DecentralizedId, MetaOf<T>>;

    /// The did of an account id.
    #[pallet::storage]
    #[pallet::getter(fn did_of)]
    pub(super) type DidOf<T: Config> = StorageMap<_, Twox128, T::AccountId, T::DecentralizedId>;

    /// The inviter did of a did.
    #[pallet::storage]
    #[pallet::getter(fn referrer_of)]
    pub(super) type ReferrerOf<T: Config> =
        StorageMap<_, Identity, T::DecentralizedId, T::DecentralizedId>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New DID assigned \[did, account, inviter\]
        Assigned(T::DecentralizedId, T::AccountId, Option<T::DecentralizedId>),
        /// DID was revoked \[did\]
        Revoked(T::DecentralizedId),
        /// DID transferred \[did, from, to\]
        Transferred(T::DecentralizedId, T::AccountId, T::AccountId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        Exists,
        NotExists,
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
                    <Metadata<T>>::contains_key(&r),
                    Error::<T>::ReferrerNotExists
                );
            }

            // 1. generate DID

            let created = <frame_system::Pallet<T>>::block_number();

            // TODO: use a HMAC-based algorithm.
            let mut raw = T::AccountId::encode(&who);
            let mut ord = T::BlockNumber::encode(&created);
            raw.append(&mut ord);

            let did = <T as Config>::Hashing::hash(&raw);
            let did = Self::truncate(&did);

            // 2. deposit

            let id = T::PalletId::get();

            let deposit = T::CreationDeposit::get();

            T::Currency::reserve_named(&id.0, &who, deposit)?;

            // 3. store metadata

            let pot = id.into_sub_account(&did);

            <Metadata<T>>::insert(
                &did,
                types::Metadata {
                    account: who.clone(),
                    pot,
                    nft: None,
                    revoked: false,
                    created,
                },
            );
            <DidOf<T>>::insert(&who, did);
            if let Some(referrer) = referrer {
                <ReferrerOf<T>>::insert(&did, referrer);
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

            <Metadata<T>>::mutate(&did, |maybe| {
                if let Some(meta) = maybe {
                    *meta = types::Metadata {
                        account: account.clone(),
                        pot: meta.pot.clone(),
                        ..*meta
                    };
                }
            });

            <DidOf<T>>::remove(&who);
            <DidOf<T>>::insert(&account, did);

            Self::deposit_event(Event::<T>::Transferred(did, who, account));

            Ok(())
        }

        /// Revoke a new DID.
        #[pallet::weight(T::WeightInfo::revoke())]
        pub fn revoke(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let did = <DidOf<T>>::get(&who).ok_or(Error::<T>::NotExists)?;

            // TODO: ensure NFT buy-back

            <Metadata<T>>::mutate(&did, |maybe| {
                if let Some(meta) = maybe {
                    *meta = types::Metadata {
                        account: meta.account.clone(),
                        revoked: true,
                        ..Default::default()
                    };
                }
            });

            <DidOf<T>>::remove(&who);

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

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (id, did) in &self.ids {
                <Metadata<T>>::insert(
                    did,
                    types::Metadata {
                        account: id.clone(),
                        pot: T::PalletId::get().into_sub_account(&did),
                        nft: None,
                        revoked: false,
                        created: Default::default(),
                    },
                );
                <DidOf<T>>::insert(&id, did);
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn lookup_address(a: MultiAddress<T::AccountId, ()>) -> Option<T::AccountId> {
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

    pub fn lookup_did(did: T::DecentralizedId) -> Option<T::AccountId> {
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
    type Source = MultiAddress<T::AccountId, ()>;
    type Target = T::AccountId;

    fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
        Self::lookup_address(a).ok_or(LookupError)
    }

    fn unlookup(i: Self::Target) -> Self::Source {
        MultiAddress::Id(i)
    }
}

pub struct EnsureDid<T>(sp_std::marker::PhantomData<T>);
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
