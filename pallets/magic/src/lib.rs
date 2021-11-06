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
    traits::{
        Currency,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        IsSubType, IsType, OriginTrait,
    },
    weights::GetDispatchInfo,
    PalletId,
};
use sp_runtime::traits::{AccountIdConversion, Dispatchable};
use sp_std::boxed::Box;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::StableAccount<AccountOf<T>, HeightOf<T>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait
        type Currency: Currency<Self::AccountId>;

        /// The overarching call type
        type Call: Parameter
            + Dispatchable<Origin = Self::Origin>
            + GetDispatchInfo
            + From<frame_system::Call<Self>>
            + IsSubType<Call<Self>>
            + IsType<<Self as frame_system::Config>::Call>;

        /// The value to transfer to magic account when create new stash account
        #[pallet::constant]
        type CreationFee: Get<BalanceOf<Self>>;

        /// The pallet id, used for deriving stash accounts
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    /// map from controller account to `StableAccount`
    #[pallet::storage]
    #[pallet::getter(fn stable_of)]
    pub(super) type StableAccountOf<T: Config> = StorageMap<_, Twox128, T::AccountId, MetaOf<T>>;

    /// map from magic account to controller account
    #[pallet::storage]
    #[pallet::getter(fn controller_of)]
    pub(super) type ControllerAccountOf<T: Config> =
        StorageMap<_, Twox128, T::AccountId, T::AccountId>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Stable account created \[stash, controller\]
        CreatedStableAccount(T::AccountId, T::AccountId),
        /// Controller changed \[stash, controller\]
        ChangedController(T::AccountId, T::AccountId),
        /// Proxy executed correctly \[ result \]
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

            let created = <frame_system::Pallet<T>>::block_number();

            // TODO: use a HMAC-based algorithm.
            let mut raw = T::AccountId::encode(&magic_account);
            let mut ord = T::BlockNumber::encode(&created);
            raw.append(&mut ord);

            let stash_account = T::PalletId::get().into_sub_account(raw);

            let sa = types::StableAccount {
                stash_account,
                controller_account,
                magic_account,
                created,
            };

            let fee = T::CreationFee::get();

            ensure!(
                T::Currency::free_balance(&sa.controller_account) >= fee + deposit,
                Error::<T>::InsufficientBalance
            );

            T::Currency::transfer(&sa.controller_account, &sa.magic_account, fee, KeepAlive)?;
            T::Currency::transfer(
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
            ));

            Ok(())
        }

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

            let free = T::Currency::free_balance(&old_controller);
            T::Currency::transfer(&old_controller, &new_controller, free, AllowDeath)?;

            sa.controller_account = new_controller.clone();

            <StableAccountOf<T>>::remove(&old_controller);
            <StableAccountOf<T>>::insert(&sa.controller_account, &sa);

            <ControllerAccountOf<T>>::mutate(&sa.magic_account, |maybe| {
                *maybe = Some(new_controller)
            });

            Self::deposit_event(Event::ChangedController(
                sa.stash_account,
                sa.controller_account,
            ));

            Ok(())
        }

        #[pallet::weight({
            let di = call.get_dispatch_info();
            (
                T::WeightInfo::codo()
                    .saturating_add(di.weight)
                    .saturating_add(T::DbWeight::get().reads_writes(1, 1)),
                di.class,
            )
        })]
        pub fn codo(origin: OriginFor<T>, call: Box<<T as Config>::Call>) -> DispatchResult {
            let controller_account = ensure_signed(origin)?;
            let sa = <StableAccountOf<T>>::get(&controller_account)
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

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        // pub stash_account: A,
        // pub controller_account: A,
        // pub magic_account: A,
        // pub created: N,
        pub accounts: Vec<(T::AccountId, T::AccountId, T::AccountId)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                accounts: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            let length = self.accounts.len();

            for i in 0..length {
                let sa = types::StableAccount {
                    stash_account: self.accounts[i].1.clone(),
                    controller_account: self.accounts[i].2.clone(),
                    magic_account: self.accounts[i].0.clone(),
                    created: Default::default(),
                };

                <StableAccountOf<T>>::insert(&sa.controller_account, &sa);
                <ControllerAccountOf<T>>::insert(&sa.magic_account, &sa.controller_account);
            }
        }
    }
}
