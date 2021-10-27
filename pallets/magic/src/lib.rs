#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use types::*;

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
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    traits::{
        Currency,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        Get, IsSubType, IsType, OriginTrait, ReservableCurrency, Time,
    },
    transactional,
    weights::GetDispatchInfo,
    PalletId,
};
use sp_runtime::traits::{AccountIdConversion, Dispatchable};
use sp_std::boxed::Box;

use weights::WeightInfo;

pub type BalanceOf<T> =
    <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// The overarching call type.
        type Call: Parameter
            + Dispatchable<Origin = Self::Origin>
            + GetDispatchInfo
            + From<frame_system::Call<Self>>
            + IsSubType<Call<Self>>
            + IsType<<Self as frame_system::Config>::Call>;

        #[pallet::constant]
        type CreationFee: Get<BalanceOf<Self>>;

        type PalletId: Get<PalletId>;

        type Time: Time;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// map from controller account to `StableAccount`
    #[pallet::storage]
    pub type StableAccountOf<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        StableAccount<<T::Time as Time>::Moment, T::AccountId>,
    >;

    /// map from magic account to controller account
    #[pallet::storage]
    pub type ControllerAccountOf<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, T::AccountId>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// CreatedStableAccount \[stash, controller, magic\]
        CreatedStableAccount(T::AccountId, T::AccountId, T::AccountId),
        /// ChangedController \[stash, controller, magic\]
        ChangedController(T::AccountId, T::AccountId, T::AccountId),
        /// Codo \[ DispatchResult \]
        Codo(DispatchResult),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        ControllerAccountUsed,
        ControllerEqualToMagic,
        InsufficientBalance,
        MagicAccountUsed,
        ObsoletedMagicAccount,
        StableAccountNotFound,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[transactional]
        #[pallet::weight(T::WeightInfo::create_stable_account())]
        pub fn create_stable_account(
            origin: OriginFor<T>,
            magic_account: T::AccountId,
            #[pallet::compact] deposit: BalanceOf<T>,
        ) -> DispatchResult {
            let controller_account = ensure_signed(origin)?;
            ensure!(
                controller_account != magic_account,
                Error::<T>::ControllerEqualToMagic
            );

            ensure!(
                <ControllerAccountOf<T>>::get(&magic_account).is_none(),
                Error::<T>::MagicAccountUsed
            );
            ensure!(
                <ControllerAccountOf<T>>::get(&controller_account).is_none(),
                Error::<T>::ControllerAccountUsed
            );

            ensure!(
                <StableAccountOf<T>>::get(&controller_account).is_none(),
                Error::<T>::ControllerAccountUsed
            );
            ensure!(
                <StableAccountOf<T>>::get(&magic_account).is_none(),
                Error::<T>::MagicAccountUsed
            );

            let timestamp = T::Time::now();
            let height = <frame_system::Pallet<T>>::block_number();

            let mut raw = T::AccountId::encode(&magic_account);
            let mut ord = T::BlockNumber::encode(&height);
            raw.append(&mut ord);

            let pallet = T::PalletId::get();
            let stash_account = pallet.into_sub_account(raw);

            let sa = StableAccount {
                created_time: timestamp,
                stash_account,
                controller_account,
                magic_account,
            };

            let fee = T::CreationFee::get();

            ensure!(
                <T as Config>::Currency::free_balance(&sa.controller_account) > fee + deposit,
                Error::<T>::InsufficientBalance
            );

            <T as Config>::Currency::transfer(
                &sa.controller_account,
                &sa.magic_account,
                fee,
                KeepAlive,
            )?;
            <T as Config>::Currency::transfer(
                &sa.controller_account,
                &sa.stash_account,
                deposit,
                KeepAlive,
            )?;

            <StableAccountOf<T>>::insert(&sa.controller_account, &sa);
            <ControllerAccountOf<T>>::insert(&sa.magic_account, &sa.controller_account);

            Self::deposit_event(Event::CreatedStableAccount(
                sa.stash_account,
                sa.controller_account,
                sa.magic_account,
            ));

            Ok(())
        }

        #[transactional]
        #[pallet::weight(T::WeightInfo::change_controller())]
        pub fn change_controller(
            origin: OriginFor<T>,
            new_controller: T::AccountId,
        ) -> DispatchResult {
            let magic_account = ensure_signed(origin)?;

            ensure!(
                new_controller != magic_account,
                Error::<T>::ControllerEqualToMagic
            );

            ensure!(
                <ControllerAccountOf<T>>::get(&new_controller).is_none(),
                Error::<T>::ControllerAccountUsed
            );
            ensure!(
                <StableAccountOf<T>>::get(&new_controller).is_none(),
                Error::<T>::ControllerAccountUsed
            );

            let old_controller = <ControllerAccountOf<T>>::get(magic_account)
                .ok_or(Error::<T>::ObsoletedMagicAccount)?;

            let mut sa = <StableAccountOf<T>>::get(&old_controller)
                .ok_or(Error::<T>::StableAccountNotFound)?;

            let free = <T as Config>::Currency::free_balance(&old_controller);
            <T as Config>::Currency::transfer(&old_controller, &new_controller, free, AllowDeath)?;

            sa.controller_account = new_controller.clone();

            <StableAccountOf<T>>::remove(&old_controller);
            <StableAccountOf<T>>::insert(&sa.controller_account, &sa);

            <ControllerAccountOf<T>>::mutate(&sa.magic_account, |maybe_ca| {
                *maybe_ca = Some(new_controller)
            });

            Self::deposit_event(Event::ChangedController(
                sa.stash_account,
                sa.controller_account,
                sa.magic_account,
            ));

            Ok(())
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
            let sa = <StableAccountOf<T>>::get(controller_account)
                .ok_or(Error::<T>::StableAccountNotFound)?;

            let mut origin: T::Origin = frame_system::RawOrigin::Signed(sa.stash_account).into();
            origin.add_filter(move |c: &<T as frame_system::Config>::Call| {
                let c = <T as Config>::Call::from_ref(c);
                match c.is_sub_type() {
                    Some(Call::create_stable_account { .. })
                    | Some(Call::change_controller { .. })
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
