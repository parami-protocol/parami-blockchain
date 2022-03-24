use crate::{Config, Pallet};
use frame_support::weights::Weight;
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
    use super::*;
    use crate::{
        DidOf, Linked as UpgradedLinked, LinksOf as UpgradedLinksOf,
        PendingOf as UpgradedPendingOf, TaskOf,
    };

    use codec::{Decode, Encode};
    use frame_support::{
        generate_storage_alias, migration::remove_storage_prefix, traits::PalletInfoAccess,
        Identity, RuntimeDebug, Twox64Concat,
    };
    use parami_primitives::Network;
    use scale_info::TypeInfo;
    use sp_std::prelude::Vec;

    #[derive(Clone, Copy, Decode, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
    pub enum AccountType {
        Unknown,

        Binance,
        Bitcoin,
        Eosio,
        Ethereum,
        Kusama,
        Polkadot,
        Solana,
        Tron,

        Discord,
        Facebook,
        Github,
        HackerNews,
        Mastodon,
        Reddit,
        Telegram,
        Twitter,
    }

    impl Into<Network> for AccountType {
        fn into(self) -> Network {
            match self {
                AccountType::Unknown => Network::Unknown,
                AccountType::Binance => Network::Binance,
                AccountType::Bitcoin => Network::Bitcoin,
                AccountType::Eosio => Network::Eosio,
                AccountType::Ethereum => Network::Ethereum,
                AccountType::Kusama => Network::Kusama,
                AccountType::Polkadot => Network::Polkadot,
                AccountType::Solana => Network::Solana,
                AccountType::Tron => Network::Tron,
                AccountType::Discord => Network::Discord,
                AccountType::Facebook => Network::Facebook,
                AccountType::Github => Network::Github,
                AccountType::HackerNews => Network::HackerNews,
                AccountType::Mastodon => Network::Mastodon,
                AccountType::Reddit => Network::Reddit,
                AccountType::Telegram => Network::Telegram,
                AccountType::Twitter => Network::Twitter,
            }
        }
    }

    generate_storage_alias!(
        Linker, LinksOf<T: Config> => DoubleMap<
            (Identity, DidOf<T>),
            (Twox64Concat, AccountType),
            Vec<u8>
        >
    );

    generate_storage_alias!(
        Linker, PendingOf<T: Config> => DoubleMap<
            (Twox64Concat, AccountType),
            (Identity, DidOf<T>),
            TaskOf<T>
        >
    );

    pub fn migrate<T: Config>() -> Weight {
        let module = <Pallet<T>>::name().as_bytes();
        remove_storage_prefix(module, b"Linked", b"");

        for (did, site, link) in <LinksOf<T>>::iter() {
            <LinksOf<T>>::remove(&did, site);

            let site: Network = site.into();
            <UpgradedLinksOf<T>>::insert(&did, site, link.clone());
            <UpgradedLinked<T>>::insert(site, link, true);
        }

        for (site, did, task) in <PendingOf<T>>::iter() {
            <PendingOf<T>>::remove(site, &did);

            let site: Network = site.into();
            <UpgradedPendingOf<T>>::insert(site, &did, task);
        }

        Weight::max_value()
    }
}
