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
pub const MATERIAL_MG: [i16; 6] = [86, 583, 420, 436, 1402, 10000];
pub const MATERIAL_EG: [i16; 6] = [135, 792, 437, 463, 1423, 10000];
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
          35,   27,   29,   49,   45,   39,   25,   38,
         -65,  -71,  -11,   -6,    7,   51,  -68,   -3,
         -11,    3,    4,   74,  132,   73,   70,   57,
         -55,  -35,   14,   23,   36,   53,   42,   41,
         -62,  -65,    8,   35,   -3,   -7,   -1,    9,
         -75,  -48,  -37,   -9,    6,  -11,  -13,  -36,
         -94,  -64,  -18,    3,   -3,   35,    9,  -64,
         -48,  -27,   -3,    4,   13,   -5,  -16,  -30,
    ],
    [ // knight
        -141,   -9,  -69,    3,    8,  -38,  -30, -101,
         -13,  -69,  -23,   14,  -51,   55,   37,  -16,
         -22,  -52,  -88,   24,  142,   11,   68,  -29,
         -33,   -6,   -2,    7,  -35,   31,    1,  102,
         -32,   13,  -12,    1,    7,   26,   27,   31,
         -66,  -29,  -12,   -8,  -10,   -1,   22,  -34,
         -49,  -37,   -6,  -16,   -6,  -29,   16,   48,
        -111,  -22,  -15,    4,    3,    6,    5,  -36,
    ],
    [ // bishop
         -44,  -46,  -13,  -35,   -7,   14,   15,  -64,
         -39,  -83,  -56,  -11,    7,  -24,  -48,  -76,
         -62,  -43,  -40,  -28,   10,  145,   37,   50,
         -52,    8,    8,   57,   41,   47,   -3,  -17,
          -7,  -22,    6,   45,   60,   -1,   33,   30,
          -1,    9,    4,   11,   -9,   24,   18,   35,
         -11,   43,   47,  -34,   -5,   -2,   69,    0,
         -31,   20,   -6,  -22,  -28,  -21,  -25,    4,
    ],
    [ // queen
         -38,  -17,    6,   29,   35,   60,   50,   30,
         -64,  -78,   -4,   -5,   12,   27,  -28,   62,
         -69,  -20,    6,   14,   39,   95,   61,   34,
         -32,  -40,    7,  -18,    9,   30,   19,   14,
         -30,  -44,  -23,    1,  -27,   33,    6,    7,
         -58,   -5,   24,  -10,  -23,   -3,    3,   -3,
         -37,  -18,   28,    5,    4,   14,   59,   -4,
         -46,  -28,  -10,   -6,  -13,  -46,  -81,  -40,
    ],
    [ // king
         -27,  -33,  -34,  -38,  -37,  -31,  -32,  -30,
         -23,  -25,  -24,  -31,  -36,  -29,  -36,  -23,
         -25,  -17,  -27,  -44,  -52,  -22,  -24,  -51,
         -29,  -22,  -41,  -91, -100,  -50,  -36,  -52,
         -28,   -7,  -39,  -82,  -76,  -86,  -65,  -98,
         -23,  -44,   28,  -68,  -85,  -59,  -34,  -20,
          52,  -80,   35,  -67,   -8,   69,   85,  102,
           2,  102,   69,  -40,  110,   -8,  101,  109,
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
          30,   39,   41,   14,   17,   51,   64,   49,
          -6,  -15,  -36,  -50,  -60,  -49,  -25,  -41,
          44,   36,   32,   -2,  -23,   -1,   11,   11,
          43,   48,   38,   16,  -14,   -6,   18,   -2,
          36,   47,   16,  -24,   -9,    9,   19,    1,
          14,    9,    2,  -33,  -38,  -23,  -20,    1,
          39,    3,   -1,  -20,  -35,  -60,  -64,   20,
           6,  -30,   -6,  -12,  -42,   -4,  -25,  -23,
    ],
    [ // knight
         -94,  -24,    0,    9,    1,   -5,  -24, -132,
          -5,   15,   29,   10,    5,  -49,  -29,  -58,
         -11,   38,   31,  -20,  -52,  -51,    1,  -28,
           3,   52,   24,   39,   47,   -5,   26,  -39,
           6,    8,   20,   16,   32,   -1,   13,  -25,
          -9,   11,   -2,   20,   21,  -37,  -14,    0,
         -44,  -12,  -22,  -22,   -6,   12,   -1,  -44,
         -78,  -15,  -32,   -9,  -21,  -52,   -5,  -63,
    ],
    [ // bishop
          11,   32,    9,   13,    7,    4,    3,  -18,
          24,   34,    6,   14,   -7,   21,   15,  -26,
          22,   44,   17,   -5,   -5,  -30,   -8,  -27,
          32,   37,    2,   17,   -3,   10,   34,   -5,
         -28,   19,   32,   20,    3,    5,  -14,  -51,
         -28,  -20,    8,    9,   19,  -31,  -68,   -1,
         -31,  -53,  -28,   23,  -11,   -8,  -13,  -22,
         -42,  -10,   -2,   -7,   -7,   -2,  -24,  -22,
    ],
    [ // queen
         -12,  -11,    1,   11,   38,   41,   14,    5,
          57,   86,   38,   44,   52,   31,   21,  -33,
          20,   40,   65,   33,   78,   34,  -26,    2,
          16,   93,   21,  107,   66,   57,   57,    9,
         -46,   36,   45,   47,   90,    9,   18,   -1,
          -8,  -39,  -66,  -41,   24,   14,  -55,  -54,
         -58,  -54,  -97,  -31,  -44,  -95,  -91,  -15,
         -68,  -91,  -91,  -76, -111, -147,  -68,  -40,
    ],
    [ // king
         -59,  -41,  -36,   -7,  -10,  -17,  -30,  -66,
         -22,   25,   34,   40,   24,   41,   33,  -16,
          -5,   51,   64,   25,   19,   88,   86,   17,
         -17,   32,   27,    0,    1,   38,   57,    4,
         -37,  -17,  -11,  -11,   -4,    1,    5,  -28,
         -34,  -21,  -35,  -17,  -11,  -16,  -27,  -44,
         -60,    4,  -25,   18,   -5,  -37,  -52, -104,
         -43,  -51,  -22,   54,  -45,   12,  -69, -143,
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
