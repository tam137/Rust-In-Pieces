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
    pub use_nnue: bool,
    pub nnue_model_path: String,
    pub use_book: bool,
    pub cache_book_in_ram: bool,
    pub book_file: String,
    pub max_zobrist_hash_entries: usize,
    /// Default capacity: 1,000,000 entries (~16 MB).
    /// Proven sweet spot in tournament play (+30 Elo over 10M entries).
    /// Keeps pawn structure evaluations inside CPU cache and avoids TLB/DRAM thrashing.
    pub max_pawn_hash_entries: usize,
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
    pub enable_endgame_mopup: bool,
    pub mopup_center_weight: i16,
    pub mopup_proximity_weight: i16,
    pub mopup_eval_threshold: i16,
    pub mopup_max_game_phase: i16,
    pub rook_behind_enemy_passed_pawn_mg: i16,
    pub rook_behind_enemy_passed_pawn_eg: i16,
    pub king_trapp_at_baseline_malus: i16,
    pub king_in_check_malus: i16,
    pub king_in_double_check_malus: i16,
    pub rook_on_seventh: i16,
    pub lazy_eval_margin_search: i16,
    pub lazy_eval_margin_qs: i16,
    pub enable_lazy_eval: bool,
    pub lazy_eval_min_game_phase: u32,
    pub king_danger_weight_1: i16,
    pub king_danger_weight_2: i16,
    pub king_danger_weight_3: i16,
    pub king_danger_weight_4: i16,
    pub king_danger_weight_5: i16,
    pub pawn_isolated_malus: i16,
    pub pawn_backward_malus: i16,
    pub knight_mobility_factor: i16,
    pub bishop_mobility_factor: i16,
    pub rook_mobility_factor: i16,
    pub queen_mobility_factor: i16,
    pub king_passer_dist_weight: i16,
    pub king_open_file_heavy_threat_malus: i16,
    pub rook_open_file_attacks_king: i16,
    pub rook_open_file_attacks_queen: i16,
    pub pawn_phalanx_mg: i16,
    pub pawn_phalanx_eg: i16,
    pub bishop_diagonal_attacks_king: i16,
    pub bishop_diagonal_attacks_queen: i16,
    pub rook_on_seventh_king_cutoff: i16,
    pub rooks_doubled_on_seventh: i16,
    pub passed_pawn_blockaded_malus: i16,
    pub candidate_passed_pawn_bonus: i16,
    pub pawn_storm_bonus: i16,
    pub pre_sort_moves: bool,
    pub use_underpromotions: bool,
    pub enable_pvs: bool,
    pub enable_lmr: bool,
    pub enable_nmp: bool,
    pub enable_aspiration: bool,
    pub enable_rfp: bool,
    pub enable_futility_pruning: bool,
    pub enable_qs_tt: bool,
    pub futility_max_depth: i32,
    pub futility_margin_base: i16,
    pub futility_margin_slope: i16,
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
    pub aspiration_window_initial_delta: i16,
    pub aspiration_window_multiplier: i16,
    /// Once the aspiration delta reaches this value, the next re-search uses a full
    /// window instead of widening further, bounding the number of root re-searches.
    pub aspiration_window_max_delta: i16,
    pub lmr_history_good_threshold: u32,
    pub lmr_history_bad_threshold: u32,
    pub rfp_margin_per_depth: i16,
    pub rfp_max_depth: i32,

    /// Enables Check Extensions: a move that gives check is searched one ply deeper,
    /// so that forcing sequences are resolved beyond the nominal horizon.
    pub enable_check_extension: bool,
    /// Upper ply bound for granting Check Extensions. Beyond this ply the search
    /// depth strictly decreases again, which keeps the search tree finite.
    pub check_extension_max_ply: i32,
    /// Restricts Check Extensions to checks that do not lose material by Static
    /// Exchange Evaluation. Cheap, but on its own it also rejects sacrificial mating
    /// checks, so it is only sound in combination with the One-Reply Extension.
    pub check_extension_require_safe: bool,
    /// Caps how many extensions a single search path may accumulate, expressed as
    /// `root_depth / divisor`. `0` disables the cap. This bounds the compounding cost
    /// of extensions without judging any individual move.
    pub check_extension_budget_divisor: i32,
    /// Restricts Check Extensions to nodes at or above this remaining depth. The
    /// counterpart to `check_extension_max_depth`: near the horizon the Quiescence Search
    /// already resolves checks, so an extension there is close to pure cost. `0` disables
    /// the restriction.
    pub check_extension_min_depth: i32,
    /// Restricts Check Extensions to nodes at or below this remaining depth. An extension
    /// granted high in the tree multiplies an entire subtree, while the horizon effect it
    /// exists to cure is a frontier phenomenon. `0` disables the restriction.
    pub check_extension_max_depth: i32,
    /// Enables One-Reply Extensions: a node with exactly one legal move is searched one
    /// ply deeper. Such a node has no branching, so the extra ply is nearly free, and it
    /// keeps forced sequences — including sacrificial checks — inside the horizon.
    pub enable_one_reply_extension: bool,
    /// Stage 0 of the `MovePicker` in `task.md` 1.2.2: search the PV or Transposition Table move
    /// before generating anything, so that a cutoff on it costs no generation at all.
    ///
    /// Mutually exclusive with `enable_one_reply_extension`, which needs the length of the full
    /// move list and therefore cannot be decided without generating it.
    pub enable_tt_move_first: bool,
    /// Whether Stage 0 snapshots the History Heuristic at node entry and ranks the deferred move
    /// list against it. This is what makes the short-circuit leave the search tree bit-identical:
    /// without it the first searched move's subtree has already moved the table on, the quiet
    /// move order changes, and the tree diverges by up to a factor of four.
    pub stage0_history_snapshot: bool,
    pub log_path: std::sync::Arc<str>,
}


