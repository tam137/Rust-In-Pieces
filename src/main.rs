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

use model::DataMapKey;
use model::QuiescenceSearchMode;
use model::UciGame;
use model::ThreadSafeDataMap;

use model::RIP_STD_IN_THREAD_PANICKED;


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



#[cfg(test)]
mod tests {

    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use crate::global_map_handler;
    use crate::config::Config;
    use crate::game_handler::game_loop;
    use crate::model::ThreadSafeDataMap;
    use crate::threads::uci_command_processor;
    use crate::threads::hash_writer;
    use crate::threads::logger_buffer_thread;

    
    struct TestEnvironment {
        tx_std_in: mpsc::Sender<String>,
        _uci_command_processor: thread::JoinHandle<()>,
        global_map: ThreadSafeDataMap,
    }
    
    fn set_up() -> TestEnvironment {
    
        let global_map = global_map_handler::create_new_global_map();
    
        let (tx_hashes, rx_hashes) = mpsc::channel();
        let (tx_std_in, rx_std_in) = mpsc::channel();
        let (tx_game_command, rx_game_command) = mpsc::channel();
        let (tx_log_buffer, rx_log_buffer) = mpsc::channel();
    
        global_map_handler::add_hash_sender(&global_map, tx_hashes);
        global_map_handler::add_std_in_sender(&global_map, tx_std_in.clone());
        global_map_handler::add_game_command_sender(&global_map, tx_game_command.clone());
        global_map_handler::add_log_buffer_sender(&global_map, tx_log_buffer);
    
        // Set up hash writer thread
        let global_map_hash_writer = global_map.clone();
        let _hash_writer = thread::spawn(move || {
            hash_writer(global_map_hash_writer, &Config::new()._for_integration_tests(), rx_hashes);
        });
    
        // Set up UCI command thread
        let global_map_command_processor = global_map.clone();
        let _uci_command_processor = thread::spawn(move || {
            uci_command_processor(global_map_command_processor, &Config::new()._for_integration_tests(), rx_std_in);
        });
    
        // Set up game loop thread
        let global_map_game_loop = global_map.clone();
        let _game_handler = thread::spawn(move || {
            game_loop(global_map_game_loop, &Config::new()._for_integration_tests(), rx_game_command);
        });
    
        // Set up logger thread
        let global_map_log_buffer = global_map.clone();
        let _logger_buffer = thread::spawn(move || {
            logger_buffer_thread(global_map_log_buffer, &Config::new()._for_integration_tests(), rx_log_buffer);
        });
    
        TestEnvironment {
            tx_std_in,
            _uci_command_processor,
            global_map
        }
    }

    fn send_uci(env: &TestEnvironment, cmd: &str, sleep_millis: i32) {
        let rip_err = "RIP Test execution error";
        env.tx_std_in.send(cmd.to_string()).expect(rip_err);
        thread::sleep(Duration::from_millis(sleep_millis as u64));
    }
    
    #[cfg(test)]
    mod tests {
        use super::*;
    
        #[test]
        fn setup_test_env_test() {
            let rip_err = "RIP Test execution error";
            let env = set_up();
    
            send_uci(&env, "debug on", 10);
            send_uci(&env, "go infinite", 500);
            send_uci(&env, "quit", 20);

            let sr = global_map_handler::_get_search_results(&env.global_map);
            assert!(sr.len() > 2);
            global_map_handler::_clear_search_result(&env.global_map);
            let sr = global_map_handler::_get_search_results(&env.global_map);
            assert_eq!(0, sr.len());
    
            env._uci_command_processor.join().expect(rip_err);
        }
    }


    
    #[test]
    fn go_multithreading_test() {
        let rip_err = "RIP Test execution error";
        let env = set_up();

        send_uci(&env, "debug on", 10);
        send_uci(&env, "position startpos", 10);
        send_uci(&env, "go wtime 60000 btime 60000", 400);
        send_uci(&env, "stop", 200);
        send_uci(&env, "quit", 20);
        env._uci_command_processor.join().expect(rip_err);
    }
}