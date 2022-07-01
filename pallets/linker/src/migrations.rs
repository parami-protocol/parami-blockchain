use crate::{Config, Pallet};
use frame_support::weights::Weight;

pub fn migrate<T: Config>() -> Weight {
    use frame_support::traits::StorageVersion;

    let _version = StorageVersion::get::<Pallet<T>>();
    let weight: Weight = 0;

    weight
}
