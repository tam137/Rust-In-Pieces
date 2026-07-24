#![allow(clippy::too_many_arguments)]

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
pub mod pawn_hash;
mod stdout_wrapper;
mod threads;
mod game_handler;
mod time_check;
mod magic;
mod pst;
pub mod polyglot;



use std::sync::Arc;
use std::thread;
use std::sync::mpsc;

use crate::config::Config;
use crate::game_handler::game_loop;
use crate::threads::std_reader;
use crate::threads::uci_command_processor;
use crate::threads::logger_buffer_thread;

use model::RIP_COULDN_JOIN_THREAD;


fn main() {
    
    



    let (tx_std_in, rx_std_in) = mpsc::channel();
    let (tx_game_command, rx_game_command) = mpsc::channel();
    let (tx_log_buffer, rx_log_buffer) = mpsc::channel(); 

    let initial_logger: std::sync::Arc<dyn Fn(String) + Send + Sync> = std::sync::Arc::new(|_| {});
    let engine_state = Arc::new(crate::model::EngineState {
        stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        zobrist_table: std::sync::RwLock::new(Arc::new(crate::zobrist::ZobristTable::with_capacity(Config::new().max_zobrist_hash_entries))),

        pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
        logger: Arc::new(std::sync::RwLock::new(initial_logger)),
        log_sender: tx_log_buffer.clone(),
    });

    // Set up std reader thread
    // Read std in and send to uci command processor
    let std_in_sender = tx_std_in.clone();
    let std_in_thread = thread::spawn(move || {
        std_reader(std_in_sender, &Config::new());
    });

    // Set up uci command thread
    // Receive uci commands and send internal command to game handler or other threads
    let engine_state_command_processor = engine_state.clone();
    let tx_game_command_clone = tx_game_command.clone();
    let _uci_command_processor = thread::spawn(move || {
        uci_command_processor(engine_state_command_processor, &Config::new(), rx_std_in, tx_game_command_clone);
    });

    // Set up game loop thread
    // receives command by uci_command_processor
    let engine_state_game_loop = engine_state.clone();
    let _game_handler = thread::spawn(move || {
        game_loop(engine_state_game_loop, &Config::new(), rx_game_command);
    });

    // Set up logger threads
    let engine_state_log_buffer = engine_state.clone();
    let _logger_buffer = thread::spawn(move || {
        logger_buffer_thread(engine_state_log_buffer, &Config::new(), rx_log_buffer);
    });

    std_in_thread.join().expect(RIP_COULDN_JOIN_THREAD);
}


