#[test]
fn test_my_bug_fen() {
    let service = suprah::service::Service::new();
    let mut board = service.fen.set_fen("r1b1k2r/ppp1b1pp/2p5/4N3/3PR3/6Pq/PPP4P/R1BQ2K1 w kq - 1 15");
    let mut moves = suprah::model::MoveList::new();
    let config = suprah::config::Config::for_tests();
    let z = suprah::zobrist::ZobristTable::with_capacity(1);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let pv = std::sync::Mutex::new(std::collections::HashMap::new());
    let history = [[0u32; 64]; 64];
    let pawn_table = suprah::pawn_hash::PawnHashTable::with_capacity(1);
    let ctx = suprah::model::SearchContext {
        zobrist_table: &z,
        pawn_table: &pawn_table,
        stop_flag: &stop,
        pv_nodes: &pv,
        killer_moves: [None; 2],
        history_table: &history,
        counter_move: None,
        start_time: std::time::Instant::now(),
        target_time: None,
        root_moves_total: 0,
        root_moves_searched: 0,
    };
    service.move_gen.generate_valid_moves_list(&mut board, &mut suprah::model::Stats::new(), &config, &ctx, true, false, &mut moves);
    let mut found = false;
    for m in moves.as_slice() {
        println!("MOVE: {}", m.to_algebraic());
        if m.to_algebraic() == "e4h4" {
            found = true;
        }
    }
    assert!(found, "e4h4 not generated!");
}

