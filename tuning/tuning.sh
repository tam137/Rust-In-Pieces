#!/bin/bash

python3 spsa_tuner.py --engine ../engines/suprah-0.15.1 --mm ../target/release/Matt-Magie --games 500 --workers 4 --time 2 --inc 100 --mutate 4.0 --lr 3.0


