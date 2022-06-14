use crate::{Config, Pallet};
use frame_support::{traits::Get, weights::Weight};
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
    use crate::{AssetsOf, BalanceOf, Config, HashOf, HeightOf, Metadata, NftOf, SlotOf};
    use codec::{Decode, Encode};
    use scale_info::TypeInfo;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_runtime::RuntimeDebug;
    use sp_std::prelude::*;

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

        //TODO: refund all ad3s
        <Metadata<T>>::translate_values(
            |_meta: MetadataV2<AccountOf<T>, BalanceOf<T>, DidOf<T>, HashOf<T>, HeightOf<T>>| {
                weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

                None
            },
        );

        <SlotOf<T>>::translate_values(
            |_slot: SlotV2<BalanceOf<T>, HashOf<T>, HeightOf<T>, NftOf<T>, AssetsOf<T>>| {
                weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

                None
            },
        );
        weight
    }
}
