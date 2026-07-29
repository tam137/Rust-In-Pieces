#!/bin/bash
echo "Testing current commit..."
(echo "uci"; echo "isready"; echo "position startpos"; echo "go depth 6"; sleep 1; echo "quit") | ./target/release/suprah | grep "info depth 6"

git checkout HEAD~1 &> /dev/null
cargo build --release &> /dev/null
echo "Testing previous commit..."
(echo "uci"; echo "isready"; echo "position startpos"; echo "go depth 6"; sleep 1; echo "quit") | ./target/release/suprah | grep "info depth 6"

git checkout master &> /dev/null
cargo build --release &> /dev/null
