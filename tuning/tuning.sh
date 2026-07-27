#!/bin/bash

cd "$(dirname "$0")"

python3 spsa_tuner.py \
  --group search_and_ordering \
  --engine ../engines/suprah-0.20.1 \
  --mm ../target/release/Matt-Magie \
  --book ../books/Performance.bin \
  --games 800 \
  --workers 3 \
  --time 0.5 \
  --inc 25 \
  --lr 2.0
