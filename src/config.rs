use crate::model::QuiescenceSearchMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggressiveness {
    Normal,
    Aggressive,
    HighAggressive,
}

#[derive(Clone, PartialEq)]
pub struct Config {
    pub version: &'static str,
    pub use_zobrist: bool,
    pub use_nnue: bool,
    pub nnue_model_path: String,
    pub use_book: bool,
    pub cache_book_in_ram: bool,
    pub book_file: String,
    /// Half-moves after which the opening book stops being consulted. `0` means unlimited.
    ///
    /// The embedded book reaches well into the middlegame on main lines. A match that is meant
    /// to price a search change has to leave the book early enough that the search decides the
    /// game, and a fixed cut-off also makes every game in a match start from a comparable amount
    /// of book guidance.
    pub book_max_ply: i32,
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
    /// Late Move Pruning: skip quiet moves appearing late in the move list at low depth.
    pub enable_lmp: bool,
    pub lmp_max_depth: i32,
    pub lmp_base_moves: i32,
    /// SEE pruning of bad captures: skip captures losing more than
    /// `bad_capture_see_threshold * depth` centipawns by Static Exchange Evaluation.
    pub enable_bad_capture_pruning: bool,
    pub bad_capture_see_threshold: i16,
    /// Razoring at depth 1: when the static evaluation trails the window by more than
    /// `razoring_margin`, verify with a Quiescence Search instead of searching the node, and
    /// return that score if it confirms the fail-low.
    pub enable_razoring: bool,
    pub razoring_margin: i16,
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
    ///
    /// Ships **disabled** since v0.34.0. The extension works — it resolves Philidor's
    /// Legacy a ply earlier and solves more LCT II positions at fixed depth — but the
    /// ply it spends costs more than it returns, because it spends it at the node class
    /// where Null Move, Reverse Futility and Futility Pruning are all disabled and LMR
    /// never reduces. Measured over 1000 games per pairing at 1000ms+100ms: disabling it
    /// is worth +23.7 Elo, 95% CI [+8, +40]. Restricting it to deep nodes
    /// (`check_extension_min_depth`) does not recover the loss (-9.7 Elo, CI [-26, +6]).
    /// See task.md 2.2.6.
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

