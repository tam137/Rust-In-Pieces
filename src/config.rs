

pub struct Config {
    pub use_quiescence: bool,
    pub use_depth_modificator: bool,
    pub search_depth: i32,
    pub eval_fuzzy: i16,
    pub move_freedom_bonus: i32,
    pub undeveloped_knight_malus: i16,
    pub undeveloped_bishop_malus: i16,
    pub pawn_on_last_rank_bonus: i16,
    pub pawn_on_before_last_rank_bonus: i16,
    pub pawn_on_before_before_last_rank_bonus: i16,
    pub early_queen_malus: i16,
    pub undeveloped_center_pawn_malus: i16,
    pub short_castle_bonus: i16,
    pub long_castle_bonus: i16,
}


impl Config {
    pub fn new() -> Config {
        Config {
            use_quiescence: false,
            use_depth_modificator: false,
            search_depth: 4,
            eval_fuzzy: 20,
            move_freedom_bonus: 6,
            undeveloped_knight_malus: 35,
            undeveloped_bishop_malus: 20,
            pawn_on_last_rank_bonus: 200,
            pawn_on_before_last_rank_bonus: 120,
            pawn_on_before_before_last_rank_bonus: 60,
            early_queen_malus: 150,
            undeveloped_center_pawn_malus: 90,
            short_castle_bonus: 110,
            long_castle_bonus: 75,
        }
    }

    pub fn unittest(&mut self) -> &Config {
        self.eval_fuzzy = 0;
        self.search_depth = 2;
        self
    }
}