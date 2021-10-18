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

use frame_support::{dispatch::DispatchResultWithPostInfo, ensure, traits::Time};
use sp_io::hashing::keccak_256;
use sp_runtime::{
    traits::{LookupError, StaticLookup},
    MultiAddress,
};
use sp_std::prelude::*;

use weights::WeightInfo;

// Use 0x34(b'4') as prefix, so you will get a `N-did` after base58encode_check.
pub type DidMethodSpecId = [u8; 20];

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type Time: Time;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    pub(super) type Metadata<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        DidMethodSpecId,
        (T::AccountId, <T::Time as Time>::Moment, bool),
    >;

    /// The account id of a did.
    #[pallet::storage]
    #[pallet::getter(fn did_of)]
    pub(super) type DidOf<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, DidMethodSpecId>;

    #[pallet::storage]
    #[pallet::getter(fn referrer_of)]
    pub(super) type ReferrerOf<T: Config> =
        StorageMap<_, Identity, DidMethodSpecId, DidMethodSpecId>;

    /// The special controller of a did, used for account recovery.
    #[pallet::storage]
    #[pallet::getter(fn controller_of)]
    pub(super) type ControllerOf<T: Config> =
        StorageMap<_, Identity, DidMethodSpecId, T::AccountId>;

    /// Tracking the latest identity update.
    #[pallet::storage]
    #[pallet::getter(fn updated_by)]
    pub(super) type UpdatedBy<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        DidMethodSpecId,
        (T::AccountId, T::BlockNumber, <T::Time as Time>::Moment),
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A DID was assigned. \[index, who, referrer\]
        Assigned(DidMethodSpecId, T::AccountId, Option<DidMethodSpecId>),
        /// A DID has been freed up (unassigned). \[index\]
        Revoked(DidMethodSpecId),
        /// Controller is changed, \[did, new_controller\]
        ControllerChanged(DidMethodSpecId, T::AccountId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        /// DID does not exist
        NotExists,
        /// DID already exists
        DidExists,
        /// The index is assigned to another account
        NotOwner,
        /// The DID was not available or revoked
        InUse,
        /// DID is revoked
        Revoked,
        /// Only accepts account ID
        AccountIdRequired,
        /// Referrer does not exist
        ReferrerNotExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new DID.
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            referrer: Option<DidMethodSpecId>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(!<DidOf<T>>::contains_key(&who), Error::<T>::DidExists);

            if let Some(r) = referrer.as_ref() {
                ensure!(
                    <Metadata<T>>::contains_key(r),
                    Error::<T>::ReferrerNotExists
                );
            }

            let raw = T::AccountId::encode(&who);
            let hash = keccak_256(&raw);
            let mut id: DidMethodSpecId = [0u8; 20];
            id.copy_from_slice(&hash[..20]);

            Metadata::<T>::try_mutate::<_, _, Error<T>, _>(id, |maybe_value| {
                ensure!(maybe_value.is_none(), Error::<T>::DidExists);

                *maybe_value = Some((who.clone(), T::Time::now(), false));
                Ok(())
            })?;
            DidOf::<T>::insert(who.clone(), id);
            if let Some(referrer) = referrer {
                ReferrerOf::<T>::insert(id, referrer);
                Self::deposit_event(Event::Assigned(id, who.clone(), Some(referrer)));
            } else {
                Self::deposit_event(Event::Assigned(id, who.clone(), None));
            }

            let now_timestamp = T::Time::now();
            let now_block_number = <frame_system::Pallet<T>>::block_number();
            <UpdatedBy<T>>::insert(id, (who, now_block_number, now_timestamp));

            Ok(().into())
        }

        /// Rovoke a DID, which will never be used in the future.
        /// This means that you refuse to use this AccountID for identify.
        #[pallet::weight(T::WeightInfo::revoke())]
        pub fn revoke(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let id = <DidOf<T>>::get(&who).ok_or(Error::<T>::NotExists)?;
            Metadata::<T>::try_mutate::<_, _, Error<T>, _>(id, |maybe_value| {
                let (account, _when, revoked) = maybe_value.take().ok_or(Error::<T>::NotExists)?;
                ensure!(&account == &who, Error::<T>::NotOwner);
                ensure!(!revoked, Error::<T>::Revoked);
                // set created timestamp = 0
                *maybe_value = Some((who.clone(), Default::default(), true));
                Ok(())
            })?;

            Self::deposit_event(Event::<T>::Revoked(id));

            let now_timestamp = T::Time::now();
            let now_block_number = <frame_system::Pallet<T>>::block_number();
            <UpdatedBy<T>>::insert(id, (who, now_block_number, now_timestamp));

            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Lookup an T::AccountIndex to get an Id, if there's one there.
    pub fn lookup_index(index: DidMethodSpecId) -> Option<T::AccountId> {
        Metadata::<T>::get(index).map(|x| x.0)
    }

    pub fn lookup_account(a: T::AccountId) -> Option<DidMethodSpecId> {
        DidOf::<T>::get(a)
    }

    /// Lookup an address to get an Id, if there's one there.
    pub fn lookup_address(a: MultiAddress<T::AccountId, ()>) -> Option<T::AccountId> {
        match a {
            MultiAddress::Id(i) => Some(i),
            MultiAddress::Address20(i) => Self::lookup_index(i),
            _ => None,
        }
    }

    pub fn is_did(a: MultiAddress<T::AccountId, ()>) -> bool {
        match a {
            MultiAddress::Address20(_) => true,
            _ => false,
        }
    }

    pub fn is_account_id(a: MultiAddress<T::AccountId, ()>) -> bool {
        match a {
            MultiAddress::Id(_) => true,
            _ => false,
        }
    }
}

impl<T: Config> StaticLookup for Pallet<T> {
    type Source = MultiAddress<T::AccountId, ()>;
    type Target = T::AccountId;

    fn lookup(a: Self::Source) -> Result<Self::Target, LookupError> {
        Self::lookup_address(a).ok_or(LookupError)
    }

    fn unlookup(a: Self::Target) -> Self::Source {
        MultiAddress::Id(a)
    }
}
