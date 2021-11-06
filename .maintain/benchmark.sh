#!/bin/bash

pallet=$1
steps=${2:-50}
repeat=${3:-20}

echo "Benchmarking ${pallet} steps ${steps} repeat ${repeat}..."

./target/release/parami benchmark \
--chain=dev \
--execution=wasm \
--wasm-execution=compiled \
--pallet="parami_${pallet}" \
--extrinsic='*' \
--steps=$steps \
--repeat=$repeat \
--template="./.maintain/frame-weight-template.hbs" \
--output="./pallets/${pallet}/src/weights.rs"
