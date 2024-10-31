#!/bin/bash

cd $(dirname $0)
WORKDIR=$PWD

cargo test && cargo test -- --ignored
