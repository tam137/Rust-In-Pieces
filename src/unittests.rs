use crate::Board;
use crate::board::GameState;
use crate::config::Config;
use crate::eval::{calc_eval, calc_eval_legacy, calc_eval_piece_map};
use crate::search;
use crate::Stats;
use crate::Turn;

macro_rules! eval {
    ($eval_map: expr) => {
        *$eval_map.get(&0).unwrap()
    };
}

macro_rules! eval_idx {
    ($eval_map:expr, $piece:expr) => {
        *$eval_map.get(&$piece).unwrap()
    };
}

macro_rules! neg {
    ($expr: expr) => {
        &(-($expr))
    };
}

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


pub fn run_unittests() {
    time_000();
    move_gen_001();
    turn_gen_002();
    piece_map_002a();
    eval_003();
    eval_003a_knight();
    eval_003b_rook();
    eval_003c_bishop();
    eval_003c_queen();
    eval_003d_king();
    eval_003e_game_phase();
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
    // move_row_016();
    // analyse();
    print!("finished unittests")
}

#[track_caller]
fn assert(condition: bool) {
    if !condition {
        let location = std::panic::Location::caller();
        panic!("Unittest failed at {}", location);
    }
}

fn time_000() {
    let mut board = Board::new();
    board.set_fen("rnbqkb1r/ppp1n2p/3p2p1/5p2/2BpP3/N4Q2/PPP1NPPP/R1B2RK1");
    time_it!(board.king_in_chess(true));

    time_it!(calc_eval_piece_map(&board, &Config::new()));
    time_it!(calc_eval(&board, &Turn::new(), &Config::new()));
    time_it!(calc_eval_legacy(&board, &Turn::new(), &Config::new()));
    time_it!(board.get_pieces_map());

    time_it!(board.generate_moves_list(true));
    time_it!(board.generate_moves_list_for_piece(true, 63));

    time_it!(board.get_game_phase());
}


fn move_gen_001() {
    let mut board = Board::new();
    assert(time_it!(board.get_turn_list(true, false, &mut Stats::new()).len() == 20));
    assert(board.get_turn_list(false, false, &mut Stats::new()).len() == 20);
    time_it!(board.get_turn_list_for_piece_on_idx(true, false, 92));
    time_it!(board.generate_moves_list(true));
    board.clear_field();
    assert(board.generate_moves_list(false).len() == 0);
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

    board = Board::new();
    board.do_turn(&Turn::generate_turns("a2a3")[0]);
    assert(board.get_turn_list(true, false, &mut Stats::new()).len() == 19);

    board = Board::new();
    assert(board.get_turn_list_for_piece_on_idx(true, false, 94).len() == 0);

    board.set_fen("r1bqk2r/pppp1ppp/2n2n2/2b5/2BPP3/5N2/PP3PPP/RNBQK2R");
    let turns_for_bishop = board.get_turn_list_for_piece_on_idx(false, false, 53);
    assert(turns_for_bishop.len() == 7);
    assert(turns_for_bishop.get(4).unwrap().gives_chess);

    board.set_fen("r2qr2k/1ppnP1pp/p1n2p2/8/2B5/5N2/PPQ2PPP/3RR1K1");
    let pawn_promotion_and_hit_move = board.get_turn_list_for_piece_on_idx(true, false, 35);
    assert(pawn_promotion_and_hit_move.get(0).unwrap().promotion);
    assert(pawn_promotion_and_hit_move.get(0).unwrap().capture == 24);
}

fn piece_map_002a() {
    let board = Board::new();
    let piece_map = board.get_pieces_map();
    assert(piece_map.len() == 12);
    assert(piece_map.get(&24).unwrap().get(0).unwrap() == &24usize);
    assert(piece_map.get(&11).unwrap().get(1).unwrap() == &98usize);
}


