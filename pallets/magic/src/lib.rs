#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use types::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

mod types;

use frame_support::{
    dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo},
    traits::{
        Currency, EnsureOrigin,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        IsSubType, IsType, OriginTrait, ReservableCurrency,
    },
    transactional,
    weights::{GetDispatchInfo, PostDispatchInfo},
    PalletId,
};
use parami_primitives::Balance;
use sp_runtime::{
    traits::{AccountIdConversion, Dispatchable, One},
    DispatchErrorWithPostInfo,
};
use sp_std::boxed::Box;

macro_rules! s {
    ($e: expr) => {
        sp_runtime::SaturatedConversion::saturated_into($e)
    };
}

pub type GlobalId = u64;

pub type StableAccountOf<T> =
    StableAccount<<T as pallet_timestamp::Config>::Moment, <T as frame_system::Config>::AccountId>;

pub type BalanceOf<T> =
    <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type ResultPost<T> = sp_std::result::Result<T, DispatchErrorWithPostInfo<PostDispatchInfo>>;

pub const UNIT: Balance = 1_000_000_000_000_000;
pub const FEE: Balance = 100 * UNIT;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Required `origin` for updating configuration
        type ConfigOrigin: EnsureOrigin<Self::Origin>;

        /// The overarching call type.
        type Call: Parameter
            + Dispatchable<Origin = Self::Origin>
            + GetDispatchInfo
            + From<frame_system::Call<Self>>
            + IsSubType<Call<Self>>
            + IsType<<Self as frame_system::Config>::Call>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// Next available ID.
    #[pallet::storage]
    pub type NextId<T: Config> = StorageValue<_, GlobalId, ValueQuery>;

    /// map from controller account to `StableAccount`
    #[pallet::storage]
    pub type StableAccounts<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, StableAccountOf<T>>;

    /// map from magic account to controller account
    #[pallet::storage]
    pub type StableAccountByMagic<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// CreatedStableAccount \[stash, controller, magic\]
        CreatedStableAccount(T::AccountId, T::AccountId, T::AccountId),
        /// PreparedControllerChanging \[stash, controller, magic, new_controller\]
        PreparedControllerChanging(
            T::AccountId,
            T::AccountId,
            T::AccountId,
            Option<T::AccountId>,
        ),
        /// ChangedController \[stash, controller, magic\]
        ChangedController(T::AccountId, T::AccountId, T::AccountId),
        /// Codo \[ DispatchResult \]
        Codo(DispatchResult),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        ObsoletedMagicAccount,
        NewControllerEqualToOldController,
        NoAvailableId,
        StableAccountNotFound,
        MagicAccountExists,
        ControllerEqualToMagic,
        ControllerIsMagic,
        MagicIsController,
        ControllerAccountExists,
        NeedActivateController,
        InvalidController,
        NewControllerNotFound,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000_000_000)]
        #[transactional]
        pub fn create_stable_account(
            origin: OriginFor<T>,
            magic_account: T::AccountId,
            #[pallet::compact] stash_deposit: Balance,
        ) -> DispatchResultWithPostInfo {
            let controller_account = ensure_signed(origin)?;
            ensure!(
                controller_account != magic_account,
                Error::<T>::ControllerEqualToMagic
            );

            ensure!(
                StableAccountByMagic::<T>::get(&magic_account).is_none(),
                Error::<T>::MagicAccountExists
            );
            ensure!(
                StableAccountByMagic::<T>::get(&controller_account).is_none(),
                Error::<T>::ControllerIsMagic
            );

            ensure!(
                StableAccounts::<T>::get(&controller_account).is_none(),
                Error::<T>::ControllerAccountExists
            );
            ensure!(
                StableAccounts::<T>::get(&magic_account).is_none(),
                Error::<T>::MagicIsController
            );

            let sa: StableAccountOf<T> = StableAccount {
                created_time: now::<T>(),
                stash_account: Self::create_stash_account(Self::inc_id()?),
                controller_account,
                magic_account,
                new_controller_account: None,
            };

            <T as Config>::Currency::transfer(
                &sa.controller_account,
                &sa.magic_account,
                s!(FEE),
                KeepAlive,
            )?;
            <T as Config>::Currency::transfer(
                &sa.controller_account,
                &sa.stash_account,
                s!(stash_deposit),
                KeepAlive,
            )?;

            StableAccounts::<T>::insert(&sa.controller_account, &sa);
            StableAccountByMagic::<T>::insert(&sa.magic_account, &sa.controller_account);

            Self::deposit_event(Event::CreatedStableAccount(
                sa.stash_account.clone(),
                sa.controller_account.clone(),
                sa.magic_account.clone(),
            ));

            Ok(().into())
        }

        #[pallet::weight(1_000_000_000)]
        #[transactional]
        pub fn change_controller(
            origin: OriginFor<T>,
            new_controller: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let magic_account = ensure_signed(origin)?;

            ensure!(
                new_controller != magic_account,
                Error::<T>::ControllerEqualToMagic
            );

            ensure!(
                StableAccountByMagic::<T>::get(&new_controller).is_none(),
                Error::<T>::ControllerIsMagic
            );
            ensure!(
                StableAccounts::<T>::get(&new_controller).is_none(),
                Error::<T>::ControllerAccountExists
            );

            let old_controller = StableAccountByMagic::<T>::get(magic_account)
                .ok_or(Error::<T>::ObsoletedMagicAccount)?;
            ensure!(
                old_controller != new_controller,
                Error::<T>::NewControllerEqualToOldController
            );

            let mut sa = StableAccounts::<T>::get(old_controller)
                .ok_or(Error::<T>::StableAccountNotFound)?;
            sa.new_controller_account = Some(new_controller);

            StableAccounts::<T>::insert(&sa.controller_account, &sa);

            Self::deposit_event(Event::PreparedControllerChanging(
                sa.stash_account.clone(),
                sa.controller_account.clone(),
                sa.magic_account.clone(),
                sa.new_controller_account.clone(),
            ));

            Ok(().into())
        }

        #[pallet::weight(1_000_000_000)]
        #[transactional]
        pub fn activate_controller(
            origin: OriginFor<T>,
            old_controller: T::AccountId,
        ) -> DispatchResultWithPostInfo {
            let new_controller = ensure_signed(origin)?;
            ensure!(
                old_controller != new_controller,
                Error::<T>::NewControllerEqualToOldController
            );

            ensure!(
                StableAccountByMagic::<T>::get(&new_controller).is_none(),
                Error::<T>::ControllerIsMagic
            );
            ensure!(
                StableAccounts::<T>::get(&new_controller).is_none(),
                Error::<T>::ControllerAccountExists
            );

            let mut sa = StableAccounts::<T>::get(&old_controller)
                .ok_or(Error::<T>::StableAccountNotFound)?;
            ensure!(
                sa.new_controller_account
                    .clone()
                    .ok_or(Error::<T>::NewControllerNotFound)?
                    == new_controller,
                Error::<T>::InvalidController
            );

            let free = <T as Config>::Currency::free_balance(&old_controller);
            <T as Config>::Currency::transfer(&old_controller, &new_controller, free, AllowDeath)?;

            sa.new_controller_account = None;
            sa.controller_account = new_controller;

            StableAccounts::<T>::remove(&old_controller);
            StableAccounts::<T>::insert(&sa.controller_account, &sa);
            StableAccountByMagic::<T>::insert(&sa.magic_account, &sa.controller_account);

            Self::deposit_event(Event::ChangedController(
                sa.stash_account.clone(),
                sa.controller_account.clone(),
                sa.magic_account.clone(),
            ));
            Ok(().into())
        }

        #[pallet::weight({
            let di = call.get_dispatch_info();
            (
                di.weight.saturating_add(1_000_000)
                    .saturating_add(T::DbWeight::get().reads_writes(1, 1)),
                di.class
            )
        })]
        #[transactional]
        pub fn codo(
            origin: OriginFor<T>,
            call: Box<<T as Config>::Call>,
        ) -> DispatchResultWithPostInfo {
            let controller_account = ensure_signed(origin)?;
            let sa = StableAccounts::<T>::get(controller_account)
                .ok_or(Error::<T>::StableAccountNotFound)?;
            ensure!(
                sa.new_controller_account.is_none(),
                Error::<T>::NeedActivateController
            );

            let mut origin: T::Origin = frame_system::RawOrigin::Signed(sa.stash_account).into();
            origin.add_filter(move |c: &<T as frame_system::Config>::Call| {
                let c = <T as Config>::Call::from_ref(c);
                match c.is_sub_type() {
                    Some(Call::create_stable_account { .. })
                    | Some(Call::change_controller { .. })
                    | Some(Call::activate_controller { .. })
                    | Some(Call::codo { .. }) => false,
                    _ => true,
                }
            });
            let e = call.dispatch(origin);
            Self::deposit_event(Event::Codo(e.map(|_| ()).map_err(|e| e.error)));

            Ok(().into())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub _phantom: PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                _phantom: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {}
    }
}

/// now is duration since unix epoch in millisecond
pub fn now<T: Config>() -> T::Moment {
    pallet_timestamp::Pallet::<T>::now()
}

impl<T: Config> Pallet<T> {
    fn create_stash_account(id: GlobalId) -> T::AccountId {
        let stab_acc = PalletId(*b"prm/stab");
        stab_acc.into_sub_account(id)
    }

    fn inc_id() -> Result<GlobalId, DispatchError> {
        NextId::<T>::try_mutate(|id| -> Result<GlobalId, DispatchError> {
            let current_id = *id;
            *id = id
                .checked_add(GlobalId::one())
                .ok_or(Error::<T>::NoAvailableId)?;
            Ok(current_id)
        })
    }
}
