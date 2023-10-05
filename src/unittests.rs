use crate::{Board, eval};
use crate::board::GameState;
use crate::Turn;
use crate::search;
use crate::Stats;
use crate::config::Config;
use crate::search::SearchAlgo;
use eval::calc_push_to_king;

pub fn run_unittests() {
    eval_003();
    pty_005();
    castle_006();
    turn_color_008();
    advanced_castle_007();
    fen_009();
    promotion_010();
    end_game_011();
    static_board_function_012();
    is_quite_board_check_013();
    zobrist_014();
    quiescence_015();
    recognize_chess_016();
    // opening_situations_050();
    //move_row_016();
    analyse();
    println!("finished unittests")
}

#[track_caller]
fn assert(condition: bool) {
    if !condition {
        let location = std::panic::Location::caller();
        panic!("Unittest failed at {}", location);
    }    
}


fn move_gen_001() {
    let mut board = Board::new();
    assert(board.get_turn_list(true, false, &mut Stats::new()).len() == 20);
    assert(board.get_turn_list(false, false, &mut Stats::new()).len() == 20);
    board.clear_field();
    assert(board.generate_moves_list(false).len() == 0);

    board = Board::new();
    board.do_turn(&Turn::generate_turns("a2a3")[0]);
    assert(board.get_turn_list(true, false, &mut Stats::new()).len() == 19);
}

fn turn_gen_002() {
    let mut turn_list = Turn::generate_turns("e2e4 d7d5");
    turn_list.push(Turn {from: 65, to: 54, capture: 20, post_villain: Vec::new(), post_my: Vec::new(), promotion: false, gives_chess:false, eval: 0}); // e4xd5
    turn_list.push(Turn {from: 24, to: 54, capture: 10, post_villain: Vec::new(), post_my: Vec::new(), promotion: false, gives_chess:false, eval: 0}); // Dd8xd5
    let mut cmp_board = Board::new();
    cmp_board.set_field_index(85, 0);
    cmp_board.set_field_index(65, 0);
    cmp_board.set_field_index(34, 0);
    cmp_board.set_field_index(24, 0);
    cmp_board.set_field_index(54, 24);
    
    let mut board = Board::new();
    turn_list.iter().for_each(|turn| board.do_turn(turn));

    assert!(board == cmp_board);

    for undo_turn in turn_list.iter().rev() {
        board.do_undo_turn(&undo_turn);
    }

    assert!(board == Board::new());

    let turn = &Turn::generate_turns("d7d5 g1f3")[0];
    assert(turn.to_algebraic(false) == "d7d5");
    let turn = &Turn::generate_turns("d7d5 g1f3")[1];
    assert(turn.to_algebraic(false) == "g1f3");
}


fn eval_003() {
    let mut eval = test_helper::get_static_eval_for_fen("8/8/8/3k4/3BN3/8/8/3K4", calc_push_to_king);
    assert(eval > 200);

    let eval = test_helper::get_static_eval_for_fen("8/8/3R4/3k4/8/3K4/3b4/8", calc_push_to_king);
    assert(eval == 0);

    let mut eval = test_helper::get_static_eval_for_fen("8/2R5/2k5/8/8/5K2/5r2/8", calc_push_to_king);
    assert(eval == 0);

    eval = test_helper::get_static_eval_for_fen("8/2R5/2k1Q3/8/8/3q1K2/5r2/8", calc_push_to_king);
    assert(eval == 0);

    eval = test_helper::get_static_eval_for_fen("8/2R1B3/2k1Q3/3N4/4n3/3q1K2/3b1r2/8", calc_push_to_king);
    assert(eval == 0);

}

fn pty_005() {
    let mut board = Board::new();
    assert(board.get_pty() == 0);
    assert(board.is_white_to_move() == true);
    let turn_list = Turn::generate_turns("e2e4 d7d5");
    turn_list.iter().for_each(|turn| board.do_turn(turn));
    assert(board.is_white_to_move() == true);
    assert(board.get_pty() == 2);
    board.do_undo_turn(turn_list.get(1).unwrap());
    assert(board.is_white_to_move() == false);
    assert(board.get_pty() == 1);
}

