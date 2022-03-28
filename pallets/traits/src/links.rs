use crate::types::Network;

use codec::MaxEncodedLen;
use frame_support::Parameter;
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

pub trait Links {
    type DecentralizedId: Parameter + Member + MaybeSerializeDeserialize + MaxEncodedLen;

    fn all_links(did: Self::DecentralizedId) -> BTreeMap<Network, Vec<Vec<u8>>>;

    fn links(did: Self::DecentralizedId, network: Network) -> Vec<Vec<u8>>;
}
