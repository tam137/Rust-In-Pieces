#!/bin/bash

python3 spsa_tuner.py --engine ../engines/suprah-0.13.11 --mm ../target/release/Matt-Magie --games 300 --workers 4 --time 2 --inc 100 --mutate 10.0 --lr 15.0 --params lmr_divisor


