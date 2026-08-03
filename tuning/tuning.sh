#!/bin/bash

python3 spsa_tuner.py --group all --engine ../engines/suprah-0.23.12-nnue --mm ../target/release/Matt-Magie --book ../books/Performance.bin --games 2500 --workers 4 --time 1 --inc 10 --mutate 10.0 --lr 5.0


