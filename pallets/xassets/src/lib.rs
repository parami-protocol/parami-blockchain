#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

use scale_info::TypeInfo;

#[cfg(feature = "std")]
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

        type AssetId: Parameter + Member + Default + Copy + MaxEncodedLen;

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
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn resourcemap)]
    pub(super) type ResourceMap<T: Config> = StorageMap<_, Identity, AssetOf<T>, ResourceId>;

    #[pallet::storage]
    pub(super) type TransactionList<T: Config> =
        StorageValue<_, Vec<Map<AccountOf<T>, BalanceOf<T>>>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Remark(<T as frame_system::Config>::Hash),
    }

    #[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct Map<A, B> {
        pub chain: ChainId,
        pub resource_id: ResourceId,
        pub to: Vec<u8>,
        pub amount: B,
        pub origin: A,
        pub bridge_id: A,
    }

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
            let source = ensure_signed(origin.clone())?;
            ensure!(
                <parami_chainbridge::Pallet<T>>::chain_whitelisted(dest_id),
                Error::<T>::InvalidTransfer
            );
            let resource_id = T::NativeTokenId::get();

            let bridge_id = <parami_chainbridge::Pallet<T>>::account_id();
            if amount.saturated_into::<u128>() > MAX_TRANSFER_ASSET {
                let mut transaction_list = <TransactionList<T>>::get();
                let transaction = Map {
                    chain: dest_id,
                    resource_id: resource_id,
                    to: recipient,
                    amount: amount.clone(),
                    origin: source,
                    bridge_id: bridge_id,
                };
                transaction_list.push(transaction);
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
            let mut result = <TransactionList<T>>::get();

            let u_index = index as usize;
            if u_index >= result.len() || u_index < 0 {
                return Err(DispatchError::from("Transaction index is not exist."));
            }
            let trx = result[u_index].clone();

            T::Currency::transfer(&trx.origin, &trx.bridge_id, trx.amount, AllowDeath)?;
            <parami_chainbridge::Pallet<T>>::transfer_fungible(
                trx.chain,
                trx.resource_id,
                trx.to,
                U256::from(trx.amount.saturated_into::<u128>()),
            )?;
            result.remove(u_index);
            return Ok(().into());
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
