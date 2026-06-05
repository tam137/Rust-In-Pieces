#!/bin/bash

python3 spsa_tuner.py --engine ../engines/suprah-0.13.11 --mm ../target/release/Matt-Magie --games 300 --workers 3 --time 1 --inc 65 --mutate 10.0 --lr 5.0 --params lmr_divisor


