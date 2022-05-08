use crate::{Config, DidOf, LinksOf, Pallet};

use parami_traits::{types::Network, Links};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

impl<T: Config> Links<DidOf<T>> for Pallet<T> {
    fn all_links(did: &DidOf<T>) -> BTreeMap<Network, Vec<Vec<u8>>> {
        let mut links = BTreeMap::<Network, Vec<Vec<u8>>>::new();

        for (network, link) in <LinksOf<T>>::iter_prefix(did) {
            links.entry(network).or_default().push(link);
        }

        links
    }

    fn links(did: &DidOf<T>, network: Network) -> Vec<Vec<u8>> {
        <LinksOf<T>>::get(did, network)
            .map(|link| vec![link])
            .unwrap_or_default()
    }
}
