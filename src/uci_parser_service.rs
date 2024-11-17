use crate::model::INIT_BOARD_FEN;
use crate::model::SearchResult;
use crate::model::Stats;


pub struct UciParserService;

impl UciParserService {

    pub fn new() -> Self {
        UciParserService {}
    }

    /// Got a "go" command and return wtime and btime in ms. (-1, -1) if no time information given.
    pub fn parse_go(&self, command: &str) -> (i32, i32) {

        let mut wtime = -1;
        let mut btime = -1;

        if !command.contains("wtime") || !command.contains("btime") {
            return (wtime, btime);
        }

        let command_parts: Vec<&str> = command.split_whitespace().collect();

        let mut iter = command_parts.iter();
        while let Some(part) = iter.next() {
            match *part {
                "wtime" => {
                    if let Some(value) = iter.next() {
                        wtime = value.parse().unwrap_or(-1);
                    }
                },
                "btime" => {
                    if let Some(value) = iter.next() {
                        btime = value.parse().unwrap_or(-1);
                    }
                },
                _ => {}
            }
       }
        (wtime, btime)
    }


    /// Parst den "position" Befehl und gibt ein Tuple (FEN, Moves) zurück
    pub fn parse_position(&self, uci_token: &str) -> (String, String) {
        let tokens: Vec<&str> = uci_token.trim().split_whitespace().collect();
        let mut fen = String::new();
        let mut moves = String::new();

        if tokens.len() < 2 {
            panic!("Ungültiger position-Befehl");
        }

        match tokens[1] {
            "startpos" => {
                fen = INIT_BOARD_FEN.to_string();
                if let Some(i) = tokens.iter().position(|&x| x == "moves") {
                    moves = tokens[i+1..].join(" ");
                }
            },
            "fen" => {
                let fen_tokens = &tokens[2..];
                if fen_tokens.is_empty() {
                    panic!("FEN-String fehlt");
                }
                if let Some(i) = fen_tokens.iter().position(|&x| x == "moves") {
                    fen = fen_tokens[..i].join(" ");
                    moves = fen_tokens[i+1..].join(" ");
                } else {
                    fen = fen_tokens.join(" ");
                }
            },
            _ => {
                panic!("Ungültiger position-Befehl");
            }
        }
        (fen, moves)
    }

    /// parse a move string example: "moves d2d4 d7d6" without \n at end 
    pub fn parse_last_move_from_moves_str(&self, moves_str: &str) -> String {
        let len = moves_str.len();
        let move_str = if matches!(moves_str.chars().rev().nth(0), Some('q' | 'k')) {
            &moves_str[len - 5..]
        } else {
            &moves_str[len - 4..]
        };
        move_str.to_string()
    }

    pub fn get_info_str(&self, search_result: &SearchResult, stats: &Stats) -> String {
        let cp = if search_result.is_white_move { search_result.get_eval() } else { search_result.get_eval() *(-1) };
        format!("info depth {} score cp {} time {} nodes {} nps {} pv {}",
            search_result.get_depth(),
            cp,
            0,
            stats.created_nodes,
            0 as usize,
            search_result.get_best_move_row())
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_go_valid_times_test() {
        let parser = UciParserService {};
        let command = "go wtime 31520 btime 1410";
        let (wtime, btime) = parser.parse_go(command);
        assert_eq!(31520, wtime);
        assert_eq!(1410, btime);

        let parser = UciParserService {};
        let command = "go";
        let (wtime, btime) = parser.parse_go(command);
        assert_eq!(-1, wtime);
        assert_eq!(-1, btime);

        let parser = UciParserService {};
        let command = "go wtime 31520 btime 1410 something extra";
        let (wtime, btime) = parser.parse_go(command);
        assert_eq!(31520, wtime);
        assert_eq!(1410, btime);
    }

    #[test]
    fn parse_position_command_test() {
        let parser = UciParserService {};

        let uci_token = "position startpos w KQkq - 0 2 moves e2e4 d7d5\n";
        let (fen, moves) = parser.parse_position(&uci_token);
        assert_eq!(INIT_BOARD_FEN, fen);
        assert_eq!("e2e4 d7d5", moves);

        let uci_token = "position fen rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2 moves e2e4 g8f6\n";
        let (fen, moves) = parser.parse_position(&uci_token);
        assert_eq!("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2", fen);
        assert_eq!("e2e4 g8f6", moves);

        let uci_token = "position fen 2kr1bnr/pppqp1pp/2n5/1B1pPb2/5P2/2P2N2/PP4PP/RNBQK2R b KQ - 4 8 moves Qd1d5\n";
        let (fen, moves) = parser.parse_position(&uci_token);
        assert_eq!("2kr1bnr/pppqp1pp/2n5/1B1pPb2/5P2/2P2N2/PP4PP/RNBQK2R b KQ - 4 8", fen);
        assert_eq!("Qd1d5", moves);

    }

    #[test]
    fn parse_last_move_from_moves_str_test() {
        let parser = UciParserService {};

        let moves_str = "moves e2e4 d7d5 d3d4";
        let best_move = parser.parse_last_move_from_moves_str(moves_str);
        assert_eq!("d3d4", best_move);

        let moves_str = "moves e2e4 d7d5 d3d4 f7f8q";
        let best_move = parser.parse_last_move_from_moves_str(moves_str);
        assert_eq!("f7f8q", best_move);
    }

}