impl Config {
    pub fn new_raw() -> Config {
        Config {
            version: env!("CARGO_PKG_VERSION"),
            use_zobrist: true,
            use_nnue: false,
            nnue_model_path: "eval_models/quantised.bin".to_string(),
            use_book: true,
            cache_book_in_ram: true,
            book_file: String::new(),
            max_zobrist_hash_entries: 50_000_000, // 800 MB
            max_pawn_hash_entries: 1_000_000, // 16 MB: Proven +30 Elo sweet spot (avoids CPU L3 & TLB thrashing)
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
            max_eval_mult: 2.0,
            aggressiveness: Aggressiveness::Normal,
            enable_positional_cap: true,
            positional_cap_damping: 5,
            move_overhead: 0,

            is_hashed_rank_bonus: 4,
            give_check_rank_bonus: 5,
            is_pv_node_rank_bonus: 18,
            give_promotion_rank_bonus_queen: 17,
            give_promotion_rank_bonus_knight: 6,

            your_turn_bonus: 19,

            undeveloped_knight_malus: 31,
            undeveloped_bishop_malus: 31,
            undeveloped_king_malus: 54,


            rook_open_file: 27,
            rook_half_open_file: 21,
            bishop_pair_bonus: 48,
            rook_doubled_bonus: 25,
            rook_behind_passed_pawn_middlegame: 13,
            rook_behind_passed_pawn_endgame: 38,
            king_ring_attack_knight: 1,
            king_ring_attack_bishop: 1,
            king_ring_attack_rook: 2,
            king_ring_attack_queen: 4,
            protected_passed_pawn_middlegame: 12,
            protected_passed_pawn_endgame: 27,
            king_opposition_bonus: 12,
            king_open_file_malus: 37,
            king_half_open_file_malus: 20,
            king_ring_defender_value: 1,

            pawn_structure: 5,
            pawn_supports_knight_outpost: 10,
            pawn_centered: 13,
            pawn_undeveloped_malus: 15,
            pawn_on_last_rank_bonus: 183,
            pawn_on_before_last_rank_bonus: 104,
            pawn_on_before_before_last_rank_bonus: 47,
            pawn_defends_bishop: 23,
            pawn_double_malus: 14,

            knight_on_rim_malus: 17,
            knight_centered: 24,
            knight_blockes_pawn: 28,
            bishop_trapped_at_rim_malus: 58,

            pawn_attacks_opponent_fig: 35,
            pawn_attacks_opponent_fig_with_tempo: 18,
            queen_in_attack: 53,
            queen_in_attack_with_tempo: 29,
            knight_attacks_bishop: 5,
            knight_attacks_rook: 16,
            knight_attacks_bishop_tempo: 9,
            knight_attacks_rook_tempo: 13,
            threat_minor_attacks_rook: 13,
            threat_minor_attacks_queen: 24,
            threat_rook_attacks_queen: 20,


            king_pawn_shield: 37,
            king_piece_shield: 17,
            king_pawn_shield_kingside: 37,
            king_pawn_shield_queenside: 25,
            king_piece_shield_kingside: 15,
            king_piece_shield_queenside: 10,
            connected_passed_pawn_mg: 15,
            connected_passed_pawn_eg: 29,
            knight_outpost_true_mg: 29,
            knight_outpost_true_eg: 15,
            bishop_outpost_true_mg: 21,
            bishop_outpost_true_eg: 11,
            opposite_bishops_draw_scale: 51,
            enable_endgame_mopup: true,
            mopup_center_weight: 10,
            mopup_proximity_weight: 15,
            mopup_eval_threshold: 400,
            mopup_max_game_phase: 60,
            rook_behind_enemy_passed_pawn_mg: 10,
            rook_behind_enemy_passed_pawn_eg: 24,
            king_trapp_at_baseline_malus: 71,
            king_in_check_malus: 135,
            king_in_double_check_malus: 337,

            rook_on_seventh: 33,

            lazy_eval_margin_search: 180,
            lazy_eval_margin_qs: 120,
            enable_lazy_eval: true,
            lazy_eval_min_game_phase: 60,
            king_danger_weight_1: 10,
            king_danger_weight_2: 50,
            king_danger_weight_3: 100,
            king_danger_weight_4: 150,
            king_danger_weight_5: 200,
            pawn_isolated_malus: 9,
            pawn_backward_malus: 11,
            knight_mobility_factor: 3,
            bishop_mobility_factor: 3,
            rook_mobility_factor: 2,
            queen_mobility_factor: 1,
            king_passer_dist_weight: 12,
            king_open_file_heavy_threat_malus: 15,
            rook_open_file_attacks_king: 15,
            rook_open_file_attacks_queen: 10,
            pawn_phalanx_mg: 8,
            pawn_phalanx_eg: 4,
            bishop_diagonal_attacks_king: 15,
            bishop_diagonal_attacks_queen: 10,
            rook_on_seventh_king_cutoff: 20,
            rooks_doubled_on_seventh: 25,
            passed_pawn_blockaded_malus: 15,
            candidate_passed_pawn_bonus: 8,
            pawn_storm_bonus: 6,
            pre_sort_moves: true,
            use_underpromotions: false,
            enable_pvs: true,
            enable_lmr: true,
            enable_nmp: true,
            enable_aspiration: true,
            enable_rfp: true,
            enable_futility_pruning: true,
            enable_qs_tt: true,
            futility_max_depth: 4,
            futility_margin_base: 120,
            futility_margin_slope: 80,
            enable_delta_pruning: false,
            delta_pruning_margin: 300,
            enable_counter_moves: true,
            enable_history_malus: false,
            killer_move_1_rank_bonus: 20000,
            killer_move_2_rank_bonus: 10000,
            counter_move_rank_bonus: 15000,
            history_max_threshold: 9000,
            lmr_move_threshold: 3,
            lmr_divisor: 185,

            lmr_table: {
                let mut table = [[0i16; 64]; 64];
                let divisor = 185.0 / 100.0;
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
            aspiration_window_initial_delta: 15,
            aspiration_window_multiplier: 4,
            aspiration_window_max_delta: 1000,
            lmr_history_good_threshold: 4000,
            lmr_history_bad_threshold: 500,
            rfp_margin_per_depth: 80,
            rfp_max_depth: 3,
            enable_check_extension: true,
            check_extension_max_ply: 64,
            check_extension_require_safe: false,
            check_extension_budget_divisor: 0,
            check_extension_min_depth: 0,
            check_extension_max_depth: 0,
            enable_one_reply_extension: false,
            enable_tt_move_first: true,
            stage0_history_snapshot: true,
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

    pub fn new() -> Config {
        Self::new_raw()
    }

    pub fn set_aggressiveness(&mut self, aggressiveness: Aggressiveness) {
        let raw = Self::new_raw();
        self.aggressiveness = aggressiveness;
        match aggressiveness {
            Aggressiveness::Normal => {
                self.king_ring_attack_knight = raw.king_ring_attack_knight;
                self.king_ring_attack_bishop = raw.king_ring_attack_bishop;
                self.king_ring_attack_rook = raw.king_ring_attack_rook;
                self.king_ring_attack_queen = raw.king_ring_attack_queen;
                self.queen_in_attack = raw.queen_in_attack;
                self.queen_in_attack_with_tempo = raw.queen_in_attack_with_tempo;
                self.knight_mobility_factor = raw.knight_mobility_factor;
                self.bishop_mobility_factor = raw.bishop_mobility_factor;
                self.rook_mobility_factor = raw.rook_mobility_factor;
                self.queen_mobility_factor = raw.queen_mobility_factor;
            }
            Aggressiveness::Aggressive => {
                self.king_ring_attack_knight = (raw.king_ring_attack_knight * 15) / 10;
                self.king_ring_attack_bishop = (raw.king_ring_attack_bishop * 15) / 10;
                self.king_ring_attack_rook = (raw.king_ring_attack_rook * 15) / 10;
                self.king_ring_attack_queen = (raw.king_ring_attack_queen * 15) / 10;
                self.queen_in_attack = (raw.queen_in_attack * 13) / 10;
                self.queen_in_attack_with_tempo = (raw.queen_in_attack_with_tempo * 13) / 10;
                self.knight_mobility_factor = raw.knight_mobility_factor;
                self.bishop_mobility_factor = raw.bishop_mobility_factor;
                self.rook_mobility_factor = raw.rook_mobility_factor;
                self.queen_mobility_factor = raw.queen_mobility_factor;
            }
            Aggressiveness::HighAggressive => {
                self.king_ring_attack_knight = raw.king_ring_attack_knight * 2;
                self.king_ring_attack_bishop = raw.king_ring_attack_bishop * 2;
                self.king_ring_attack_rook = raw.king_ring_attack_rook * 2;
                self.king_ring_attack_queen = raw.king_ring_attack_queen * 2;
                self.queen_in_attack = (raw.queen_in_attack * 16) / 10;
                self.queen_in_attack_with_tempo = (raw.queen_in_attack_with_tempo * 16) / 10;
                self.knight_mobility_factor = raw.knight_mobility_factor;
                self.bishop_mobility_factor = raw.bishop_mobility_factor;
                self.rook_mobility_factor = raw.rook_mobility_factor;
                self.queen_mobility_factor = raw.queen_mobility_factor;
            }
        }
    }

    /// Sets turn_bonus and all tempo attack boni at 0
    pub fn _for_evel_equal_tests() -> Self {
        let mut config = Config::new();
        config.use_nnue = false;
        config.enable_endgame_mopup = false;
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
        config.king_open_file_heavy_threat_malus = 0;
        config.rook_open_file_attacks_king = 0;
        config.rook_open_file_attacks_queen = 0;
        config.pawn_phalanx_mg = 0;
        config.pawn_phalanx_eg = 0;
        config.bishop_diagonal_attacks_king = 0;
        config.bishop_diagonal_attacks_queen = 0;
        config.rook_on_seventh_king_cutoff = 0;
        config.rooks_doubled_on_seventh = 0;
        config.passed_pawn_blockaded_malus = 0;
        config.candidate_passed_pawn_bonus = 0;
        config.pawn_storm_bonus = 0;
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
        config.use_nnue = false;
        config.enable_endgame_mopup = false;
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
        config.max_pawn_hash_entries = 1_000_000;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_aggressiveness_scaling() {
        let base_config = Config::new_raw();
        
        let mut normal_config = base_config.clone();
        normal_config.set_aggressiveness(Aggressiveness::Normal);
        assert_eq!(normal_config.king_ring_attack_knight, base_config.king_ring_attack_knight);
        assert_eq!(normal_config.queen_in_attack, base_config.queen_in_attack);
        assert_eq!(normal_config.knight_mobility_factor, base_config.knight_mobility_factor);
        assert_eq!(normal_config.queen_mobility_factor, base_config.queen_mobility_factor);

        let mut aggressive_config = base_config.clone();
        aggressive_config.set_aggressiveness(Aggressiveness::Aggressive);
        assert_eq!(aggressive_config.king_ring_attack_knight, (base_config.king_ring_attack_knight * 15) / 10);
        assert_eq!(aggressive_config.queen_in_attack, (base_config.queen_in_attack * 13) / 10);
        assert_eq!(aggressive_config.knight_mobility_factor, base_config.knight_mobility_factor);
        assert_eq!(aggressive_config.queen_mobility_factor, base_config.queen_mobility_factor);

        let mut high_aggressive_config = base_config.clone();
        high_aggressive_config.set_aggressiveness(Aggressiveness::HighAggressive);
        assert_eq!(high_aggressive_config.king_ring_attack_knight, base_config.king_ring_attack_knight * 2);
        assert_eq!(high_aggressive_config.queen_in_attack, (base_config.queen_in_attack * 16) / 10);
        assert_eq!(high_aggressive_config.knight_mobility_factor, base_config.knight_mobility_factor);
        assert_eq!(high_aggressive_config.queen_mobility_factor, base_config.queen_mobility_factor);
    }

    #[test]
    fn test_config_default_initialization() {
        let config = Config::new();
        assert_eq!(config.aggressiveness, Aggressiveness::Normal);
        assert_eq!(config.max_pawn_hash_entries, 1_000_000);
        assert_eq!(config.max_zobrist_hash_entries, 50_000_000);
        assert!(!config.use_nnue);
        assert_eq!(config.lmr_divisor, 185);
        assert_eq!(config.lmr_move_threshold, 3);
        assert_eq!(config.aspiration_window_initial_delta, 15);
        assert_eq!(config.aspiration_window_multiplier, 4);
        assert_eq!(config.aspiration_window_max_delta, 1000);
        assert_eq!(config.rfp_margin_per_depth, 80);
        assert_eq!(config.rfp_max_depth, 3);
        assert_eq!(config.lmr_history_good_threshold, 4000);
        assert_eq!(config.lmr_history_bad_threshold, 500);
        assert!(config.enable_check_extension);
        assert_eq!(config.check_extension_max_ply, 64);
        assert!(!config.check_extension_require_safe);
        assert_eq!(config.check_extension_budget_divisor, 0);
        assert_eq!(config.check_extension_min_depth, 0);
        assert_eq!(config.check_extension_max_depth, 0);
        assert!(!config.enable_one_reply_extension);
    }
}