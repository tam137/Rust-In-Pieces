#[derive(Debug)]
pub struct CastleInformation {
    pub white_possible_to_castle_long: bool,
    pub white_possible_to_castle_short: bool,
    pub black_possible_to_castle_long: bool,
    pub black_possible_to_castle_short: bool,
}

impl CastleInformation {
    // Constructor
    pub fn new(white_possible_to_castle_long: bool, white_possible_to_castle_short: bool,
               black_possible_to_castle_long: bool, black_possible_to_castle_short: bool) -> Self {
        CastleInformation {
            white_possible_to_castle_long,
            white_possible_to_castle_short,
            black_possible_to_castle_long,
            black_possible_to_castle_short,
        }
    }
}
