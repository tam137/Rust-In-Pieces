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
use model::UciGame;
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

    let (tx, rx) = mpsc::channel();

    log("Engine startet".to_string());


    let _handle = thread::spawn(move || {

        loop {
            let mut uci_token = String::new();
            match io::stdin().read_line(&mut uci_token) {
                Ok(_) => {
                    if uci_token.trim() == "uci" {
                        log("send ID back".to_string());
                        println!("id name SupraH V00b");
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
                        sleep(Duration::from_millis(10));
                        tx.send(format!("go")).unwrap();
                    }
                    else if uci_token.starts_with("test") {
                        tx.send(format!("test")).unwrap();
                    }
                    else if uci_token.starts_with("quit") {
                        std::process::exit(0);
                    }
                    else {
                        println!("cmd unknown or empty: {}", uci_token);
                        thread::sleep(Duration::from_millis(2));
                    }
                },
                Err(error) => println!("Error reading input: {}", error),
            }
            io::stdout().flush().unwrap();
        }
    });


    let service = &Service::new();
    let mut stats = Stats::new();
    let config = Config::new();
    let mut white;

    let starting_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut game = UciGame::new(service.fen.set_fen(starting_fen));


    loop {        

        let received = rx.recv().unwrap();

        if received == "go" {

            let calc_time = Instant::now();

            white = game.white_to_move();

            // info depth 2 score cp 214 time 1242 nodes 2124 nps 34928 pv e2e4 e7e5 g1f3

            let search_result = &service.search.get_moves(&mut game.board, config.search_depth, white, &mut stats, &config, &service);
            game.do_move(&search_result.get_best_move_algebraic());
            
            let calc_time_ms: u128 = calc_time.elapsed().as_millis().try_into().unwrap();
            let move_row = search_result.get_best_move_row();

            println!("info depth {} score cp {} time {} nodes {} nps {} pv {}", search_result.get_depth(),
                    search_result.get_eval(), calc_time_ms, stats.created_nodes, stats.created_nodes / (calc_time_ms + 1) as usize, move_row);
            println!("bestmove {}", search_result.get_best_move_algebraic());

            stats.reset_stats();
        } else if received.starts_with("move") {
            let algebraic_notation;
            if received.chars().nth(9) == Some('q') {
                println!("log found promotion");
                algebraic_notation = &received[5..10];
            } else {
                algebraic_notation = &received[5..9];
            };

            game.do_move(algebraic_notation);
        } else if received == "ucinewgame" {
            game = UciGame::new(service.fen.set_fen(starting_fen).clone());
            white = game.white_to_move();
            continue;
        } else if received == "test" {
            run_time_check();
        }        
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
    let mut stats = Stats::new();

    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut board = time_it!(service.fen.set_fen(fen)); // ~3µs / ~11µs
    time_it!(service.move_gen.generate_valid_moves_list(&mut board, &mut stats, service)); // ~ 13µs - 18µs / ~43µs
    time_it!(service.eval.calc_eval(&board, &mut config)); // ~ 300ns / ~1µs
    time_it!(service.search.get_moves(&mut service.fen.set_fen(fen), 6, true, &mut Stats::new(), &Config::new(), service)); // ~ 950ms
}
