use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use rand::{Rng, rngs::StdRng, SeedableRng};

use crate::model::{Board, DataMap};
use crate::config::Config;

const BOARD_SIZE: usize = 120;
const NUM_PIECES: usize = 12;
const FIG: [usize; 12] = [10, 11, 12, 13, 14, 15, 20, 21, 22, 23, 24, 25];

#[derive(Debug, Clone)]
pub struct ZobristTable {
    hash_map: HashMap<u64, i16>,
    entries: u32,
}

impl ZobristTable {
    pub(crate) fn new() -> Self {

        Self { hash_map: HashMap::with_capacity(1000), entries: 0 }
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


}



pub fn gen(board: &Board) -> u64 {
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

    let white_to_move: u64 = rng.gen();

    let mut hash = 0u64;
    if board.white_to_move {
        hash ^= white_to_move;
    }
    for i in 0..BOARD_SIZE {
        let piece = board.field[i];
        if piece > 0 {
            let piece_index = fig_map.get(&(piece as usize)).unwrap();
            hash ^= table[i][*piece_index];
        }
    }
    hash
}


pub fn clean_up_hash_if_needed(global_map: Arc<RwLock<DataMap>>, config: &Config) -> u32 {

    let mut zobrist_table;
    {
    zobrist_table = global_map.read()
        .expect("RIP Could not lock global map")
        .get_data::<Arc<Mutex<ZobristTable>>>(crate::model::DataMapKey::ZobristTable)
        .expect("RIP Could not load Zobrist Table")
        .lock().expect("RIP Cant lock zobrist table").clone();
    }

    if config.max_zobrist_hash_entries < zobrist_table.entries {
        zobrist_table.hash_map.clear();
        let ret = zobrist_table.entries;
        zobrist_table.entries = 0;
        ret
    } else {
        0
    }
}
