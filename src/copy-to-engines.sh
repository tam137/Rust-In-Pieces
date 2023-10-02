#!/bin/bash

cd $(dirname $0)
WORKDIR=$PWD


cargo build --release
cargo build
cp ../target/release/rust_in_pieces ~/engines
