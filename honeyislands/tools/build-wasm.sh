#!/bin/bash
# Build the Rust engine for the web worker and run the tests.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTFLAGS="--remap-path-prefix=$(pwd)=/source --remap-path-prefix=$HOME=/build"
cargo build --profile wasm --offline --manifest-path rust/Cargo.toml --target wasm32-unknown-unknown --lib
cp rust/target/wasm32-unknown-unknown/wasm/hib_engine.wasm analyze/wasm/hib_engine.wasm
ls -l analyze/wasm/hib_engine.wasm
