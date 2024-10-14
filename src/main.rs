mod fen_service;
mod move_gen_service;
mod notation_util;
mod model;
mod eval_service;
mod config;
mod search_service;
mod service;

use crate::config::Config;
use crate::model::Stats;
use crate::service::Service;


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

    let service = &Service::new();
    let mut config = &Config::new();

    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut board = time_it!(service.fen.set_fen(fen)); // ~3.000µs
    time_it!(service.move_gen.generate_valid_moves_list(&mut board, &mut Stats::new(), service)); // ~ 13.000µs
    time_it!(service.eval.calc_eval(&board, &mut config)); // ~ 300ns
    let result = time_it!(service.search.get_moves(&mut service.fen.set_fen(fen), 1, true, &mut Stats::new(), &Config::new(), service)); // ~ 18.000µs
    println!("{:?}", result);


}
