#!/usr/bin/env sh

set -e

cargo test --no-default-features
cargo test --all-features
wasm-pack test --node -- --no-default-features
wasm-pack test --node
wasm-pack test --node -- --all-features
wasm-pack test --firefox --headless -- --no-default-features
wasm-pack test --firefox --headless
wasm-pack test --firefox --headless -- --all-features
wasm-pack test --chrome --headless -- --no-default-features
wasm-pack test --chrome --headless
wasm-pack test --chrome --headless -- --all-features
# wasm-pack test --safari --headless -- --no-default-features
# wasm-pack test --safari --headless
# wasm-pack test --safari --headless -- --all-features
