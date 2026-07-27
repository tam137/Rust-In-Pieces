#!/bin/bash

python3 spsa_tuner.py --group all --engine ../engines/suprah-0.18.1 --mm ../target/release/Matt-Magie --book ../books/Performance.bin --games 500 --workers 4 --time 2 --inc 100 --lr 3.0


