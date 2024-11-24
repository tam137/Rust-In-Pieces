#!/bin/bash

cd $(dirname $0)
WORKDIR=$PWD

cargo test
if [ $? -ne 0 ]; then
  echo "Tests failed. Aborting build process."
  exit 1
fi

cargo build --release
cargo build --target x86_64-pc-windows-gnu --release
cargo build --target=aarch64-unknown-linux-gnu --release

mkdir -p buildVersions

cp target/release/suprah buildVersions/suprah-x86
cp target/aarch64-unknown-linux-gnu/release/suprah buildVersions/suprah-arm
cp target/x86_64-pc-windows-gnu/release/suprah.exe buildVersions

echo "Build and copy process completed successfully."

