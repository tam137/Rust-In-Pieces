use crate::model::Board;
use std::collections::HashMap;
use rand::{Rng, rngs::StdRng, SeedableRng};
use crate::config::Config;

const BOARD_SIZE: usize = 120;
const NUM_PIECES: usize = 12;
const FIG: [usize; 12] = [10, 11, 12, 13, 14, 15, 20, 21, 22, 23, 24, 25];

#[derive(Debug, Clone)]
pub struct ZobristTable {
    table: [[u64; NUM_PIECES]; BOARD_SIZE],
    fig_map: HashMap<usize, usize>,
    white_to_move: u64,
    hash_map: HashMap<u64, i16>,
    entries: u32,
}

impl ZobristTable {
    pub(crate) fn new() -> Self {

        let mut rng = StdRng::seed_from_u64(137);
        let mut table = [[0u64; NUM_PIECES]; BOARD_SIZE];

        let mut fig_map = HashMap::new();
        fig_map.insert(10, 0);
        fig_map.insert(11, 1);
        fig_map.insert(12, 2);
        fig_map.insert(13, 3);
        fig_map.insert(14, 4);
        fig_map.insert(15, 5);
        fig_map.insert(20, 6);
        fig_map.insert(21, 7);
        fig_map.insert(22, 8);
        fig_map.insert(23, 9);
        fig_map.insert(24, 10);
        fig_map.insert(25, 11);

        for i in 0..BOARD_SIZE {
            for j in FIG {
                let fig_index = fig_map.get(&(j as usize)).unwrap();
                table[i][*fig_index] = rng.gen();
            }
        }
        let white_to_move = rng.gen();
        Self { table, fig_map, white_to_move, hash_map: HashMap::with_capacity(1000), entries: 0 }
    }

    pub fn gen(&self, board: &Board) -> u64 {
        let mut hash = 0u64;
        if board.white_to_move {
            hash ^= self.white_to_move;
        }
        for i in 0..BOARD_SIZE {
            let piece = board.field[i];
            if piece > 0 {
                let piece_index = self.fig_map.get(&(piece as usize)).unwrap();
                hash ^= self.table[i][*piece_index];
            }
        }
        hash
    }

    pub fn get_eval_for_hash(&self, hash: &u64) -> Option<&i16> {
        self.hash_map.get(hash)
    }

    pub fn set_new_hash(&mut self, hash: &u64, eval: i16) {
        self.entries += 1;
        self.hash_map.insert(*hash, eval);
    }

    pub fn reset_hash(&mut self) {
        self.hash_map.clear();
    }

    pub fn clean_up_hash_if_needed(&mut self, config: &Config) -> u32 {
        if config.max_zobrist_hash_entries < self.entries {
            self.hash_map.clear();
            let ret = self.entries;
            self.entries = 0;
            ret
        } else {
            0
        }
    }
}