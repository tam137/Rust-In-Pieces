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
pub const MATERIAL_MG: [i16; 6] = [98, 662, 464, 507, 1437, 10000];
pub const MATERIAL_EG: [i16; 6] = [83, 575, 316, 335, 1163, 10000];
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
           0,    0,    0,    0,    0,    0,    0,    0,
           4,    8,    8,    8,    8,    8,    8,    4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
           0,    0,    0,    4,    4,    0,    0,    0,
    ],
    [ // knight
         -40,  -32,  -24,  -24,  -24,  -24,  -32,  -40,
         -32,  -16,    0,    0,    0,    0,  -16,  -32,
         -24,    0,    8,   12,   12,    8,    0,  -24,
         -24,    4,   12,   16,   16,   12,    4,  -24,
         -24,    0,   12,   16,   16,   12,    0,  -24,
         -24,    4,    8,   12,   12,    8,    4,  -24,
         -32,  -16,    0,    4,    4,    0,  -16,  -32,
         -40,  -32,  -24,  -24,  -24,  -24,  -32,  -40,
    ],
    [ // bishop
         -16,   -8,   -8,   -8,   -8,   -8,   -8,  -16,
          -8,    0,    0,    0,    0,    0,    0,   -8,
          -8,    0,    4,    8,    8,    4,    0,   -8,
          -8,    4,    4,    8,    8,    4,    4,   -8,
          -8,    0,    8,    8,    8,    8,    0,   -8,
          -8,    8,    8,    8,    8,    8,    8,   -8,
          -8,    4,    0,    0,    0,    0,    4,   -8,
         -16,   -8,   -8,   -8,   -8,   -8,   -8,  -16,
    ],
    [ // queen
         -16,   -8,   -8,   -4,   -4,   -8,   -8,  -16,
          -8,    0,    0,    0,    0,    0,    0,   -8,
          -8,    0,    4,    4,    4,    4,    0,   -8,
          -4,    0,    4,    4,    4,    4,    0,   -4,
           0,    0,    4,    4,    4,    4,    0,   -4,
          -8,    4,    4,    4,    4,    4,    0,   -8,
          -8,    0,    4,    0,    0,    0,    0,   -8,
         -16,   -8,   -8,   -4,   -4,   -8,   -8,  -16,
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
           0,    0,    0,    0,    0,    0,    0,    0,
           4,    8,    8,    8,    8,    8,    8,    4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
          -4,    0,    0,    0,    0,    0,    0,   -4,
           0,    0,    0,    4,    4,    0,    0,    0,
    ],
    [ // knight
         -40,  -32,  -24,  -24,  -24,  -24,  -32,  -40,
         -32,  -16,    0,    0,    0,    0,  -16,  -32,
         -24,    0,    8,   12,   12,    8,    0,  -24,
         -24,    4,   12,   16,   16,   12,    4,  -24,
         -24,    0,   12,   16,   16,   12,    0,  -24,
         -24,    4,    8,   12,   12,    8,    4,  -24,
         -32,  -16,    0,    4,    4,    0,  -16,  -32,
         -40,  -32,  -24,  -24,  -24,  -24,  -32,  -40,
    ],
    [ // bishop
         -16,   -8,   -8,   -8,   -8,   -8,   -8,  -16,
          -8,    0,    0,    0,    0,    0,    0,   -8,
          -8,    0,    4,    8,    8,    4,    0,   -8,
          -8,    4,    4,    8,    8,    4,    4,   -8,
          -8,    0,    8,    8,    8,    8,    0,   -8,
          -8,    8,    8,    8,    8,    8,    8,   -8,
          -8,    4,    0,    0,    0,    0,    4,   -8,
         -16,   -8,   -8,   -8,   -8,   -8,   -8,  -16,
    ],
    [ // queen
         -16,   -8,   -8,   -4,   -4,   -8,   -8,  -16,
          -8,    0,    0,    0,    0,    0,    0,   -8,
          -8,    0,    4,    4,    4,    4,    0,   -8,
          -4,    0,    4,    4,    4,    4,    0,   -4,
           0,    0,    4,    4,    4,    4,    0,   -4,
          -8,    4,    4,    4,    4,    4,    0,   -8,
          -8,    0,    4,    0,    0,    0,    0,   -8,
         -16,   -8,   -8,   -4,   -4,   -8,   -8,  -16,
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
