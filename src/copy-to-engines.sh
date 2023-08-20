#!/bin/bash

cd $(dirname $0)
WORKDIR=$PWD


cargo build --release
cp ../target/release/rust_in_pieces ~/engines
