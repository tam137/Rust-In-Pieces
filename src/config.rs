use crate::model::QuiescenceSearchMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggressiveness {
    Normal,
    Aggressive,
    HighAggressive,
}

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
    #[allow(dead_code)]
    pub search_threads: i32,
    pub use_pv_nodes: bool,
    pub min_thinking_time: u64,
    #[allow(dead_code)]
    pub game_loop: u64,
    pub smp_thread_eval_noise: i16,
    pub skip_strong_validation: bool,
    pub max_eval_mult: f32,
    pub aggressiveness: Aggressiveness,
    pub enable_positional_cap: bool,
    pub positional_cap_damping: i16,
    pub move_overhead: u64,

    pub is_hashed_rank_bonus: i32,
    pub give_check_rank_bonus: i32,
    pub is_pv_node_rank_bonus: i32,
    pub give_promotion_rank_bonus_queen: i32,
    pub give_promotion_rank_bonus_knight: i32,

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
    pub rook_open_file: i16,
    pub rook_half_open_file: i16,
    pub bishop_pair_bonus: i16,
    pub rook_doubled_bonus: i16,
    pub rook_behind_passed_pawn_middlegame: i16,
    pub rook_behind_passed_pawn_endgame: i16,
    pub king_ring_attack_knight: i16,
    pub king_ring_attack_bishop: i16,
    pub king_ring_attack_rook: i16,
    pub king_ring_attack_queen: i16,
    pub protected_passed_pawn_middlegame: i16,
    pub protected_passed_pawn_endgame: i16,
    pub king_opposition_bonus: i16,

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


    pub king_pawn_shield: i16,
    pub king_piece_shield: i16,
    pub king_trapp_at_baseline_malus: i16,
    pub king_in_check_malus: i16,
    pub king_in_double_check_malus: i16,

    pub rook_on_seventh: i16,
    pub pawn_isolated_malus: i16,
    pub pawn_backward_malus: i16,
    pub knight_mobility_factor: i16,
    pub bishop_mobility_factor: i16,
    pub rook_mobility_factor: i16,
    pub pre_sort_moves: bool,
    pub use_underpromotions: bool,
    pub enable_pvs: bool,
    pub enable_lmr: bool,
    pub enable_nmp: bool,
    pub enable_aspiration: bool,
    pub enable_rfp: bool,
    pub enable_delta_pruning: bool,
    pub delta_pruning_margin: i16,
    pub enable_counter_moves: bool,
    pub enable_history_malus: bool,
    pub killer_move_1_rank_bonus: i32,
    pub killer_move_2_rank_bonus: i32,
    pub counter_move_rank_bonus: i32,
    pub history_max_threshold: u32,
    pub lmr_depth_threshold: i32,
    pub lmr_move_threshold: i32,

    /// Precalculated logarithmic LMR reduction lookup table indexed by [depth][move_index].
    pub lmr_table: [[i16; 64]; 64],
    pub nmp_depth_threshold: i32,
    pub nmp_reduction: i32,
    pub nmp_verification_threshold: i32,
    pub nmp_dynamic_divisor: i32,
}