    /// Enables Singular Extensions: when the Transposition Table move is provably better than
    /// every alternative at this node, it is searched one ply deeper.
    ///
    /// Ships **enabled** since v0.37.0 on roughly **+5 to +10 Elo over 2591 games** against
    /// v0.36.0 — small and positive, not the +30.6 the first stopped SPRT reported. Two
    /// independent round robins both accepted the *does it hurt* hypothesis. `task.md` 4.2, and
    /// 4.3 for why a stopped SPRT's point estimate is not the effect size. The Check Extension of 8.2 is the cautionary case that made
    /// the games mandatory: it looked right on every static metric and measured -23.7 Elo.
    pub enable_singular_extensions: bool,
    /// Lowest remaining depth at which singularity is verified.
    ///
    /// The published rule triggers at depth 8. This engine reaches a root depth of 9 to 10 at the
    /// match time control, where a trigger of 8 fires at plies 0 to 1 and nowhere else, which is
    /// too rare to price. The threshold is therefore a parameter rather than a constant, and its
    /// default is chosen against this harness: a fixed-depth census settled on **6**, where the
    /// rule grants three times the extensions of 7 for the same tree cost. `task.md` 4.1.
    pub singular_min_depth: i32,
    /// How much shallower than the current node the Transposition Table entry may be and still
    /// be trusted as the singularity candidate: the entry qualifies at
    /// `entry.depth >= depth - singular_tt_depth_margin`.
    pub singular_tt_depth_margin: i32,
    /// Centipawns per ply by which every alternative must fall short of the Transposition Table
    /// score before the move counts as singular. The threshold is `singular_margin * depth`, so
    /// the demand grows with the depth the score was established at.
    pub singular_margin: i16,
    /// Subtracted from the `(depth - 1) / 2` verification depth. `0` is the published reduction;
    /// larger values buy a cheaper, blunter verification.
    pub singular_depth_reduction: i32,
    /// The singular-beta multicut: the second reading of the verification search.
    ///
    /// The search asks whether any move other than the Transposition Table move reaches
    /// `tt_eval - singular_margin * depth`. A fail low means singular and extends. A fail *high*
    /// says some other move reaches the threshold too, and when the threshold is itself at or
    /// above `beta` that is a reduced-depth demonstration that this node beats `beta` without the
    /// table move being searched at all — so the node can be cut.
    ///
    /// This is the rebate the extension has never had. Singular Extensions ship enabled and
    /// measure -1.4 Elo (`task.md` 10.10) because the implementation extends and never collects,
    /// at +51.2% tree by depth 11 (4.4). Off by default until a fixed-N run prices it.
    pub enable_singular_multicut: bool,
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
            book_max_ply: 0,
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
            enable_lmp: true,
            lmp_max_depth: 4,
            lmp_base_moves: 3,
            enable_bad_capture_pruning: true,
            bad_capture_see_threshold: -50,
            // On by default since the round-robin gauntlet against suprah-0.35.2 on host C:
            // 1034 games, 517 pairs, SPRT H1 accepted (elo0=-10, elo1=0), LLR +2.991 against a
            // +2.944 bound, razoring ahead by 14.1 Elo, paired 95% CI [-1, +30]. See `task.md`
            // section 3.3.
            enable_razoring: true,
            razoring_margin: 300,
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
            enable_check_extension: false,
            check_extension_max_ply: 64,
            check_extension_require_safe: false,
            check_extension_budget_divisor: 0,
            check_extension_min_depth: 0,
            check_extension_max_depth: 0,
            enable_one_reply_extension: false,
            // On by default since v0.37.0: about +5 to +10 Elo over 2591 games against v0.36.0.
            // The trigger
            // depth is deliberately below the published 8 because the match search reaches depth
            // 9 to 10 — see `task.md` 4.1 for the census that chose it.
            enable_singular_extensions: true,
            singular_min_depth: 6,
            singular_tt_depth_margin: 3,
            singular_margin: 2,
            singular_depth_reduction: 0,
            enable_singular_multicut: false,
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
        config
    }

    // like integration test with 1 thread
    pub fn _for_integration_tests_wo_pv_nodes(&self) -> Self {
        let mut config = Config::_for_integration_tests(self);
        config.use_pv_nodes = false;
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every option the engine advertises must actually reach a field.
    ///
    /// `setoption` matches the option name with separators removed, and before v0.37.2 the arms
    /// were written in snake_case while the name arrived as one CamelCase token. Thirteen of the
    /// advertised options had no lowercase alias and were accepted and silently ignored - the
    /// engine told a GUI it supported them and then did nothing. Three options are legitimately
    /// not config-backed and are listed here by name rather than skipped by a pattern, so adding
    /// a fourth has to be a deliberate edit.
    #[test]
    fn test_every_advertised_uci_option_is_accepted() {
        // `Hash` resizes the transposition table and `Threads` is answered with a log line: both
        // are handled in the UCI thread before the configuration is consulted. `Aggressiveness`
        // takes a word rather than a number and is exercised separately below.
        const HANDLED_ELSEWHERE: [&str; 3] = ["Hash", "Threads", "Aggressiveness"];

        let defaults = Config::new();
        let mut checked = 0;
        for line in crate::threads::uci_options(&defaults) {
            let name = line
                .strip_prefix("option name ")
                .and_then(|rest| rest.split(" type ").next())
                .expect("every advertised option names itself");
            if HANDLED_ELSEWHERE.contains(&name) {
                continue;
            }
            let mut config = Config::new();
            assert_ne!(
                config.apply_uci_option(name, "1"),
                UciOptionEffect::Unknown,
                "advertised option '{}' is not accepted by setoption", name);
            checked += 1;
        }
        assert!(checked > 50, "expected the advertised option list to be non-trivial, got {}", checked);
    }

    /// Accepting the name is not enough: the value has to land in the configuration.
    ///
    /// Every advertised `spin` option is set to a value different from its own advertised default
    /// and the resulting configuration must differ from an untouched one. A silently ignored
    /// option passes the acceptance test above only if its arm exists at all, but this one fails
    /// for any arm that parses the value and then drops it.
    #[test]
    fn test_every_advertised_spin_option_changes_the_configuration() {
        const HANDLED_ELSEWHERE: [&str; 2] = ["Hash", "Threads"];

        let defaults = Config::new();
        let untouched = Config::new();
        for line in crate::threads::uci_options(&defaults) {
            let Some(rest) = line.strip_prefix("option name ") else { continue };
            let Some((name, spec)) = rest.split_once(" type ") else { continue };
            if !spec.starts_with("spin ") || HANDLED_ELSEWHERE.contains(&name) {
                continue;
            }
            let advertised: i64 = spec
                .split_whitespace()
                .nth(2)
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| panic!("spin option '{}' has no numeric default: {}", name, line));
            let max: i64 = spec
                .split_whitespace()
                .last()
                .and_then(|v| v.parse().ok())
                .unwrap_or(advertised + 1);
            // Stay inside the advertised range, so the test never asks for a value a GUI could not.
            let probe = if advertised < max { advertised + 1 } else { advertised - 1 };

            let mut config = Config::new();
            config.apply_uci_option(name, &probe.to_string());
            assert!(
                config != untouched,
                "advertised option '{}' accepted value {} without changing anything", name, probe);
        }
    }

    /// The name is matched case-insensitively and without separators, so the spelling the engine
    /// advertises, the snake_case spelling the SPSA tuner sends, and a spaced spelling are one
    /// option.
    #[test]
    fn test_uci_option_names_ignore_case_and_separators() {
        for spelling in ["ConnectedPassedPawnMg", "connected_passed_pawn_mg", "Connected Passed Pawn Mg"] {
            let mut config = Config::new();
            assert_eq!(config.apply_uci_option(spelling, "77"), UciOptionEffect::Stored,
                       "'{}' must be accepted", spelling);
            assert_eq!(config.connected_passed_pawn_mg, 77, "'{}' must reach the field", spelling);
        }
        let mut config = Config::new();
        assert_eq!(config.apply_uci_option("NoSuchOption", "1"), UciOptionEffect::Unknown);
        assert!(config == Config::new(), "an unknown option must change nothing");
    }

    /// The three options that invalidate a loaded opening book report that to the caller instead
    /// of reaching for a book `Config` does not own.
    #[test]
    fn test_book_options_report_their_effect() {
        let mut config = Config::new();
        assert_eq!(config.apply_uci_option("BookFile", "book.bin"), UciOptionEffect::BookFileChanged);
        assert_eq!(config.book_file, "book.bin");
        assert_eq!(config.apply_uci_option("OwnBook", "true"), UciOptionEffect::BookEnabledChanged);
        assert!(config.use_book);
        assert_eq!(config.apply_uci_option("CacheBookInRam", "false"), UciOptionEffect::BookCacheChanged);
        assert!(!config.cache_book_in_ram);
    }

    /// `lmp_max_depth` is inert above 4 (`task.md` 10.6), so neither the UCI facade nor the SPSA
    /// parameter file may offer a wider range for a tuner to wander over.
    #[test]
    fn test_lmp_max_depth_advertises_only_its_live_range() {
        let defaults = Config::new();
        let line = crate::threads::uci_options(&defaults)
            .into_iter()
            .find(|l| l.starts_with("option name LmpMaxDepth "))
            .expect("LmpMaxDepth must be advertised");
        assert!(line.ends_with(" max 4"), "LmpMaxDepth must advertise max 4, got: {}", line);

        let registered = std::fs::read_to_string("tuning/parameters.json").expect("parameters.json");
        let entry = registered
            .split("\"lmp_max_depth\"")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .expect("lmp_max_depth must be registered for tuning");
        assert!(entry.contains("\"max\": 4"),
                "tuning/parameters.json must not offer lmp_max_depth above 4, got: {}", entry);
    }

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
        assert!(!config.enable_check_extension);
        assert_eq!(config.check_extension_max_ply, 64);
        assert!(!config.check_extension_require_safe);
        assert_eq!(config.check_extension_budget_divisor, 0);
        assert_eq!(config.check_extension_min_depth, 0);
        assert_eq!(config.check_extension_max_depth, 0);
        assert!(!config.enable_one_reply_extension);
    }
}
/// What the caller must do after a `setoption` beyond storing the value.
///
/// `Config` owns the option table but not the opening book, so the three options that invalidate
/// a loaded book report that back instead of reaching for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UciOptionEffect {
    /// The name was not recognised and nothing was changed.
    Unknown,
    /// The value was stored and nothing else is required.
    Stored,
    /// `BookFile` changed: drop the cached book and load the new one eagerly.
    BookFileChanged,
    /// `OwnBook` changed: the book may have been named before it was switched on.
    BookEnabledChanged,
    /// `CacheBookInRam` changed: drop the cache when it is now off.
    BookCacheChanged,
}

