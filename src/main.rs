mod fen_service;
mod move_gen_service;
mod notation_util;
mod model;
mod eval;
mod config;

use crate::config::Config;
use crate::eval::calc_eval;
use crate::fen_service::FenServiceImpl;
use crate::move_gen_service::MoveGenService;


macro_rules! time_it {
    ($expr: expr) => {{
        let start = std::time::Instant::now();
        let result = { $expr };
        let end = std::time::Instant::now();
        let duration = end.duration_since(start);
        println!("Time taken [{}]: {:?}", stringify!($expr), duration);
        result
    }};
}

fn main() {

    let fen_service = FenServiceImpl;
    let move_gen_service = MoveGenService;

    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut board = time_it!(fen_service.set_fen(fen));
    time_it!(move_gen_service.generate_valid_moves_list(&mut board)); // ~ 12.000µs
    time_it!(calc_eval(&board, &Config::new())); // ~ 300ns

}
