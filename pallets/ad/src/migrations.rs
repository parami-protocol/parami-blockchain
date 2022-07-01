use crate::StorageVersion;
use crate::{Config, Pallet};
use frame_support::weights::Weight;

#[cfg(feature = "try-runtime")]
use frame_support::traits::OnRuntimeUpgradeHelpersExt;

pub fn migrate<T: Config>() -> Weight {
    let _version = StorageVersion::get::<Pallet<T>>();
    let weight: Weight = 0;

    weight
}
