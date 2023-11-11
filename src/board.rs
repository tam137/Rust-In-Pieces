use std::collections::HashMap;

use crate::{eval, Turn};
use crate::config::Config;
use crate::stats::Stats;
use crate::zobrist::ZobristTable;

static TARGETS_FOR_SHORT_WHITE: [i32; 3] = [95, 96, 97];
static TARGETS_FOR_LONG_WHITE: [i32; 3] = [95, 94, 93];
static TARGETS_FOR_SHORT_BLACK: [i32; 3] = [25, 26, 27];
static TARGETS_FOR_LONG_BLACK: [i32; 3] = [25, 24, 23];

#[derive(Clone)]
pub struct Board {
    field: [i32; 120],
    king_pos: [usize; 2],
    pty: u32,
    fifty_move_rule: u32,
    state: GameState,
    moves: String,
    turns: Vec<Turn>,
    position_map: HashMap<String, i32>,
    hash: ZobristTable,
    config: Config,
    current_best: i16,
    diagonal_8a: [usize; 8],
    diagonal_7b: [usize; 8],
    diagonal_6c: [usize; 8],
    diagonal_5d: [usize; 8],
    diagonal_4e: [usize; 8],
    diagonal_3f: [usize; 8],
    diagonal_2g: [usize; 8],
    diagonal_1h: [usize; 8],
    diagonal_a8: [usize; 8],
    diagonal_b7: [usize; 8],
    diagonal_c6: [usize; 8],
    diagonal_d5: [usize; 8],
    diagonal_e4: [usize; 8],
    diagonal_f3: [usize; 8],
    diagonal_g2: [usize; 8],
    diagonal_h1: [usize; 8],

    diagonal_8h: [usize; 8],
    diagonal_7g: [usize; 8],
    diagonal_6f: [usize; 8],
    diagonal_5e: [usize; 8],
    diagonal_4d: [usize; 8],
    diagonal_3c: [usize; 8],
    diagonal_2b: [usize; 8],
    diagonal_1a: [usize; 8],
    diagonal_g7: [usize; 8],
    diagonal_f6: [usize; 8],
    diagonal_e5: [usize; 8],
    diagonal_d4: [usize; 8],
    diagonal_c3: [usize; 8],
    diagonal_b2: [usize; 8],
    diagonal_a1: [usize; 8],

    horizontale_1: [usize; 8],
    horizontale_2: [usize; 8],
    horizontale_3: [usize; 8],
    horizontale_4: [usize; 8],
    horizontale_5: [usize; 8],
    horizontale_6: [usize; 8],
    horizontale_7: [usize; 8],
    horizontale_8: [usize; 8],

    vertikale_a: [usize; 8],
    vertikale_b: [usize; 8],
    vertikale_c: [usize; 8],
    vertikale_d: [usize; 8],
    vertikale_e: [usize; 8],
    vertikale_f: [usize; 8],
    vertikale_g: [usize; 8],
    vertikale_h: [usize; 8],
}

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
pub enum GameState {
    Draw,
    WhiteWin,
    BlackWin,
    Normal,
    WhiteWinByTime,
    BlackWinByTime,
}

impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field
    }
}


pub fn get_index_from_notation(notation: &str) -> Option<usize> {
    let chars: Vec<char> = notation.chars().collect();
    if chars.len() != 2 {
        return None;
    }
    let col = match chars[0] {
        'a' => 1,
        'b' => 2,
        'c' => 3,
        'd' => 4,
        'e' => 5,
        'f' => 6,
        'g' => 7,
        'h' => 8,
        _ => return None,
    };
    let row = match chars[1].to_digit(10) {
        Some(digit) => 10 - digit as usize,
        None => return None,
    };
    if row < 2 || row > 9 {
        return None;
    }
    Some((row * 10) + col)
}


impl Board {