fn eval_003() {
    // let mut eval = test_helper::get_static_eval_for_fen("8/8/8/3k4/3BN3/8/8/3K4", calc_push_to_king);
    // assert(eval > 200);
    //
    // let eval = test_helper::get_static_eval_for_fen("8/8/3R4/3k4/8/3K4/3b4/8", calc_push_to_king);
    // assert(eval == 0);
    //
    // let mut eval = test_helper::get_static_eval_for_fen("8/2R5/2k5/8/8/5K2/5r2/8", calc_push_to_king);
    // assert(eval == 0);
    //
    // eval = test_helper::get_static_eval_for_fen("8/2R5/2k1Q3/8/8/3q1K2/5r2/8", calc_push_to_king);
    // assert(eval == 0);
    //
    // eval = test_helper::get_static_eval_for_fen("8/2R1B3/2k1Q3/3N4/4n3/3q1K2/3b1r2/8", calc_push_to_king);
    // assert(eval == 0);

    let mut board = Board::new();
    let mut eval_map = calc_eval_piece_map(&board, &Config::new());
    let eval = eval!(eval_map);
    assert(eval == 0);

    board.set_field_index(21, 0);
    board.set_field_index(31, 0);
    board.set_field_index(92, 0);
    board.set_field_index(93, 0);
    eval_map = calc_eval_piece_map(&board, &Config::new());
    assert(eval!(eval_map) < 100 && eval!(eval_map) > -100);
}

fn eval_003a_knight() {
    let mut board = Board::new();
    board.clear_field();
    board.set_field_index(55, 10); // e5 pawn
    board.set_field_index(46, 12); // f6 knight
    let eval_map = calc_eval_piece_map(&board, &Config::new());
    let white_eval = eval_map.get(&55).unwrap();
    assert(white_eval >= &130);

    board.clear_field();
    board.set_field_index(65, 20); // e4 pawn
    board.set_field_index(76, 22); // f3 knight
    let eval_map = calc_eval_piece_map(&board, &Config::new());
    let black_eval = eval_map.get(&65).unwrap();
    assert(neg!(black_eval) == white_eval);

    // TODO attacks queen or rook
}

fn eval_003b_rook() {
    let mut board = Board::new();
    let config = Config::new();
    let eval_map = calc_eval_piece_map(&board, &Config::new());
    assert(eval_map.get(&21).unwrap() == eval_map.get(&28).unwrap());
    assert(eval_map.get(&91).unwrap() == eval_map.get(&98).unwrap());
    assert(eval_map.get(&91).unwrap() == neg!(eval_map.get(&28).unwrap()));
    assert(eval_map.get(&91).unwrap() >= &config.piece_eval_rook);
}

fn eval_003c_bishop() {
    let mut board = Board::new();
    board.set_fen("rnbqkbnr/pppp1ppp/4p3/8/8/4P3/PPPP1PPP/RNBQKBNR");
    let config = &Config::new();
    let eval_map = calc_eval_piece_map(&board, config);
    assert(eval!(eval_map) == 0);
    let bishop_eval_white = eval_idx!(eval_map, 96);
    let bishop_eval_black = eval_idx!(eval_map, 26);
    assert(bishop_eval_black < -config.piece_eval_bishop);
    assert(bishop_eval_white > config.piece_eval_bishop);
}

fn eval_003c_queen() {
    let mut board = Board::new();
    board.set_fen("rnbqkbnr/ppp2ppp/4p3/3p4/3P4/4P3/PPP2PPP/RNBQKBNR");
    let config = &Config::new();
    let eval_map = calc_eval_piece_map(&board, config);
    assert(eval!(eval_map) == 0);
    let queen_eval_white = eval_idx!(eval_map, 94);
    let queen_eval_black = eval_idx!(eval_map, 24);
    assert(queen_eval_white > config.piece_eval_queen);
    assert(queen_eval_black < -config.piece_eval_queen);
}