fn castle_006() {
    let mut cmp_board = Board::new();
    let mut board = Board::new();
    board.set_field_index(97, 0);
    board.set_field_index(96, 0);
    let turn = &Turn::generate_turns("e1g1")[0];
    board.do_turn(turn);
    cmp_board.set_field_index(98, 0);
    cmp_board.set_field_index(97, 15);
    cmp_board.set_field_index(96, 11);
    cmp_board.set_field_index(95, 0);
    assert(board == cmp_board);

    let mut cmp_board = Board::new();
    let mut board = Board::new();
    board.set_field_index(92, 0);
    board.set_field_index(93, 0);
    board.set_field_index(94, 0);
    let turn = &Turn::generate_turns("e1c1")[0];
    board.do_turn(turn);
    cmp_board.set_field_index(91, 0);
    cmp_board.set_field_index(92, 0);
    cmp_board.set_field_index(93, 15);
    cmp_board.set_field_index(94, 11);
    cmp_board.set_field_index(95, 0);
    assert(board == cmp_board);

    let mut cmp_board = Board::new();
    let mut board = Board::new();
    board.set_field_index(27, 0);
    board.set_field_index(26, 0);
    let turn = &Turn::generate_turns("e8g8")[0];
    board.do_turn(turn);
    cmp_board.set_field_index(28, 0);
    cmp_board.set_field_index(27, 25);
    cmp_board.set_field_index(26, 21);
    cmp_board.set_field_index(25, 0);
    assert(board == cmp_board);

    let mut cmp_board = Board::new();
    let mut board = Board::new();
    board.set_field_index(22, 0);
    board.set_field_index(23, 0);
    board.set_field_index(24, 0);
    let turn = &Turn::generate_turns("e8c8")[0];
    board.do_turn(turn);
    cmp_board.set_field_index(21, 0);
    cmp_board.set_field_index(22, 0);
    cmp_board.set_field_index(23, 25);
    cmp_board.set_field_index(24, 21);
    cmp_board.set_field_index(25, 0);
    assert(board == cmp_board);

    let mut cmp_board = Board::new();
    cmp_board.set_field_index(22, 0);
    cmp_board.set_field_index(23, 0);
    cmp_board.set_field_index(24, 0);
    let mut board = Board::new();
    board.set_field_index(22, 0);
    board.set_field_index(23, 0);
    board.set_field_index(24, 0);
    let turn = &Turn::generate_turns("e8c8")[0];
    board.do_turn(turn);
    board.do_undo_turn(turn);
    assert(board == cmp_board);

    let mut cmp_board = Board::new();
    cmp_board.set_field_index(26, 0);
    cmp_board.set_field_index(27, 0);
    let mut board = Board::new();
    board.set_field_index(26, 0);
    board.set_field_index(27, 0);
    let turn = &Turn::generate_turns("e8g8")[0];
    board.do_turn(turn);
    board.do_undo_turn(turn);
    assert(board == cmp_board);

    let mut cmp_board = Board::new();
    cmp_board.set_field_index(92, 0);
    cmp_board.set_field_index(93, 0);
    cmp_board.set_field_index(94, 0);
    let mut board = Board::new();
    board.set_field_index(92, 0);
    board.set_field_index(93, 0);
    board.set_field_index(94, 0);
    let turn = &Turn::generate_turns("e1c1")[0];
    board.do_turn(turn);
    board.do_undo_turn(turn);
    assert(board == cmp_board);

    let mut cmp_board = Board::new();
    cmp_board.set_field_index(96, 0);
    cmp_board.set_field_index(97, 0);
    let mut board = Board::new();
    board.set_field_index(96, 0);
    board.set_field_index(97, 0);
    let turn = &Turn::generate_turns("e1g1")[0];
    board.do_turn(turn);
    board.do_undo_turn(turn);
    assert(board == cmp_board);
}

pub fn advanced_castle_007() {
    let mut board = Board::new();
    let fen =  "4k3/8/8/8/8/8/7P/4K2R";
    let fen2 = "4k3/8/8/8/8/3n4/7P/4K2R"; // white king is in chess (can not castle)
    board.set_fen(fen);
    let move_list = board.get_turn_list(true, false, &mut Stats::new());
    board.set_fen(fen2);
    let move_list_2 = board.get_turn_list(true, false, &mut Stats::new());
    assert!(move_list_2.len() + 6 == move_list.len());

    board.set_fen(fen);
    let castle_move = Turn::generate_turns("e1g1");
    board.do_turn(&castle_move[0]);
    assert(board.get_fen() == "4k3/8/8/8/8/8/7P/5RK1");

    board.set_fen("4k3/8/b7/8/8/8/7P/4K2R");
    let move_list = board.get_turn_list(true, false, &mut Stats::new());
    assert!(move_list.len() == 7);

    let mut board = Board::new();
    board.set_fen("r1bqk1nr/pppp1pp1/2nb3p/4p3/2B1P3/2NP1N2/PPP2PPP/R1BQK2R");
    let best_white_move = search::get_best_move(&mut board, 2, true, &mut Stats::new(), &Config::new().unittest()).0.unwrap();
    //assert(best_white_move.to_algebraic(false) == "e1g1");
}


