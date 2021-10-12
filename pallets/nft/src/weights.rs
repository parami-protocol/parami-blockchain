use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for parami_nft.
pub trait WeightInfo {
    fn mint(i: u32) -> Weight;
}

/// Weights for parami_did using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    fn mint(i: u32) -> Weight {
        (456_053_000 as Weight)
            .saturating_add((29_136_000 as Weight).saturating_mul(i as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(3 as Weight))
            .saturating_add(T::DbWeight::get().writes((2 as Weight).saturating_mul(i as Weight)))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    fn mint(i: u32) -> Weight {
        (456_053_000 as Weight)
            .saturating_add((29_136_000 as Weight).saturating_mul(i as Weight))
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes((2 as Weight).saturating_mul(i as Weight)))
    }
}
