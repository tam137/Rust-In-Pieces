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
    let ctx = suprah::model::SearchContext {
        zobrist_table: &z,
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

#[test]
fn test_easy_move_failing() {
    let service = suprah::service::Service::new();
    let mut board = service.fen.set_fen("r2q1knr/p1pb2p1/2pb1pp1/3p2BQ/3P4/8/PPP2PPP/RN3RK1 w - - 0 12");
    let active_config = suprah::config::Config::new();
    let engine_state = std::sync::Arc::new(suprah::model::EngineState::new());
    
    let mut best_result: Option<suprah::model::SearchResult> = None;
    let mut pv_stable_count = 0;
    let mut last_best_move = String::new();
    
    let go_start_time = std::time::Instant::now();
    
    for depth in 2..=10 {
        let mut stats = suprah::model::Stats::default();
        let is_white = board.white_to_move;
        
        let search_result = service.search.get_moves(
            &mut board,
            depth,
            is_white,
            &mut stats,
            &active_config,
            &service,
            &engine_state,
            go_start_time,
            Some(10000), // target time 10 seconds
        );
        
        if search_result.completed {
            best_result = Some(search_result.clone());
            let current_best_move = search_result.get_best_move_algebraic();
            println!(
                "Depth {}: best_move={}, best_score={}, second_best_score={}",
                depth,
                current_best_move,
                search_result.best_score,
                search_result.second_best_score
            );
            
            if current_best_move == last_best_move {
                pv_stable_count += 1;
            } else {
                pv_stable_count = 0;
                last_best_move = current_best_move;
            }
            
            let is_easy = if is_white {
                search_result.best_score.saturating_sub(search_result.second_best_score) >= active_config.easy_move_margin
            } else {
                search_result.second_best_score.saturating_sub(search_result.best_score) >= active_config.easy_move_margin
            };
            
            println!(
                "  pv_stable_count={}, is_easy={}, margin={}",
                pv_stable_count,
                is_easy,
                active_config.easy_move_margin
            );
        }
    }
}

