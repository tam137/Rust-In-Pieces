#!/bin/bash

cd $(dirname $0)
WORKDIR=$PWD

cargo build --release
/home/tam137/git/rust-in-pieces/target/release/rust_in_pieces "unittest"
