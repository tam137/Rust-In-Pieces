#!/bin/bash

python3 spsa_tuner.py --engine ../engines/suprah-0.15.1 --mm ../target/release/Matt-Magie --games 300 --workers 4 --time 2 --inc 100 --mutate 10.0 --lr 15.0 --params king_pawn_shield_kingside,king_pawn_shield_queenside,king_piece_shield_kingside,king_piece_shield_queenside,connected_passed_pawn_mg,connected_passed_pawn_eg,knight_outpost_true_mg,knight_outpost_true_eg,bishop_outpost_true_mg,bishop_outpost_true_eg,opposite_bishops_draw_scale,rook_behind_enemy_passed_pawn_mg,rook_behind_enemy_passed_pawn_eg


