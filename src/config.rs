#[derive(Clone)]
pub struct Config {
    pub use_zobrist: bool,
    pub max_zobrist_hash_entries: u32,
    pub search_depth: i32,
    pub truncate_bad_moves: usize,
    pub in_debug: bool,

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

    // additional values for new eval
    pub pawn_structure: i16,
    pub pawn_supports_knight_outpost: i16,
    pub pawn_centered: i16,
    pub pawn_undeveloped_malus: i16,
    pub pawn_attacks_opponent_fig: i16,
    pub knight_on_rim_malus: i16,
    pub knight_centered: i16,
    pub queen_in_attack: i16,
    pub king_shield: i16,
    pub king_in_check_malus: i16,
    pub king_in_double_check_malus: i16,
    pub king_centered: i16
}


impl Config {
    pub fn new() -> Config {
        Config {
            use_zobrist: true,
            max_zobrist_hash_entries: 100_000, // 100.000 = 1GB
            search_depth: 4,
            truncate_bad_moves: 30,
            in_debug: true,

            undeveloped_knight_malus: 36,
            undeveloped_bishop_malus: 25,
            undeveloped_king_malus: 45,

            piece_eval_pawn: 100,
            piece_eval_rook: 500,
            piece_eval_knight: 300,
            piece_eval_bishop: 300,
            piece_eval_queen: 950,
            piece_eval_king: 15000,

            pawn_structure: 8,
            pawn_supports_knight_outpost: 15,
            pawn_centered: 14,
            pawn_undeveloped_malus: 10,
            pawn_attacks_opponent_fig: 25,
            pawn_on_last_rank_bonus: 200,
            pawn_on_before_last_rank_bonus: 125,
            pawn_on_before_before_last_rank_bonus: 60,
            pawn_defends_bishop: 30,

            knight_on_rim_malus: 8,
            knight_centered: 25,

            queen_in_attack: 65,

            king_shield: 30,
            king_in_check_malus: 140,
            king_in_double_check_malus: 300,
            king_centered: 120,
        }
    }
}