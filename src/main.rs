mod fen_service;
mod move_gen_service;
mod notation_util;
mod model;

use crate::fen_service::FenServiceImpl;
use crate::move_gen_service::MoveGenService;

fn main() {

    let fen_service = FenServiceImpl;
    let move_gen_service = MoveGenService;

    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut board = fen_service.set_fen(fen);
    let valid_moves = move_gen_service.generate_valid_moves_list(&mut board);
    println!("Valid Moves: {:?}", valid_moves);
}
