#!/bin/bash

python3 spsa_tuner.py --engine ../engines/suprah-0.13.3 --mm ../target/release/Matt-Magie --games 750 --workers 3 --time 1 --inc 65 --mutate 3.0 --lr 15.0
