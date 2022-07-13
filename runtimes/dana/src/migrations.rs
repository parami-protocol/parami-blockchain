use crate::VERSION;
use frame_support::storage::migration::{remove_storage_prefix, storage_key_iter};
use frame_support::storage::PrefixIterator;
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::weights::Weight;
use sp_std::prelude::*;

const DEPRECATED_PALLETS: &'static [&'static [u8]] = &[
    b"Staking",
    b"Authorship",
    b"Session",
    b"ImOnline",
    b"AuthorityDiscovery",
    b"Offences",
    b"Historical",
    b"BagsList",
    b"ChildBounties",
    b"PhragmenElection",
    b"Bounties",
    b"Contracts",
    b"ElectionProviderMultiPhase",
    b"RandomnessCollectiveFlip",
    b"Recovery",
    b"Society",
    b"Vesting",
    b"NominationPools",
];

pub struct RemoveDeprecatedPallets;

impl OnRuntimeUpgrade for RemoveDeprecatedPallets {
    fn on_runtime_upgrade() -> Weight {
        if VERSION.spec_version > 334 {
            return 0;
        }

        for module in DEPRECATED_PALLETS {
            let key = sp_io::hashing::twox_128(module);
            let result = frame_support::storage::unhashed::kill_prefix(&key, None);
            match result {
                sp_io::KillStorageResult::AllRemoved(i) => log::info!("all removed, {:?}", i),
                sp_io::KillStorageResult::SomeRemaining(i) => log::info!("some remain, {:?}", i),
            }
        }
        1
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<(), &'static str> {
        use core::str;
        let modules: Vec<&[u8]> = vec![b"Staking"];

        if VERSION.spec_version > 334 {
            return Ok(());
        }

        for module in DEPRECATED_PALLETS {
            log::info!(
                "RemoveDeprecatedPallet, module = {:?}, key_count: {:?}",
                str::from_utf8(module),
                pallet_key_count(module),
            );
        }
        Ok(())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade() -> Result<(), &'static str> {
        if VERSION.spec_version > 334 {
            return Ok(());
        }

        for module in DEPRECATED_PALLETS {
            assert_eq!(pallet_key_count(module), 0);
        }
        Ok(())
    }
}

pub fn pallet_key_count(module: &[u8]) -> usize {
    let mut prefix = Vec::new();
    let key = sp_io::hashing::twox_128(module);
    prefix.extend_from_slice(&key);

    let previous_key = prefix.clone();
    let closure = |_raw_key_without_prefix: &[u8], mut _raw_value: &[u8]| Ok(());
    PrefixIterator::<()>::new(prefix, previous_key, closure).count()
}