#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use std::sync::Arc;

    use crate::config::Config;
    use crate::game_handler::game_loop;
    use crate::threads::uci_command_processor;
    use crate::threads::logger_buffer_thread;
    
    struct TestEnvironment {
        tx_std_in: mpsc::Sender<String>,
        _uci_command_processor: thread::JoinHandle<()>,
        #[allow(dead_code)]
        engine_state: Arc<crate::model::EngineState>,
    }
    
    fn set_up(config: &Config) -> TestEnvironment {
        let (tx_std_in, rx_std_in) = mpsc::channel();
        let (tx_game_command, rx_game_command) = mpsc::channel();
        let (tx_log_buffer, rx_log_buffer) = mpsc::channel();
    
        let initial_logger: std::sync::Arc<dyn Fn(String) + Send + Sync> = std::sync::Arc::new(|_| {});
        let engine_state = Arc::new(crate::model::EngineState {
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: std::sync::RwLock::new(Arc::new(crate::zobrist::ZobristTable::with_capacity(config.max_zobrist_hash_entries))),

            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(initial_logger)),
            log_sender: tx_log_buffer.clone(),
        });
    
        // Set up UCI command thread
        let engine_state_command_processor = engine_state.clone();
        let config_uci_command_processor = config.clone();
        let tx_game_command_clone = tx_game_command.clone();
        let _uci_command_processor = thread::spawn(move || {
            uci_command_processor(engine_state_command_processor, &config_uci_command_processor, rx_std_in, tx_game_command_clone);
        });
    
        // Set up game loop thread
        let engine_state_game_loop = engine_state.clone();
        let config_game_handler = config.clone();
        let _game_handler = thread::spawn(move || {
            game_loop(engine_state_game_loop, &config_game_handler, rx_game_command);
        });
    
        // Set up logger thread
        let engine_state_log_buffer = engine_state.clone();
        let config_logger_buffer = config.clone();
        let _logger_buffer = thread::spawn(move || {
            logger_buffer_thread(engine_state_log_buffer, &config_logger_buffer, rx_log_buffer);
        });
    
        TestEnvironment {
            tx_std_in,
            _uci_command_processor,
            engine_state,
        }
    }

    fn send_uci(env: &TestEnvironment, cmd: &str, sleep_millis: i32) {
        let rip_err = "RIP Test execution error";
        env.tx_std_in.send(cmd.to_string()).expect(rip_err);
        thread::sleep(Duration::from_millis(sleep_millis as u64));
    }
    
    #[test]
    #[ignore]
    fn setup_test_env_test() {
        let rip_err = "RIP Test execution error";
        let env = set_up(&Config::new()._for_integration_tests());

        send_uci(&env, "debug on", 10);
        send_uci(&env, "go infinite", 500);
        send_uci(&env, "quit", 20);

        env._uci_command_processor.join().expect(rip_err);
    }
    
    #[test]
    fn go_multithreading_test() {
        let rip_err = "RIP Test execution error";
        let env = set_up(&Config::new()._for_integration_tests());

        send_uci(&env, "debug on", 10);
        send_uci(&env, "position startpos", 10);
        send_uci(&env, "go wtime 5000 btime 5000", 500);
        send_uci(&env, "stop", 100);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);        
    }

    #[test]
    #[ignore]
    fn go_infinite_test() {
        let rip_err = "RIP Test execution error";
        let env = set_up(&Config::new()._for_integration_tests());

        send_uci(&env, "debug on", 10);
        send_uci(&env, "go infinite", 500);
        send_uci(&env, "stop", 100);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);
    }

    #[test]
    #[ignore]
    fn pv_time_test() {
        let rip_err = "RIP Test execution error";

        let env = set_up(&Config::new()._for_integration_tests_with_pv_nodes());
        send_uci(&env, "debug on", 10);
        send_uci(&env, "position fen r1bqr1k1/1p2bppp/p1n5/2p5/P2pPP2/5NQ1/BPP3PP/R1B2RK1 b - - 0 14", 3000);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        let env = set_up(&Config::new()._for_integration_tests_wo_pv_nodes());
        send_uci(&env, "position fen r1bqr1k1/1p2bppp/p1n5/2p5/P2pPP2/5NQ1/BPP3PP/R1B2RK1 b - - 0 14", 3000);
        send_uci(&env, "debug on", 10);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        let env = set_up(&Config::new()._for_integration_tests_with_pv_nodes());
        send_uci(&env, "debug on", 10);
        send_uci(&env, "position fen r1b2rk1/p3ppbp/2p2np1/6B1/2B5/2N1P3/PP3PPP/3R1RK1 b - - 0 12", 3000);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        let env = set_up(&Config::new()._for_integration_tests_wo_pv_nodes());
        send_uci(&env, "position fen r1b2rk1/p3ppbp/2p2np1/6B1/2B5/2N1P3/PP3PPP/3R1RK1 b - - 0 12", 3000);
        send_uci(&env, "debug on", 10);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        let env = set_up(&Config::new()._for_integration_tests_with_pv_nodes());
        send_uci(&env, "debug on", 10);
        send_uci(&env, "position fen r1bqk2r/pppp1ppp/2n1pn2/8/1b2P3/3P1N2/PPPN1PPP/R1BQKB1R w KQkq - 3 5", 3000);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        let env = set_up(&Config::new()._for_integration_tests_wo_pv_nodes());
        send_uci(&env, "position fen r1bqk2r/pppp1ppp/2n1pn2/8/1b2P3/3P1N2/PPPN1PPP/R1BQKB1R w KQkq - 3 5", 3000);
        send_uci(&env, "debug on", 10);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);
    }
}

