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
    traits::{Currency, EnsureOrigin, NamedReservableCurrency, OnUnbalanced, Time},
    PalletId,
};

use weights::WeightInfo;

type BalanceOf<T> =
    <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type Currency: NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]>;

        #[pallet::constant]
        type MinimalDeposit: Get<BalanceOf<Self>>;

        #[pallet::constant]
        type PalletId: Get<PalletId>;

        type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

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
    #[pallet::getter(fn blocked)]
    pub(super) type Blocked<T: Config> = StorageMap<_, Identity, T::DecentralizedId, bool>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Deposited(T::DecentralizedId, BalanceOf<T>),
        Blocked(T::DecentralizedId),
    }

    #[pallet::error]
    pub enum Error<T> {
        Blocked,
        ExistentialDeposit,
        Exists,
        NotExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as pallet::Config>::WeightInfo::deposit())]
        pub fn deposit(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = T::CallOrigin::ensure_origin(origin)?;

            ensure!(!<Blocked<T>>::contains_key(&did), Error::<T>::Blocked);

            let minimal = T::MinimalDeposit::get();

            let id = T::PalletId::get();

            let reserved = T::Currency::reserved_balance_named(&id.0, &who);

            ensure!(reserved + value >= minimal, Error::<T>::ExistentialDeposit);

            T::Currency::reserve_named(&id.0, &who, value)?;

            Self::deposit_event(Event::Deposited(did, reserved + value));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn block(origin: OriginFor<T>, advertiser: T::DecentralizedId) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;

            let meta =
                parami_did::Pallet::<T>::metadata(advertiser).ok_or(Error::<T>::NotExists)?;

            let id = T::PalletId::get();

            let imb = T::Currency::slash_all_reserved_named(&id.0, &meta.account);

            T::Slash::on_unbalanced(imb);

            <Blocked<T>>::insert(&advertiser, true);

            Self::deposit_event(Event::Blocked(advertiser));

            Ok(())
        }
    }
}

pub struct EnsureAdvertiser<T>(sp_std::marker::PhantomData<T>);
impl<T: pallet::Config> EnsureOrigin<T::Origin> for EnsureAdvertiser<T> {
    type Success = (T::DecentralizedId, T::AccountId);

    fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
        use frame_support::traits::{Get, OriginTrait};

        let (did, who) = parami_did::EnsureDid::<T>::ensure_origin(o).or(Err(T::Origin::none()))?;

        let minimal = T::MinimalDeposit::get();

        let id = T::PalletId::get();

        let reserved = T::Currency::reserved_balance_named(&id.0, &who);

        if reserved >= minimal {
            Ok((did, who))
        } else {
            Err(T::Origin::none())
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> T::Origin {
        use frame_system::RawOrigin;

        T::Origin::from(RawOrigin::Root)
    }
}