fn eval_003d_king() {
    let mut board = Board::new();
    board.set_fen("6k1/5ppp/8/8/8/8/5PPP/6K1");
    let config = &Config::new();
    let eval_map = calc_eval_piece_map(&board, config);
    assert(eval!(eval_map) == 0);

    board.set_field_index(37, 0);
    let eval_map = calc_eval_piece_map(&board, config);
    let king_eval_white = eval_idx!(eval_map, 97);
    let king_eval_black = eval_idx!(eval_map, 27);
    assert(king_eval_white > *neg!(king_eval_black));

    board.set_field_index(87, 0);
    board.set_field_index(88, 0);
    let eval_map = calc_eval_piece_map(&board, config);
    let king_eval_white = eval_idx!(eval_map, 97);
    let king_eval_black = eval_idx!(eval_map, 27);
    assert(king_eval_white < *neg!(king_eval_black));
}

fn eval_003e_game_phase() {
    let mut board = Board::new();
    assert_eq!(board.get_game_phase(), 256);
    board.clear_field();
    assert_eq!(board.get_game_phase(), 0);
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
    // 8/4P3/8/8/1K4p1/6k1/7n/8
    //test_helper::assert::equal_move(res, "e7e8q");

    let mut board = Board::new();
    board.clear_field();
    board.set_field_index(33, 10);
    board.set_field_index(83, 20);
    board.set_field_index(98, 15);
    board.set_field_index(96, 25);
    let mut cmp_board = board.clone();
    let mut turn_white = &mut cmp_board.get_turn_list(true, false, &mut Stats::new())[0];
    assert(turn_white.is_promotion() == true);
    let mut turn_black = &mut board.get_turn_list(false, false, &mut Stats::new())[0];
    assert(turn_black.is_promotion() == true);
    board.do_turn(turn_white);
    board.do_turn(turn_black);
    board.do_undo_turn(turn_white);
    board.do_undo_turn(turn_black);
    assert(cmp_board == board);

    assert(board.get_fen() == "8/2P5/8/8/8/8/2p5/5k1K");
    let best_white_move = search::get_best_move(&mut board, 2, true, &mut Stats::new(), &mut Config::new().unittest()).0.unwrap();
    board.do_turn(&best_white_move);

    let min_max_res = test_helper::get_bestmove_for_fen("8/2P5/8/8/8/8/2p5/1R3k1K", false);
    board.set_fen("8/2P5/8/8/8/8/2p5/1R3k1K");
    let mut best_black_turn = min_max_res.get_best_turn().clone();
    best_black_turn.enrich_move_promotion(&board, false);
    assert(best_black_turn.to_algebraic(false) == "c2b1q");

    board.do_turn(&best_black_turn);
    board.do_undo_turn(&best_black_turn);
    let board_fen = board.get_fen();
    assert(board.get_fen() == "8/2P5/8/8/8/8/2p5/1R3k1K");

    board.set_fen("8/5P2/8/8/1k6/8/1K5p/8");
    let all_turns = board.get_turn_list(true, false, &mut Stats::new());
    let mut turn = &mut Turn::generate_turns("f7f8q")[0];
    turn.enrich_move_promotion(&board, true);
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
     test_helper::assert::equal_move(&res, "f3e4");

    res = test_helper::get_bestmove_for_fen("1k6/8/3n2b1/5p2/4n3/3P1PP1/8/1K1RQ3", true);
    let best_move_str = res.get_best_move_row_str();
    println!("move row: {}", best_move_str);
    //test_helper::assert::equal_move(&res, "e1b4"); // or e1b4, d3e4

    res = test_helper::get_bestmove_for_fen("1k6/8/3n2b1/p4p2/1p2n3/5PP1/8/1K1RQ3", true);
    let best_move_str = res.get_best_move_row_str();
    test_helper::assert::equal_move(&res, "f3e4");

    let res = test_helper::get_bestmove_for_fen("8/7r/1k1q1p2/8/7B/8/2K2R2/8", true);
    //test_helper::assert::equal_move(&res, "f2f6");
    let move_row = res.get_best_move_row_str();
    println!("move row: {}", move_row);

    // for black
    let mut res = test_helper::get_bestmove_for_fen("1k6/5Q2/4p1p1/5Rn1/2b5/7B/1K6/8", false);
    test_helper::assert::equal_move(&res, "g5f7"); // g5f7, g6f5
}

