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

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, EnsureOrigin, NamedReservableCurrency, OnUnbalanced},
    PalletId,
};
use parami_did::{EnsureDid, Pallet as Did};

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <CurrencyOf<T> as Currency<AccountOf<T>>>::Balance;
type CurrencyOf<T> = <T as parami_did::Config>::Currency;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type NegativeImbOf<T> = <CurrencyOf<T> as Currency<AccountOf<T>>>::NegativeImbalance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Minimum deposit to become an advertiser
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts of deposits
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Handler for the unbalanced reduction when slashing an advertiser
        type Slash: OnUnbalanced<NegativeImbOf<Self>>;

        /// The origin which may forcibly block an advertiser or otherwise alter privileged attributes
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Blocked DIDs
    #[pallet::storage]
    #[pallet::getter(fn blocked)]
    pub(super) type Blocked<T: Config> = StorageMap<_, Identity, DidOf<T>, bool>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Advertiser deposited \[id, value\]
        Deposited(DidOf<T>, BalanceOf<T>),
        /// Advertiser was blocked \[id\]
        Blocked(DidOf<T>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        Blocked,
        ExistentialDeposit,
        Exists,
        NotExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::deposit())]
        pub fn deposit(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(!<Blocked<T>>::contains_key(&did), Error::<T>::Blocked);

            let minimum = T::MinimumDeposit::get();

            let id = <T as Config>::PalletId::get();

            let reserved = T::Currency::reserved_balance_named(&id.0, &who);

            ensure!(reserved + value >= minimum, Error::<T>::ExistentialDeposit);

            T::Currency::reserve_named(&id.0, &who, value)?;

            Self::deposit_event(Event::Deposited(did, value));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::force_block())]
        pub fn force_block(origin: OriginFor<T>, advertiser: DidOf<T>) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;

            let meta = Did::<T>::meta(&advertiser).ok_or(Error::<T>::NotExists)?;

            let id = <T as Config>::PalletId::get();

            let imb = T::Currency::slash_all_reserved_named(&id.0, &meta.account);

            T::Slash::on_unbalanced(imb);

            <Blocked<T>>::insert(&advertiser, true);

            Self::deposit_event(Event::Blocked(advertiser));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub blocked: Vec<DidOf<T>>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                blocked: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for id in &self.blocked {
                <Blocked<T>>::insert(id, true);
            }
        }
    }
}

pub struct EnsureAdvertiser<T>(sp_std::marker::PhantomData<T>);
impl<T: pallet::Config> EnsureOrigin<T::Origin> for EnsureAdvertiser<T> {
    type Success = (DidOf<T>, AccountOf<T>);

    fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
        use frame_support::traits::{Get, OriginTrait};

        let (did, who) = EnsureDid::<T>::ensure_origin(o).or(Err(T::Origin::none()))?;

        let minimum = T::MinimumDeposit::get();

        let id = <T as Config>::PalletId::get();

        let reserved = T::Currency::reserved_balance_named(&id.0, &who);

        if reserved >= minimum {
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
