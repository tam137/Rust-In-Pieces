use crate::board;
use crate::Board;

#[derive(Clone)]
pub struct Turn {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) capture: i8,
    pub promotion: bool,
    pub gives_chess: bool,
    //pub null_move_chess: bool, // fig capture villains king in null move
    pub post_villain: Vec<usize>,
    pub post_my: Vec<usize>,
    pub eval: i16,
}



impl Turn {

    pub fn generate_turns(move_row: &str) -> Vec<Turn> {
        let mut turn_list = Vec::new();
        let algebraic_move_list: Vec<&str> = move_row.split_whitespace().collect();
    
        for algebraic_move in algebraic_move_list {
            let from = board::get_index_from_notation(&algebraic_move[0..2]).unwrap();
            let to = board::get_index_from_notation(&algebraic_move[2..4]).unwrap();
            let promotion = if algebraic_move.len() == 5 {
                algebraic_move.chars().nth(4) == Some('q')
            } else {
                false
            };
    
            turn_list.push(Turn {
                from,
                to,
                capture: -1, // symbol for not do a validation check
                post_villain:  Vec::new(),
                post_my: Vec::new(),
                promotion,
                gives_chess: false,
                eval: 0,
            });
        }
    
        turn_list
    }

    pub fn is_promotion(&self) -> bool {
        self.promotion
    }

    pub fn to_algebraic(&self, pgn_promotion_symbol: bool) -> String {
        let column_from = (self.from % 10 + 96) as u8;
        let row_from = (10 - (self.from / 10) + 48) as u8;
        let column_to = (self.to % 10 + 96) as u8;
        let row_to = (10 - (self.to / 10) + 48) as u8;
        let promotional_lit;
        if pgn_promotion_symbol {
            promotional_lit = if self.is_promotion() { if self.to/10 == 9 { "=Q" } else {"=Q"}} else { "" };
        } else {
            promotional_lit = if self.is_promotion() { if self.to/10 == 9 { "q" } else {"q"}} else { "" };
        }
        format!("{}{}{}{}{}", column_from as char, row_from as char, column_to as char, row_to as char, promotional_lit)
    }

    pub fn enrich_move_promotion(&mut self, board: &Board, white: bool) -> () {
        let promotion = if self.from / 10 == if white { 3 } else { 8 } && board.get_field()[self.from] == if white { 10 } else { 20 } { true } else { false };
        self.promotion = promotion;
    }

    pub fn enrich_move_gives_chess(&mut self, board: &Board, white: bool) -> () {
        let villain_king = if !white { board.index_of_white_king() } else  { board.index_of_black_king() };
        let gives_chess = self.post_my.iter().any(|&x: &usize| x as i32 == villain_king);
        self.gives_chess = gives_chess;
    }

}