    pub fn new() -> Board {
        Board {
            field: [
                -11, -11, -11, -11, -11, -11, -11, -11, -11, -11,
                -11, -11, -11, -11, -11, -11, -11, -11, -11, -11,
                //   a   b   c   d   e   f   g   h
                -11, 21, 22, 23, 24, 25, 23, 22, 21, -11, //20 - 8
                -11, 20, 20, 20, 20, 20, 20, 20, 20, -11, //30 - 7
                -11,  0,  0,  0,  0,  0,  0,  0,  0, -11, //40 - 6
                -11,  0,  0,  0,  0,  0,  0,  0,  0, -11, //50 - 5
                -11,  0,  0,  0,  0,  0,  0,  0,  0, -11, //60 - 4
                -11,  0,  0,  0,  0,  0,  0,  0,  0, -11, //70 - 3
                -11, 10, 10, 10, 10, 10, 10, 10, 10, -11, //80 - 2
                -11, 11, 12, 13, 14, 15, 13, 12, 11, -11, //90 - 1
                //    1   2   3   4   5   6   7   8 <- Indexbezeichnungen
                -11, -11, -11, -11, -11, -11, -11, -11, -11, -11,
                -11, -11, -11, -11, -11, -11, -11, -11, -11, -11,
            ],
            king_pos: [95, 25],
            pty: 0,
            fifty_move_rule: 0,
            state: GameState::Normal,
            moves: String::new(),
            turns: Vec::with_capacity(200),
            position_map: HashMap::new(),
            hash: ZobristTable::new(),
            config: Config::new(),
            current_best: 0,
            diagonal_8a: [21, 0, 0, 0, 0, 0, 0, 0],
            diagonal_7b: [31, 22, 0, 0, 0, 0, 0, 0],
            diagonal_6c: [41, 32, 23, 0, 0, 0, 0, 0],
            diagonal_5d: [51, 42, 33, 24, 0, 0, 0, 0],
            diagonal_4e: [61, 52, 43, 34, 25, 0, 0, 0],
            diagonal_3f: [71, 62, 53, 44, 35, 26, 0, 0],
            diagonal_2g: [81, 72, 63, 54, 45, 36, 27, 0],
            diagonal_1h: [91, 82, 73, 64, 55, 46, 37, 28],
            diagonal_a8: [91, 82, 73, 64, 55, 46, 37, 28],
            diagonal_b7: [92, 83, 74, 65, 56, 47, 38, 0],
            diagonal_c6: [93, 84, 75, 66, 57, 48, 0, 0],
            diagonal_d5: [94, 85, 76, 67, 58, 0, 0, 0],
            diagonal_e4: [95, 86, 77, 68, 0, 0, 0, 0],
            diagonal_f3: [96, 87, 78, 0, 0, 0, 0, 0],
            diagonal_g2: [97, 88, 0, 0, 0, 0, 0, 0],
            diagonal_h1: [98, 0, 0, 0, 0, 0, 0, 0],

            diagonal_8h: [28, 0, 0, 0, 0, 0, 0, 0],
            diagonal_7g: [38, 27, 0, 0, 0, 0, 0, 0],
            diagonal_6f: [48, 37, 26, 0, 0, 0, 0, 0],
            diagonal_5e: [58, 47, 36, 25, 0, 0, 0, 0],
            diagonal_4d: [68, 57, 46, 35, 24, 0, 0, 0],
            diagonal_3c: [78, 67, 56, 45, 34, 23, 0, 0],
            diagonal_2b: [88, 77, 66, 55, 44, 33, 22, 0],
            diagonal_1a: [98, 87, 76, 65, 54, 43, 32, 21],
            diagonal_g7: [97, 86, 75, 64, 53, 42, 31, 0],
            diagonal_f6: [96, 85, 74, 63, 52, 41, 0, 0],
            diagonal_e5: [95, 84, 73, 62, 51, 0, 0, 0],
            diagonal_d4: [94, 83, 72, 61, 0, 0, 0, 0],
            diagonal_c3: [93, 82, 71, 0, 0, 0, 0, 0],
            diagonal_b2: [92, 81, 0, 0, 0, 0, 0, 0],
            diagonal_a1: [91, 0, 0, 0, 0, 0, 0, 0],

            horizontale_1: [91, 92, 93, 94, 95, 96, 97, 98],
            horizontale_2: [81, 82, 83, 84, 85, 86, 87, 88],
            horizontale_3: [71, 72, 73, 74, 75, 76, 77, 78],
            horizontale_4: [61, 62, 63, 64, 65, 66, 67, 68],
            horizontale_5: [51, 52, 53, 54, 55, 56, 57, 58],
            horizontale_6: [41, 42, 43, 44, 45, 46, 47, 48],
            horizontale_7: [31, 32, 33, 34, 35, 36, 37, 38],
            horizontale_8: [21, 22, 23, 24, 25, 26, 27, 28],

            vertikale_a: [91, 81, 71, 61, 51, 41, 31, 21],
            vertikale_b: [92, 82, 72, 62, 52, 42, 32, 22],
            vertikale_c: [93, 83, 73, 63, 53, 43, 33, 23],
            vertikale_d: [94, 84, 74, 64, 54, 44, 34, 24],
            vertikale_e: [95, 85, 75, 65, 55, 45, 35, 25],
            vertikale_f: [96, 86, 76, 66, 56, 46, 36, 26],
            vertikale_g: [97, 87, 77, 67, 57, 47, 37, 27],
            vertikale_h: [98, 88, 78, 68, 58, 48, 38, 28],
        }
    }

