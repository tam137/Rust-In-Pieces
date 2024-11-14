mod fen_service;
mod notation_util;
mod model;
mod eval_service;
mod config;
mod search_service;
mod service;
mod move_gen_service;
mod book;
mod uci_parser_service;
mod zobrist;
mod stdout_wrapper;

use std::io::Write;
use std::thread;
use std::io;
use std::thread::sleep;
use std::time::Duration;
use std::fs::OpenOptions;
use chrono::Local;
use model::UciGame;
use uci_parser_service::UciParserService;
use std::sync::mpsc;
use std::time::Instant;

use crate::config::Config;
use crate::model::Stats;
use crate::service::Service;
use crate::book::Book;



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

fn main() {

    let (tx, rx) = mpsc::channel();

    let version = "V00h";

    log(format!("Engine startet: {}", version));

    let benchmark_value = calculate_benchmark(10000);
    log(format!("Benchmark Value: {}", benchmark_value));

    let _handle = thread::spawn(move || {

        let stdout = Service::new().stdout;

        loop {
            let mut uci_token = String::new();
            match io::stdin().read_line(&mut uci_token) {
                Ok(_) => {
                    if uci_token.trim() == "uci" {
                        stdout.write(&format!("id name SupraH {}", version));
                        stdout.write("id author Jan Lange");
                        stdout.write("uciok");
                    }
                    else if uci_token.trim() == "uciready" {
                        stdout.write("readyok");
                    }
                    else if uci_token.trim() == "ucinewgame" {
                        tx.send(format!("ucinewgame")).expect("RIP Could not send 'ucinewgame' as internal cmd");
                    }
                    else if uci_token.trim() == "isready" {
                        stdout.write("readyok");
                    }
                    else if uci_token.trim().starts_with("position startpos moves") {
                        let len = uci_token.len();
                        let move_str = if matches!(uci_token.chars().rev().nth(1), Some('q' | 'k')) {
                            &uci_token[len - 6..len -1]
                        } else {
                            &uci_token[len - 5..len -1]
                        };
                        tx.send(format!("move {}", move_str)).expect("RIP Could not send 'move' as internal cmd");
                    }
                    else if uci_token.starts_with("go") {
                        sleep(Duration::from_millis(5));
                        tx.send(uci_token).expect("RIP Could not send 'go' as internal cmd");
                    }
                    else if uci_token.starts_with("test") {
                        tx.send(format!("test")).expect("RIP Could not send 'test' as internal cmd");
                    }
                    else if uci_token.starts_with("quit") {
                        tx.send("quit".to_string()).expect("RIP Could not send 'quit' as internal cmd");
                        break;
                    }
                    else {
                        if !uci_token.is_empty() {
                            log("cmd unknown".to_string() + &uci_token);
                        }                        
                        thread::sleep(Duration::from_millis(5));
                    }
                },
                Err(_) => {
                    panic!("RIP Error reading std input");
                }
            }
            if let Err(_e) = io::stdout().flush() {
                log("RIP failed to flush stdout".to_string());
            };
        }
    });

    let book = Book::new();
    let service = &Service::new();
    let uci_parser = &UciParserService::new();
    let mut stats = Stats::new();
    let config = Config::new();
    let mut white;
    let stdout = &service.stdout;

    let mut game = UciGame::new(service.fen.set_init_board());


    loop {

        let received = match rx.recv() {
            Ok(msg) => msg,
            Err(_) => {
                log("Message Channel closed, exiting".to_string());
                break;
            }
        };

        if received.starts_with("go") {

            let calc_time = Instant::now();

            white = game.white_to_move();

            // info depth 2 score cp 214 time 1242 nodes 2124 nps 34928 pv e2e4 e7e5 g1f3
            // go wtime 31520 btime 1410

            let game_fen = service.fen.get_fen(&game.board);
            let book_move = book.get_random_book_move(&game_fen);

            let times: (i32, i32) = uci_parser.parse_go(received.as_str());
            let wtime = times.0;
            let btime = times.1;

            if book_move.is_empty() {

                let my_time_ms = if white { wtime } else { btime };
                let calculated_depth = calculate_depth(&config, game.board.calculate_complexity(), benchmark_value, my_time_ms);

                let search_result = &service.search.get_moves(&mut game.board, calculated_depth, white, &mut stats, &config, &service);
                game.do_move(&search_result.get_best_move_algebraic());
                
                let calc_time_ms: u128 = calc_time.elapsed().as_millis().try_into().expect("RIP Could not collect elapsed time");
                stats.calc_time_ms = calc_time_ms as usize;
                stats.calculate();
                let cleaned = game.board.zobrist.clean_up_hash_if_needed(&config);
                if cleaned > 0 { log(format!("cleaned {} entries from cache", cleaned)); }

                let move_row = search_result.get_best_move_row();

                let cp = if white { search_result.get_eval() } else { search_result.get_eval() *(-1) };
    
                if let Err(_e) = stdout.write_get_result(&format!("info depth {} score cp {} time {} nodes {} nps {} pv {}",
                search_result.get_depth(),
                cp,
                calc_time_ms,
                stats.created_nodes,
                stats.created_nodes / (calc_time_ms + 1) as usize,
                move_row)
                ) {
                    log("Std Channel closed exiting".to_string());
                    break;
                }
                
                stdout.write(&format!("bestmove {}", search_result.get_best_move_algebraic()));

                if config.in_debug {
                    log(format!("{:?}", stats));
                }                
            } else {
                if config.in_debug {    
                    log(format!("found Book move: {} for position {}", book_move, game_fen));
                }
                game.do_move(book_move);
                stdout.write(&format!("bestmove {}", book_move));
            }

            stats.reset_stats();

        } else if received.starts_with("move") {
            let algebraic_notation = &received[5..];
            log(format!("uci: received move '{}' ", algebraic_notation));
            game.do_move(algebraic_notation);
        } else if received == "ucinewgame" {
            log(format!("uci: received 'ucinewgame'"));
            game = UciGame::new(service.fen.set_init_board());
            continue;
        } else if received == "test" {
            run_time_check();
        } else if received == "quit" {
            log(format!("uci: received 'quit'"));
            break;
        }       
    }
}

