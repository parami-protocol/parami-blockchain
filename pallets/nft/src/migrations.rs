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
    use super::*;

    use frame_support::storage::{migration::move_prefix, storage_prefix};

    pub fn migrate<T: Config>() -> Weight {
        let module = <Pallet<T>>::name().as_bytes();

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

mod v3 {
    use super::*;

    use crate::{MetaOf, NftOf};

    use frame_support::{generate_storage_alias, Identity, Twox64Concat};

    generate_storage_alias!(
        Nft, Metadata<T: Config> => Map<
            (Twox64Concat, NftOf<T>),
            MetaOf<T>
        >
    );

    generate_storage_alias!(
        Nft, Account<T: Config> => DoubleMap<
            (Identity, T::DecentralizedId),
            (Twox64Concat, NftOf<T>),
            bool
        >
    );

    pub fn migrate<T: Config>() -> Weight {
        let weight: Weight = 0;

        for (id, meta) in <Metadata<T>>::iter() {
            <Account<T>>::insert(meta.owner, id, true);
        }

        weight
    }
}
