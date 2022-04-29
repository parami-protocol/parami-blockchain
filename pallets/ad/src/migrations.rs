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
    use crate::{types, BalanceOf, Config, HashOf, HeightOf, NftOf, SlotOf as UpgradedSlotOf};
    use codec::{Decode, Encode};
    use frame_support::{generate_storage_alias, RuntimeDebug, Twox64Concat};
    use scale_info::TypeInfo;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_runtime::traits::Zero;

    #[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub struct Slot<B, H, N, T> {
        pub nft: T,
        #[codec(compact)]
        pub budget: B,
        #[codec(compact)]
        pub remain: B,
        #[codec(compact)]
        pub tokens: B,
        pub created: N,
        pub ad: H,
    }

    generate_storage_alias!(
        Ad, SlotOf<T: Config> => Map<
            (Twox64Concat, NftOf<T>),
            Slot<BalanceOf<T>, HashOf<T>, HeightOf<T>, NftOf<T>>
        >
    );

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;

        for (nft, slot) in <SlotOf<T>>::iter() {
            <SlotOf<T>>::remove(nft);

            <UpgradedSlotOf<T>>::insert(
                nft,
                types::Slot {
                    ad_id: slot.ad,
                    nft_id: slot.nft,
                    fungible_id: None,
                    budget: slot.budget,
                    remain: slot.remain,
                    fractions_remain: slot.tokens,
                    fungibles_budget: Zero::zero(),
                    fungibles_remain: Zero::zero(),
                    created: slot.created,
                },
            );

            weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 2));
        }

        weight
    }
}