fn calculate_depth(config: &Config, complexity: i32, benchmark: i32, time: i32) -> i32 {
    let time_in_sec = (time / 1000) + 1;
    let value = time_in_sec * benchmark / (complexity + 1);

    if value > 70 {
        if config.in_debug {
            log(format!("time threshold: {} -> depth: {}", value, 8));
        }        
        return 8;
    } else if value > 50 {
        if config.in_debug {
            log(format!("time threshold: {} -> depth: {}", value, 6));
        }        
        return 6;
    } else if value >= 6 {
        if config.in_debug {
            log(format!("time threshold: {} -> depth: {}", value, 4));
        }
        return 4;
    } else {
        if config.in_debug {
            log(format!("time threshold: {} -> depth: {}", value, 2));
        }
        return 2;
    }
}


fn log(msg: String) {
    let timestamp = Local::now().format("%H:%M:%S%.3f");
    let log_entry = format!("{} {}", timestamp, msg + "\n");
    match OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("rust-in-piece2.log") {
            Ok(mut file) => {
                match file.write_all(log_entry.as_bytes()) {
                    Ok(_) => (),
                    Err(e) => panic!("RIP Error writing to file {}", e),
                }
            },
            Err(e) => panic!("Error opening file: {}", e),
        }
}

fn calculate_benchmark (normalized_value: i32) -> i32 {
    let mut board = Service::new().fen.set_fen("r1bqkbnr/1ppp1ppp/p1n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4");
    let service = Service::new();
    normalized_value / get_time_it!(service.search.get_moves(&mut board, 4, true, &mut Stats::new(), &Config::new(), &service))
}

fn run_time_check() {
    let service = &Service::new();
    let mut config = &Config::new();
    let mut stats = Stats::new();

    let mut board = time_it!(service.fen.set_init_board()); // ~3µs / ~11µs
    
    time_it!(service.move_gen.generate_valid_moves_list(&mut board, &mut stats, service)); // ~ 13µs - 18µs / ~43µs
    time_it!(service.eval.calc_eval(&board, &mut config, &service.move_gen)); // ~ 300ns / ~1µs    
    
    time_it!(service.search.get_moves(&mut service.fen.set_init_board(), 6, true, &mut Stats::new(), &Config::new(), service)); 
    // ~ 950ms -> 1900ms

    let mid_game_fen = "r1bqr1k1/ppp2ppp/2np1n2/2b1p3/2BPP3/2P1BN2/PPQ2PPP/RN3RK1 b - - 5 8";
    time_it!(service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, false, &mut Stats::new(), &Config::new(), service));
    // ~ 210ms -> 310ms

    let mid_game_fen = "r1bqr1k1/2p2ppp/p1np1n2/1pb1p1N1/2BPP3/2P1B3/PPQ2PPP/RN3RK1 w - - 0 10";
    time_it!(service.search.get_moves(&mut service.fen.set_fen(mid_game_fen), 4, true, &mut Stats::new(), &Config::new(), service));
    // ~ 360ms -> 140ms

    println!("Benchmark Value: {}", calculate_benchmark(10000));
}
