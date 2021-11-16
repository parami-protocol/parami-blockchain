#![cfg_attr(not(feature = "std"), no_std)]

mod swaps;

pub use swaps::Swaps;

mod tags;

pub use tags::Tags;
