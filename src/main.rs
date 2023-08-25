mod board;
mod turn;
mod unittests;
mod eval;
mod stats;
mod search;
mod config;
mod zobrist;
use turn::Turn;
use board::Board;
use stats::Stats;
use std::io::Write;
use std::thread;
use std::time::Instant;
use std::io;
use std::sync::mpsc;
use config::Config;
use std::fs::OpenOptions;
use chrono::Local;
use std::time::Duration;


fn main() {
    unittests::run_unittests();
    
    let mut board = Board::new();
    let (tx, rx) = mpsc::channel();

    log("Programm gestartet".to_string());

    let _handle = thread::spawn(move || {
        
        loop {
            let mut uci_token = String::new();
            match io::stdin().read_line(&mut uci_token) {
                Ok(_) => {
                    if uci_token.trim() == "uci" {
                        log("send ID back".to_string());

                        println!("id name RustInPieces V25");

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
                        tx.send(format!("go")).unwrap();
                    }
                    else if uci_token.starts_with("test") {
                        tx.send(format!("test")).unwrap();
                    } 
                    else {
                        //println!("cmd unknown or empty: {}", uci_token);
                    }
                },
                Err(error) => println!("Error reading input: {}", error),
            }
            io::stdout().flush().unwrap();
            thread::sleep(Duration::from_millis(5));
        }
    });
    
    let mut stats = Stats::new();
    let config = Config::new();
    let mut white = true;

    loop {
        let received = rx.recv().unwrap();

        if received == "go" {
            let complexity = board.get_complexity();

            let mut depth_modificator = 0;
            if config.use_depth_modificator {
                depth_modificator = if complexity < 5 { 2 } else { if complexity < 3 { 4 } else { 0 } };
                println!("log warn use depth_modificator");
            }            
            
            let calc_time = Instant::now();

            let best_move = &search::get_best_move(&mut board, config.search_depth + depth_modificator, white, &mut stats, &config).0.unwrap();
            stats.set_calc_time(calc_time.elapsed().as_millis().try_into().unwrap());
            board.do_turn(best_move);
            println!("bestmove {}", best_move.to_algebraic(false));
            stats.reset_stats();

        } else if received.starts_with("move") {
            let algebraic_notation = &received[received.len() - 5..];
            let turn = Turn::generate_turns(algebraic_notation);
            board.do_turn(&turn[0]);
            white = board.is_white_field(turn[0].to);
        } else if received == "ucinewgame" { board = Board::new(); white = true; continue;
        } else if received == "test" { test_game() }

        white = !white;
    }
}






fn test_game() {
    let mut white = true;
    let mut board = Board::new();
    let mut stats = Stats::new();
    let config = Config::new();
    for _i in 0..10 {
        let calc_time = Instant::now();
        let best_move = &search::get_best_move(&mut board, 2, white, &mut stats, &config).0.unwrap();
        stats.set_calc_time(calc_time.elapsed().as_millis().try_into().unwrap());
        board.do_turn(best_move);
        println!("{} {}", best_move.to_algebraic(false), stats.to_string());
        white = !white;
        stats.reset_stats();
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