    pub fn get_horizontales(&self, idx: usize) -> ([usize; 8], [usize; 8]) {
        let file = idx % 10;
        let rank = 10 - (idx / 10);

        let vertical = match file {
            1 => { self.vertikale_a },
            2 => { self.vertikale_b },
            3 => { self.vertikale_c },
            4 => { self.vertikale_d },
            5 => { self.vertikale_e },
            6 => { self.vertikale_f },
            7 => { self.vertikale_g },
            8 => { self.vertikale_h },
            _ => { [0; 8] }
        };

        let horizontal = match rank {
            1 => { self.horizontale_1 },
            2 => { self.horizontale_2 },
            3 => { self.horizontale_3 },
            4 => { self.horizontale_4 },
            5 => { self.horizontale_5 },
            6 => { self.horizontale_6 },
            7 => { self.horizontale_7 },
            8 => { self.horizontale_8 },
            _ => { [0; 8] }
        };
        (vertical, horizontal)
    }

    pub fn get_diagonals(&self, idx: usize) -> ([usize; 8], [usize; 8]) {
        let file = (idx % 10) as i32;
        let rank = (10 - (idx / 10)) as i32;

        let first = match file - rank {
            0 => self.diagonal_a8,
            1 => self.diagonal_b7,
            2 => self.diagonal_c6,
            3 => self.diagonal_d5,
            4 => self.diagonal_e4,
            5 => self.diagonal_f3,
            6 => self.diagonal_g2,
            7 => self.diagonal_h1,
            -1 => self.diagonal_2g,
            -2 => self.diagonal_3f,
            -3 => self.diagonal_4e,
            -4 => self.diagonal_5d,
            -5 => self.diagonal_6c,
            -6 => self.diagonal_7b,
            -7 => self.diagonal_8a,
            _ => { [0; 8] }
        };

        let second = match file + rank {
            16 => self.diagonal_8h,
            15 => self.diagonal_7g,
            14 => self.diagonal_6f,
            13 => self.diagonal_5e,
            12 => self.diagonal_4d,
            11 => self.diagonal_3c,
            10 => self.diagonal_2b,
            9 => self.diagonal_1a,
            8 => self.diagonal_g7,
            7 => self.diagonal_f6,
            6 => self.diagonal_e5,
            5 => self.diagonal_d4,
            4 => self.diagonal_c3,
            3 => self.diagonal_b2,
            2 => self.diagonal_a1,
            _ => { [0; 8] }
        };
        (first, second)
    }

    pub fn get_pty(&self) -> u32 {
        self.pty
    }

