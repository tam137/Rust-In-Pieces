use crate::model::QuiescenceSearchMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggressiveness {
    Normal,
    Aggressive,
    HighAggressive,
}

#[derive(Clone)]
pub struct Config {
    pub version: &'static str,
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
    pub king_open_file_malus: i16,
    pub king_half_open_file_malus: i16,
    pub king_ring_defender_value: i16,

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
    pub threat_minor_attacks_rook: i16,
    pub threat_minor_attacks_queen: i16,
    pub threat_rook_attacks_queen: i16,


    pub king_pawn_shield: i16,
    pub king_piece_shield: i16,
    pub king_pawn_shield_kingside: i16,
    pub king_pawn_shield_queenside: i16,
    pub king_piece_shield_kingside: i16,
    pub king_piece_shield_queenside: i16,
    pub connected_passed_pawn_mg: i16,
    pub connected_passed_pawn_eg: i16,
    pub knight_outpost_true_mg: i16,
    pub knight_outpost_true_eg: i16,
    pub bishop_outpost_true_mg: i16,
    pub bishop_outpost_true_eg: i16,
    pub opposite_bishops_draw_scale: i16,
    pub rook_behind_enemy_passed_pawn_mg: i16,
    pub rook_behind_enemy_passed_pawn_eg: i16,
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
    pub lmr_move_threshold: i32,
    pub lmr_divisor: i32,


    /// Precalculated logarithmic LMR reduction lookup table indexed by [depth][move_index].
    pub lmr_table: [[i16; 64]; 64],
    pub nmp_depth_threshold: i32,
    pub nmp_reduction: i32,
    pub nmp_verification_threshold: i32,
    pub nmp_dynamic_divisor: i32,
    pub log_path: std::sync::Arc<str>,
}


impl Config {
    pub fn new() -> Config {
        Config {
            version: concat!("V", env!("CARGO_PKG_VERSION")),
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

            is_hashed_rank_bonus: 4,
            give_check_rank_bonus: 5,
            is_pv_node_rank_bonus: 17,
            give_promotion_rank_bonus_queen: 18,
            give_promotion_rank_bonus_knight: 6,

            your_turn_bonus: 21,

            undeveloped_knight_malus: 31,
            undeveloped_bishop_malus: 34,
            undeveloped_king_malus: 54,


            rook_open_file: 26,
            rook_half_open_file: 22,
            bishop_pair_bonus: 46,
            rook_doubled_bonus: 25,
            rook_behind_passed_pawn_middlegame: 13,
            rook_behind_passed_pawn_endgame: 36,
            king_ring_attack_knight: 1,
            king_ring_attack_bishop: 1,
            king_ring_attack_rook: 2,
            king_ring_attack_queen: 4,
            protected_passed_pawn_middlegame: 12,
            protected_passed_pawn_endgame: 26,
            king_opposition_bonus: 12,
            king_open_file_malus: 38,
            king_half_open_file_malus: 20,
            king_ring_defender_value: 1,

            pawn_structure: 5,
            pawn_supports_knight_outpost: 10,
            pawn_centered: 14,
            pawn_undeveloped_malus: 15,
            pawn_on_last_rank_bonus: 178,
            pawn_on_before_last_rank_bonus: 105,
            pawn_on_before_before_last_rank_bonus: 47,
            pawn_defends_bishop: 23,
            pawn_double_malus: 14,

            knight_on_rim_malus: 17,
            knight_centered: 22,
            knight_blockes_pawn: 27,
            bishop_trapped_at_rim_malus: 57,

            pawn_attacks_opponent_fig: 34,
            pawn_attacks_opponent_fig_with_tempo: 18,
            queen_in_attack: 53,
            queen_in_attack_with_tempo: 29,
            knight_attacks_bishop: 5,
            knight_attacks_rook: 17,
            knight_attacks_bishop_tempo: 9,
            knight_attacks_rook_tempo: 13,
            threat_minor_attacks_rook: 13,
            threat_minor_attacks_queen: 25,
            threat_rook_attacks_queen: 19,


            king_pawn_shield: 39,
            king_piece_shield: 16,
            king_pawn_shield_kingside: 39,
            king_pawn_shield_queenside: 25,
            king_piece_shield_kingside: 16,
            king_piece_shield_queenside: 10,
            connected_passed_pawn_mg: 15,
            connected_passed_pawn_eg: 30,
            knight_outpost_true_mg: 30,
            knight_outpost_true_eg: 15,
            bishop_outpost_true_mg: 20,
            bishop_outpost_true_eg: 10,
            opposite_bishops_draw_scale: 50,
            rook_behind_enemy_passed_pawn_mg: 10,
            rook_behind_enemy_passed_pawn_eg: 25,
            king_trapp_at_baseline_malus: 72,
            king_in_check_malus: 136,
            king_in_double_check_malus: 343,

            rook_on_seventh: 32,
            pawn_isolated_malus: 9,
            pawn_backward_malus: 12,
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
            lmr_move_threshold: 3,
            lmr_divisor: 150,

            lmr_table: {
                let mut table = [[0i16; 64]; 64];
                let divisor = 150.0 / 100.0;
                for (depth, row) in table.iter_mut().enumerate().skip(1) {
                    for (move_idx, item) in row.iter_mut().enumerate().take(64).skip(1) {
                        let d = depth as f64;
                        let m = move_idx as f64;
                        let reduction = (d.ln() * m.ln() / divisor) as i16;
                        *item = reduction.max(0);
                    }
                }
                table
            },
            nmp_depth_threshold: 3,
            nmp_reduction: 2,
            nmp_verification_threshold: 6,
            nmp_dynamic_divisor: 6,
            log_path: std::sync::Arc::from(""),
        }
    }

