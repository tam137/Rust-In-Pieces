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
    
    fn set_up(config: &Config) -> TestEnvironment {
    
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
        let config_hash_writer = config.clone();
        let _hash_writer = thread::spawn(move || {
            hash_writer(global_map_hash_writer, &config_hash_writer, rx_hashes);
        });
    
        // Set up UCI command thread
        let global_map_command_processor = global_map.clone();
        let config_uci_command_processor = config.clone();
        let _uci_command_processor = thread::spawn(move || {
            uci_command_processor(global_map_command_processor, &config_uci_command_processor, rx_std_in);
        });
    
        // Set up game loop thread
        let global_map_game_loop = global_map.clone();
        let config_game_handler = config.clone();
        let _game_handler = thread::spawn(move || {
            game_loop(global_map_game_loop, &config_game_handler, rx_game_command);
        });
    
        // Set up logger thread
        let global_map_log_buffer = global_map.clone();
        let config_logger_buffer = config.clone();
        let _logger_buffer = thread::spawn(move || {
            logger_buffer_thread(global_map_log_buffer, &config_logger_buffer, rx_log_buffer);
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
    


    #[test]
    #[ignore]
    fn setup_test_env_test() {
        let rip_err = "RIP Test execution error";
        let env = set_up(&Config::new()._for_integration_tests());

        send_uci(&env, "debug on", 10);
        send_uci(&env, "go infinite", 500);
        send_uci(&env, "quit", 20);

        let sr = global_map_handler::_get_search_results(&env.global_map);
        assert!(sr.len() > 2);
        global_map_handler::clear_search_result(&env.global_map);
        let sr = global_map_handler::_get_search_results(&env.global_map);
        assert_eq!(0, sr.len());
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

        // First Test gain ~ 10%

        // Stats { best_turn_nr: 0, turn_number_gt_threshold: 0, created_nodes: 247208, created_capture_node: 3740, calculated_nodes: 8835, eval_nodes: 247208, calc_time_ms: 2000, zobrist_hit: 42867, cuts: 97, capture_share: 1, nodes_per_ms: 101, logging: [] }
        let env = set_up(&&Config::new()._for_integration_tests_with_pv_nodes());
        send_uci(&env, "debug on", 10);
        send_uci(&env, "position fen r1bqr1k1/1p2bppp/p1n5/2p5/P2pPP2/5NQ1/BPP3PP/R1B2RK1 b - - 0 14", 3000);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        // Stats { best_turn_nr: 0, turn_number_gt_threshold: 0, created_nodes: 272441, created_capture_node: 4474, calculated_nodes: 9716, eval_nodes: 272441, calc_time_ms: 2124, zobrist_hit: 43485, cuts: 97, capture_share: 1, nodes_per_ms: 110, logging: [] }
        let env = set_up(&Config::new()._for_integration_tests_wo_pv_nodes());
        send_uci(&env, "position fen r1bqr1k1/1p2bppp/p1n5/2p5/P2pPP2/5NQ1/BPP3PP/R1B2RK1 b - - 0 14", 3000);
        send_uci(&env, "debug on", 10);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        // Second Test gain ~ 30%

        // Stats { best_turn_nr: 0, turn_number_gt_threshold: 0, created_nodes: 177998, created_capture_node: 4294, calculated_nodes: 7158, eval_nodes: 177998, calc_time_ms: 1386, zobrist_hit: 38887, cuts: 96, capture_share: 3, nodes_per_ms: 99, logging: [] }
        let env = set_up(&&Config::new()._for_integration_tests_with_pv_nodes());
        send_uci(&env, "debug on", 10);
        send_uci(&env, "position fen r1b2rk1/p3ppbp/2p2np1/6B1/2B5/2N1P3/PP3PPP/3R1RK1 b - - 0 12", 3000);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        // Stats { best_turn_nr: 0, turn_number_gt_threshold: 0, created_nodes: 283071, created_capture_node: 3683, calculated_nodes: 10198, eval_nodes: 283071, calc_time_ms: 2068, zobrist_hit: 30211, cuts: 97, capture_share: 1, nodes_per_ms: 128, logging: [] }
        let env = set_up(&Config::new()._for_integration_tests_wo_pv_nodes());
        send_uci(&env, "position fen r1b2rk1/p3ppbp/2p2np1/6B1/2B5/2N1P3/PP3PPP/3R1RK1 b - - 0 12", 3000);
        send_uci(&env, "debug on", 10);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        // Third Test no gain ~ -30%

        // Stats { best_turn_nr: 2, turn_number_gt_threshold: 0, created_nodes: 98934, created_capture_node: 1742, calculated_nodes: 4592, eval_nodes: 98934, calc_time_ms: 1023, zobrist_hit: 9487, cuts: 96, capture_share: 1, nodes_per_ms: 83, logging: [] }
        let env = set_up(&&Config::new()._for_integration_tests_with_pv_nodes());
        send_uci(&env, "debug on", 10);
        send_uci(&env, "position fen r1bqk2r/pppp1ppp/2n1pn2/8/1b2P3/3P1N2/PPPN1PPP/R1BQKB1R w KQkq - 3 5", 3000);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);

        // Stats { best_turn_nr: 1, turn_number_gt_threshold: 0, created_nodes: 66408, created_capture_node: 1023, calculated_nodes: 2918, eval_nodes: 66408, calc_time_ms: 733, zobrist_hit: 9914, cuts: 96, capture_share: 1, nodes_per_ms: 69, logging: [] }
        let env = set_up(&Config::new()._for_integration_tests_wo_pv_nodes());
        send_uci(&env, "position fen r1bqk2r/pppp1ppp/2n1pn2/8/1b2P3/3P1N2/PPPN1PPP/R1BQKB1R w KQkq - 3 5", 3000);
        send_uci(&env, "debug on", 10);
        send_uci(&env, "go depth 4", 2500);
        send_uci(&env, "quit", 0);
        env._uci_command_processor.join().expect(rip_err);
        
    }


}