    pub fn get_state(&self) -> &GameState {
        &self.state
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn get_field(&self) -> &[i32; 120] {
        &self.field
    }

    pub fn get_list_of_pieces(&self) -> Vec<i32> {
        let mut pieces_list = Vec::with_capacity(32);
        for i in 21..99 {
            if self.field[i] > 0 { pieces_list.push(self.field[i]) }
        }
        pieces_list
    }

    pub fn set_field_index(&mut self, index: usize, piece: i32) {
        self.field[index] = piece;
    }

    pub fn clear_field(&mut self) {
        for i in 21..99 {
            if self.field[i] > 0 { self.field[i] = 0 };
        }
        self.hash.reset_hash();
    }

    pub fn get_hash(&self) -> u64 {
        return self.hash.gen(self);
    }

    pub fn set_current_best(&mut self, eval: i16) {
        self.current_best = eval;
    }

    pub fn get_current_best(&self) -> i16 {
        self.current_best
    }

    pub fn get_eval_for_hash(&self, hash: &u64) -> Option<&i16> {
        self.hash.get_eval_for_hash(hash)
    }

    pub fn set_new_hash(&mut self, hash: &u64, eval: i16) {
        self.hash.set_new_hash(hash, eval);
    }

    pub fn reset_hash(&mut self) {
        self.hash.reset_hash();
    }

    pub fn clean_up_hash_if_needed(&mut self) -> u32 {
        self.hash.clean_up_hash_if_needed(&self.config)
    }


    pub fn is_quite_board_for_white(moves_white: &Vec<usize>, moves_black: &Vec<usize>) -> bool {
        let black_targets = Board::get_target_fields_of_raw_moves(moves_black);
        let white_sources = Board::get_source_fields_of_raw_moves(moves_white);
        for element in black_targets {
            if white_sources.contains(&(element)) {
                return false;
            }
        }
        true
    }

    pub fn is_quite_board_for_black(moves_white: &Vec<usize>, moves_black: &Vec<usize>) -> bool {
        let white_targets = Board::get_target_fields_of_raw_moves(moves_white);
        let black_sources = Board::get_source_fields_of_raw_moves(moves_black);
        for element in white_targets {
            if black_sources.contains(&(element)) {
                return false;
            }
        }
        true
    }

    pub fn is_quite_board(moves_1: &Vec<usize>, moves_2: &Vec<usize>) -> bool {
        let m1_targets = Board::get_target_fields_of_raw_moves(moves_1);
        let m2_targets = Board::get_target_fields_of_raw_moves(moves_2);
        let m1_sources = Board::get_source_fields_of_raw_moves(moves_1);
        let m2_sources = Board::get_source_fields_of_raw_moves(moves_2);

        for element in m1_targets {
            if m2_sources.contains(&(element)) {
                return false;
            }
        }
        for element in m2_targets {
            if m1_sources.contains(&(element)) {
                return false;
            }
        }
        true
    }


    pub(crate) fn get_source_fields_of_raw_moves(raw_moves: &Vec<usize>) -> Vec<i32> {
        let mut villains_target_fields: Vec<i32> = Vec::with_capacity(60);
        for (i, num) in raw_moves.iter().enumerate() {
            if i % 2 == 0 {
                villains_target_fields.push(*num as i32);
            }
        }
        villains_target_fields
    }


    pub fn get_turn_list_for_piece_on_idx(&mut self, white: bool, only_capture: bool, idx: usize) -> Vec<Turn> {
        let mut moves = self.generate_moves_list_for_piece(white, idx);

        if white {
            if let Some(position) = moves.iter().position(|&x| self.field.get(x) == Some(&25)) {
                if position > 0 {
                    moves.remove(position);
                    moves.remove(position - 1);
                }
            }
        } else {
            if let Some(position) = moves.iter().position(|&x| self.field.get(x) == Some(&15)) {
                if position > 0 {
                    moves.remove(position);
                    moves.remove(position - 1);
                }
            }
        }
        self.generate_unsorted_turn_list_from_raw_moves(moves, white, only_capture)
    }


    pub fn get_turn_list(&mut self, white: bool, only_capture: bool, stats: &mut Stats) -> Vec<Turn> {
        let moves = self.generate_moves_list(white);
        let mut turn_list = Vec::with_capacity(50);
        turn_list = self.generate_unsorted_turn_list_from_raw_moves(moves, white, only_capture);

        self.sort_move_list_by_eval(&mut turn_list, white, stats);

        if turn_list.len() == 0 {
            if self.is_in_chess(&Board::get_target_fields_of_raw_moves(&self.generate_moves_list(!white)), white) {
                self.state = if white { GameState::BlackWin } else { GameState::WhiteWin }
            } else {
                self.state = GameState::Draw;
            }
        }
        if self.insufficient_material() {
            self.state = GameState::Draw;
        }

        self.state = if self.position_map.values().any(|&value| value > 2) { GameState::Draw } else { self.state };
        turn_list
    }


    fn generate_unsorted_turn_list_from_raw_moves(&mut self, moves: Vec<usize>, white: bool, only_capture: bool) -> Vec<Turn> {
        let mut last_from: usize = 0;
        let mut last_to: usize;

        let mut turn_list = Vec::with_capacity(50);

        for (i, &mv) in moves.iter().enumerate() {
            if i % 2 == 0 {
                last_from = mv;
            } else {
                last_to = mv;
                let capture = self.field[last_to] as i8;
                if only_capture && capture == 0 { continue }
                turn_list.push(Turn {
                    from: last_from,
                    to: last_to,
                    capture,
                    post_villain:  Vec::new(),
                    post_my: Vec::new(),
                    promotion: false,
                    gives_chess: false,
                    eval: 0,
                });
            }
        }

        // main loop
        for turn in turn_list.iter_mut() {
            self.do_turn(turn);
            turn.post_villain = self.generate_moves_list(!self.is_white_field(turn.to));
            let prune: bool = self.prune_illegal_moves(turn);
            if prune {
                self.do_undo_turn(turn);
                continue;
            }
            turn.post_my = self.generate_moves_list(self.is_white_field(turn.to));
            turn.enrich_move_gives_chess(self, white);
            self.do_undo_turn(turn);
            turn.enrich_move_promotion(self, white);
        }
        turn_list.retain(|turn| !turn.post_my.is_empty());
        turn_list.clone()
    }

    fn insufficient_material(&self) -> bool {
        self.get_pieces_on_field() == 2
    }

    pub(crate) fn is_in_chess(&self, villains_target_fields: &Vec<i32>, white: bool) -> bool {
        let idx_of_king = if white { self.index_of_white_king() } else  { self.index_of_black_king() };
        if villains_target_fields.contains(&idx_of_king) { return true }
        else { false }
    }

    fn sort_move_list_by_give_chess(&mut self, turn_list: &mut Vec<Turn>, white: bool) {
        if white {
            turn_list.sort_by_key(|turn| !turn.gives_chess);
        } else {
            turn_list.sort_by_key(|turn| !turn.gives_chess);
        }
    }



    fn sort_move_list_by_capture(&mut self, turn_list: &mut Vec<Turn>, white: bool) -> () {
        if white {
            turn_list.sort_by(|a, b| self.config.get_eval_value_for_piece(a.capture).cmp(&self.config.get_eval_value_for_piece(b.capture)));
        } else {
            turn_list.sort_by(|a, b| self.config.get_eval_value_for_piece(b.capture).cmp(&self.config.get_eval_value_for_piece(a.capture)));
        }
    }

    fn sort_move_list_by_eval(&mut self, turn_list: &mut Vec<Turn>, white: bool, stats: &mut Stats) -> () {
        for turn in &mut *turn_list {
            self.do_turn(turn);
            if self.config.use_zobrist {
                let board_hash = self.get_hash();
                match self.get_eval_for_hash(&board_hash) {
                    Some(eval) => {
                        stats.add_zobrist_hit(1);
                        turn.eval = *eval;
                    },
                    None => {
                        stats.add_eval_nodes(1);
                        let eval =  eval::calc_eval(self, turn, &self.config);
                        self.set_new_hash(&board_hash, eval);
                        turn.eval = eval;
                    }
                }
            } else {
                turn.eval = eval::calc_eval(self, turn, &self.config);
            }
            if turn.gives_chess {
                turn.eval += if white { 20 } else { -20 };
            }
            self.do_undo_turn(turn);
        }

        if white {
            //turn_list.par_sort_by(|a, b| b.eval.cmp(&a.eval));
            turn_list.sort_unstable_by(|a, b| b.eval.cmp(&a.eval));
        } else {
            //turn_list.par_sort_by(|a, b| a.eval.cmp(&b.eval));
            turn_list.sort_unstable_by(|a, b| a.eval.cmp(&b.eval));
        }
        let len = turn_list.len();
        if len > self.config.truncate_bad_moves {
            turn_list.truncate(self.config.truncate_bad_moves);
        }
    }

    pub fn do_turn(&mut self, turn: &Turn) {
        if self.field[turn.from] % 10 == 0 || self.field[turn.to] != 0 { self.fifty_move_rule = 0 } else { self.fifty_move_rule += 1 };
        self.validate_turn(turn);
        if turn.from == 95 || turn.from == 25 {
            if      turn.from == 95 && turn.to == 97 && self.field[turn.from] == 15 && self.field[96] == 0 { self.field[98] = 0;  self.field[96] = 11; }
            else if turn.from == 95 && turn.to == 93 && self.field[turn.from] == 15 && self.field[94] == 0 && self.field[93] == 0 && self.field[92] == 0 { self.field[91] = 0;  self.field[94] = 11; }
            else if turn.from == 25 && turn.to == 27 && self.field[turn.from] == 25 && self.field[26] == 0 { self.field[28] = 0;  self.field[26] = 21; }
            else if turn.from == 25 && turn.to == 23 && self.field[turn.from] == 25 && self.field[24] == 0 && self.field[23] == 0 && self.field[22] == 0 { self.field[21] = 0;  self.field[24] = 21; }
        }

        if self.field[turn.from] == 15 {
            self.king_pos[0] = turn.to;
        }

        if self.field[turn.from] == 25 {
            self.king_pos[1] = turn.to;
        }

        if turn.is_promotion() {
            self.field[turn.to] = if self.is_white_field(turn.from) { 14 } else { 24 };
        } else {
            self.field[turn.to] = self.field[turn.from];
        }
        self.field[turn.from] = 0;
        self.pty += 1;
        self.moves += " ";
        self.moves += &turn.to_algebraic(false).clone();
        self.turns.push(turn.clone());
    }


    pub fn do_turn_and_return_long_algebraic(&mut self, turn: &Turn) -> String {
        let figure_sign = self.get_piece_for_field(turn.from);
        let long_algebraic = format!("{}{}", figure_sign, turn.to_algebraic(true));
        self.do_turn(turn);
        long_algebraic
    }


    pub fn do_undo_turn(&mut self, turn: &Turn) {
        if turn.from == 95 || turn.from == 25 {
            if      turn.from == 95 && turn.to == 97 && self.field[turn.to] == 15 { self.field[98] = 11;  self.field[96] = 0; }
            else if turn.from == 95 && turn.to == 93 && self.field[turn.to] == 15 { self.field[91] = 11;  self.field[94] = 0; }
            else if turn.from == 25 && turn.to == 27 && self.field[turn.to] == 25 { self.field[28] = 21;  self.field[26] = 0; }
            else if turn.from == 25 && turn.to == 23 && self.field[turn.to] == 25 { self.field[21] = 21;  self.field[24] = 0; }
        }

        if self.field[turn.to] == 15 {
            self.king_pos[0] = turn.from;
        }

        if self.field[turn.to] == 25 {
            self.king_pos[1] = turn.from;
        }

        if turn.is_promotion() {
            self.field[turn.from] = if self.is_white_field(turn.to) { 10 } else { 20 };
        } else {
            self.field[turn.from] = self.field[turn.to];
        }
        self.field[turn.to] = if turn.capture == -1 { 0 } else { turn.capture as i32 };
        self.pty -= 1;
        self.state = GameState::Normal;
        self.moves = self.moves[..self.moves.len() - 5].to_string();
        self.turns.remove(self.turns.len() - 1);
    }


    pub(crate) fn prune_illegal_moves(&self, turn: &mut Turn) -> bool {
        let villains_target_fields = &Board::get_target_fields_of_raw_moves(&turn.post_villain);
        let white = self.is_white_field(turn.to);
        if self.is_in_chess(villains_target_fields, white) { return true }
        match (turn.from, turn.to) {
            (95, 97) if self.field[turn.to] == 15 && TARGETS_FOR_SHORT_WHITE.iter().any(|&target| villains_target_fields.contains(&target)) => return true,
            (95, 93) if self.field[turn.to] == 15 && TARGETS_FOR_LONG_WHITE.iter().any(|&target| villains_target_fields.contains(&target)) => return true,
            (25, 27) if self.field[turn.to] == 25 && TARGETS_FOR_SHORT_BLACK.iter().any(|&target| villains_target_fields.contains(&target)) => return true,
            (25, 23) if self.field[turn.to] == 25 && TARGETS_FOR_LONG_BLACK.iter().any(|&target| villains_target_fields.contains(&target)) => return true,
            _ => return false,
        }
    }


    pub(crate) fn get_target_fields_of_raw_moves(raw_moves: &Vec<usize>) -> Vec<i32> {
        let mut villains_target_fields: Vec<i32> = Vec::with_capacity(60);
        for (i, num) in raw_moves.iter().enumerate() {
            if i % 2 == 1 {
                villains_target_fields.push(*num as i32);
            }
        }
        villains_target_fields
    }


    pub(crate) fn index_of_white_king(&self) -> i32 {
        self.field.iter().position(|&x| x == 15).unwrap() as i32
    }

    pub(crate) fn index_of_black_king(&self) -> i32 {
        self.field.iter().position(|&x| x == 25).unwrap() as i32
    }


    pub(crate) fn validate_turn(&self, turn: &Turn) {
        if self.field[turn.from] < 10 { panic!("turn.from points not to a piece ({} {})", self.moves, turn.to_algebraic(true)) };
        if self.field[turn.to] != 0 && turn.capture == 0 { panic!("turn.to points not to an empty field") };
        if turn.capture != -1 && (self.field[turn.to] == 0 && turn.capture != 0) { panic!("turn.to is expected to capture") };
        if self.field[turn.to] < 0 { panic!("turn.to points not no a valid field") };
    }


    pub fn is_white_field(&self, field_index: usize) -> bool {
        if self.field[field_index] < 10 || self.field[field_index] > 25 { panic!("Can not determine turn color [{}] index:{}", self.moves, field_index) }
        if self.field[field_index] / 10 == 1 { true } else { false }
    }

    pub fn is_white_to_move(&self) -> bool {
        if self.pty % 2 == 0 { true } else { false }
    }


    pub fn set_fen(&mut self, fen: &str) {
        self.clear_field();

        let mut index = 21;

        for c in fen.chars() {
            if c == ' ' {
                break; // Stop processing FEN string once we reach the end of the board position section
            }
            if c == '/' {
                index += 2;
            } else if c.is_digit(10) {
                index += c.to_digit(10).unwrap() as usize;
            } else {
                let piece = match c {
                    'K' => 15,
                    'Q' => 14,
                    'R' => 11,
                    'B' => 13,
                    'N' => 12,
                    'P' => 10,
                    'k' => 25,
                    'q' => 24,
                    'r' => 21,
                    'b' => 23,
                    'n' => 22,
                    'p' => 20,
                    _ => 0, // Ignore invalid characters
                };
                if piece != 0 {
                    self.field[index] = piece; // Place the piece on the board
                    if piece == 15 { self.king_pos[0] = index};
                    if piece == 25 { self.king_pos[1] = index};
                }
                index += 1;
            }
        }
    }


    pub fn get_fen(&self) -> String {
        let mut fen = String::new();
        let mut empty_count = 0;

        for i in 21..99 {
            if i % 10 == 0 { continue }
            let piece = self.field[i];
            if self.field[i] == -11 {
                if empty_count > 0 {
                    fen.push_str(&empty_count.to_string());
                    empty_count = 0;
                }
                fen.push('/');
                continue;
            }
            if piece != 0 {
                if empty_count > 0 {
                    fen.push_str(&empty_count.to_string());
                    empty_count = 0;
                }
                fen.push(match piece {
                    10 => 'P',
                    11 => 'R',
                    12 => 'N',
                    13 => 'B',
                    14 => 'Q',
                    15 => 'K',
                    20 => 'p',
                    21 => 'r',
                    22 => 'n',
                    23 => 'b',
                    24 => 'q',
                    25 => 'k',
                    _ => 'x', // Placeholder for invalid pieces
                });
            } else {
                empty_count += 1;
            }
        }
        if empty_count != 0 { fen.push_str(&empty_count.to_string()) }
        fen
    }


    pub fn add_position_for_3_move_repetition_check(&mut self, fen: String) {
        *self.position_map.entry(fen).or_insert(0) += 1;
    }


    pub fn get_complexity(&self) -> i32 {
        ((self.generate_moves_list(true).len() / 2) + (self.generate_moves_list(false).len() / 2)) as i32 / 10
    }


    pub fn get_pieces_on_field(&self) -> i32 {
        self.field.iter().filter(|&x| *x > 1).count() as i32
    }

    pub fn get_piece_for_field(&self, field_nr: usize) -> &str {
        let figure = self.get_field()[field_nr] % 10;
        match figure {
            1 => "R",
            2 => "N",
            3 => "B",
            4 => "Q",
            5 => "K",
            _ => "", // Pawn
        }
    }


    pub fn get_all_made_turns(&self) -> &Vec<Turn> {
        return &self.turns;
    }

    pub fn get_last_turn(&self) -> &Turn {
        return &self.turns[self.turns.len() - 1];
    }


    /// return Value of 255 means early game and values towards 0 means endgamephase
    /// a middle value like 128 respects early and late game evaluation in the same weight
    pub fn get_game_phase(&self) -> u32 {
        let field = self.field;
        let mut phase = 0;
        for idx in 21..99 {
            if field[idx] > 0 { phase = phase + 8; } else { continue };
        }
        phase
    }


    pub fn get_pieces_map(&self) -> HashMap<i32, Vec<usize>> {
        let mut pieces_map: HashMap<i32, Vec<usize>> = HashMap::new();
        for idx in 21..99 {
            if self.field[idx] > 0 {
                pieces_map.entry(self.field[idx]).or_insert_with(Vec::new).push(idx);
            }
        }
        pieces_map
    }


    /// return the position of the kings
    /// idx 0 -> white king idx
    /// idx 1 -> black king idx
    pub fn get_king_positions(&self) -> [usize; 2] {
        self.king_pos
    }


    pub fn generate_moves_list(&self, white: bool) -> Vec<usize> {
        self.generate_moves_list_for_piece(white, 0)
    }

    pub fn generate_moves_list_for_piece(&self, white: bool, idx: usize) -> Vec<usize> {
        let king_value = if white { 15 } else { 25 };
        let queen_value = if white { 14 } else { 24 };
        let rook_value = if white { 11 } else { 21 };
        let bishop_value = if white { 13 } else { 23 };
        let knight_value = if white { 12 } else { 22 };
        let pawn_value = if white { 10 } else { 20 };

        let field = &self.field;
        let mut moves = Vec::with_capacity(64);

        let start_idx: usize = if idx == 0 { 21 } else { idx };
        let end_idx: usize = if idx == 0 { 99 } else { idx+1 };

        for i in start_idx..end_idx {
            if field[i] <= 0 { continue; }
            if field[i] >= 10 && field[i] <= 15 && !white { continue; }
            if field[i] >= 20 && field[i] <= 25 && white { continue; }

            if field[i] == king_value {
                for &offset in &[-11, -10, -9, -1, 1, 9, 10, 11] {
                    let target = (i as i32 + offset) as usize;
                    if (field[target] == 0 || field[target] / 10 == if white { 2 } else { 1 }) && field[target] != -11 {
                        moves.push(i);
                        moves.push(target);
                    }
                }
                if i == 95 {
                    if !self.turns.iter().any(|t| t.from == 95) {
                        if !self.turns.iter().any(|t| t.from == 98) {
                            if field[96] == 0 && field[97] == 0 && field[98] == 11 {
                                moves.push(i);
                                moves.push(i + 2);
                            }
                        }
                        if !self.turns.iter().any(|t| t.from == 91) {
                            if field[94] == 0 && field[93] == 0 && field[92] == 0 && field[91] == rook_value {
                                moves.push(i);
                                moves.push(i - 2);
                            }
                        }
                    }
                }
                if i == 25 {
                    if !self.turns.iter().any(|t| t.from == 25) {
                        if !self.turns.iter().any(|t| t.from == 28) {
                            if field[26] == 0 && field[27] == 0 && field[28] == 21 {
                                moves.push(i);
                                moves.push(i + 2);
                            }
                        }
                        if !self.turns.iter().any(|t| t.from == 21) {
                            if field[24] == 0 && field[23] == 0 && field[22] == 0 && field[21] == rook_value {
                                moves.push(i);
                                moves.push(i - 2);
                            }
                        }
                    }
                }
            }

            if field[i] == pawn_value {
                if white {
                    if field[i - 10] == 0 {
                        moves.push(i);
                        moves.push(i - 10);
                        if i >= 81 && i <= 88 && field[i - 20] == 0 {
                            moves.push(i);
                            moves.push(i - 20);
                        }
                    }
                    if field[i - 9] >= 20 {
                        moves.push(i);
                        moves.push(i - 9);
                    }
                    if field[i - 11] >= 20 {
                        moves.push(i);
                        moves.push(i - 11);
                    }
                } else {
                    if field[i + 10] == 0 {
                        moves.push(i);
                        moves.push(i + 10);
                        if i >= 31 && i <= 38 && field[i + 20] == 0 {
                            moves.push(i);
                            moves.push(i + 20);
                        }
                    }
                    if field[i + 9] < 20 && field[i + 9] > 0 {
                        moves.push(i);
                        moves.push(i + 9);
                    }
                    if field[i + 11] < 20 && field[i + 11] > 0 {
                        moves.push(i);
                        moves.push(i + 11);
                    }
                }
            }

            if field[i] == knight_value {
                for &offset in &[-21, -19, -12, -8, 8, 12, 19, 21] {
                    let target = (i as i32 + offset) as usize;
                    if field[target] == 0 || field[target] / 10 == if white { 2 } else { 1 } && field[target] != -11 {
                        moves.push(i);
                        moves.push(target);
                    }
                }
            }

            if field[i] == bishop_value {
                for &offset in &[-11, -9, 9, 11] {
                    let mut target = (i as i32 + offset) as usize;
                    while field[target] == 0 || field[target] / 10 == if white { 2 } else { 1 } {
                        moves.push(i);
                        moves.push(target);
                        if field[target] != 0 { break }
                        target = (target as i32 + offset) as usize;
                    }
                }
            }

            if field[i] == queen_value {
                for &offset in &[-11, -10, -9, -1, 1, 9, 10, 11] {
                    let mut target = (i as i32 + offset) as usize;
                    while (field[target] == 0  || field[target] / 10 == if white { 2 } else { 1 }) && field[target] != -11 {
                        moves.push(i);
                        moves.push(target);
                        if field[target] != 0 { break }
                        target = (target as i32 + offset) as usize;
                    }
                }
            }

            if field[i] == rook_value {
                for &offset in &[-10, 10, -1, 1] {
                    let mut target = (i as i32 + offset) as usize;
                    while (field[target] == 0  || field[target] / 10 == if white { 2 } else { 1 }) && field[target] != -11 {
                        moves.push(i);
                        moves.push(target);
                        if field[target] != 0 { break }
                        target = (target as i32 + offset) as usize;
                    }
                }
            }

        }
        moves
    }

    pub fn king_in_chess(&self, white_king: bool) -> bool {
        let king_idx = if white_king {
            self.index_of_white_king()
        } else {
            self.index_of_black_king()
        } as isize;

        for &offset in &[-21, -19, -12, -8, 8, 12, 19, 21] {
            let target = (king_idx + offset) as usize;
            if white_king && self.field[target] == 22 {
                return true;
            }
            if !white_king && self.field[target] == 12 {
                return true;
            }
        }

        let enemy_bishop_queen = if white_king { [23, 24] } else { [13, 14] };

        for &offset in &[-11, -9, 9, 11, ] {
            let mut target = king_idx + offset;

            // Keep checking along the diagonal until you find a piece or go out of bounds
            while target >= 0 && target < self.field.len() as isize {
                // Convert the target back to usize for indexing
                let target_idx = target as usize;

                // If there's a piece that is not a bishop/queen, stop checking this diagonal
                if self.field[target_idx] != 0 && !enemy_bishop_queen.contains(&self.field[target_idx]) {
                    break;
                }

                // If an enemy bishop/queen is found, the king is in check
                if enemy_bishop_queen.contains(&self.field[target_idx]) {
                    return true;
                }

                // Update target to the next cell in the diagonal
                target += offset;
            }
        }


        let enemy_rook_queen = if white_king { [21, 24] } else { [11, 14] };

        for &offset in &[-10, -1, 1, 10] {
            let mut target = king_idx;

            // Loop to check the straight lines from the king's position
            while target >= 0 && target < self.field.len() as isize {
                let target_idx = target as usize;

                // Check if the current target position is valid for a rook or queen
                if self.field[target_idx] != 0 && !enemy_rook_queen.contains(&self.field[target_idx]) {
                    break;
                }

                // If an enemy rook/queen is found, the king is in check
                if enemy_rook_queen.contains(&self.field[target_idx]) {
                    return true;
                }

                // Move to the next cell in the line
                target += offset;

                // Check boundaries to make sure we don't wrap around to the other side of the board
                if offset.abs() == 1 && (target % 10 == 9 || target % 10 == 0) {
                    // Break if we reach the end of the row in horizontal directions
                    break;
                }
            }
        }

        if white_king {
            if king_idx-9 == 20 || king_idx-11 == 20 { return true }
        } else {
            if king_idx+9 == 10 || king_idx+11 == 10 { return true }
        }

        false
    }



}