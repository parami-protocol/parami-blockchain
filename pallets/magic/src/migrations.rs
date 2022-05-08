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

    if version < 3 {
        weight.saturating_accrue(v3::migrate::<T>());
        StorageVersion::new(3).put::<Pallet<T>>();
    }

    weight
}

mod v2 {
    use frame_support::storage::{
        migration::{move_prefix, remove_storage_prefix},
        storage_prefix,
    };

    use super::*;

    pub fn migrate<T: Config>() -> Weight {
        let module = <Pallet<T>>::name().as_bytes();
        remove_storage_prefix(module, b"StorageVersion", b"");

        move_prefix(
            &storage_prefix(module, b"StableAccountOf"),
            &storage_prefix(module, b"Metadata"),
        );
        move_prefix(
            &storage_prefix(module, b"Controller"),
            &storage_prefix(module, b"Codoer"),
        );
        move_prefix(
            &storage_prefix(module, b"ControllerAccountOf"),
            &storage_prefix(module, b"Controller"),
        );

        Weight::max_value()
    }
}

mod v3 {
    use super::*;
    use crate::Metadata;

    use frame_support::{
        storage::migration::remove_storage_prefix,
        traits::{Currency, ExistenceRequirement},
    };
    use parami_did::Pallet as Did;

    pub fn migrate<T: Config>() -> Weight {
        // let mut weight: Weight = 0;

        for meta in <Metadata<T>>::iter_values() {
            // weight.saturating_accrue(T::DbWeight::get().reads(1));

            let stash = T::Currency::free_balance(&meta.stash_account);
            let _ = T::Currency::transfer(
                &meta.stash_account,
                &meta.controller_account,
                stash,
                ExistenceRequirement::AllowDeath,
            );

            // weight.saturating_accrue(T::DbWeight::get().reads_writes(2, 1));

            let magic = T::Currency::free_balance(&meta.magic_account);
            let _ = T::Currency::transfer(
                &meta.magic_account,
                &meta.controller_account,
                magic,
                ExistenceRequirement::AllowDeath,
            );

            // weight.saturating_accrue(T::DbWeight::get().reads_writes(2, 1));

            if let Some(did) = Did::<T>::did_of(&meta.stash_account) {
                let _ = Did::<T>::assign(&did, &meta.controller_account);

                // weight.saturating_accrue(T::DbWeight::get().reads_writes(3, 3));
            }
        }

        let module = <Pallet<T>>::name().as_bytes();
        remove_storage_prefix(module, b"Metadata", b"");
        remove_storage_prefix(module, b"Controller", b"");
        remove_storage_prefix(module, b"Codoer", b"");

        Weight::max_value()

        // weight
    }
}
