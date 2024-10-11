use std::collections::HashMap;


#[derive(Debug, PartialEq, Clone)]
pub enum GameStatus {
    Normal,
    Remis,
    WhiteWon,
    BlackWon,
}


#[derive(Debug, Clone)]
pub struct Turn {
    pub from: i32,
    pub to: i32,
    pub capture: i32,
    pub promotion: i32,
    pub eval: i32,
}

impl Turn {
    // Constructor with all fields
    pub fn new(from: i32, to: i32, capture: i32, promotion: i32, eval: i32) -> Self {
        Turn {
            from,
            to,
            capture,
            promotion,
            eval,
        }
    }

    // Copy constructor
    pub fn from_other(other: &Turn) -> Self {
        Turn {
            from: other.from,
            to: other.to,
            capture: other.capture,
            promotion: other.promotion,
            eval: other.eval,
        }
    }

    // Constructor with only 'from' and 'to' fields
    pub fn from_to(from: i32, to: i32) -> Self {
        Turn {
            from,
            to,
            capture: 0,
            promotion: 0,
            eval: 0,
        }
    }

    // Check if the move is a promotion
    pub fn is_promotion(&self) -> bool {
        self.promotion != 0
    }

    // Set promotion with fluent interface
    pub fn set_promotion(mut self, promotion: i32) -> Self {
        self.promotion = promotion;
        self
    }
}



#[derive(Debug)]
pub struct MoveInformation {
    pub castle_information: CastleInformation,
    pub hash: u64,
    pub en_passante: i32,
}

impl MoveInformation {
    // Constructor
    pub fn new(castle_information: CastleInformation, hash: u64, en_passante: i32) -> Self {
        MoveInformation {
            castle_information,
            hash,
            en_passante,
        }
    }
}


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


#[derive(Debug, Clone)]
pub struct Board {
    pub field: [i32; 120],
    pub white_possible_to_castle_long: bool,
    pub white_possible_to_castle_short: bool,
    pub black_possible_to_castle_long: bool,
    pub black_possible_to_castle_short: bool,
    pub field_for_en_passante: i32,  // 0 if no en passant possible, only used by fen import
    pub white_to_move: bool,
    pub move_count: i32,
    pub game_status: GameStatus,
    pub move_repetition_map: HashMap<u64, i32>,
}

impl Board {
    // Constructor
    pub fn new(
        field: [i32; 120],
        white_possible_to_castle_long: bool,
        white_possible_to_castle_short: bool,
        black_possible_to_castle_long: bool,
        black_possible_to_castle_short: bool,
        field_for_en_passante: i32,
        white_to_move: bool,
        move_count: i32,
    ) -> Self {
        Board {
            field,
            white_possible_to_castle_long,
            white_possible_to_castle_short,
            black_possible_to_castle_long,
            black_possible_to_castle_short,
            field_for_en_passante,
            white_to_move,
            move_count,
            game_status: GameStatus::Normal,
            move_repetition_map: HashMap::new(),
        }
    }

    // Method for performing a move
    pub fn do_move(&mut self, turn: &Turn) -> MoveInformation {
        self.field_for_en_passante = 0;

        let old_castle_information = self.get_castle_information();

        // Handling castling for white and black
        if self.field[turn.from as usize] == 15 || self.field[turn.from as usize] == 25 {
            match (turn.from, turn.to) {
                (25, 27) => {
                    self.field[28] = 0;
                    self.field[26] = 21;
                }
                (25, 23) => {
                    self.field[21] = 0;
                    self.field[24] = 21;
                }
                (95, 97) => {
                    self.field[98] = 0;
                    self.field[96] = 11;
                }
                (95, 93) => {
                    self.field[91] = 0;
                    self.field[94] = 11;
                }
                _ => {}
            }
        }

        // Update castling rights
        match turn.from {
            21 => self.black_possible_to_castle_long = false,
            28 => self.black_possible_to_castle_short = false,
            25 => {
                self.black_possible_to_castle_long = false;
                self.black_possible_to_castle_short = false;
            }
            91 => self.white_possible_to_castle_long = false,
            98 => self.white_possible_to_castle_short = false,
            95 => {
                self.white_possible_to_castle_long = false;
                self.white_possible_to_castle_short = false;
            }
            _ => {}
        }

        // Handle promotion
        if turn.is_promotion() {
            self.field[turn.to as usize] = turn.promotion;
        } else {
            self.field[turn.to as usize] = self.field[turn.from as usize];
        }

        self.field[turn.from as usize] = 0;

        // Increment move count if it's black's turn
        if !self.white_to_move {
            self.move_count += 1;
        }
        self.white_to_move = !self.white_to_move;

        // Calculate the board hash and update the move repetition map
        let board_hash = self.hash();
        self.move_repetition_map
            .entry(board_hash)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // Check for 3-move repetition
        if let Some(&count) = self.move_repetition_map.get(&board_hash) {
            if count == 3 {
                self.game_status = GameStatus::Remis;
            }
        }
        MoveInformation::new(old_castle_information, board_hash, self.field_for_en_passante)
    }