pub fn recognize_chess_016() {
    let mut board = Board::new();
    board.set_fen("8/8/2k5/3B4/8/8/8/8");
    assert(board.king_in_chess(false));

    let mut board = Board::new();
    board.set_fen("r1bqk1nr/pppp1Bp1/2nb3p/4p3/4P3/2NP1N2/PPP2PPP/R1BQK2R");
    assert(board.king_in_chess(false));

    let mut board = Board::new();
    board.set_fen("8/8/2k5/8/8/8/2K5/3B4");
    let sorted_turn_list = board.get_turn_list(true, false, &mut Stats::new());
    assert(sorted_turn_list.get(0).unwrap().gives_chess);

    board.set_fen("8/8/8/8/8/8/1K1k1r2/8");
    let sorted_turn_list = board.get_turn_list(false, false, &mut Stats::new());
    assert(sorted_turn_list.get(0).unwrap().gives_chess);

    board.set_fen("rnbqkbnr/ppp2ppp/4p3/3p4/Q1PP4/4P3/PP3PPP/RNB1KBNR");
    assert(board.king_in_chess(false));
    board.set_fen("rnbqkbnr/pp3ppp/2p1p3/3p4/Q1PP4/4P3/PP3PPP/RNB1KBNR");
    assert(!board.king_in_chess(false));

    board.set_fen("rnbqkbnr/ppp1pppp/4P3/7Q/4N3/8/PPPP1PPP/RNB1KB1R");

    let turns = board.get_turn_list(false, false, &mut Stats::new());
    assert(turns.iter().filter(|t| t.gives_chess).collect_into(&mut Vec::new()).len() == 1);
}


pub fn opening_situations_050() {
    // rochade in spanish
    let mut res = test_helper::get_bestmove_for_fen("r1bqkb1r/1pp2ppp/p1np1n2/4p3/B3P3/3P1N2/PPP2PPP/RNBQK2R", true);
    test_helper::assert::equal_move(&res, "e1g1");

    // liver I
    let mut res = test_helper::get_bestmove_for_fen("r1bqkb1r/ppp2ppp/2n2n2/3Pp1N1/2B5/8/PPPP1PPP/RNBQK2R", false);
    test_helper::assert::equal_move(&res, "f6d5");

    // liver II
    let mut res = test_helper::get_bestmove_for_fen("r1bqkb1r/ppp2ppp/5n2/n2Pp1N1/2B5/8/PPPP1PPP/RNBQK2R", true);
    test_helper::assert::equal_move(&res, "d2d3");

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

}




pub mod test_helper {
    use std::collections::HashMap;

    use crate::board::Board;
    use crate::config::Config;
    use crate::eval::SemiResultKeys;
    use crate::search;
    use crate::search::MinMaxResult;
    use crate::stats::Stats;

    pub fn get_bestmove_for_fen(fen: &str, white: bool) -> MinMaxResult {
        let mut board = Board::new();
        let mut stats = Stats::new();
        let mut config = Config::new();
        config.search_depth = 2;
        config.search_depth_quite = 99;
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

        pub fn equal_move(minMaxResult: &MinMaxResult, expected_move: &str) {
            let mut turn = minMaxResult.get_best_turn().clone();
            let move_calc = turn.to_algebraic(false);
            if !move_calc.eq(expected_move) {
                println!("actual: {}, expected: {}", move_calc, expected_move);
            }
            assert(move_calc.eq(expected_move));
        }
    }
}