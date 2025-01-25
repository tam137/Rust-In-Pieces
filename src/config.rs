use crate::model::QuiescenceSearchMode;

#[derive(Clone)]
pub struct Config {
    pub version: String,
    pub use_zobrist: bool,
    pub use_book: bool,
    pub max_zobrist_hash_entries: usize,
    pub search_depth: i32,
    pub max_depth: i32,
    pub truncate_bad_moves: usize,
    pub in_debug: bool,
    pub _print_commands: bool,
    pub print_eval_per_figure: bool,
    pub log_to_console: bool,
    pub quiescence_search_mode: QuiescenceSearchMode,
    pub print_info_string_during_search: bool,
    pub write_hash_buffer_size: usize,
    pub search_threads: i32,
    pub use_pv_nodes: bool,
    pub min_thinking_time: u64,
    pub game_loop: u64,
    pub smp_thread_eval_noise: i16,
    pub skip_strong_validation: bool,
    pub max_eval_mult: f32,

    pub your_turn_bonus: i16,

    pub undeveloped_knight_malus: i16,
    pub undeveloped_bishop_malus: i16,
    pub undeveloped_king_malus: i16,
    pub pawn_on_last_rank_bonus: i16,
    pub pawn_on_before_last_rank_bonus: i16,
    pub pawn_on_before_before_last_rank_bonus: i16,
    pub pawn_defends_bishop: i16,

    pub piece_eval_pawn: i16,
    pub piece_eval_rook: i16,
    pub piece_eval_knight: i16,
    pub piece_eval_bishop: i16,
    pub piece_eval_queen: i16,
    pub piece_eval_king: i16,

    pub pawn_structure: i16,
    pub pawn_supports_knight_outpost: i16,
    pub pawn_centered: i16,
    pub pawn_undeveloped_malus: i16,
    pub pawn_double_malus: i16,

    pub knight_on_rim_malus: i16,
    pub knight_centered: i16,
    pub knight_blockes_pawn: i16,
    pub bishop_trapped_at_rim_malus: i16,

    pub pawn_attacks_opponent_fig: i16,
    pub pawn_attacks_opponent_fig_with_tempo: i16,
    pub queen_in_attack: i16,
    pub queen_in_attack_with_tempo: i16,
    pub knight_attacks_bishop: i16,
    pub knight_attacks_rook: i16,
    pub knight_attacks_bishop_tempo: i16,
    pub knight_attacks_rook_tempo: i16,

    pub king_shield: i16,
    pub king_trapp_at_baseline_malus: i16,
    pub king_in_check_malus: i16,
    pub king_in_double_check_malus: i16,
    pub king_centered: i16
}


impl Config {
    pub fn new() -> Config {
        Config {
            version: "V0.2.1-candidate-5".to_string(),
            use_zobrist: true,
            use_book: true,
            max_zobrist_hash_entries: 10_000_000, // 1.000.000 = 75MB
            search_depth: 4, // only used as default for tests
            max_depth: 99,
            truncate_bad_moves: 99,
            in_debug: true,
            _print_commands: false,
            print_eval_per_figure: false,
            log_to_console: false,
            quiescence_search_mode: QuiescenceSearchMode::Alpha3,
            print_info_string_during_search: false,
            write_hash_buffer_size: 10,
            search_threads: 1,
            use_pv_nodes: true,
            min_thinking_time: 2,
            game_loop: 3,
            smp_thread_eval_noise: 0,
            skip_strong_validation: false,
            max_eval_mult: 1.1,

            your_turn_bonus: 20,

            undeveloped_knight_malus: 42,
            undeveloped_bishop_malus: 30,
            undeveloped_king_malus: 55,

            piece_eval_pawn: 100,
            piece_eval_rook: 500,
            piece_eval_knight: 300,
            piece_eval_bishop: 300,
            piece_eval_queen: 950,
            piece_eval_king: 10000,

            pawn_structure: 5,
            pawn_supports_knight_outpost: 10,
            pawn_centered: 12,
            pawn_undeveloped_malus: 12,
            pawn_on_last_rank_bonus: 180,
            pawn_on_before_last_rank_bonus: 110,
            pawn_on_before_before_last_rank_bonus: 40,
            pawn_defends_bishop: 20,
            pawn_double_malus: 16,

            knight_on_rim_malus: 12,
            knight_centered: 20,
            knight_blockes_pawn: 24,
            bishop_trapped_at_rim_malus: 50,

            pawn_attacks_opponent_fig: 35,
            pawn_attacks_opponent_fig_with_tempo: 150,
            queen_in_attack: 60,
            queen_in_attack_with_tempo: 700,
            knight_attacks_bishop: 5,
            knight_attacks_rook: 15,
            knight_attacks_bishop_tempo: 10,
            knight_attacks_rook_tempo: 100,

            king_shield: 40,
            king_trapp_at_baseline_malus: 75,
            king_in_check_malus: 140,
            king_in_double_check_malus: 350,
            king_centered: 85,
        }
    }

    /// Sets turn_bonus and all tempo attack boni at 0
    pub fn _for_evel_equal_tests() -> Self {
        let mut config = Config::new();
        config.your_turn_bonus = 0;
        config.pawn_attacks_opponent_fig_with_tempo = 0;
        config.queen_in_attack_with_tempo = 0;
        config.knight_attacks_rook_tempo = 0;
        config.knight_attacks_bishop_tempo = 0;
        config
    }

    /// This config is used for tests, it uses the ALPHA2 cutting algo in quiescence search and will not print uci info string
    /// It disable also all evaluation with TEMPO
    /// Also ZOBRIST hash is disabled
    pub fn for_timing_tests(&self) -> Self {
        let mut config = Config::new();
        config.print_info_string_during_search = false;
        config.quiescence_search_mode = QuiescenceSearchMode::Alpha2;
        config.use_zobrist = false;
        config
    }

    /// This config is used for tests, it uses the ALPHA2 cutting algo in quiescence search and will not print uci info string
    /// It disable also all evaluation with TEMPO
    /// Also ZOBRIST hash is disabled
    pub fn for_tests() -> Self {
        let mut config = Config::new();
        config.print_info_string_during_search = false;
        config.quiescence_search_mode = QuiescenceSearchMode::Alpha2;
        config.use_zobrist = false;
        config.queen_in_attack_with_tempo = 0;
        config
    }

    /// This config is used for tests, it will log to console
    /// It uses the alpha2 cutting algo in quiescence search and
    /// will not print uci info string
    /// The zobrist hash is smaller with 1 Mio entries
    /// Book is disabled
    pub fn _for_integration_tests(&self) -> Self {
        let mut config = Config::new();
        config._print_commands = true;
        config.log_to_console = true;
        config.print_info_string_during_search = false;
        config.use_book = false;
        config.search_threads = 4;
        config.write_hash_buffer_size = config.write_hash_buffer_size;
        config.max_zobrist_hash_entries = 10_000_000;
        config
    }

    // like integration test but wo pv nodes and 1 thread
    pub fn _for_integration_tests_with_pv_nodes(&self) -> Self {
        let mut config = Config::_for_integration_tests(&self);
        config.use_pv_nodes = true;
        config.search_threads = 1;
        config
    }

    // like integration test with 1 thread
    pub fn _for_integration_tests_wo_pv_nodes(&self) -> Self {
        let mut config = Config::_for_integration_tests(&self);
        config.use_pv_nodes = false;
        config.search_threads = 1;
        config
    }
}