    // Undo move
    pub fn undo_move(&mut self, turn: &Turn, move_information: MoveInformation) {
        self.game_status = GameStatus::Normal;

        let castle_information = move_information.castle_information;

        // Handle promotion undo
        if turn.is_promotion() {
            self.field[turn.from as usize] = 10; // Reset to pawn
            if self.white_to_move {
                self.field[turn.from as usize] += 10; // Black pawn for black promotion
            }
        } else {
            self.field[turn.from as usize] = self.field[turn.to as usize];
        }

        self.field[turn.to as usize] = turn.capture.max(0);

        // Restore castling rights
        self.white_possible_to_castle_long = castle_information.white_possible_to_castle_long;
        self.white_possible_to_castle_short = castle_information.white_possible_to_castle_short;
        self.black_possible_to_castle_long = castle_information.black_possible_to_castle_long;
        self.black_possible_to_castle_short = castle_information.black_possible_to_castle_short;

        // Handle castling undo
        if self.field[turn.from as usize] == 15 || self.field[turn.from as usize] == 25 {
            match (turn.from, turn.to) {
                (25, 27) => {
                    self.field[28] = 21;
                    self.field[26] = 0;
                }
                (25, 23) => {
                    self.field[21] = 21;
                    self.field[24] = 0;
                }
                (95, 97) => {
                    self.field[98] = 11;
                    self.field[96] = 0;
                }
                (95, 93) => {
                    self.field[91] = 11;
                    self.field[94] = 0;
                }
                _ => {}
            }
        }

        // Decrement move count if it was white's move
        if self.white_to_move {
            self.move_count -= 1;
        }
        self.white_to_move = !self.white_to_move;

        // Update the move repetition map
        if let Some(count) = self.move_repetition_map.get_mut(&move_information.hash) {
            if *count > 1 {
                *count -= 1;
            } else {
                self.move_repetition_map.remove(&move_information.hash);
            }
        }
    }

    // Generate the castle information based on the current state
    pub fn get_castle_information(&self) -> CastleInformation {
        CastleInformation {
            white_possible_to_castle_long: self.white_possible_to_castle_long,
            white_possible_to_castle_short: self.white_possible_to_castle_short,
            black_possible_to_castle_long: self.black_possible_to_castle_long,
            black_possible_to_castle_short: self.black_possible_to_castle_short,
        }
    }

    // Hash function for the board (used for 3-move repetition)
    pub fn hash(&self) -> u64 {
        let mut hash = 17u64;
        hash = hash.wrapping_mul(31).wrapping_add(self.field.iter().fold(0, |acc, &x| acc.wrapping_add(x as u64)));
        hash = hash.wrapping_mul(31).wrapping_add(self.white_possible_to_castle_long as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.white_possible_to_castle_short as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.black_possible_to_castle_long as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.black_possible_to_castle_short as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.field_for_en_passante as u64);
        hash = hash.wrapping_mul(31).wrapping_add(self.white_to_move as u64);
        hash
    }
}

// Implement `PartialEq` manually for the `Board` struct, for unittests
impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        // Check if all the fields of the Board match
        self.white_possible_to_castle_long == other.white_possible_to_castle_long &&
            self.white_possible_to_castle_short == other.white_possible_to_castle_short &&
            self.black_possible_to_castle_long == other.black_possible_to_castle_long &&
            self.black_possible_to_castle_short == other.black_possible_to_castle_short &&
            self.field_for_en_passante == other.field_for_en_passante &&
            self.white_to_move == other.white_to_move &&
            self.move_count == other.move_count &&
            self.game_status == other.game_status &&
            self.field == other.field &&  // Direct comparison of arrays (fixed-size arrays implement PartialEq)
            self.move_repetition_map == other.move_repetition_map  // HashMap comparison
    }
}
