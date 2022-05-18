#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

use frame_support::{
    dispatch::DispatchResultWithPostInfo,
    ensure,
    traits::{
        tokens::fungibles::Transfer as FungTransfer, Currency, EnsureOrigin,
        ExistenceRequirement::AllowDeath, Get, StorageVersion,
    },
};
use frame_system::ensure_signed;
use parami_chainbridge::{ChainId, ResourceId};
use sp_core::U256;
use sp_runtime::traits::SaturatedConversion;
use sp_std::prelude::*;

use weights::WeightInfo;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AssetOf<T> = <T as Config>::AssetId;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
const MAX_TRANSFER_ASSET: u128 = 1000;

type AmountOf<T> = <<T as Config>::Currency as Currency<AccountOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_chainbridge::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
        type BridgeOrigin: EnsureOrigin<
            <Self as frame_system::Config>::Origin,
            Success = <Self as frame_system::Config>::AccountId,
        >;

        type Assets: FungTransfer<
            AccountOf<Self>,
            AssetId = AssetOf<Self>,
            Balance = BalanceOf<Self>,
        >;

        type AssetId: Parameter + Member + Default + Copy;

        /// The currency mechanism.
        type Currency: Currency<<Self as frame_system::Config>::AccountId>;

        /// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
        type HashId: Get<ResourceId>;

        type NativeTokenId: Get<ResourceId>;

        /// Weight information for extrinsics in this pallet
        type WeightInfo: WeightInfo;

        type ForceOrigin: EnsureOrigin<Self::Origin>;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Remark(<T as frame_system::Config>::Hash),
    }

    #[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct Map<T: Config> {
        pub chain: ChainId,
        pub resourceId: ResourceId,
        pub to: Vec<u8>,
        pub amount: U256,
        pub origin: AccountOf<T>,
    }

    #[pallet::storage]
    #[pallet::getter(fn resourcemap)]
    pub(super) type ResourceMap<T: Config> = StorageMap<_, Identity, AssetOf<T>, ResourceId>;

    #[pallet::storage]
    pub(super) type TransferMap<T: Config> = StorageMap<_, Identity, u32, Map<T>>;

    #[pallet::storage]
    pub(super) type MapLen<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        InvalidTransfer,
        NotExists,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::transfer_hash())]
        pub fn transfer_hash(
            origin: OriginFor<T>,
            hash: <T as frame_system::Config>::Hash,
            dest_id: ChainId,
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let resource_id = T::HashId::get();
            let metadata: Vec<u8> = hash.as_ref().to_vec();
            <parami_chainbridge::Pallet<T>>::transfer_generic(dest_id, resource_id, metadata)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::transfer_native())]
        pub fn transfer_native(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            recipient: Vec<u8>,
            dest_id: ChainId,
        ) -> DispatchResultWithPostInfo {
            let source = ensure_signed(origin)?;
            ensure!(
                <parami_chainbridge::Pallet<T>>::chain_whitelisted(dest_id),
                Error::<T>::InvalidTransfer
            );
            let bridge_id = <parami_chainbridge::Pallet<T>>::account_id();
            T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;
            let resource_id = T::NativeTokenId::get();

            let bridge_id = <parami_chainbridge::Pallet<T>>::account_id();
            if amount.saturated_into::<u128>() > MAX_TRANSFER_ASSET {
                let maplen = <MapLen<T>>::get();
                <TransferMap<T>>::insert(
                    maplen,
                    Map {
                        chain: dest_id,
                        resourceId: resource_id,
                        to: recipient,
                        amount: U256::from(amount.saturated_into::<u128>()),
                        origin: source,
                    },
                );

                <MapLen<T>>::put(maplen + 1);
                return Ok(().into());
            }

            T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;
            <parami_chainbridge::Pallet<T>>::transfer_fungible(
                dest_id,
                resource_id,
                recipient,
                U256::from(amount.saturated_into::<u128>()),
            )?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::approve_transfer_native())]
        pub fn approve_transfer_native(origin: OriginFor<T>, index: u32) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;
            let result = <TransferMap<T>>::get(index);
            let _ = match result {
                None => return Err(DispatchError::from("Remote Keystore not supported.")),

                Some(trx) => {
                    <parami_chainbridge::Pallet<T>>::transfer_fungible(
                        trx.chain,
                        trx.resourceId,
                        trx.to,
                        trx.amount,
                    )?;
                    return Ok(().into());
                }
            };
        }

        #[pallet::weight(<T as Config>::WeightInfo::transfer_token())]
        pub fn transfer_token(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            recipient: Vec<u8>,
            dest_id: ChainId,
            asset: AssetOf<T>,
        ) -> DispatchResultWithPostInfo {
            let source = ensure_signed(origin)?;
            ensure!(
                <parami_chainbridge::Pallet<T>>::chain_whitelisted(dest_id),
                Error::<T>::InvalidTransfer
            );
            let bridge_id = <parami_chainbridge::Pallet<T>>::account_id();
            let resource_id = <ResourceMap<T>>::get(asset).ok_or(Error::<T>::NotExists)?;

            T::Assets::transfer(asset, &source, &bridge_id, amount, false)?;
            <parami_chainbridge::Pallet<T>>::transfer_fungible(
                dest_id,
                resource_id,
                recipient,
                U256::from(amount.saturated_into::<u128>()),
            )?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::set_storage_map())]
        pub fn set_storage_map(
            origin: OriginFor<T>,
            resource_id: ResourceId,
            asset_id: AssetOf<T>,
        ) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;
            <ResourceMap<T>>::insert(asset_id, resource_id);
            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::transfer())]
        pub fn transfer(
            origin: OriginFor<T>,
            to: <T as frame_system::Config>::AccountId,
            amount: BalanceOf<T>,
            _r_id: ResourceId,
        ) -> DispatchResultWithPostInfo {
            let source = T::BridgeOrigin::ensure_origin(origin)?;
            <T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
            Ok(().into())
        }

        #[pallet::weight(<T as Config>::WeightInfo::remark())]
        pub fn remark(
            origin: OriginFor<T>,
            hash: <T as frame_system::Config>::Hash,
            _r_id: ResourceId,
        ) -> DispatchResultWithPostInfo {
            T::BridgeOrigin::ensure_origin(origin)?;
            Self::deposit_event(Event::Remark(hash));
            Ok(().into())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig {}

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self {}
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {}
    }
}
