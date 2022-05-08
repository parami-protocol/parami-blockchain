use crate::{Config, Pallet};
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
    use crate::{types::Score, InfluencesOf, PersonasOf};

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;

        <PersonasOf<T>>::translate_values(|score| {
            Some(Pallet::<T>::accrue(&Score::default(), score))
        });
        <InfluencesOf<T>>::translate_values(|score| {
            Some(Pallet::<T>::accrue(&Score::default(), score))
        });

        weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

        weight
    }
}
