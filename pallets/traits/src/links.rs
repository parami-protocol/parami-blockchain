use crate::types::Network;

use sp_std::{collections::btree_map::BTreeMap, prelude::*};

type Profile = Vec<u8>;
type Map = BTreeMap<Network, Vec<Profile>>;

pub trait Links<DecentralizedId> {
    fn all_links(did: &DecentralizedId) -> Map;

    fn links(did: &DecentralizedId, network: Network) -> Vec<Profile>;
}

impl<DecentralizedId> Links<DecentralizedId> for () {
    fn all_links(_did: &DecentralizedId) -> Map {
        BTreeMap::new()
    }

    fn links(_did: &DecentralizedId, _network: Network) -> Vec<Profile> {
        Vec::new()
    }
}
