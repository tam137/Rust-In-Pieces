use rand::{Rng, rngs::StdRng, SeedableRng};
use once_cell::sync::Lazy;

use crate::model::Board;
use crate::config::Config;

const NUM_PIECES: usize = 12;
const BOARD_SIZE: usize = 120;


const MAX_PIECE_VALUE: usize = 25;

static FIG_MAP_ARRAY: Lazy<[Option<usize>; MAX_PIECE_VALUE + 1]> = Lazy::new(|| {
    let mut fig_map = [None; MAX_PIECE_VALUE + 1];
    fig_map[10] = Some(0);
    fig_map[11] = Some(1);
    fig_map[12] = Some(2);
    fig_map[13] = Some(3);
    fig_map[14] = Some(4);
    fig_map[15] = Some(5);
    fig_map[20] = Some(6);
    fig_map[21] = Some(7);
    fig_map[22] = Some(8);
    fig_map[23] = Some(9);
    fig_map[24] = Some(10);
    fig_map[25] = Some(11);
    fig_map
});

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
    hash_map: std::collections::HashMap<u64, i16>,
}

impl ZobristTable {
    pub(crate) fn new() -> Self {
        Self {
            hash_map: std::collections::HashMap::with_capacity(1000),
        }
    }

    pub fn get_eval_for_hash(&self, hash: &u64) -> Option<&i16> {
        self.hash_map.get(hash)
    }

    pub fn set_new_hash(&mut self, hash: &u64, eval: i16) {
        self.hash_map.insert(*hash, eval);
    }

    pub fn reset_hash(&mut self) {
        self.hash_map.clear();
    }

    pub fn _size(&mut self) -> usize {
        self.hash_map.len()
    }

    pub fn clean_up_hash_if_needed(&mut self, config: &Config) -> usize {
        if config.max_zobrist_hash_entries <= self.hash_map.len() {
            let size = self.hash_map.len();
            self.reset_hash();
            size
        } else {
            0
        }
    }
}


pub fn gen(board: &Board) -> u64 {
    let mut hash = 0u64;
    if board.white_to_move {
        hash ^= *WHITE_TO_MOVE;
    }
    for i in 0..BOARD_SIZE {
        let piece = board.field[i];
        if piece > 0 && (piece as usize) <= MAX_PIECE_VALUE {
            if let Some(piece_index) = FIG_MAP_ARRAY[piece as usize] {
                hash ^= ZOBRIST_TABLE[i][piece_index];
            }
        }
    }
    hash
}
