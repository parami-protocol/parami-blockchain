use crate::StorageVersion;
use crate::{Config, Pallet};
use frame_support::generate_storage_alias;
use frame_support::migration::*;
use frame_support::{pallet_prelude::*, traits::Get, weights::Weight};
use sp_runtime::traits::Saturating;

#[cfg(feature = "try-runtime")]
use frame_support::traits::OnRuntimeUpgradeHelpersExt;

pub fn migrate<T: Config>() -> Weight {
    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    if version < 3 {
        weight.saturating_accrue(v3::migrate::<T>());
        StorageVersion::new(3).put::<Pallet<T>>();
    }
    weight
}

pub mod v3 {
    use super::*;
    use crate::{
        AssetsOf, BalanceOf, Config, DeadlineOf, Did, EndtimeOf, HashOf, HeightOf, Metadata, NftOf,
        SlotOf,
    };
    use codec::{Decode, Encode};
    use scale_info::TypeInfo;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_runtime::RuntimeDebug;
    use sp_std::collections::btree_map::BTreeMap;
    use sp_std::prelude::*;

    use frame_support::traits::{
        tokens::fungibles::Transfer, Currency, ExistenceRequirement::AllowDeath, OnRuntimeUpgrade,
    };

    #[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct MetadataV2<A, B, D, H, N> {
        pub id: H,
        pub creator: D,
        pub pot: A,
        #[codec(compact)]
        pub budget: B,
        #[codec(compact)]
        pub remain: B,
        pub metadata: Vec<u8>,
        pub reward_rate: u16,
        pub created: N,
        pub payout_base: B,
        pub payout_min: B,
        pub payout_max: B,
    }

    #[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
    pub struct MetaInfo<T: Config> {
        pot: AccountOf<T>,
        owner_balance: BalanceOf<T>,
        pot_balance: BalanceOf<T>,
    }

    pub struct MigrateToV3<T: Config>(PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            return 0;
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<(), &'static str> {
            let version = StorageVersion::get::<Pallet<T>>();

            if version == 2 {
                generate_storage_alias!(
                    Ad, Metadata<T: Config> => Map<
                        (Identity, HashOf<T>),
                        MetadataV2<AccountOf<T>, BalanceOf<T>, DidOf<T>, HashOf<T>, HeightOf<T>>
                    >
                );
                log::info!("running pre uprade");
                let metadata_count: u32 = <Metadata<T>>::iter().count() as u32;
                log::info!("meta data count, {}", metadata_count);

                Self::set_temp_storage(metadata_count, "metadata_count");
                let slots_of_keys =
                    storage_iter::<Vec<NftOf<T>>>(<Pallet<T>>::name().as_bytes(), b"SlotsOf")
                        .count();
                assert!(slots_of_keys > 0);
                log::info!("slots of key count, {}", slots_of_keys);

                let mut iter = storage_iter::<
                    MetadataV2<AccountOf<T>, BalanceOf<T>, DidOf<T>, HashOf<T>, HeightOf<T>>,
                >(<Pallet<T>>::name().as_bytes(), b"Metadata");

                let mut ad_metas = BTreeMap::new();
                while let Some((_, ad_meta)) = iter.next() {
                    let owner_account = Did::<T>::lookup_did(ad_meta.creator).unwrap();
                    ad_metas.insert(
                        ad_meta.id,
                        MetaInfo::<T> {
                            pot: ad_meta.pot.clone(),
                            owner_balance: T::Currency::total_balance(&owner_account),
                            pot_balance: T::Currency::total_balance(&ad_meta.pot),
                        },
                    );
                }

                Self::set_temp_storage(ad_metas, "ad_metas");
            }
            Ok(())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade() -> Result<(), &'static str> {
            let version = StorageVersion::get::<Pallet<T>>();
            if version == 3 {
                log::info!("running post uprade");
                let metadata_count: Option<u32> = Self::get_temp_storage("metadata_count");
                assert_eq!(
                    <Metadata<T>>::iter().count(),
                    metadata_count.unwrap() as usize
                );
                assert_eq!(<SlotOf<T>>::iter().count(), 0);
                assert_eq!(<EndtimeOf<T>>::iter().count(), 0);
                assert_eq!(<DeadlineOf<T>>::iter().count(), 0);
                let slots_of_keys =
                    storage_iter::<Vec<NftOf<T>>>(<Pallet<T>>::name().as_bytes(), b"SlotsOf")
                        .count();
                assert_eq!(slots_of_keys, 0);

                let ad_metas: BTreeMap<HashOf<T>, MetaInfo<T>> =
                    Self::get_temp_storage("ad_metas").unwrap();
                let mut iter = <Metadata<T>>::iter();
                while let Some((_, ad_meta)) = iter.next() {
                    log::info!("ad meta: {:?}", ad_meta);
                    let meta = &ad_metas[&ad_meta.id];
                    let owner_account = Did::<T>::lookup_did(ad_meta.creator).unwrap();

                    assert_eq!(T::Currency::total_balance(&meta.pot), 0u32.into());

                    assert_eq!(
                        T::Currency::total_balance(&owner_account),
                        meta.owner_balance + meta.pot_balance
                    );
                }
            }
            Ok(())
        }
    }

    #[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct SlotV2<Balance, Hash, Height, NftId, TokenId> {
        pub ad_id: Hash,
        pub nft_id: NftId,
        pub fungible_id: Option<TokenId>,
        #[codec(compact)]
        pub budget: Balance,
        #[codec(compact)]
        pub remain: Balance,
        #[codec(compact)]
        pub fractions_remain: Balance,
        #[codec(compact)]
        pub fungibles_budget: Balance,
        #[codec(compact)]
        pub fungibles_remain: Balance,
        pub created: Height,
    }

    type AccountOf<T> = <T as frame_system::Config>::AccountId;
    type DidOf<T> = <T as parami_did::Config>::DecentralizedId;

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;

        let mut ad_id_2_meta = BTreeMap::new();

        // remove SlotsOf
        remove_storage_prefix(<Pallet<T>>::name().as_bytes(), b"SlotsOf", b"");

        <Metadata<T>>::translate_values(
            |meta: MetadataV2<AccountOf<T>, BalanceOf<T>, DidOf<T>, HashOf<T>, HeightOf<T>>| {
                weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
                ad_id_2_meta.insert(meta.id, meta.clone());

                <EndtimeOf<T>>::remove(meta.id);
                Some(crate::types::Metadata {
                    id: meta.id,
                    creator: meta.creator,
                    metadata: meta.metadata,
                    reward_rate: meta.reward_rate,
                    created: meta.created,
                    payout_base: meta.payout_base,
                    payout_min: meta.payout_min,
                    payout_max: meta.payout_max,
                })
            },
        );

        <SlotOf<T>>::translate_values(
            |slot: SlotV2<BalanceOf<T>, HashOf<T>, HeightOf<T>, NftOf<T>, AssetsOf<T>>| {
                weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

                let ad_meta = &ad_id_2_meta[&slot.ad_id];

                let owner_account = Did::<T>::lookup_did(ad_meta.creator).unwrap();

                T::Currency::transfer(&ad_meta.pot, &owner_account, slot.remain, AllowDeath)
                    .expect("transfer failed");
                T::Assets::transfer(
                    slot.nft_id,
                    &ad_meta.pot,
                    &owner_account,
                    slot.fractions_remain,
                    false,
                )
                .unwrap();

                if let Some(fungible_id) = slot.fungible_id {
                    T::Assets::transfer(
                        fungible_id,
                        &ad_meta.pot,
                        &owner_account,
                        slot.fungibles_remain,
                        false,
                    )
                    .unwrap();
                }

                crate::Pallet::<T>::deposit_event(crate::Event::End(
                    slot.nft_id,
                    slot.ad_id,
                    slot.remain,
                ));

                None
            },
        );
        weight
    }
}
