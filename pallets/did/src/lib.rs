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

use codec::Codec;
use frame_support::{
    dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo},
    ensure,
    traits::{Currency, ReservableCurrency, Time, UnfilteredDispatchable},
    weights::GetDispatchInfo,
};
use scale_info::TypeInfo;
use sp_core::sr25519;
use sp_io::hashing::keccak_256;
use sp_runtime::{
    traits::{IdentifyAccount, LookupError, Member, StaticLookup, Verify},
    MultiAddress,
};
use sp_std::prelude::*;

use weights::WeightInfo;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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

        /// The currency trait.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The deposit needed for reserving a did.
        #[pallet::constant]
        type Deposit: Get<BalanceOf<Self>>;

        /// The public key type, MultiSigner
        type Public: IdentifyAccount<AccountId = Self::AccountId>
            + AsRef<[u8]>
            + From<sr25519::Public>
            + Member
            + Codec
            + TypeInfo;

        type Signature: Verify<Signer = Self::Public> + Member + Codec;

        /// A sudo-able call.
        type Call: Parameter + UnfilteredDispatchable<Origin = Self::Origin> + GetDispatchInfo;

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
        (T::AccountId, BalanceOf<T>, <T::Time as Time>::Moment, bool),
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

    /// Tracking total dids
    #[pallet::storage]
    #[pallet::getter(fn total_dids)]
    pub(super) type TotalDids<T: Config> = StorageValue<_, u32>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A DID was assigned. \[index, who, referrer\]
        Assigned(DidMethodSpecId, T::AccountId, Option<DidMethodSpecId>),
        /// A DID has been freed up (unassigned). \[index\]
        Revoked(DidMethodSpecId),
        /// A Did call is done
        CallDone(DispatchResult),
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
        /// Deposit to low
        InsufficientDeposit,
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

            let raw = T::AccountId::encode(&who);
            let hash = keccak_256(&raw);
            let mut id = [0u8; 20];
            id.copy_from_slice(&hash[12..]);

            Self::register_did(who.clone(), id, referrer)?;

            let now_timestamp = T::Time::now();
            let now_block_number = <frame_system::Pallet<T>>::block_number();
            <UpdatedBy<T>>::insert(id, (who, now_block_number, now_timestamp));

            Ok(().into())
        }

        /// Lock balance.
        #[pallet::weight(T::WeightInfo::lock())]
        pub fn lock(
            origin: OriginFor<T>,
            #[pallet::compact] amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(amount >= T::Deposit::get(), Error::<T>::InsufficientDeposit);
            let id = <DidOf<T>>::get(&who).ok_or(Error::<T>::NotExists)?;
            Metadata::<T>::try_mutate(id, |maybe_value| {
                let (_account, current_amount, _when, revoked) =
                    maybe_value.as_mut().ok_or(Error::<T>::NotExists)?;
                ensure!(!(*revoked), Error::<T>::Revoked);
                *current_amount += amount;
                T::Currency::reserve(&who, amount)
            })?;

            Ok(().into())
        }

        /// Rovoke a DID, which will never be used in the future.
        /// This means that you refuse to use this AccountID for identify.
        #[pallet::weight(T::WeightInfo::revoke())]
        pub fn revoke(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            let id = <DidOf<T>>::get(&who).ok_or(Error::<T>::NotExists)?;
            Metadata::<T>::try_mutate::<_, _, Error<T>, _>(id, |maybe_value| {
                let (account, amount, _when, revoked) =
                    maybe_value.take().ok_or(Error::<T>::NotExists)?;
                ensure!(&account == &who, Error::<T>::NotOwner);
                ensure!(!revoked, Error::<T>::Revoked);
                T::Currency::unreserve(&who, amount);
                // set created timestamp = 0
                *maybe_value = Some((who.clone(), amount, Default::default(), true));
                Ok(())
            })?;
            TotalDids::<T>::mutate(|v| {
                *v = Some(v.as_ref().copied().unwrap_or_default() - 1);
            });

            Self::deposit_event(Event::<T>::Revoked(id));

            let now_timestamp = T::Time::now();
            let now_block_number = <frame_system::Pallet<T>>::block_number();
            <UpdatedBy<T>>::insert(id, (who, now_block_number, now_timestamp));

            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn register_did(
        who: T::AccountId,
        id: DidMethodSpecId,
        referrer: Option<DidMethodSpecId>,
    ) -> Result<(), DispatchError> {
        if let Some(r) = referrer.as_ref() {
            ensure!(
                <Metadata<T>>::contains_key(r),
                Error::<T>::ReferrerNotExists
            );
        }

        Metadata::<T>::try_mutate::<_, _, Error<T>, _>(id, |maybe_value| {
            ensure!(maybe_value.is_none(), Error::<T>::DidExists);

            *maybe_value = Some((who.clone(), Default::default(), T::Time::now(), false));
            Ok(())
        })?;
        DidOf::<T>::insert(who.clone(), id);
        // TODO: handle overflow?
        TotalDids::<T>::mutate(|v| {
            *v = Some(v.as_ref().copied().unwrap_or_default() + 1);
        });

        if let Some(referrer) = referrer {
            ReferrerOf::<T>::insert(id, referrer);
            Self::deposit_event(Event::Assigned(id, who, Some(referrer)));
        } else {
            Self::deposit_event(Event::Assigned(id, who, None));
        }

        Ok(())
    }

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
