#[derive(Clone)]
pub struct Config {
    pub use_depth_modificator: bool,
    pub use_book: bool,
    pub use_zobrist: bool,
    pub max_zobrist_hash_entries: u32,
    pub search_depth: i32,
    pub search_depth_quite: i32,
    pub calc_variants: i16,
    pub truncate_bad_moves: usize,
    pub in_debug: bool,

    pub move_freedom_bonus: i32,
    pub undeveloped_knight_malus: i16,
    pub undeveloped_bishop_malus: i16,
    pub undeveloped_king_malus: i16,
    pub pawn_on_last_rank_bonus: i16,
    pub pawn_on_before_last_rank_bonus: i16,
    pub pawn_on_before_before_last_rank_bonus: i16,
    pub pawn_defends_bishop: i16,
    pub early_queen_malus: i16,
    pub undeveloped_center_pawn_malus: i16,
    pub short_castle_bonus: i16,
    pub long_castle_bonus: i16,
    pub max_push_bonus: i16,

    pub piece_eval_pawn: i16,
    pub piece_eval_rook: i16,
    pub piece_eval_knight: i16,
    pub piece_eval_bishop: i16,
    pub piece_eval_queen: i16,
    pub piece_eval_king: i16,

    // additional values for new eval
    pub pawn_structure: i16,
    pub pawn_supports_knight_outpost: i16,
    pub pawn_centered: i16,
    pub pawn_undeveloped_malus: i16,
    pub pawn_attacks_opponent_fig: i16,
    pub rooks_on_same_rank: i16,
    pub knight_on_rim_malus: i16,
    pub knight_attacks_rook: i16,
    pub knight_attacks_queen: i16,
    pub knight_attacks_bishop: i16,
    pub knight_centered: i16,
    pub queen_in_rook_line_malus: i16,
    pub queen_in_attack: i16,
    pub king_shield: i16,
    pub king_in_check_malus: i16,
    pub king_in_double_check_malus: i16,
    pub king_centered: i16
}


impl Config {
    pub fn new() -> Config {
        Config {
            use_depth_modificator: false,
            use_book: false,
            use_zobrist: true,
            max_zobrist_hash_entries: 100_000, // 100.000 = 1GB
            search_depth: 4,
            search_depth_quite: 12,
            calc_variants: 1,
            truncate_bad_moves: 30,
            in_debug: true,

            move_freedom_bonus: 3,
            undeveloped_knight_malus: 36,
            undeveloped_bishop_malus: 25,
            undeveloped_king_malus: 45,
            early_queen_malus: 80,
            undeveloped_center_pawn_malus: 90,
            short_castle_bonus: 110,
            long_castle_bonus: 75,
            max_push_bonus: 15,

            piece_eval_pawn: 100,
            piece_eval_rook: 500,
            piece_eval_knight: 300,
            piece_eval_bishop: 300,
            piece_eval_queen: 950,
            piece_eval_king: 15000,

            // additional values for new eval
            pawn_structure: 8,
            pawn_supports_knight_outpost: 15,
            pawn_centered: 14,
            pawn_undeveloped_malus: 10,
            pawn_attacks_opponent_fig: 25,
            pawn_on_last_rank_bonus: 400,
            pawn_on_before_last_rank_bonus: 200,
            pawn_on_before_before_last_rank_bonus: 100,
            pawn_defends_bishop: 50,
            rooks_on_same_rank: 20,
            knight_on_rim_malus: 8,
            knight_attacks_rook: 10,
            knight_attacks_queen: 10,
            knight_attacks_bishop: 10,
            knight_centered: 25,
            queen_in_rook_line_malus: 30,
            queen_in_attack: 65,
            king_shield: 30,
            king_in_check_malus: 140,
            king_in_double_check_malus: 300,
            king_centered: 120,
        }
    }

    pub fn get_eval_value_for_piece(&self, fig: i8) -> i16 {
        match fig {
            10 => self.piece_eval_pawn,
            20 => -self.piece_eval_pawn,
            11 => self.piece_eval_rook,
            21 => -self.piece_eval_rook,
            12 => self.piece_eval_knight,
            22 => -self.piece_eval_knight,
            13 => self.piece_eval_bishop,
            23 => -self.piece_eval_bishop,
            14 => self.piece_eval_queen,
            24 => -self.piece_eval_queen,
            15 => self.piece_eval_king,
            25 => -self.piece_eval_king,
            _ => 0
        }
    }
}