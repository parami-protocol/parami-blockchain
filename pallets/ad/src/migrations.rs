use crate::{Config, Pallet};
use frame_support::migration::remove_storage_prefix;
use frame_support::{pallet_prelude::*, traits::Get, weights::Weight};
use sp_runtime::traits::Saturating;
pub fn migrate<T: Config>() -> Weight {
    use frame_support::traits::StorageVersion;

    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    if version < 3 {
        weight.saturating_accrue(v3::migrate::<T>());
        StorageVersion::new(3).put::<Pallet<T>>();
    }
    weight
}

mod v3 {
    use super::*;
    use crate::{
        AssetsOf, BalanceOf, Config, Did, EndtimeOf, HashOf, HeightOf, Metadata, NftOf, SlotOf,
    };
    use codec::{Decode, Encode};
    use scale_info::TypeInfo;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_runtime::RuntimeDebug;
    use sp_std::collections::btree_map::BTreeMap;
    use sp_std::prelude::*;

    use frame_support::traits::{
        tokens::fungibles::Transfer, Currency, ExistenceRequirement::AllowDeath,
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
