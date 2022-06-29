use crate::StorageVersion;
use crate::{Config, Pallet};
use frame_support::migration::*;
use frame_support::{pallet_prelude::*, traits::Get, weights::Weight};
use sp_runtime::traits::Saturating;

#[cfg(feature = "try-runtime")]
use frame_support::traits::OnRuntimeUpgradeHelpersExt;

pub fn migrate<T: Config>() -> Weight {
    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    weight
}