impl Config {
    /// Applies one UCI `setoption` to this configuration and reports what it did.
    ///
    /// The name is matched case-insensitively and with separators removed, so
    /// `ConnectedPassedPawnMg`, `connected_passed_pawn_mg` and `Connected Passed Pawn Mg` are one
    /// option. Matching the raw lowercased name used to require every arm to carry a hand-written
    /// alias for the CamelCase spelling, and thirteen of the sixty-one advertised options had
    /// none: the engine advertised them, accepted them, and silently ignored the value.
    pub fn apply_uci_option(&mut self, name: &str, value: &str) -> UciOptionEffect {
        let key: String = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        let value = value.trim().to_string();
        match key.as_str() {
            "aggressiveness" => {
                if value.to_lowercase().contains("high") {
                    self.set_aggressiveness(crate::config::Aggressiveness::HighAggressive);
                } else if value.to_lowercase().contains("aggressive") {
                    self.set_aggressiveness(crate::config::Aggressiveness::Aggressive);
                } else {
                    self.set_aggressiveness(crate::config::Aggressiveness::Normal);
                }
            }
            "enablelazyeval" => {
                self.enable_lazy_eval = value.to_lowercase() == "true";
            }
            "usennue" => {
                self.use_nnue = value.to_lowercase() == "true";
            }
            "nnuemodelpath" => {
                self.nnue_model_path = value.clone();
            }
            "enablepositionalcap" => {
                self.enable_positional_cap = value.to_lowercase() == "true";
            }
            "moveoverhead" => {
                if let Ok(overhead) = value.parse::<u64>() { self.move_overhead = overhead; }
            }
            "nmpdepththreshold" => if let Ok(v) = value.parse::<i32>() { self.nmp_depth_threshold = v; },
            "nmpreduction" => if let Ok(v) = value.parse::<i32>() { self.nmp_reduction = v; },
            "nmpverificationthreshold" => if let Ok(v) = value.parse::<i32>() { self.nmp_verification_threshold = v; },
            "nmpdynamicdivisor" => if let Ok(v) = value.parse::<i32>() { self.nmp_dynamic_divisor = v; },
            "lmrmovethreshold" => if let Ok(v) = value.parse::<i32>() { self.lmr_move_threshold = v; },
            "lmrdivisor" | "lmrdivisorscaled" => if let Ok(v) = value.parse::<i32>() { self.lmr_divisor = v; self.recalculate_lmr_table(); },
            "killermove1rankbonus" => if let Ok(v) = value.parse::<i32>() { self.killer_move_1_rank_bonus = v; },
            "killermove2rankbonus" => if let Ok(v) = value.parse::<i32>() { self.killer_move_2_rank_bonus = v; },
            "countermoverankbonus" => if let Ok(v) = value.parse::<i32>() { self.counter_move_rank_bonus = v; },
            "ishashedrankbonus" => if let Ok(v) = value.parse::<i32>() { self.is_hashed_rank_bonus = v; },
            "givecheckrankbonus" => if let Ok(v) = value.parse::<i32>() { self.give_check_rank_bonus = v; },
            "ispvnoderankbonus" => if let Ok(v) = value.parse::<i32>() { self.is_pv_node_rank_bonus = v; },
            "givepromotionrankbonusqueen" => if let Ok(v) = value.parse::<i32>() { self.give_promotion_rank_bonus_queen = v; },
            "givepromotionrankbonusknight" => if let Ok(v) = value.parse::<i32>() { self.give_promotion_rank_bonus_knight = v; },
            "historymaxthreshold" => if let Ok(v) = value.parse::<u32>() { self.history_max_threshold = v; },
            "aspirationwindowinitialdelta" => if let Ok(v) = value.parse::<i16>() { self.aspiration_window_initial_delta = v; },
            "aspirationwindowmultiplier" => if let Ok(v) = value.parse::<i16>() { self.aspiration_window_multiplier = v; },
            "aspirationwindowmaxdelta" => if let Ok(v) = value.parse::<i16>() { self.aspiration_window_max_delta = v; },
            "lmrhistorygoodthreshold" => if let Ok(v) = value.parse::<u32>() { self.lmr_history_good_threshold = v; },
            "lmrhistorybadthreshold" => if let Ok(v) = value.parse::<u32>() { self.lmr_history_bad_threshold = v; },
            "rfpmarginperdepth" => if let Ok(v) = value.parse::<i16>() { self.rfp_margin_per_depth = v; },
            "rfpmaxdepth" => if let Ok(v) = value.parse::<i32>() { self.rfp_max_depth = v; },
            "enablecheckextension" => self.enable_check_extension = value.eq_ignore_ascii_case("true"),
            "checkextensionmaxply" => if let Ok(v) = value.parse::<i32>() { self.check_extension_max_ply = v; },
            "checkextensionrequiresafe" => self.check_extension_require_safe = value.eq_ignore_ascii_case("true"),
            "checkextensionbudgetdivisor" => if let Ok(v) = value.parse::<i32>() { self.check_extension_budget_divisor = v; },
            "checkextensionmindepth" => if let Ok(v) = value.parse::<i32>() { self.check_extension_min_depth = v; },
            "checkextensionmaxdepth" => if let Ok(v) = value.parse::<i32>() { self.check_extension_max_depth = v; },
            "enableonereplyextension" => self.enable_one_reply_extension = value.eq_ignore_ascii_case("true"),
            "enablesingularextensions" => self.enable_singular_extensions = value.eq_ignore_ascii_case("true"),
            "singularmindepth" => if let Ok(v) = value.parse::<i32>() { self.singular_min_depth = v; },
            "singularttdepthmargin" => if let Ok(v) = value.parse::<i32>() { self.singular_tt_depth_margin = v; },
            "singularmargin" => if let Ok(v) = value.parse::<i16>() { self.singular_margin = v; },
            "singulardepthreduction" => if let Ok(v) = value.parse::<i32>() { self.singular_depth_reduction = v; },
            "enablesingularmulticut" => self.enable_singular_multicut = value.eq_ignore_ascii_case("true"),
            "enablelmp" => self.enable_lmp = value.eq_ignore_ascii_case("true"),
            "lmpmaxdepth" => if let Ok(v) = value.parse::<i32>() { self.lmp_max_depth = v; },
            "lmpbasemoves" => if let Ok(v) = value.parse::<i32>() { self.lmp_base_moves = v; },
            "enablebadcapturepruning" => self.enable_bad_capture_pruning = value.eq_ignore_ascii_case("true"),
            "badcaptureseethreshold" => if let Ok(v) = value.parse::<i16>() { self.bad_capture_see_threshold = v; },
            "yourturnbonus" => if let Ok(v) = value.parse::<i16>() { self.your_turn_bonus = v; },
            "positionalcapdamping" => {
                if let Ok(v) = value.parse::<i16>() { self.positional_cap_damping = v; }
            },
            "kingopenfilemalus" => if let Ok(v) = value.parse::<i16>() { self.king_open_file_malus = v; },
            "kinghalfopenfilemalus" => if let Ok(v) = value.parse::<i16>() { self.king_half_open_file_malus = v; },
            "kingringdefendervalue" => if let Ok(v) = value.parse::<i16>() { self.king_ring_defender_value = v; },
            "threatminorattacksrook" => if let Ok(v) = value.parse::<i16>() { self.threat_minor_attacks_rook = v; },
            "threatminorattacksqueen" => if let Ok(v) = value.parse::<i16>() { self.threat_minor_attacks_queen = v; },
            "threatrookattacksqueen" => if let Ok(v) = value.parse::<i16>() { self.threat_rook_attacks_queen = v; },
            "logpath" => { self.log_path = std::sync::Arc::from(value.as_str()); },
            "bookfile" => {
                self.book_file = value.to_string();
                return UciOptionEffect::BookFileChanged;
            },
            "bookmaxply" => if let Ok(v) = value.parse::<i32>() { self.book_max_ply = v; },
            "cachebookinram" => {
                self.cache_book_in_ram = value.to_lowercase() == "true";
                return UciOptionEffect::BookCacheChanged;
            },
            "ownbook" | "usebook" => {
                self.use_book = value.to_lowercase() == "true";
                return UciOptionEffect::BookEnabledChanged;
            },
            "pawnstructure" => if let Ok(v) = value.parse::<i16>() { self.pawn_structure = v; },
            "pawnsupportsknightoutpost" => if let Ok(v) = value.parse::<i16>() { self.pawn_supports_knight_outpost = v; },
            "pawncentered" => if let Ok(v) = value.parse::<i16>() { self.pawn_centered = v; },
            "pawnundevelopedmalus" => if let Ok(v) = value.parse::<i16>() { self.pawn_undeveloped_malus = v; },
            "pawnonlastrankbonus" => if let Ok(v) = value.parse::<i16>() { self.pawn_on_last_rank_bonus = v; },
            "pawnonbeforelastrankbonus" => if let Ok(v) = value.parse::<i16>() { self.pawn_on_before_last_rank_bonus = v; },
            "pawnonbeforebeforelastrankbonus" => if let Ok(v) = value.parse::<i16>() { self.pawn_on_before_before_last_rank_bonus = v; },
            "pawndefendsbishop" => if let Ok(v) = value.parse::<i16>() { self.pawn_defends_bishop = v; },
            "pawndoublemalus" => if let Ok(v) = value.parse::<i16>() { self.pawn_double_malus = v; },
            "pawnisolatedmalus" => if let Ok(v) = value.parse::<i16>() { self.pawn_isolated_malus = v; },
            "pawnbackwardmalus" => if let Ok(v) = value.parse::<i16>() { self.pawn_backward_malus = v; },
            "protectedpassedpawnmiddlegame" => if let Ok(v) = value.parse::<i16>() { self.protected_passed_pawn_middlegame = v; },
            "protectedpassedpawnendgame" => if let Ok(v) = value.parse::<i16>() { self.protected_passed_pawn_endgame = v; },
            "undevelopedknightmalus" => if let Ok(v) = value.parse::<i16>() { self.undeveloped_knight_malus = v; },
            "knightonrimmalus" => if let Ok(v) = value.parse::<i16>() { self.knight_on_rim_malus = v; },
            "knightcentered" => if let Ok(v) = value.parse::<i16>() { self.knight_centered = v; },
            "knightblockespawn" => if let Ok(v) = value.parse::<i16>() { self.knight_blockes_pawn = v; },
            "knightmobilityfactor" => if let Ok(v) = value.parse::<i16>() { self.knight_mobility_factor = v; },
            "undevelopedbishopmalus" => if let Ok(v) = value.parse::<i16>() { self.undeveloped_bishop_malus = v; },
            "bishoppairbonus" => if let Ok(v) = value.parse::<i16>() { self.bishop_pair_bonus = v; },
            "bishoptrappedatrimmalus" => if let Ok(v) = value.parse::<i16>() { self.bishop_trapped_at_rim_malus = v; },
            "bishopmobilityfactor" => if let Ok(v) = value.parse::<i16>() { self.bishop_mobility_factor = v; },
            "rookopenfile" => if let Ok(v) = value.parse::<i16>() { self.rook_open_file = v; },
            "rookhalfopenfile" => if let Ok(v) = value.parse::<i16>() { self.rook_half_open_file = v; },
            "rookdoubledbonus" => if let Ok(v) = value.parse::<i16>() { self.rook_doubled_bonus = v; },
            "rookbehindpassedpawnmiddlegame" => if let Ok(v) = value.parse::<i16>() { self.rook_behind_passed_pawn_middlegame = v; },
            "rookbehindpassedpawnendgame" => if let Ok(v) = value.parse::<i16>() { self.rook_behind_passed_pawn_endgame = v; },
            "rookonseventh" => if let Ok(v) = value.parse::<i16>() { self.rook_on_seventh = v; },
            "rookmobilityfactor" => if let Ok(v) = value.parse::<i16>() { self.rook_mobility_factor = v; },
            "queenmobilityfactor" => if let Ok(v) = value.parse::<i16>() { self.queen_mobility_factor = v; },
            "kingpasserdistweight" => if let Ok(v) = value.parse::<i16>() { self.king_passer_dist_weight = v; },
            "undevelopedkingmalus" => if let Ok(v) = value.parse::<i16>() { self.undeveloped_king_malus = v; },
            "kingringattackknight" => if let Ok(v) = value.parse::<i16>() { self.king_ring_attack_knight = v; },
            "kingringattackbishop" => if let Ok(v) = value.parse::<i16>() { self.king_ring_attack_bishop = v; },
            "kingringattackrook" => if let Ok(v) = value.parse::<i16>() { self.king_ring_attack_rook = v; },
            "kingringattackqueen" => if let Ok(v) = value.parse::<i16>() { self.king_ring_attack_queen = v; },
            "kingoppositionbonus" => if let Ok(v) = value.parse::<i16>() { self.king_opposition_bonus = v; },
            "kingpawnshield" => if let Ok(v) = value.parse::<i16>() { self.king_pawn_shield = v; },
            "kingpieceshield" => if let Ok(v) = value.parse::<i16>() { self.king_piece_shield = v; },
            "kingpawnshieldkingside" => if let Ok(v) = value.parse::<i16>() { self.king_pawn_shield_kingside = v; },
            "kingpawnshieldqueenside" => if let Ok(v) = value.parse::<i16>() { self.king_pawn_shield_queenside = v; },
            "kingpieceshieldkingside" => if let Ok(v) = value.parse::<i16>() { self.king_piece_shield_kingside = v; },
            "kingpieceshieldqueenside" => if let Ok(v) = value.parse::<i16>() { self.king_piece_shield_queenside = v; },
            "connectedpassedpawnmg" => if let Ok(v) = value.parse::<i16>() { self.connected_passed_pawn_mg = v; },
            "connectedpassedpawneg" => if let Ok(v) = value.parse::<i16>() { self.connected_passed_pawn_eg = v; },
            "knightoutposttruemg" => if let Ok(v) = value.parse::<i16>() { self.knight_outpost_true_mg = v; },
            "knightoutposttrueeg" => if let Ok(v) = value.parse::<i16>() { self.knight_outpost_true_eg = v; },
            "bishopoutposttruemg" => if let Ok(v) = value.parse::<i16>() { self.bishop_outpost_true_mg = v; },
            "bishopoutposttrueeg" => if let Ok(v) = value.parse::<i16>() { self.bishop_outpost_true_eg = v; },
            "oppositebishopsdrawscale" => if let Ok(v) = value.parse::<i16>() { self.opposite_bishops_draw_scale = v; },
            "enableendgamemopup" => { self.enable_endgame_mopup = value.to_lowercase() == "true"; },
            "mopupcenterweight" => if let Ok(v) = value.parse::<i16>() { self.mopup_center_weight = v; },
            "mopupproximityweight" => if let Ok(v) = value.parse::<i16>() { self.mopup_proximity_weight = v; },
            "mopupevalthreshold" => if let Ok(v) = value.parse::<i16>() { self.mopup_eval_threshold = v; },
            "mopupmaxgamephase" => if let Ok(v) = value.parse::<i16>() { self.mopup_max_game_phase = v; },
            "rookbehindenemypassedpawnmg" => if let Ok(v) = value.parse::<i16>() { self.rook_behind_enemy_passed_pawn_mg = v; },
            "rookbehindenemypassedpawneg" => if let Ok(v) = value.parse::<i16>() { self.rook_behind_enemy_passed_pawn_eg = v; },
            "kingtrappatbaselinemalus" => if let Ok(v) = value.parse::<i16>() { self.king_trapp_at_baseline_malus = v; },
            "kingincheckmalus" => if let Ok(v) = value.parse::<i16>() { self.king_in_check_malus = v; },
            "kingindoublecheckmalus" => if let Ok(v) = value.parse::<i16>() { self.king_in_double_check_malus = v; },
            "pawnattacksopponentfig" => if let Ok(v) = value.parse::<i16>() { self.pawn_attacks_opponent_fig = v; },
            "pawnattacksopponentfigwithtempo" => if let Ok(v) = value.parse::<i16>() { self.pawn_attacks_opponent_fig_with_tempo = v; },
            "queeninattack" => if let Ok(v) = value.parse::<i16>() { self.queen_in_attack = v; },
            "queeninattackwithtempo" => if let Ok(v) = value.parse::<i16>() { self.queen_in_attack_with_tempo = v; },
            "knightattacksbishop" => if let Ok(v) = value.parse::<i16>() { self.knight_attacks_bishop = v; },
            "knightattacksrook" => if let Ok(v) = value.parse::<i16>() { self.knight_attacks_rook = v; },
            "knightattacksbishoptempo" => if let Ok(v) = value.parse::<i16>() { self.knight_attacks_bishop_tempo = v; },
            "knightattacksrooktempo" => if let Ok(v) = value.parse::<i16>() { self.knight_attacks_rook_tempo = v; },
            "deltapruningmargin" => if let Ok(v) = value.parse::<i16>() { self.delta_pruning_margin = v; },
            "lazyevalmarginsearch" => if let Ok(v) = value.parse::<i16>() { self.lazy_eval_margin_search = v; },
            "lazyevalmarginqs" => if let Ok(v) = value.parse::<i16>() { self.lazy_eval_margin_qs = v; },
            "lazyevalmingamephase" => if let Ok(v) = value.parse::<u32>() { self.lazy_eval_min_game_phase = v; },
            "kingdangerweight1" => if let Ok(v) = value.parse::<i16>() { self.king_danger_weight_1 = v; },
            "kingdangerweight2" => if let Ok(v) = value.parse::<i16>() { self.king_danger_weight_2 = v; },
            "kingdangerweight3" => if let Ok(v) = value.parse::<i16>() { self.king_danger_weight_3 = v; },
            "kingdangerweight4" => if let Ok(v) = value.parse::<i16>() { self.king_danger_weight_4 = v; },
            "kingdangerweight5" => if let Ok(v) = value.parse::<i16>() { self.king_danger_weight_5 = v; },
            "enablefutilitypruning" => { self.enable_futility_pruning = value.to_lowercase() == "true"; },
            "enableqstt" => { self.enable_qs_tt = value.to_lowercase() == "true"; },
            "futilitymaxdepth" => if let Ok(v) = value.parse::<i32>() { self.futility_max_depth = v; },
            "futilitymarginbase" => if let Ok(v) = value.parse::<i16>() { self.futility_margin_base = v; },
            "kingopenfileheavythreatmalus" => if let Ok(v) = value.parse::<i16>() { self.king_open_file_heavy_threat_malus = v; },
            "rookopenfileattacksking" => if let Ok(v) = value.parse::<i16>() { self.rook_open_file_attacks_king = v; },
            "rookopenfileattacksqueen" => if let Ok(v) = value.parse::<i16>() { self.rook_open_file_attacks_queen = v; },
            "pawnphalanxmg" => if let Ok(v) = value.parse::<i16>() { self.pawn_phalanx_mg = v; },
            "pawnphalanxeg" => if let Ok(v) = value.parse::<i16>() { self.pawn_phalanx_eg = v; },
            "bishopdiagonalattacksking" => if let Ok(v) = value.parse::<i16>() { self.bishop_diagonal_attacks_king = v; },
            "bishopdiagonalattacksqueen" => if let Ok(v) = value.parse::<i16>() { self.bishop_diagonal_attacks_queen = v; },
            "rookonseventhkingcutoff" => if let Ok(v) = value.parse::<i16>() { self.rook_on_seventh_king_cutoff = v; },
            "rooksdoubledonseventh" => if let Ok(v) = value.parse::<i16>() { self.rooks_doubled_on_seventh = v; },
            "passedpawnblockadedmalus" => if let Ok(v) = value.parse::<i16>() { self.passed_pawn_blockaded_malus = v; },
            "candidatepassedpawnbonus" => if let Ok(v) = value.parse::<i16>() { self.candidate_passed_pawn_bonus = v; },
            "pawnstormbonus" => if let Ok(v) = value.parse::<i16>() { self.pawn_storm_bonus = v; },
            "futilitymarginslope" => if let Ok(v) = value.parse::<i16>() { self.futility_margin_slope = v; },
            "enablerazoring" => { self.enable_razoring = value.to_lowercase() == "true"; },
            "razoringmargin" => if let Ok(v) = value.parse::<i16>() { self.razoring_margin = v; },
            _ => return UciOptionEffect::Unknown,
        }
        UciOptionEffect::Stored
    }
}
