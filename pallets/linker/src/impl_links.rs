use crate::{Config, DidOf, LinksOf, Pallet};

use parami_traits::{types::Network, Links};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

impl<T: Config> Links for Pallet<T> {
    type DecentralizedId = DidOf<T>;

    fn all_links(did: Self::DecentralizedId) -> BTreeMap<Network, Vec<Vec<u8>>> {
        let mut links = BTreeMap::<Network, Vec<Vec<u8>>>::new();

        for (network, link) in <LinksOf<T>>::iter_prefix(&did) {
            links.entry(network).or_default().push(link);
        }

        links
    }

    fn links(did: Self::DecentralizedId, network: Network) -> Vec<Vec<u8>> {
        <LinksOf<T>>::get(&did, network)
            .map(|link| vec![link])
            .unwrap_or_default()
    }
}
