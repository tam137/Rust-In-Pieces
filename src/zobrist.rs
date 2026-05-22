use rand::{Rng, rngs::StdRng, SeedableRng};
use chashmap::CHashMap;
use once_cell::sync::Lazy;

use crate::model::Board;

const NUM_PIECES: usize = 12;
const BOARD_SIZE: usize = 64;

static ZOBRIST_DATA: Lazy<([[u64; NUM_PIECES]; BOARD_SIZE], u64)> = Lazy::new(|| {
    let mut rng = StdRng::seed_from_u64(137);
    let mut table = [[0u64; NUM_PIECES]; BOARD_SIZE];

    for i in 0..BOARD_SIZE {
        for piece_index in 0..NUM_PIECES {
            table[i][piece_index] = rng.gen();
        }
    }

    let white_to_move = rng.gen();

    (table, white_to_move)
});

static ZOBRIST_TABLE: Lazy<[[u64; NUM_PIECES]; BOARD_SIZE]> = Lazy::new(|| ZOBRIST_DATA.0);
static WHITE_TO_MOVE: Lazy<u64> = Lazy::new(|| ZOBRIST_DATA.1);

#[derive(Debug, Clone)]
pub struct ZobristTable {
    pub hash_map: CHashMap<u64, i16>,
}

impl ZobristTable {
    pub(crate) fn new() -> Self {
        Self {
            hash_map: CHashMap::with_capacity(1000),
        }
    }

    pub fn get_eval_for_hash(&self, hash: &u64) -> Option<i16> {
        self.hash_map.get(hash).map(|value| *value)
    }

    pub fn _size(&mut self) -> usize {
        self.hash_map.len()
    }
}

pub fn gen(board: &Board) -> u64 {
    let mut hash = 0u64;
    if board.white_to_move {
        hash ^= *WHITE_TO_MOVE;
    }
    for piece_idx in 0..12 {
        let mut bb = board.bitboards[piece_idx];
        while bb != 0 {
            let square = bb.trailing_zeros() as usize;
            hash ^= ZOBRIST_TABLE[square][piece_idx];
            bb &= bb - 1; // Clear least significant set bit
        }
    }
    hash
}
