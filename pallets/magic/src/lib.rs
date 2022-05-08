#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod migrations;
mod types;

use frame_support::traits::StorageVersion;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<AccountOf<T>, HeightOf<T>>;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {}

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
        fn on_runtime_upgrade() -> Weight {
            migrations::migrate::<T>()
        }
    }
}
