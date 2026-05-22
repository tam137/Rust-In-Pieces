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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspositionType {
    Exact,      // PV Node (Exact score)
    LowerBound, // Cut Node (Beta cutoff - score is at least this)
    UpperBound, // All Node (Alpha cutoff - score is at most this)
}

#[derive(Debug, Clone, Copy)]
pub struct TranspositionEntry {
    pub eval: i16,
    pub depth: i32,
    pub entry_type: TranspositionType,
    pub best_move: Option<crate::model::Turn>,
}

#[derive(Debug, Clone)]
pub struct ZobristTable {
    pub hash_map: CHashMap<u64, TranspositionEntry>,
}

impl ZobristTable {
    pub(crate) fn new() -> Self {
        Self {
            hash_map: CHashMap::with_capacity(1000),
        }
    }

    pub fn get_entry(&self, hash: &u64) -> Option<TranspositionEntry> {
        self.hash_map.get(hash).map(|value| *value)
    }

    pub fn insert_entry(&self, hash: u64, entry: TranspositionEntry) {
        self.hash_map.insert(hash, entry);
    }

    pub fn get_eval_for_hash(&self, hash: &u64) -> Option<i16> {
        self.hash_map.get(hash).map(|value| value.eval)
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
