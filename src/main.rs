mod fen_service;
mod notation_util;
mod model;
mod eval_service;
mod config;
mod search_service;
mod service;
mod move_gen_service;

use std::io::Write;
use std::thread;
use std::io;
use std::thread::sleep;
use std::time::Duration;
use std::fs::OpenOptions;
use chrono::Local;
use model::Uci_game;
use std::sync::mpsc;
use std::time::Instant;

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

    //run_time_check();

    let service = &Service::new();
    let mut config = &Config::new();

    let (tx, rx) = mpsc::channel();

    log("Programm gestartet".to_string());


    let _handle = thread::spawn(move || {

        loop {
            let mut uci_token = String::new();
            match io::stdin().read_line(&mut uci_token) {
                Ok(_) => {
                    if uci_token.trim() == "uci" {
                        log("send ID back".to_string());

                        println!("id name RustInPieces V88_");

                        println!("id author Jan Lange");
                        println!("uciok");
                    }
                    else if uci_token.trim() == "uciready" {
                        println!("readyok");
                    }
                    else if uci_token.trim() == "ucinewgame" {
                        tx.send(format!("ucinewgame")).unwrap();
                    }
                    else if uci_token.trim() == "isready" {
                        println!("readyok");
                    }
                    else if uci_token.starts_with("position startpos moves") {
                        let last_four_chars = &uci_token[uci_token.len() - 5..];
                        tx.send(format!("move {}", last_four_chars)).unwrap();
                    }
                    else if uci_token.starts_with("go") {
                        sleep(Duration::from_millis(100));
                        tx.send(format!("go")).unwrap();
                    }
                    else if uci_token.starts_with("test") {
                        tx.send(format!("test")).unwrap();
                    }
                    else if uci_token.starts_with("quit") {
                        std::process::exit(0);
                    }
                    else {
                        //println!("cmd unknown or empty: {}", uci_token);
                        thread::sleep(Duration::from_millis(2));
                    }
                },
                Err(error) => println!("Error reading input: {}", error),
            }
            io::stdout().flush().unwrap();
        }
    });


    let mut stats = Stats::new();
    let config = Config::new();
    let mut white = true;

    let mut board = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let game = Uci_game::new(board);


    loop {
        let received = rx.recv().unwrap();

        if received == "go" {

            let calc_time = Instant::now();

            let algebraic_turns = game.made_moves_str;


            let best_move = &service.search.get_moves(&mut board, config.search_depth, white, &mut stats, &config, &service);
            game.do_move(&best_move.get_best_move_algebraic());
            // info depth 2 score cp 214 time 1242 nodes 2124 nps 34928 pv e2e4 e7e5 g1f3
            let calc_time: u128 = calc_time.elapsed().as_millis().try_into().unwrap();
            let move_row = best_move.get_best_move_row_str();
            println!("info depth {} score cp {} time {} nodes {} nps {} pv {}", best_move.get_depth(), best_move.get_eval(), 0, stats.created_nodes, stats.created_nodes / (calc_time + 1) as usize, move_row);
            println!("bestmove {}", best_move.get_best_turn().to_algebraic(false));
        
            stats.set_calc_time(calc_time.elapsed().as_millis().try_into().unwrap());
            stats.reset_stats();
            board.clean_up_hash_if_needed();
        } else if received.starts_with("move") {
            let algebraic_notation;
            if received.chars().nth(9) == Some('q') {
                println!("log found promotion");
                algebraic_notation = &received[5..10];
            } else {
                algebraic_notation = &received[5..9];
            };

            let turn = Turn::generate_turns(algebraic_notation);
            board.do_turn(&turn[0]);
            white = board.is_white_field(turn[0].to);
        } else if received == "ucinewgame" { board = Board::new(); white = true; continue;
        } else if received == "test" { test_game() }

        white = !white;
    }
}






fn log(msg: String) {
    let timestamp = Local::now().format("%H:%M:%S");
    let log_entry = format!("{} {}", timestamp, msg + "\n");
    match OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("/home/tam137/engines/log.txt") {
            Ok(mut file) => {
                match file.write_all(log_entry.as_bytes()) {
                    Ok(_) => (),
                    Err(e) => println!("Error writing to file: {}", e),
                }
            },
            Err(e) => println!("Error opening file: {}", e),
        }
}



fn run_time_check() {

    let service = &Service::new();
    let mut config = &Config::new();

    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut board = time_it!(service.fen.set_fen(fen)); // ~3µs
    time_it!(service.move_gen.generate_valid_moves_list(&mut board, &mut Stats::new(), service)); // ~ 13µs - 18µs
    time_it!(service.eval.calc_eval(&board, &mut config)); // ~ 300ns
    let result = time_it!(service.search.get_moves(&mut service.fen.set_fen(fen), 6, true, &mut Stats::new(), &Config::new(), service)); // ~ 950ms


}
