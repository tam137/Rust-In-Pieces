use rand::{Rng, rngs::StdRng, SeedableRng};
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
    pub key: u64, // Full 64-bit Zobrist key to prevent index collisions
    pub eval: i16,
    pub depth: i32,
    pub entry_type: TranspositionType,
    pub best_move: Option<crate::model::Turn>,
}

impl Default for TranspositionEntry {
    fn default() -> Self {
        Self {
            key: 0,
            eval: 0,
            depth: -1, // -1 signals empty slot
            entry_type: TranspositionType::Exact,
            best_move: None,
        }
    }
}

#[derive(Debug)]
pub struct ZobristTable {
    pub table: std::sync::RwLock<Vec<TranspositionEntry>>,
}

impl ZobristTable {
    pub fn new() -> Self {
        Self::with_capacity(10_000_000)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            table: std::sync::RwLock::new(vec![TranspositionEntry::default(); capacity.max(1)]),
        }
    }

    pub fn get_entry(&self, hash: &u64) -> Option<TranspositionEntry> {
        let table = self.table.read().unwrap();
        let index = (*hash as usize) % table.len();
        let entry = table[index];
        if entry.depth != -1 && entry.key == *hash {
            Some(entry)
        } else {
            None
        }
    }

    pub fn insert_entry(&self, hash: u64, entry: TranspositionEntry) {
        let mut table = self.table.write().unwrap();
        let index = (hash as usize) % table.len();
        let existing = &mut table[index];
        // Depth-Preferred replacement policy (always overwrite if key is identical)
        if existing.depth == -1 || existing.key == hash || entry.depth >= existing.depth {
            *existing = entry;
        }
    }

    pub fn get_eval_for_hash(&self, hash: &u64) -> Option<i16> {
        let table = self.table.read().unwrap();
        let index = (*hash as usize) % table.len();
        let entry = table[index];
        if entry.depth != -1 && entry.key == *hash {
            Some(entry.eval)
        } else {
            None
        }
    }

    pub fn _size(&self) -> usize {
        let table = self.table.read().unwrap();
        table.iter().filter(|e| e.depth != -1).count()
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
