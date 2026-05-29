use suprah::fen_service;
use suprah::move_gen_service;

fn main() {
    let board = fen_service::parse_fen("r1b1k2r/ppp1b1pp/2p5/4N3/3PR3/6Pq/PPP4P/R1BQ2K1 w kq - 1 15");
    let moves = move_gen_service::generate_all_legal_moves(&board);
    for m in moves {
        println!("{}", m.to_string());
    }
}
