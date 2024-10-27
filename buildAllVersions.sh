#!/bin/bash

cd $(dirname $0)
WORKDIR=$PWD

cargo build --release
cargo build --target x86_64-pc-windows-gnu --release
cargo build --target=aarch64-unknown-linux-gnu --release
cargo test

mkdir -p buildVersions
cp target/release/suprah buildVersions/suprah-x86
cp target/aarch64-unknown-linux-gnu/release/suprah buildVersions/suprah-arm
cp target/x86_64-pc-windows-gnu/release/suprah.exe buildVersions
