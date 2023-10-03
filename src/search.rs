use std::cmp::min;
use std::collections::VecDeque;
use crate::Board;
use crate::config::Config;
use crate::stats::Stats;

use crate::Turn;

mod search_basic;
mod search_alphabeta;
mod search_zobrist;
mod search_quite;


#[derive(Clone)]
pub enum SearchAlgo {
    Basic,
    AlphaBeta,
    Zobrist,
    Quiescence
}

pub fn get_best_move_as_min_max_result(mut board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> MinMaxResult {
    let min_max_raw_result: Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)> = search_quite::get_moves(board, depth, white, stats, config);
    MinMaxResult::new(min_max_raw_result)
}

pub fn get_best_move(mut board: &mut Board, depth: i32, white: bool, stats: &mut Stats, config: &Config) -> (Option<Turn>, i16, VecDeque<Option<Turn>>) {
    let min_max_result: Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)> = search_quite::get_moves(board, depth, white, stats, config);
    min_max_result.get(0).unwrap().clone()
}

pub struct MinMaxResult {
    pub moves: Vec<(Vec<Turn>, i16)>,
}

impl MinMaxResult {
    pub fn new(min_max_raw_result: Vec<(Option<Turn>, i16, VecDeque<Option<Turn>>)>) -> MinMaxResult {
        let mut moves = vec![];
        for (turn, eval, move_row) in min_max_raw_result {
            let move_row: Vec<Turn> = move_row.iter().filter_map(|option_turn| option_turn.clone())
                                              .collect();
            moves.push((move_row, eval));
        }
        MinMaxResult {
            moves,
        }
    }

    pub fn get_best_turn(&self) -> &Turn {
        self.moves.get(0).unwrap().0.get(0).unwrap()
    }

    pub fn get_best_move_row_str(&self) -> String {
        let mut res = self.moves.get(0).unwrap().clone().0;

        let move_row: String = res.iter()
            .map(|t| t.to_algebraic(false))
            .collect::<Vec<String>>()
            .join(" ");

        String::from(move_row)
        //String::from(res.join(" "))
    }

    pub fn get_depth(&self) -> usize {
        0
    }

    pub fn get_eval(&self) -> i16 {
        0
    }
}