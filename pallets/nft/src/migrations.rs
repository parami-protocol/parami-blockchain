use crate::{Config, Pallet};
use frame_support::{traits::PalletInfoAccess, weights::Weight};
use sp_runtime::traits::Saturating;
use frame_support::traits::Get;

pub fn migrate<T: Config>() -> Weight {
    use frame_support::traits::StorageVersion;

    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    if version < 2 {
        weight.saturating_accrue(v2::migrate::<T>());
        StorageVersion::new(2).put::<Pallet<T>>();
    }

    if version < 3 {
        weight.saturating_accrue(v3::migrate::<T>());
        StorageVersion::new(3).put::<Pallet<T>>();
    }

    if version < 4 {
        weight.saturating_accrue(v4::migrate::<T>());
        StorageVersion::new(4).put::<Pallet<T>>();
    }

    weight
}

mod v2 {
    use super::*;

    use frame_support::storage::{migration::move_prefix, storage_prefix};

    pub fn migrate<T: Config>() -> Weight {
        let module = <Pallet<T>>::name().as_bytes();

        move_prefix(
            &storage_prefix(module, b"NftMetaStore"),
            &storage_prefix(module, b"Metadata"),
        );

        move_prefix(
            &storage_prefix(module, b"NextNftId"),
            &storage_prefix(module, b"NextClassId"),
        );

        Weight::max_value()
    }
}

mod v3 {
    use super::*;

    use crate::{MetaOf, NftOf};

    use frame_support::{generate_storage_alias, Identity, Twox64Concat};

    generate_storage_alias!(
        Nft, Metadata<T: Config> => Map<
            (Twox64Concat, NftOf<T>),
            MetaOf<T>
        >
    );

    generate_storage_alias!(
        Nft, Account<T: Config> => DoubleMap<
            (Identity, T::DecentralizedId),
            (Twox64Concat, NftOf<T>),
            bool
        >
    );

    pub fn migrate<T: Config>() -> Weight {
        let weight: Weight = 0;

        for (id, meta) in <Metadata<T>>::iter() {
            <Account<T>>::insert(meta.owner, id, true);
        }

        weight
    }
}

mod v4 {
    use super::*;

    mod old {
        use crate::{AccountOf, AssetOf, DidOf, Config, NftOf};
        use codec::{Encode, Decode};
        use frame_support::{generate_storage_alias, RuntimeDebug, Twox64Concat};
        use scale_info::TypeInfo;

        #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
        pub struct OldMetadata<Did, AccountId, NftClassId, AssetId> {
            pub owner: Did,
            pub pot: AccountId,
            pub class_id: NftClassId,
            pub minted: bool,
            pub token_asset_id: AssetId,
        }

        type OldMetaOf<T> = OldMetadata<DidOf<T>, AccountOf<T>, NftOf<T>, AssetOf<T>>;

        generate_storage_alias!(Nft, Metadata<T: Config> => Map<
            (Twox64Concat, NftOf<T>),
            OldMetaOf<T>
        >);

        pub fn iter<T>() -> frame_support::storage::PrefixIterator<(AssetOf<T>, OldMetaOf<T>)> where T: Config {
            <Metadata<T>>::iter()
        }
    }

    mod new {
        use crate::{AccountOf, AssetOf, BalanceOf, DidOf, Config, NftOf};
        use codec::{Encode, Decode};
        use frame_support::{generate_storage_alias, RuntimeDebug, Twox64Concat};
        use scale_info::TypeInfo;

        #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
        pub struct NewMetadata<Did, AccountId, NftClassId, AssetId, Balance> {
            pub owner: Did,
            pub pot: AccountId,
            pub class_id: NftClassId,
            pub minted: bool,
            pub token_asset_id: AssetId,
            pub swap_init_quote_reservation: Balance, // farming initial quote value
            pub back_up_reservation: Balance,
            pub own_reservation: Balance,
            pub farming_reward_reservation: Balance, // reserved farming reward value
        }

        type NewMetaOf<T> = NewMetadata<DidOf<T>, AccountOf<T>, NftOf<T>, AssetOf<T>, BalanceOf<T>>;

        generate_storage_alias!(Nft, Metadata<T: Config> => Map<
            (Twox64Concat, NftOf<T>),
            NewMetaOf<T>
        >);

        pub fn insert<T>(nftId: NftOf<T>, meta: NewMetaOf<T>) where T: Config {
            <Metadata<T>>::insert(nftId, meta);
        }

        pub fn count<T>() -> usize where T: Config {
            <Metadata<T>>::iter().count()
        }
    }

    pub fn migrate<T>() -> Weight where T: Config {

        let weight: Weight = 0;

        for (nft, oldMeta) in old::iter::<T>() {
            new::insert::<T>(
                nft,
                new::NewMetadata {
                    owner: oldMeta.owner,
                    pot: oldMeta.pot,
                    class_id: oldMeta.class_id,
                    minted: oldMeta.minted,
                    token_asset_id: oldMeta.token_asset_id,
                    swap_init_quote_reservation: 1_000_000u32.into(),
                    back_up_reservation: 1_000_000u32.into(),
                    own_reservation: 1_000_000u32.into(),
                    farming_reward_reservation: 7_000_000u32.into(),
            });
        }

        let count = new::count::<T>();
        // Return the weight consumed by the migration.
        T::DbWeight::get().reads_writes(count as Weight + 1, count as Weight + 1)
    }
}
