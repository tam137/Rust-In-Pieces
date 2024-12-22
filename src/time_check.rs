use std::sync::{RwLock, Arc};

use crate::ThreadSafeDataMap;
use crate::Service;
use crate::DataMap;
use crate::Config;
use crate::Stats;
use crate::notation_util::NotationUtil;



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

macro_rules! get_time_it {
    ($expr: expr) => {{
        let start = std::time::Instant::now();
        $expr;
        let end = std::time::Instant::now();
        let duration = end.duration_since(start);
        let duration_ms: i32 = duration.as_millis().min(i32::MAX as u128) as i32;
        duration_ms
    }};
}


pub fn run_time_check(global_map: &ThreadSafeDataMap, local_map: &DataMap) {
    let service = &Service::new();
    let config = &Config::new().for_tests();
    let mut stats = Stats::new();

    println!("expected <10µs");
    let mut board = time_it!(service.fen.set_fen("r1bqr1k1/ppp2ppp/2np1n2/2b1p3/2BPP3/2P1BN2/PPQ2PPP/RN3RK1 b - - 5 8"));

    let turn = &NotationUtil::get_turn_from_notation("f6e4");

    println!("\nexpected <5µs");
    time_it!(board.hash());

    println!("\nexpected <5µs");
    let mi = time_it!(board.do_move(turn));

    println!("\nexpected <1µs");
    time_it!(board.undo_move(turn, mi));

    println!("\nexpected ~1µs");
    time_it!(service.move_gen.get_check_idx_list(&board.field, true));

    println!("\nexpected <10µs>");
    time_it!(service.move_gen.generate_moves_list_for_piece(&board, 0));

    println!("\nexpected <130µs");
    time_it!(service.move_gen.generate_valid_moves_list(&mut board, &mut stats, service, config, global_map, &local_map));

    println!("\nexpected <10µs");
    time_it!(service.move_gen.generate_valid_moves_list_capture(&mut board, &mut stats, config, service, global_map, &local_map));

    println!("\nexpected ~1µs");
    time_it!(service.eval.calc_eval(&board, &config, &service.move_gen));

    println!("\nexpected <1000ns");
    let board = service.fen.set_fen("r1q2r1k/1pp1bpp1/p2p1n2/4P2p/2Q2B2/2N4P/PPPR1PP1/3R2K1 b - - 3 16");
    time_it!(service.move_gen.get_attack_idx_list(&board.field, board.white_to_move, 44));

    println!("\nexpected <2µs");
    let board = service.fen.set_fen("r1q2r1k/1pp1bpp1/p2p1n2/4P2p/2Q2B2/2N4P/PPPR1PP1/3R2K1 b - - 3 16");
    time_it!(service.move_gen.get_attack_idx_list_with_shadow(&board.field, board.white_to_move, 44));
    time_it!(service.move_gen.get_attack_idx_list_with_shadow(&board.field, board.white_to_move, 33));

    println!("\nexpected ~100ns");
    let _my_field: [i32; 120] = time_it!(board.field.try_into().expect("RIP Invalid field size"));
    
    println!("\nexpected <220ms");
    let mid_game_fen = "r1bqr1k1/ppp2ppp/2np1n2/2b1p3/2BPP3/2P1BN2/PPQ2PPP/RN3RK1 b - - 5 8";
    time_it!(service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, false, &mut Stats::new(), config, service, global_map, &local_map));
    
    println!("\nexpected <180ms");
    let mid_game_fen = "r1bqr1k1/2p2ppp/p1np1n2/1pb1p1N1/2BPP3/2P1B3/PPQ2PPP/RN3RK1 w - - 0 10";
    time_it!(service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, true, &mut Stats::new(), config, service, global_map, &local_map));

    println!("\nexpected ~715");
    println!("Benchmark Value: {}", calculate_benchmark(global_map, &local_map));
}


pub fn calculate_benchmark (global_map: &Arc<RwLock<DataMap>>, local_map: &DataMap) -> i32 {
    let mut board = Service::new().fen.set_fen("r1bqkbnr/1ppp1ppp/p1n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
    let service = Service::new();
    let config = &Config::new().for_tests();
    10000 / get_time_it!(service.search.get_moves(&mut board, 3, true, &mut Stats::new(), &config, &service, global_map, local_map))
}