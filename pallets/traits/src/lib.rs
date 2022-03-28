#![cfg_attr(not(feature = "std"), no_std)]

mod accounts;

pub use accounts::Accounts;

mod links;

pub use links::Links;

mod swaps;

pub use swaps::Swaps;

mod tags;

pub use tags::Tags;

pub mod types {
    pub use parami_primitives::{Network, Task};
}
