use crate::model::{WHITE_PAWN, WHITE_ROOK, WHITE_KNIGHT, WHITE_BISHOP, WHITE_QUEEN, WHITE_KING, BLACK_PAWN, BLACK_ROOK, BLACK_KNIGHT, BLACK_BISHOP, BLACK_QUEEN, BLACK_KING};

pub const PIECE_EVAL_PAWN: i16 = 100;
pub const PIECE_EVAL_ROOK: i16 = 500;
pub const PIECE_EVAL_KNIGHT: i16 = 300;
pub const PIECE_EVAL_BISHOP: i16 = 300;
pub const PIECE_EVAL_QUEEN: i16 = 950;
pub const PIECE_EVAL_KING: i16 = 10000;

const PAWN_PST: [i16; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0
];

const KNIGHT_PST: [i16; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50
];

const BISHOP_PST: [i16; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20
];

const ROOK_PST: [i16; 64] = [
      0,  0,  0,  0,  0,  0,  0,  0,
      5, 10, 10, 10, 10, 10, 10,  5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
      0,  0,  0,  5,  5,  0,  0,  0
];

const QUEEN_PST: [i16; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20
];

const KING_MIDDLEGAME_PST: [i16; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20
];

const KING_ENDGAME_PST: [i16; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50
];

const fn flip_sq(sq: usize) -> usize {
    let rank = sq / 8;
    let file = sq % 8;
    (7 - rank) * 8 + file
}

const fn build_pst_mg() -> [[i16; 64]; 12] {
    let mut pst = [[0; 64]; 12];
    let mut sq = 0;
    while sq < 64 {
        let p_w = (PAWN_PST[flip_sq(sq)] * 8) / 10 + PIECE_EVAL_PAWN;
        let p_b = -((PAWN_PST[sq] * 8) / 10 + PIECE_EVAL_PAWN);
        pst[WHITE_PAWN][sq] = p_w;
        pst[BLACK_PAWN][sq] = p_b;

        let r_w = (ROOK_PST[flip_sq(sq)] * 8) / 10 + PIECE_EVAL_ROOK;
        let r_b = -((ROOK_PST[sq] * 8) / 10 + PIECE_EVAL_ROOK);
        pst[WHITE_ROOK][sq] = r_w;
        pst[BLACK_ROOK][sq] = r_b;

        let n_w = (KNIGHT_PST[flip_sq(sq)] * 8) / 10 + PIECE_EVAL_KNIGHT;
        let n_b = -((KNIGHT_PST[sq] * 8) / 10 + PIECE_EVAL_KNIGHT);
        pst[WHITE_KNIGHT][sq] = n_w;
        pst[BLACK_KNIGHT][sq] = n_b;

        let b_w = (BISHOP_PST[flip_sq(sq)] * 8) / 10 + PIECE_EVAL_BISHOP;
        let b_b = -((BISHOP_PST[sq] * 8) / 10 + PIECE_EVAL_BISHOP);
        pst[WHITE_BISHOP][sq] = b_w;
        pst[BLACK_BISHOP][sq] = b_b;

        let q_w = (QUEEN_PST[flip_sq(sq)] * 8) / 10 + PIECE_EVAL_QUEEN;
        let q_b = -((QUEEN_PST[sq] * 8) / 10 + PIECE_EVAL_QUEEN);
        pst[WHITE_QUEEN][sq] = q_w;
        pst[BLACK_QUEEN][sq] = q_b;

        let k_w = (KING_MIDDLEGAME_PST[flip_sq(sq)] * 8) / 10 + PIECE_EVAL_KING;
        let k_b = -((KING_MIDDLEGAME_PST[sq] * 8) / 10 + PIECE_EVAL_KING);
        pst[WHITE_KING][sq] = k_w;
        pst[BLACK_KING][sq] = k_b;

        sq += 1;
    }
    pst
}

const fn build_pst_eg() -> [[i16; 64]; 12] {
    let mut pst = build_pst_mg();
    let mut sq = 0;
    while sq < 64 {
        let k_w = (KING_ENDGAME_PST[flip_sq(sq)] * 8) / 10 + PIECE_EVAL_KING;
        let k_b = -((KING_ENDGAME_PST[sq] * 8) / 10 + PIECE_EVAL_KING);
        pst[WHITE_KING][sq] = k_w;
        pst[BLACK_KING][sq] = k_b;
        sq += 1;
    }
    pst
}

pub const PST_MG: [[i16; 64]; 12] = build_pst_mg();
pub const PST_EG: [[i16; 64]; 12] = build_pst_eg();
