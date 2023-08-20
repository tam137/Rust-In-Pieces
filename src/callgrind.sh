#!/bin/bash

cd $(dirname $0)
WORKDIR=$PWD


valgrind --tool=callgrind --callgrind-out-file=callgrind.out ../target/debug/rust_in_pieces
