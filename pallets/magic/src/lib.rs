#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod migrations;
mod types;

use frame_support::traits::{
    tokens::fungibles::{Inspect, Transfer},
    Currency, StorageVersion,
};
#[cfg(feature = "try-runtime")]
use log::info;
use sp_runtime::traits::{AtLeast32BitUnsigned, Bounded};

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<AccountOf<T>, HeightOf<T>>;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {
        /// Fragments (fungible token) ID type
        type AssetId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + AtLeast32BitUnsigned
            + Default
            + Bounded
            + Copy;

        /// The assets trait to create, mint, and transfer fractions (fungible token)
        type Assets: Inspect<AccountOf<Self>, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
            + Transfer<AccountOf<Self>, AssetId = Self::AssetId, Balance = BalanceOf<Self>>;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Metadata of a magic-stash account, by controller.
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Blake2_256, AccountOf<T>, MetaOf<T>>;

    /// Controller account of magic account
    #[pallet::storage]
    #[pallet::getter(fn controller)]
    pub(super) type Controller<T: Config> = StorageMap<_, Blake2_256, AccountOf<T>, AccountOf<T>>;

    /// Controller account of stash account
    #[pallet::storage]
    #[pallet::getter(fn codoer)]
    pub(super) type Codoer<T: Config> = StorageMap<_, Blake2_256, AccountOf<T>, AccountOf<T>>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            // assert!(StorageVersion::<T>::get() == Releases::V0, "Storage version too high.");

            info!("migration: magic storage version v4 PRE migration checks succesful!");

            Ok(())
        }

        fn on_runtime_upgrade() -> Weight {
            migrations::migrate::<T>()
        }
    }
}
