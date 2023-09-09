use std::collections::HashMap;
use crate::search::SearchAlgo;

#[derive(Clone)]
pub struct Config {
    pub search_algo: SearchAlgo,
    pub use_quiescence: bool,
    pub use_depth_modificator: bool,
    pub use_book: bool,
    pub use_zobrist: bool,
    pub search_depth: i32,
    pub search_depth_quite: i32,
    pub eval_fuzzy: i16,
    pub move_freedom_bonus: i32,
    pub undeveloped_knight_malus: i16,
    pub undeveloped_bishop_malus: i16,
    pub pawn_on_last_rank_bonus: i16,
    pub pawn_on_before_last_rank_bonus: i16,
    pub pawn_on_before_before_last_rank_bonus: i16,
    pub early_queen_malus: i16,
    pub undeveloped_center_pawn_malus: i16,
    pub short_castle_bonus: i16,
    pub long_castle_bonus: i16,
    pub piece_eval_pawn: i16,
    pub piece_eval_rook: i16,
    pub piece_value_knight: i16,
    pub piece_eval_bishop: i16,
    pub piece_eval_queen: i16,
    pub piece_eval_king: i16,
}


impl Config {
    pub fn new() -> Config {
        Config {
            search_algo: SearchAlgo::Zobrist,
            use_quiescence: false,
            use_depth_modificator: false,
            use_book: true,
            use_zobrist: true,
            search_depth: 4,
            search_depth_quite: 4,
            eval_fuzzy: 0,
            move_freedom_bonus: 4,
            undeveloped_knight_malus: 35,
            undeveloped_bishop_malus: 20,
            pawn_on_last_rank_bonus: 200,
            pawn_on_before_last_rank_bonus: 120,
            pawn_on_before_before_last_rank_bonus: 60,
            early_queen_malus: 150,
            undeveloped_center_pawn_malus: 90,
            short_castle_bonus: 110,
            long_castle_bonus: 75,

            piece_eval_pawn: 100,
            piece_eval_rook: 500,
            piece_value_knight: 300,
            piece_eval_bishop: 300,
            piece_eval_queen: 950,
            piece_eval_king: 15000,
        }
    }

    pub fn get_eval_value_for_piece(&self, fig: i8) -> i16 {
        match fig {
            10 => self.piece_eval_pawn,
            20 => -self.piece_eval_pawn,
            11 => self.piece_eval_rook,
            21 => -self.piece_eval_rook,
            12 => self.piece_value_knight,
            22 => -self.piece_value_knight,
            13 => self.piece_eval_bishop,
            23 => -self.piece_eval_bishop,
            14 => self.piece_eval_queen,
            24 => -self.piece_eval_queen,
            15 => self.piece_eval_pawn,
            25 => -self.piece_eval_pawn,
            _ => 0
        }
    }

    pub fn unittest(&mut self) -> &Config {
        self.eval_fuzzy = 0;
        self.search_depth = 2;
        self
    }
}