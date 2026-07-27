#!/bin/bash

python3 spsa_tuner.py --group all --engine ../engines/suprah-0.20.0 --mm ../target/release/Matt-Magie --book ../books/Performance.bin --games 1000 --workers 3 --time 1 --inc 60 --lr 3.0


