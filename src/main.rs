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
mod global_map_handler;
mod threads;
mod game_handler;
mod time_check;

use std::thread;
use std::sync::mpsc;
use std::time::Instant;

use model::DataMapKey;
use model::QuiescenceSearchMode;
use model::UciGame;
use model::ThreadSafeDataMap;
use model::RIP_COULDN_LOCK_ZOBRIST;
use model::RIP_MISSED_DM_KEY;
use model::RIP_STD_IN_THREAD_PANICKED;

use crate::book::Book;
use crate::config::Config;
use crate::model::Stats;
use crate::model::DataMap;
use crate::service::Service;
use crate::game_handler::game_loop;
use crate::threads::std_reader;
use crate::threads::uci_command_processor;
use crate::threads::hash_writer;
use crate::threads::logger_buffer_thread;



fn main() {
    
    let global_map  = global_map_handler::create_new_global_map();

    let config = Config::new();

    let mut local_map = DataMap::new();
    local_map.insert(DataMapKey::CalcTime, Instant::now());

    if config.quiescence_search_mode == QuiescenceSearchMode::Alpha3 {
        local_map.insert(DataMapKey::WhiteThreshold, 0);
        local_map.insert(DataMapKey::BlackThreshold, 0);
        
    }

    let (tx_hashes, rx_hashes) = mpsc::channel();
    let (tx_std_in, rx_std_in) = mpsc::channel();
    let (tx_game_command, rx_game_command) = mpsc::channel();
    let (tx_log_buffer, rx_log_buffer) = mpsc::channel(); 

    
    global_map_handler::add_hash_sender(&global_map, tx_hashes);
    global_map_handler::add_std_in_sender(&global_map, tx_std_in);
    global_map_handler::add_game_command_sender(&global_map, tx_game_command);
    global_map_handler::add_log_buffer_sender(&global_map, tx_log_buffer);

    // Set up hash writer thread
    let global_map_hash_writer = global_map.clone();
    let config = config.clone();
    let _hash_writer = thread::spawn(move || {
        hash_writer(global_map_hash_writer, &config, rx_hashes);
    });

    // Set up std reader thread
    // Read std in and send to uci command processor
    let global_map_std_in = global_map.clone();
    let std_in_thread = thread::spawn(move || {
        std_reader(global_map_std_in, &Config::new());
    });

    // Set up uci command thread
    // Receive uci commands and send internal command to game handler or other threads
    let global_map_command_processor = global_map.clone();
    let _uci_command_processor = thread::spawn(move || {
        uci_command_processor(global_map_command_processor, &Config::new(), rx_std_in);
    });

    // Set up game loop thread
    // receives command by uci_command_processor
    let global_map_game_loop = global_map.clone();
    let _game_handler = thread::spawn(move || {
        game_loop(global_map_game_loop, &Config::new(), rx_game_command);
    });

    // Set up logger threads
    let global_map_log_buffer = global_map.clone();
    let _logger_buffer = thread::spawn(move || {
        logger_buffer_thread(global_map_log_buffer, &Config::new(), rx_log_buffer);
    });

    std_in_thread.join().expect(RIP_STD_IN_THREAD_PANICKED);

}

