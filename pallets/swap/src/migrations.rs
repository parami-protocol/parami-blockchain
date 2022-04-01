use crate::{Config, Pallet};
use frame_support::{traits::Get, weights::Weight};
use sp_runtime::traits::Saturating;

pub fn migrate<T: Config>() -> Weight {
    use frame_support::traits::StorageVersion;

    let version = StorageVersion::get::<Pallet<T>>();
    let mut weight: Weight = 0;

    if version < 2 {
        weight.saturating_accrue(v2::migrate::<T>());
        StorageVersion::new(2).put::<Pallet<T>>();
    }

    weight
}

mod v2 {

    use crate::{HeightOf, BalanceOf};
    use super::*;
    pub mod old {
        use codec::{Encode, Decode};
        use frame_support::{generate_storage_alias, Twox64Concat, RuntimeDebug};
        use scale_info::TypeInfo;

        use crate::{HeightOf, BalanceOf, Config, AssetOf};

        generate_storage_alias!(Swap, Metadata<T: Config> => Map<(Twox64Concat, AssetOf<T>), OldSwapOf<T>>);

        pub(crate) type OldSwapOf<T> = OldSwap<HeightOf<T>, BalanceOf<T>>;

        #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
        pub struct OldSwap<N, B> {
            pub created: N,
            pub liquidity: B,
        }

        pub(crate) fn iter<T>() -> frame_support::storage::PrefixIterator<(AssetOf<T>, OldSwapOf<T>)> where T: Config {
            Metadata::<T>::iter()  
        }
    }

    pub mod new {
        use codec::{Decode, Encode};
        use frame_support::{generate_storage_alias, Twox64Concat, RuntimeDebug};
        use scale_info::TypeInfo;
        use crate::{AssetOf, Config, HeightOf, BalanceOf};

        generate_storage_alias!(Swap, Metadata<T: Config> => Map<(Twox64Concat, AssetOf<T>), NewSwapOf<T>>);

        pub(crate) type NewSwapOf<T> = NewSwap<HeightOf<T>, BalanceOf<T>>;

        #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, TypeInfo)]
        pub struct NewSwap<N, B> {
            pub created: N,
            pub liquidity: B,
            pub initial_quote: B,
            pub farming_reward_quote: B,
        }

        pub(super) fn insert<T, N, R>(token_id: AssetOf<T>, newSwap: NewSwapOf<T>) where T: Config {
            <Metadata<T>>::insert(
                token_id,
                newSwap
            );
        }

        pub(crate) fn count<T>() -> usize where T: Config {
            <Metadata<T>>::iter().count()
        }
    }

    pub fn migrate<T>() -> Weight where T: Config {
        
        for (token_id, old_meta) in old::iter::<T>() {
            new::insert::<T, HeightOf<T>, BalanceOf<T>>(
                token_id, 
                new::NewSwap {
                    created: old_meta.created,
                    liquidity: old_meta.liquidity,
                    initial_quote: 1_000_000u32.into(),
                    farming_reward_quote: 7_000_000u32.into(),
                }
            );
        }

        let count = new::count::<T>();
        // Return the weight consumed by the migration.
        T::DbWeight::get().reads_writes(count as Weight + 1, count as Weight + 1)
    }
}