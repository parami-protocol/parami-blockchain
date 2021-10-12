use frame_support::{pallet_prelude::PhantomData, weights::Weight};

pub trait WeightInfo {
    fn set_threshold() -> Weight;

    fn set_resource() -> Weight;

    fn remove_resource() -> Weight;

    fn whitelist_chain() -> Weight;

    fn add_relayer() -> Weight;

    fn remove_relayer() -> Weight;

    fn acknowledge_proposal(dispatch_weight: Weight) -> Weight;

    fn reject_proposal() -> Weight;

    fn eval_vote_state(dispatch_weight: Weight) -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn set_threshold() -> Weight {
        195_000_000 as Weight
    }

    fn set_resource() -> Weight {
        195_000_000 as Weight
    }

    fn remove_resource() -> Weight {
        195_000_000 as Weight
    }

    fn whitelist_chain() -> Weight {
        195_000_000 as Weight
    }

    fn add_relayer() -> Weight {
        195_000_000 as Weight
    }

    fn remove_relayer() -> Weight {
        195_000_000 as Weight
    }

    fn acknowledge_proposal(dispatch_weight: Weight) -> Weight {
        (195_000_000 as Weight).saturating_add(dispatch_weight)
    }

    fn reject_proposal() -> Weight {
        195_000_000 as Weight
    }

    fn eval_vote_state(dispatch_weight: Weight) -> Weight {
        (195_000_000 as Weight).saturating_add(dispatch_weight)
    }
}
