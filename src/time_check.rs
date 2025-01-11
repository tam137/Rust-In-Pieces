use std::sync::{RwLock, Arc};
use std::thread;

use crate::ThreadSafeDataMap;
use crate::Service;
use crate::DataMap;
use crate::Config;
use crate::Stats;
use crate::notation_util::NotationUtil;

use crate::RIP_STD_IN_THREAD_PANICKED;


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


pub fn run_time_check(global_map: &ThreadSafeDataMap, mut local_map: &mut DataMap) {
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

    println!("\nexpected <30µs");
    local_map.insert(crate::model::DataMapKey::ForceSkipValidationFlag, true);
    time_it!(service.move_gen.generate_valid_moves_list(&mut board, &mut stats, service, config, global_map, &local_map));

    println!("\nexpected <10µs");
    local_map.insert(crate::model::DataMapKey::ForceSkipValidationFlag, false);
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
    time_it!(service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, false, &mut Stats::new(), config, service, global_map, &mut local_map));
    
    println!("\nexpected <180ms");
    let mid_game_fen = "r1bqr1k1/2p2ppp/p1np1n2/1pb1p1N1/2BPP3/2P1B3/PPQ2PPP/RN3RK1 w - - 0 10";
    time_it!(service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, true, &mut Stats::new(), config, service, global_map, &mut local_map));

    println!("\nexpected ~550-600");
    println!("Benchmark Value: {}\n", calculate_benchmark(global_map, &mut local_map));


    // tactical midgame
    let global_map_t1 = Arc::clone(global_map);
    let mut local_map_t1 = local_map.clone();
    let tactical_modgame_test_thread = thread::spawn(move || {
        let mut fen_list = Vec::default();
        fen_list.push("r1bqkb1r/pppp1ppp/2n2n2/4p1N1/2B1P3/8/PPPP1PPP/RNBQK2R b KQkq - 5 4");
        fen_list.push("r1bqk2r/p4pp1/2p2n1p/n1b5/B2Pp3/5N2/PPP1QPPP/RNB1K2R b KQkq - 0 11");
        fen_list.push("r1bq1rk1/pppp1ppp/1b3n2/n3p1B1/2BPP3/2P2N2/PP3PPP/RN1Q1RK1 w - - 5 8");
        fen_list.push("r3qrk1/ppp3p1/2n1b2p/2bnpp2/8/PQPP1NBP/1P1N1PP1/2R1KB1R b K - 3 13");
        fen_list.push("r2qk2r/p1p2pp1/2n1bn1p/1pbpp3/4P2B/1PNP1N2/P1P1BPPP/R2Q1RK1 b kq - 0 9");
        count_and_print_nodes("tactical midgame Queen and Rooks", 897, fen_list, &global_map_t1, &mut local_map_t1);
    });

    // quite d4 opening (positional)
    let mut fen_list = Vec::default();
    fen_list.push("rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1");
    fen_list.push("rnbqkbnr/ppp2ppp/4p3/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR b KQkq - 1 3");
    fen_list.push("rnbqkb1r/pp3ppp/2p2n2/3p4/3P1B2/2N2N2/PP2PPPP/R2QKB1R b KQkq - 1 6");
    fen_list.push("rn1q1rk1/pp2bppp/2p2n2/3p1b2/3P1B2/2N1PN1P/PP3PP1/R2QKB1R w KQ - 1 9");
    fen_list.push("r4rk1/pp1nbppp/1qp2n2/3p4/3P1B2/2NQPN1P/PP3PP1/R4RK1 w - - 3 12");
    count_and_print_nodes("positional d4 opening", 138, fen_list, global_map, local_map);

    // e4 opening (some tactics)
    let mut fen_list = Vec::default();
    fen_list.push("rnbqkb1r/pppp1ppp/5n2/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3");
    fen_list.push("r1bqkbnr/1ppp1ppp/p1n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
    fen_list.push("r1bqkbnr/1pp2ppp/p1np4/1B2p3/3PP3/5N2/PPP2PPP/RNBQ1RK1 b kq - 1 5");
    fen_list.push("r1bqkb1r/1ppp1ppp/p1n2n2/4p3/B3P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 3 5");
    fen_list.push("r1bqk2r/2pp1ppp/p1n5/1pb1p3/4P1n1/1BPP1N2/PP3PPP/RNBQK2R w KQkq - 1 8");
    count_and_print_nodes("some tactical e4 opening", 42, fen_list, global_map, local_map);

    // quite midgame
    let mut fen_list = Vec::default();
    fen_list.push("r1bq1rk1/2pp1ppp/p1n2n2/1pb1p3/4P3/1BPP1N1P/PP3PP1/RNBQ1RK1 w - - 1 10");
    fen_list.push("rnb2rk1/p1p1qpp1/1p5p/3p4/3P4/4PN2/PP3PPP/R2QKB1R w KQ - 0 11");
    fen_list.push("rnbq1rk1/p3bpp1/2p1pn1p/1p1p4/2PP1B2/2N1PNP1/PP3PBP/R2QK2R b KQ - 1 9");
    fen_list.push("r1b1rnk1/pp3pp1/2pq1n1p/3p4/3P4/2NBPN1P/PPQ2PP1/1R3RK1 b - - 1 14");
    fen_list.push("r1b3k1/pp3pp1/2p2n1p/3pq1n1/1P6/2NBP2P/P1Q2PP1/1R3RK1 w - - 0 18");
    count_and_print_nodes("quite midgame Queen and Rooks", 280, fen_list, global_map, local_map);

    // engame with rooks
    let mut fen_list = Vec::default();
    fen_list.push("2r5/3k4/1p1pRK2/5P2/6P1/7P/8/8 w - - 0 1");
    fen_list.push("4b3/3kn3/1p1p4/p1r1PP2/5KP1/2R1NN2/8/8 w - - 0 1");
    fen_list.push("8/3k4/1r6/p1r5/1p3K2/1P4R1/P1P2R2/8 b - - 0 1");
    fen_list.push("8/3k4/1r6/p6q/1p3K2/1P1Q2R1/P7/8 b - - 0 1");
    fen_list.push("3r4/1n3K2/4RP2/6k1/8/4P3/8/8 b - - 0 1");
    count_and_print_nodes("engame with rooks", 7, fen_list, global_map, local_map);

    tactical_modgame_test_thread.join().expect(RIP_STD_IN_THREAD_PANICKED);
    

}

pub fn count_and_print_nodes(description: &str, expected_count: i32, fen_list: Vec<&str>, global_map: &ThreadSafeDataMap, mut local_map: &mut DataMap) {
    let mut stats = Stats::new();
    let service = Service::new();
    let config = Config::new().for_tests();
    let mut node_count = 0;

    for fen in fen_list {
        // println!("calc {}", fen);
        let board = &mut service.fen.set_fen(&fen);
        service.search.get_moves(board, 4, board.white_to_move, &mut stats, &config, &service, global_map, &mut local_map);
        node_count = node_count + stats.calculated_nodes;
        stats = Stats::new();
    }

    println!("{}k\t{}k\t{}", node_count / 1000, expected_count, description);
}


pub fn calculate_benchmark(global_map: &Arc<RwLock<DataMap>>, mut local_map: &mut DataMap) -> i32 {
    let mut board = Service::new().fen.set_fen("r1bqkbnr/1ppp1ppp/p1n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
    let service = Service::new();
    let config = &Config::new().for_tests();
    10000 / get_time_it!(service.search.get_moves(&mut board, 3, true, &mut Stats::new(), &config, &service, global_map, &mut local_map))
}