pub fn turn_color_008() {
    let board = Board::new();
    let turns = Turn::generate_turns("d2d3 e7e6");
    assert(board.is_white_field(turns[0].from) == true);
    assert(board.is_white_field(turns[1].from) == false);
}

pub fn fen_009() {
    let mut board = Board::new();
    assert(board.get_fen() == "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
    let turns = Turn::generate_turns("e2e4 e7e5 f1c4 f8b4 c2c3 d8f6 c3b4 f6f3 g1f3 d7d5 e1g1");
    for turn in turns {
        board.do_turn(&turn);
    }
    assert(board.get_fen() == "rnb1k1nr/ppp2ppp/8/3pp3/1PB1P3/5N2/PP1P1PPP/RNBQ1RK1");
    board.clear_field();

    board.set_fen("rnb1k1nr/ppp2ppp/8/3pp3/1PB1P3/5N2/PP1P1PPP/RNBQ1RK1");
    assert(board.get_fen() == "rnb1k1nr/ppp2ppp/8/3pp3/1PB1P3/5N2/PP1P1PPP/RNBQ1RK1");

    board.set_fen("4P3/8/8/2k5/7P/4K3/p7/4P3");
    assert(board.get_fen() == "4P3/8/8/2k5/7P/4K3/p7/4P3");   
}

pub fn promotion_010() {
    let turn_list = Turn::generate_turns("e7e8q");
    let mut board = Board::new();
    board.set_fen("8/4P3/8/8/1K4p1/6k1/7n/8");
    board.do_turn(turn_list.get(0).unwrap());
    test_helper::get_bestmove_for_fen(&*board.get_fen(), false);

    let res = test_helper::get_bestmove_for_fen("8/4P3/8/8/1K4p1/6k1/7n/8", true);
    test_helper::assert::equal_move(res, "e7e8q");

    let mut board = Board::new();
    board.clear_field();
    board.set_field_index(33, 10);
    board.set_field_index(83, 20);
    board.set_field_index(98, 15);
    board.set_field_index(96, 25);
    let mut cmp_board = board.clone();
    let turn_white = &cmp_board.get_turn_list(true, false, &mut Stats::new())[0];
    assert(turn_white.is_promotion() == true);
    let turn_black = &board.get_turn_list(false, false, &mut Stats::new())[0];
    assert(turn_black.is_promotion() == true);
    board.do_turn(turn_white);
    board.do_turn(turn_black);
    board.do_undo_turn(turn_white);
    board.do_undo_turn(turn_black);
    assert(cmp_board == board);

    assert(board.get_fen() == "8/2P5/8/8/8/8/2p5/5k1K");
    let best_white_move = search::get_best_move(&mut board, 2, true, &mut Stats::new(), &mut Config::new().unittest()).0.unwrap();
    board.do_turn(&best_white_move);

    board.set_fen("8/2P5/8/8/8/8/2p5/1R3k1K");
    let best_black_move = search::get_best_move(&mut board, 2, false, &mut Stats::new(), &mut Config::new().unittest()).0.unwrap();
    board.do_turn(&best_white_move);
    assert(best_black_move.to_algebraic(false) == "c2b1q");
    board.do_undo_turn(&best_white_move);
    assert(board.get_fen() == "8/2P5/8/8/8/8/2p5/1R3k1K");

    board.set_fen("8/5P2/8/8/1k6/8/1K5p/8");
    let all_turns = board.get_turn_list(true, false, &mut Stats::new());
    let mut turn = &mut Turn::generate_turns("f7f8q")[0];
    turn.enrich_move(&board, true);
    board.do_turn(turn);
    assert(board.get_field()[26] == 14);

    let turn = &Turn::generate_turns("d7d8q")[0];
    assert!(turn.is_promotion());

}

pub fn end_game_011() {
    let mut board = Board::new();
    board.set_fen("r3k2r/p1p2pp1/2p5/3p3p/3Pn1b1/4P1q1/PPP5/RNB1KQNR");
    let turn_list = board.get_turn_list(true, false, &mut Stats::new());
    assert(turn_list.len() == 1);
    //let best_move = &search::get_best_move(&mut board, 4, true, &mut Stats::new(), &.0.unwrap();
    //assert("f1f2" == best_move.to_algebraic());

    board.set_fen("8/8/8/2K5/k7/8/R2N4/8");
    let turn_list = board.get_turn_list(false, false, &mut Stats::new());
    assert(turn_list.len() == 0);

    board.clear_field();
    board.set_field_index(33, 25);
    board.set_field_index(63, 15);
    assert(board.get_state() != &GameState::Draw);
    board.get_turn_list(true, false, &mut Stats::new());
    assert(board.get_state() == &GameState::Draw);
}

pub fn static_board_function_012() {
    let mut board = Board::new();
    assert(board.get_complexity() == 4);
    assert(board.get_pieces_on_field() == 32);
    board.set_field_index(91, 0);
    assert(board.get_complexity() == 4);
    assert(board.get_pieces_on_field() == 31);
    board.set_field_index(86, 0);
    assert(board.get_pieces_on_field() == 30);
    assert(board.get_complexity() == 3);
}

pub fn is_quite_board_check_013() {
    let mut board = Board::new();
    let mut white_moves = board.generate_moves_list(true);
    let mut black_moves = board.generate_moves_list(false);
    let mut quite = Board::is_quite_board(&white_moves, &black_moves);
    assert(quite == true);
    board.set_field_index(84, 0);
    white_moves = board.generate_moves_list(true);
    black_moves = board.generate_moves_list(false);
    quite = Board::is_quite_board(&white_moves, &black_moves);
    assert(quite == false);
    let quite_white = Board::is_quite_board_for_white(&white_moves, &black_moves);
    let quite_black = Board::is_quite_board_for_black(&white_moves, &black_moves);
    assert(quite_white == true);
    assert(quite_black == false);
}

pub fn zobrist_014() {
    let mut board = Board::new();
    assert(board.get_hash() == 9054072837742332314);
    let turn = &Turn::generate_turns("e2e4")[0];
    board.do_turn(turn);
    assert(board.get_hash() == 12430707902562095564);

    board = Board::new();
    let mut stats = Stats::new();
    let turn = search::get_best_move(&mut board, 4, true, &mut stats, &mut Config::new());
    board.do_turn(&turn.0.unwrap());
    stats.reset_stats();
    search::get_best_move(&mut board, 4, false, &mut stats, &mut Config::new());
}

pub fn quiescence_015() {
     let mut res = test_helper::get_bestmove_for_fen("1k6/8/3n2b1/p4p2/1p2n3/5PP1/8/1K2Q3", true);
     test_helper::assert::equal_move(res, "f3e4");

    res = test_helper::get_bestmove_for_fen("1k6/8/3n2b1/5p2/4n3/3P1PP1/8/1K1RQ3", true);
    let best_move_str = res.get_best_move_row_str();
    test_helper::assert::equal_move(res, "e1b4");

    res = test_helper::get_bestmove_for_fen("1k6/8/3n2b1/p4p2/1p2n3/5PP1/8/1K1RQ3", true);
    test_helper::assert::equal_move(res, "f3e4");

    let res = test_helper::get_bestmove_for_fen("8/7r/1k1q1p2/8/7B/8/2K2R2/8", true);
    //test_helper::assert::equal_move(&turn, "f2f6");
    let move_row = res.get_best_move_row_str();
    println!("move row: {}", move_row);

    // for black
    let mut res = test_helper::get_bestmove_for_fen("1k6/5Q2/4p1p1/5Rn1/2b5/7B/1K6/8", false);
    test_helper::assert::equal_move(res, "g5f7");
}

pub fn recognize_chess_016() {
    let mut board = Board::new();
    board.set_fen("8/8/2k5/8/8/8/2K5/3B4");
    let sorted_turn_list = board.get_turn_list(true, false, &mut Stats::new());
    assert(sorted_turn_list.get(0).unwrap().gives_chess);

    board.set_fen("8/8/2k5/8/4K3/8/6B1/8");
    let sorted_turn_list = board.get_turn_list(true, false, &mut Stats::new());
    assert(sorted_turn_list.get(0).unwrap().gives_chess);
    assert(sorted_turn_list.get(1).unwrap().gives_chess);
    assert(sorted_turn_list.get(2).unwrap().gives_chess);


    board.set_fen("8/8/8/8/8/8/1K1k1r2/8");
    let sorted_turn_list = board.get_turn_list(false, false, &mut Stats::new());
    assert(sorted_turn_list.get(0).unwrap().gives_chess);


    //for black:
    board.set_fen("rnbqkbnr/ppp2ppp/4p3/3p4/2PP4/4P3/PP3PPP/RNBQKBNR");
    let sorted_turn_list = board.get_turn_list(false, false, &mut Stats::new());
    assert(sorted_turn_list.get(0).unwrap().gives_chess);
    assert(!sorted_turn_list.get(1).unwrap().gives_chess);
}


pub fn opening_situations_050() {
    // rochade in spanish
    let mut res = test_helper::get_bestmove_for_fen("r1bqkb1r/1pp2ppp/p1np1n2/4p3/B3P3/3P1N2/PPP2PPP/RNBQK2R", true);
    test_helper::assert::equal_move(res, "e1g1");

    // liver I
    let mut res = test_helper::get_bestmove_for_fen("r1bqkb1r/ppp2ppp/2n2n2/3Pp1N1/2B5/8/PPPP1PPP/RNBQK2R", false);
    test_helper::assert::equal_move(res, "f6d5");

    // liver II
    let mut res = test_helper::get_bestmove_for_fen("r1bqkb1r/ppp2ppp/5n2/n2Pp1N1/2B5/8/PPPP1PPP/RNBQK2R", true);
    test_helper::assert::equal_move(res, "d2d3");

}



pub fn analyse() {
    let mut board = Board::new();
    let mut config = Config::new();
    board.set_fen("2r3r1/Pb1kP1b1/6n1/2P3p1/2p3P1/6N1/pB1Kp1B1/2R3R1");
    let best_white = search::get_best_move(&mut board, 2, true, &mut Stats::new(), &mut Config::new().unittest());
    let best_black = search::get_best_move(&mut board, 2, false, &mut Stats::new(), &mut Config::new().unittest());
    //println!("{} {}", best_white.1, best_black.1);
    //assert(best_white.1 == best_black.1 * (-1));
    board.set_field_index(84, 0);

    let mut board = Board::new();
    board.set_fen("r1b1kbnr/ppp1qppp/3p4/3P4/1n6/P1N1B3/1PP2PPP/R2QKBNR");
    //let best_black = search::get_best_move(&mut board, 4, false, &mut Stats::new(), &mut ;
    //println!("{}", best_black.0.unwrap().to_algebraic());

    let mut board = Board::new();
    let turns = Turn::generate_turns("e2e3 e7e6");
    for turn in turns {
        board.do_turn(&turn);
    }

    config.set_search_alg(SearchAlgo::Zobrist);
    let best = search::get_best_move(&mut board, 4, true, &mut Stats::new(), &config);
    config.set_search_alg(SearchAlgo::AlphaBeta);
    let best = search::get_best_move(&mut board, 4, true, &mut Stats::new(), &config);
    // best is g1h3 in both
}




pub mod test_helper {
    use std::collections::{HashMap, VecDeque};
    use crate::board::Board;
    use crate::config::Config;
    use crate::eval::SemiResultKeys;
    use crate::search;
    use crate::search::{MinMaxResult, SearchAlgo};
    use crate::stats::Stats;

    pub fn get_bestmove_for_fen(fen: &str, white: bool) -> MinMaxResult {
        let mut board = Board::new();
        let mut stats = Stats::new();
        let mut config = Config::new();
        config.search_depth = 2;
        config.search_depth_quite = 99;
        config.set_search_alg(SearchAlgo::Quiescence);
        board.set_fen(fen);
        return search::get_best_move_as_min_max_result(&mut board, config.search_depth, white, &mut stats, &config);
    }

    pub fn get_static_eval_for_fen(fen: &str, calc_function: fn(&Board, &Config, &mut HashMap<SemiResultKeys, i32>) -> i16) -> i16 {
        let mut board = Board::new();
        board.set_fen(fen);
        let config = Config::new();
        calc_function(&board, &config, &mut HashMap::new())
    }


    pub(crate) mod assert {
        use std::println;
        use crate::search::MinMaxResult;
        use crate::unittests::assert;

        pub fn equal_move(minMaxResult: MinMaxResult, expected_move: &str) {
            let unwraped_turn = minMaxResult.get_best_turn();
            let move_calc = unwraped_turn.to_algebraic(false);
            if !move_calc.eq(expected_move) {
                println!("actual: {}, expected: {}", move_calc, expected_move);
            }
            assert(move_calc.eq(expected_move));
        }
    }
}