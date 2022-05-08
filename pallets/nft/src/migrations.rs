use crate::{Config, Pallet};
use frame_support::{traits::PalletInfoAccess, weights::Weight};
use sp_runtime::traits::Saturating;

pub fn migrate<T: Config>() -> Weight {
    use frame_support::traits::StorageVersion;

    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    if version < 2 {
        weight.saturating_accrue(v2::migrate::<T>());
        StorageVersion::new(2).put::<Pallet<T>>();
    }

    weight
}

mod v2 {
    use super::*;

    use frame_support::storage::{
        migration::{move_prefix, remove_storage_prefix},
        storage_prefix,
    };

    pub fn migrate<T: Config>() -> Weight {
        let module = <Pallet<T>>::name().as_bytes();
        remove_storage_prefix(module, b"Account", b"");

        move_prefix(
            &storage_prefix(module, b"NftMetaStore"),
            &storage_prefix(module, b"Metadata"),
        );

        move_prefix(
            &storage_prefix(module, b"NextNftId"),
            &storage_prefix(module, b"NextClassId"),
        );

        Weight::max_value()
    }
}