    pub fn recalculate_lmr_table(&mut self) {
        let divisor = self.lmr_divisor as f64 / 100.0;
        for (depth, row) in self.lmr_table.iter_mut().enumerate().skip(1) {
            for (move_idx, item) in row.iter_mut().enumerate().take(64).skip(1) {
                let d = depth as f64;
                let m = move_idx as f64;
                let reduction = (d.ln() * m.ln() / divisor) as i16;
                *item = reduction.max(0);
            }
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
        config.threat_minor_attacks_rook = 0;
        config.threat_minor_attacks_queen = 0;
        config.threat_rook_attacks_queen = 0;
        config.king_open_file_malus = 0;
        config.king_half_open_file_malus = 0;
        config.undeveloped_knight_malus = 0;
        config.undeveloped_bishop_malus = 0;
        config.undeveloped_king_malus = 0;
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
        let mut config = Config::_for_integration_tests(self);
        config.use_pv_nodes = true;
        config.search_threads = 1;
        config
    }

    // like integration test with 1 thread
    pub fn _for_integration_tests_wo_pv_nodes(&self) -> Self {
        let mut config = Config::_for_integration_tests(self);
        config.use_pv_nodes = false;
        config.search_threads = 1;
        config
    }

    pub fn log_all_parameters(&self, logger: &std::sync::mpsc::Sender<String>) {
        if self.log_path.is_empty() {
            return;
        }
        let mut msg = String::new();
        msg.push_str("Current Engine Parameters:\n");
        msg.push_str(&format!("  is_hashed_rank_bonus: {}\n", self.is_hashed_rank_bonus));
        msg.push_str(&format!("  give_check_rank_bonus: {}\n", self.give_check_rank_bonus));
        msg.push_str(&format!("  is_pv_node_rank_bonus: {}\n", self.is_pv_node_rank_bonus));
        msg.push_str(&format!("  give_promotion_rank_bonus_queen: {}\n", self.give_promotion_rank_bonus_queen));
        msg.push_str(&format!("  give_promotion_rank_bonus_knight: {}\n", self.give_promotion_rank_bonus_knight));
        msg.push_str(&format!("  your_turn_bonus: {}\n", self.your_turn_bonus));
        msg.push_str(&format!("  undeveloped_knight_malus: {}\n", self.undeveloped_knight_malus));
        msg.push_str(&format!("  undeveloped_bishop_malus: {}\n", self.undeveloped_bishop_malus));
        msg.push_str(&format!("  undeveloped_king_malus: {}\n", self.undeveloped_king_malus));
        msg.push_str(&format!("  rook_open_file: {}\n", self.rook_open_file));
        msg.push_str(&format!("  rook_half_open_file: {}\n", self.rook_half_open_file));
        msg.push_str(&format!("  bishop_pair_bonus: {}\n", self.bishop_pair_bonus));
        msg.push_str(&format!("  rook_doubled_bonus: {}\n", self.rook_doubled_bonus));
        msg.push_str(&format!("  rook_behind_passed_pawn_middlegame: {}\n", self.rook_behind_passed_pawn_middlegame));
        msg.push_str(&format!("  rook_behind_passed_pawn_endgame: {}\n", self.rook_behind_passed_pawn_endgame));
        msg.push_str(&format!("  king_ring_attack_knight: {}\n", self.king_ring_attack_knight));
        msg.push_str(&format!("  king_ring_attack_bishop: {}\n", self.king_ring_attack_bishop));
        msg.push_str(&format!("  king_ring_attack_rook: {}\n", self.king_ring_attack_rook));
        msg.push_str(&format!("  king_ring_attack_queen: {}\n", self.king_ring_attack_queen));
        msg.push_str(&format!("  protected_passed_pawn_middlegame: {}\n", self.protected_passed_pawn_middlegame));
        msg.push_str(&format!("  protected_passed_pawn_endgame: {}\n", self.protected_passed_pawn_endgame));
        msg.push_str(&format!("  king_opposition_bonus: {}\n", self.king_opposition_bonus));
        msg.push_str(&format!("  pawn_structure: {}\n", self.pawn_structure));
        msg.push_str(&format!("  pawn_supports_knight_outpost: {}\n", self.pawn_supports_knight_outpost));
        msg.push_str(&format!("  pawn_centered: {}\n", self.pawn_centered));
        msg.push_str(&format!("  pawn_undeveloped_malus: {}\n", self.pawn_undeveloped_malus));
        msg.push_str(&format!("  pawn_on_last_rank_bonus: {}\n", self.pawn_on_last_rank_bonus));
        msg.push_str(&format!("  pawn_on_before_last_rank_bonus: {}\n", self.pawn_on_before_last_rank_bonus));
        msg.push_str(&format!("  pawn_on_before_before_last_rank_bonus: {}\n", self.pawn_on_before_before_last_rank_bonus));
        msg.push_str(&format!("  pawn_defends_bishop: {}\n", self.pawn_defends_bishop));
        msg.push_str(&format!("  pawn_double_malus: {}\n", self.pawn_double_malus));
        msg.push_str(&format!("  knight_on_rim_malus: {}\n", self.knight_on_rim_malus));
        msg.push_str(&format!("  knight_centered: {}\n", self.knight_centered));
        msg.push_str(&format!("  knight_blockes_pawn: {}\n", self.knight_blockes_pawn));
        msg.push_str(&format!("  bishop_trapped_at_rim_malus: {}\n", self.bishop_trapped_at_rim_malus));
        msg.push_str(&format!("  pawn_attacks_opponent_fig: {}\n", self.pawn_attacks_opponent_fig));
        msg.push_str(&format!("  pawn_attacks_opponent_fig_with_tempo: {}\n", self.pawn_attacks_opponent_fig_with_tempo));
        msg.push_str(&format!("  queen_in_attack: {}\n", self.queen_in_attack));
        msg.push_str(&format!("  queen_in_attack_with_tempo: {}\n", self.queen_in_attack_with_tempo));
        msg.push_str(&format!("  knight_attacks_bishop: {}\n", self.knight_attacks_bishop));
        msg.push_str(&format!("  knight_attacks_rook: {}\n", self.knight_attacks_rook));
        msg.push_str(&format!("  knight_attacks_bishop_tempo: {}\n", self.knight_attacks_bishop_tempo));
        msg.push_str(&format!("  knight_attacks_rook_tempo: {}\n", self.knight_attacks_rook_tempo));
        msg.push_str(&format!("  king_pawn_shield: {}\n", self.king_pawn_shield));
        msg.push_str(&format!("  king_piece_shield: {}\n", self.king_piece_shield));
        msg.push_str(&format!("  king_pawn_shield_kingside: {}\n", self.king_pawn_shield_kingside));
        msg.push_str(&format!("  king_pawn_shield_queenside: {}\n", self.king_pawn_shield_queenside));
        msg.push_str(&format!("  king_piece_shield_kingside: {}\n", self.king_piece_shield_kingside));
        msg.push_str(&format!("  king_piece_shield_queenside: {}\n", self.king_piece_shield_queenside));
        msg.push_str(&format!("  connected_passed_pawn_mg: {}\n", self.connected_passed_pawn_mg));
        msg.push_str(&format!("  connected_passed_pawn_eg: {}\n", self.connected_passed_pawn_eg));
        msg.push_str(&format!("  knight_outpost_true_mg: {}\n", self.knight_outpost_true_mg));
        msg.push_str(&format!("  knight_outpost_true_eg: {}\n", self.knight_outpost_true_eg));
        msg.push_str(&format!("  bishop_outpost_true_mg: {}\n", self.bishop_outpost_true_mg));
        msg.push_str(&format!("  bishop_outpost_true_eg: {}\n", self.bishop_outpost_true_eg));
        msg.push_str(&format!("  opposite_bishops_draw_scale: {}\n", self.opposite_bishops_draw_scale));
        msg.push_str(&format!("  rook_behind_enemy_passed_pawn_mg: {}\n", self.rook_behind_enemy_passed_pawn_mg));
        msg.push_str(&format!("  rook_behind_enemy_passed_pawn_eg: {}\n", self.rook_behind_enemy_passed_pawn_eg));
        msg.push_str(&format!("  king_trapp_at_baseline_malus: {}\n", self.king_trapp_at_baseline_malus));
        msg.push_str(&format!("  king_in_check_malus: {}\n", self.king_in_check_malus));
        msg.push_str(&format!("  king_in_double_check_malus: {}\n", self.king_in_double_check_malus));
        msg.push_str(&format!("  pawn_isolated_malus: {}\n", self.pawn_isolated_malus));
        msg.push_str(&format!("  pawn_backward_malus: {}\n", self.pawn_backward_malus));
        msg.push_str(&format!("  knight_mobility_factor: {}\n", self.knight_mobility_factor));
        msg.push_str(&format!("  bishop_mobility_factor: {}\n", self.bishop_mobility_factor));
        msg.push_str(&format!("  rook_mobility_factor: {}\n", self.rook_mobility_factor));
        msg.push_str(&format!("  rook_on_seventh: {}\n", self.rook_on_seventh));
        msg.push_str(&format!("  lmr_move_threshold: {}\n", self.lmr_move_threshold));
        msg.push_str(&format!("  lmr_divisor: {}\n", self.lmr_divisor));

        msg.push_str(&format!("  king_open_file_malus: {}\n", self.king_open_file_malus));
        msg.push_str(&format!("  king_half_open_file_malus: {}\n", self.king_half_open_file_malus));
        msg.push_str(&format!("  king_ring_defender_value: {}\n", self.king_ring_defender_value));
        msg.push_str(&format!("  threat_minor_attacks_rook: {}\n", self.threat_minor_attacks_rook));
        msg.push_str(&format!("  threat_minor_attacks_queen: {}\n", self.threat_minor_attacks_queen));
        msg.push_str(&format!("  threat_rook_attacks_queen: {}\n", self.threat_rook_attacks_queen));
        let _ = logger.send(msg);
    }
}