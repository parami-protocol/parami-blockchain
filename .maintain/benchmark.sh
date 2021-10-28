#!/bin/bash

./target/release/parami benchmark \
--chain dev \
--execution wasm \
--wasm-execution compiled \
--pallet parami_$1 \
--extrinsic '*' \
--steps 50 \
--repeat 20 \
--template=./.maintain/frame-weight-template.hbs \
--output ./pallets/$1/src/weights.rs
