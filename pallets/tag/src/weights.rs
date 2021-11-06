// This file is part of Substrate.

// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for parami_tag
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-11-08, STEPS: `2`, REPEAT: 50, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/parami
// benchmark
// --chain=dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet=parami_tag
// --extrinsic=*
// --steps=2
// --repeat=50
// --template=./.maintain/frame-weight-template.hbs
// --output=./pallets/tag/src/weights.rs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for parami_tag.
pub trait WeightInfo {
    fn create(n: u32, ) -> Weight;
    fn force_create(n: u32, ) -> Weight;
}

/// Weights for parami_tag using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
    // Storage: Did DidOf (r:1 w:0)
    // Storage: Balances Reserves (r:1 w:0)
    // Storage: Tag Metadata (r:1 w:1)
    fn create(n: u32, ) -> Weight {
        (36_622_000 as Weight)
            // Standard Error: 0
            .saturating_add((4_000 as Weight).saturating_mul(n as Weight))
            .saturating_add(T::DbWeight::get().reads(3 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
    // Storage: Tag Metadata (r:1 w:1)
    fn force_create(n: u32, ) -> Weight {
        (15_000_000 as Weight)
            // Standard Error: 0
            .saturating_add((4_000 as Weight).saturating_mul(n as Weight))
            .saturating_add(T::DbWeight::get().reads(1 as Weight))
            .saturating_add(T::DbWeight::get().writes(1 as Weight))
    }
}

// For backwards compatibility and tests
impl WeightInfo for () {
    // Storage: Did DidOf (r:1 w:0)
    // Storage: Balances Reserves (r:1 w:0)
    // Storage: Tag Metadata (r:1 w:1)
    fn create(n: u32, ) -> Weight {
        (36_622_000 as Weight)
            // Standard Error: 0
            .saturating_add((4_000 as Weight).saturating_mul(n as Weight))
            .saturating_add(RocksDbWeight::get().reads(3 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
    // Storage: Tag Metadata (r:1 w:1)
    fn force_create(n: u32, ) -> Weight {
        (15_000_000 as Weight)
            // Standard Error: 0
            .saturating_add((4_000 as Weight).saturating_mul(n as Weight))
            .saturating_add(RocksDbWeight::get().reads(1 as Weight))
            .saturating_add(RocksDbWeight::get().writes(1 as Weight))
    }
}
