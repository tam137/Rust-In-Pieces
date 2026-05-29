#!/bin/bash

python3 spsa_tuner.py --engine ../engines/suprah-0.11.4 --mm ../target/release/Matt-Magie --workers 2 --time 1 --inc 80 --mutate 5.0