impl Config {
    pub fn new() -> Config {
        Config {
            version: concat!("V", env!("CARGO_PKG_VERSION")).to_string(),
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
            quiescence_search_mode: QuiescenceSearchMode::Alpha2,
            print_info_string_during_search: false,

            search_threads: 2,
            use_pv_nodes: true,
            min_thinking_time: 2,
            game_loop: 3,
            smp_thread_eval_noise: 0,
            skip_strong_validation: false,
            max_eval_mult: 2.0,
            aggressiveness: Aggressiveness::Normal,
            enable_positional_cap: true,
            positional_cap_damping: 5,
            move_overhead: 0,

            is_hashed_rank_bonus: 3,
            give_check_rank_bonus: 5,
            is_pv_node_rank_bonus: 15,
            give_promotion_rank_bonus_queen: 15,
            give_promotion_rank_bonus_knight: 6,

            your_turn_bonus: 20,

            undeveloped_knight_malus: 42,
            undeveloped_bishop_malus: 30,
            undeveloped_king_malus: 55,

            piece_eval_pawn: 100,
            piece_eval_rook: 500,
            piece_eval_knight: 300, // TODO knights a stronger pairwise
            piece_eval_bishop: 300,
            piece_eval_queen: 950,
            piece_eval_king: 10000,
            rook_open_file: 35,
            rook_half_open_file: 18,
            bishop_pair_bonus: 40,
            rook_doubled_bonus: 20,
            rook_behind_passed_pawn_middlegame: 15,
            rook_behind_passed_pawn_endgame: 30,
            king_ring_attack_knight: 1,
            king_ring_attack_bishop: 1,
            king_ring_attack_rook: 2,
            king_ring_attack_queen: 4,
            protected_passed_pawn_middlegame: 12,
            protected_passed_pawn_endgame: 24,
            king_opposition_bonus: 12,

            pawn_structure: 5,
            pawn_supports_knight_outpost: 12,
            pawn_centered: 12,
            pawn_undeveloped_malus: 12,
            pawn_on_last_rank_bonus: 180,
            pawn_on_before_last_rank_bonus: 110,
            pawn_on_before_before_last_rank_bonus: 40,
            pawn_defends_bishop: 20,
            pawn_double_malus: 16,

            knight_on_rim_malus: 16,
            knight_centered: 20,
            knight_blockes_pawn: 24,
            bishop_trapped_at_rim_malus: 50,

            pawn_attacks_opponent_fig: 35,
            pawn_attacks_opponent_fig_with_tempo: 15,
            queen_in_attack: 60,
            queen_in_attack_with_tempo: 30,
            knight_attacks_bishop: 5,
            knight_attacks_rook: 15,
            knight_attacks_bishop_tempo: 10,
            knight_attacks_rook_tempo: 10,


            king_pawn_shield: 40,
            king_piece_shield: 15,
            king_trapp_at_baseline_malus: 75,
            king_in_check_malus: 140,
            king_in_double_check_malus: 350,

            rook_on_seventh: 25,
            pawn_isolated_malus: 8,
            pawn_backward_malus: 14,
            knight_mobility_factor: 2,
            bishop_mobility_factor: 1,
            rook_mobility_factor: 1,
            pre_sort_moves: true,
            use_underpromotions: false,
            enable_pvs: true,
            enable_lmr: true,
            enable_nmp: true,
            enable_aspiration: true,
            enable_rfp: true,
            enable_delta_pruning: false,
            delta_pruning_margin: 300,
            enable_counter_moves: true,
            enable_history_malus: false,
            killer_move_1_rank_bonus: 20000,
            killer_move_2_rank_bonus: 10000,
            counter_move_rank_bonus: 15000,
            history_max_threshold: 9000,
            lmr_depth_threshold: 3,
            lmr_move_threshold: 3,

            lmr_table: {
                let mut table = [[0i16; 64]; 64];
                let divisor = 1.95;
                for depth in 1..64 {
                    for move_idx in 1..64 {
                        let d = depth as f64;
                        let m = move_idx as f64;
                        let reduction = (d.ln() * m.ln() / divisor) as i16;
                        table[depth][move_idx] = reduction.max(0);
                    }
                }
                table
            },
            nmp_depth_threshold: 3,
            nmp_reduction: 2,
            nmp_verification_threshold: 6,
            nmp_dynamic_divisor: 6,
        }
    }

    /// Sets turn_bonus and all tempo attack boni at 0
    pub fn _for_evel_equal_tests() -> Self {
        let mut config = Config::new();
        config.aggressiveness = Aggressiveness::Normal;
        config.enable_positional_cap = false;
        config.move_overhead = 0;
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
        config.use_underpromotions = true;
        config.move_overhead = 0;
        config
    }

    /// This config is used for tests, it uses the ALPHA2 cutting algo in quiescence search and will not print uci info string
    /// It disable also all evaluation with TEMPO
    /// Also ZOBRIST hash is disabled
    pub fn for_tests() -> Self {
        let mut config = Config::new();
        config.aggressiveness = Aggressiveness::Normal;
        config.enable_positional_cap = false;
        config.print_info_string_during_search = false;
        config.quiescence_search_mode = QuiescenceSearchMode::Alpha2;
        config.use_zobrist = false;
        config.queen_in_attack_with_tempo = 0;
        config.use_underpromotions = true;
        config.move_overhead = 0;
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
        config.max_zobrist_hash_entries = 10_000_000;
        config.use_underpromotions = true;
        config.move_overhead = 0;
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