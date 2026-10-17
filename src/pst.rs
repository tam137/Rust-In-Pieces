use crate::model::{WHITE_PAWN, WHITE_KING, BLACK_PAWN};

pub const PIECE_EVAL_PAWN: i16 = 100;
pub const PIECE_EVAL_ROOK: i16 = 500;
pub const PIECE_EVAL_KNIGHT: i16 = 300;
pub const PIECE_EVAL_BISHOP: i16 = 300;
pub const PIECE_EVAL_QUEEN: i16 = 950;
pub const PIECE_EVAL_KING: i16 = 10000;

// The evaluation's material and piece-square tables (`task.md` 29.5 E2). The `PIECE_EVAL_*` values
// above serve the Static Exchange Evaluation, move ordering and the pruning margins and are not
// tuned; the block below is what the Texel tuner fits and rewrites. Until v0.46.4 the tables were
// the Chess Programming Wiki's "Simplified Evaluation Function" times 0.8, the same in both phases
// except for the king, with the `PIECE_EVAL_*` values as material.
// BEGIN TEXEL GENERATED - written by `suprah texel-export` (feature `texel`); re-tune, do not edit.
/// Material per piece in `WHITE_PAWN..=WHITE_KING` order: pawn, rook, knight, bishop, queen, king.
pub const MATERIAL_MG: [i16; 6] = [116, 782, 543, 580, 1883, 10000];
pub const MATERIAL_EG: [i16; 6] = [83, 565, 309, 329, 972, 10000];
/// Piece-square tables without material, from White's side, rank 8 first (middlegame).
pub const TABLE_MG: [[i16; 64]; 6] = [
    [ // pawn
           0,    0,    0,    0,    0,    0,    0,    0,
          40,   40,   40,   40,   40,   40,   40,   40,
           8,    8,   16,   24,   24,   16,    8,    8,
           4,    4,    8,   20,   20,    8,    4,    4,
           0,    0,    0,   16,   16,    0,    0,    0,
           4,   -4,   -8,    0,    0,   -8,   -4,    4,
           4,    8,    8,  -16,  -16,    8,    8,    4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    [ // rook
          51,   41,   43,   50,   53,   36,   26,   49,
         -43,  -44,   32,   37,   44,   67,  -26,   24,
          -7,    4,   25,   76,  103,   77,   44,   68,
         -58,  -47,    1,    8,   24,   50,   31,   53,
         -66,  -82,   -5,   47,  -23,   -9,   -2,   33,
         -87,  -55,  -45,  -18,   -5,  -10,   -1,  -27,
        -110,  -76,  -25,   -7,  -22,    9,    4,  -85,
         -63,  -41,  -16,  -10,    4,  -22,  -14,  -42,
    ],
    [ // knight
        -100,  -21,  -40,   -6,   -6,  -36,  -28,  -69,
           2,  -65,   22,   17,  -21,   75,   29,  -18,
         -15,  -16,  -73,   82,  138,   47,   65,  -15,
         -42,  -23,   15,   31,  -15,   49,   -5,   84,
         -63,   -9,  -19,   -8,   -6,   13,  -13,   27,
         -89,  -31,   -5,   -7,  -19,   -5,   14,  -68,
         -48,  -54,  -13,  -23,   -9,  -34,   14,    3,
         -86,  -40,  -35,   -3,  -27,  -19,   11,  -39,
    ],
    [ // bishop
         -37,  -27,  -10,  -24,   -5,    6,   12,  -47,
         -37,  -79,  -48,    3,   22,   -4,  -17,  -37,
         -51,  -37,  -17,   -4,   42,  136,   47,   81,
         -48,  -13,   -8,   52,   24,   36,  -15,  -16,
         -18,  -23,    3,   30,   50,   -4,   16,    1,
          -3,    9,   -1,   16,  -11,   13,   -2,   35,
         -12,   37,   67,  -49,  -13,  -11,   66,  -12,
         -41,   18,  -28,  -30,  -42,  -42,  -18,    7,
    ],
    [ // queen
         -37,  -10,   11,   26,   46,   64,   44,   36,
         -54,  -68,    4,   17,   33,   66,    5,   92,
         -66,  -18,   17,   22,   55,  141,   95,   89,
         -41,  -51,    2,   -6,   14,   29,   28,   24,
         -40,  -62,  -31,   -1,  -33,   22,   -8,    0,
         -76,  -12,   27,   -7,  -18,  -10,   -3,   -8,
         -59,  -37,   32,   -3,   -7,   -7,   31,  -18,
         -67,  -67,  -35,  -18,  -41,  -75,  -70,  -53,
    ],
    [ // king
         -24,  -32,  -32,  -40,  -40,  -32,  -32,  -24,
         -24,  -32,  -32,  -40,  -40,  -32,  -32,  -24,
         -24,  -32,  -32,  -40,  -40,  -32,  -32,  -24,
         -24,  -32,  -32,  -40,  -40,  -32,  -32,  -24,
         -16,  -24,  -24,  -32,  -32,  -24,  -24,  -16,
          -8,  -16,  -16,  -16,  -16,  -16,  -16,   -8,
          16,   16,    0,    0,    0,    0,   16,   16,
          16,   24,    8,    0,    0,    8,   24,   16,
    ],
];
/// Piece-square tables without material, from White's side, rank 8 first (endgame).
pub const TABLE_EG: [[i16; 64]; 6] = [
    [ // pawn
           0,    0,    0,    0,    0,    0,    0,    0,
          40,   40,   40,   40,   40,   40,   40,   40,
           8,    8,   16,   24,   24,   16,    8,    8,
           4,    4,    8,   20,   20,    8,    4,    4,
           0,    0,    0,   16,   16,    0,    0,    0,
           4,   -4,   -8,    0,    0,   -8,   -4,    4,
           4,    8,    8,  -16,  -16,    8,    8,    4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    [ // rook
          24,   38,   42,   24,   24,   43,   53,   50,
          -4,  -18,  -38,  -68,  -57,  -37,  -12,  -22,
          32,   31,   20,  -12,  -16,    1,   12,   12,
          44,   44,   39,   17,  -27,  -14,   17,   -5,
          32,   42,   15,  -27,   -7,    2,    7,   -4,
          21,    2,    6,  -29,  -26,  -26,  -28,   -5,
          29,  -11,    4,  -12,  -22,  -48,  -68,    3,
          13,  -27,    6,   12,  -28,   17,  -33,  -23,
    ],
    [ // knight
         -81,  -32,  -13,   -2,  -17,  -23,  -31, -100,
         -13,    6,   29,    3,    5,  -27,  -25,  -43,
         -11,   22,   32,  -22,   -8,  -36,   22,  -20,
         -14,   67,   16,   31,   12,   -8,   15,  -28,
           7,    4,   14,    8,   27,    0,    4,  -43,
         -13,   12,    7,   30,   31,  -37,  -16,    8,
         -38,  -21,  -24,  -16,    1,    6,   -5,  -38,
         -58,  -21,  -36,  -16,  -17,  -48,  -17,  -53,
    ],
    [ // bishop
          -6,   10,   -8,  -10,    6,    2,    5,  -22,
          16,   33,    2,    1,  -16,   27,   19,  -28,
           5,   44,   27,  -14,   -2,   -6,    9,  -10,
          24,   37,    3,   26,    8,   17,   21,  -13,
         -31,    1,   28,   40,    5,   -1,  -19,  -50,
         -29,  -20,    9,   13,   22,  -26,  -53,   -6,
         -27,  -48,  -24,   40,   -2,    0,    3,  -23,
         -40,   -4,  -14,  -12,   -5,   -9,  -21,  -18,
    ],
    [ // queen
         -33,  -16,    1,   18,   41,   53,   18,    8,
          23,   52,   29,   37,   54,   48,   23,   -1,
          -7,   17,   38,   28,   78,   70,   17,   30,
         -13,   57,   13,   85,   63,   55,   45,    9,
         -42,   23,   32,   43,   71,   13,   28,    3,
         -13,  -50,  -64,  -50,    8,   13,  -37,  -37,
         -51,  -43, -112,  -38,  -29,  -65,  -49,  -19,
         -62,  -80,  -84, -105,  -93, -113,  -49,  -36,
    ],
    [ // king
         -40,  -32,  -24,  -16,  -16,  -24,  -32,  -40,
         -24,  -16,   -8,    0,    0,   -8,  -16,  -24,
         -24,   -8,   16,   24,   24,   16,   -8,  -24,
         -24,   -8,   24,   32,   32,   24,   -8,  -24,
         -24,   -8,   24,   32,   32,   24,   -8,  -24,
         -24,   -8,   16,   24,   24,   16,   -8,  -24,
         -24,  -24,    0,    0,    0,    0,  -24,  -24,
         -40,  -24,  -24,  -24,  -24,  -24,  -24,  -40,
    ],
];
// END TEXEL GENERATED

const fn flip_sq(sq: usize) -> usize {
    let rank = sq / 8;
    let file = sq % 8;
    (7 - rank) * 8 + file
}

/// Material plus table, White positive and Black negative, indexed by bitboard index and square.
const fn build_pst(material: [i16; 6], tables: [[i16; 64]; 6]) -> [[i16; 64]; 12] {
    let mut pst = [[0; 64]; 12];
    let mut piece = WHITE_PAWN;
    while piece <= WHITE_KING {
        let mut sq = 0;
        while sq < 64 {
            pst[piece][sq] = tables[piece][flip_sq(sq)] + material[piece];
            pst[BLACK_PAWN + piece][sq] = -(tables[piece][sq] + material[piece]);
            sq += 1;
        }
        piece += 1;
    }
    pst
}

pub const PST_MG: [[i16; 64]; 12] = build_pst(MATERIAL_MG, TABLE_MG);
pub const PST_EG: [[i16; 64]; 12] = build_pst(MATERIAL_EG, TABLE_EG);
