use crate::{Config, Metadata, Pallet};
use frame_support::{traits::Get, weights::Weight};
use sp_runtime::traits::Saturating;

pub fn migrate<T: Config>() -> Weight {
    use frame_support::traits::StorageVersion;

    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    if version < 1 {
        weight.saturating_accrue(v1::migrate::<T>());
        StorageVersion::new(1).put::<Pallet<T>>();
    }

    weight
}

mod v1 {
    use super::*;

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;
        <Metadata<T>>::translate(|_did, meta| {
            weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
            Some(meta)
        });

        weight
    }
}
