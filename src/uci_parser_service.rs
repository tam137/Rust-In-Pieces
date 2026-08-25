use crate::model::TimeInfo;
use crate::model::SearchResult;
use crate::model::Stats;
use crate::model::TimeMode;

use crate::model::INIT_BOARD_FEN;

pub struct UciParserService;

impl UciParserService {

    pub fn new() -> Self {
        UciParserService {}
    }

    /// Got a "go" command and return wtime and btime in ms. (-1, -1) if no time information given.
    pub fn parse_go(&self, command: &str) -> TimeInfo {

        if (!command.contains("wtime") || !command.contains("btime")) && !command.contains("movetime") && !command.contains("depth") {
            return TimeInfo {
                wtime: 0,
                btime: 0,
                winc: 0,
                binc: 0,
                moves_to_go: 0,
                depth: 0,
                time_mode: TimeMode::None,
            }
        }

        let command_parts: Vec<&str> = command.split_whitespace().collect();
        let mut time_mode = TimeMode::None;
        let mut wtime = 0;
        let mut btime = 0;
        let mut winc = 0;
        let mut binc = 0;
        let mut moves_to_go = 0;
        let mut depth = 0;

        let mut iter = command_parts.iter();
        while let Some(part) = iter.next() {
            match *part {
                "wtime" => {
                    if let Some(value) = iter.next() {
                        wtime = value.parse().unwrap_or(0);
                        time_mode = TimeMode::HourGlas;
                    }
                },
                "btime" => {
                    if let Some(value) = iter.next() {
                        btime = value.parse().unwrap_or(0);
                        time_mode = TimeMode::HourGlas;
                    }
                },
                "winc" => {
                    if let Some(value) = iter.next() {
                        winc = value.parse().unwrap_or(0);
                    }
                },
                "binc" => {
                    if let Some(value) = iter.next() {
                        binc = value.parse().unwrap_or(0);
                    }
                },
                "movetime" => {
                    if let Some(value) = iter.next() {
                        wtime = value.parse().unwrap_or(0);
                        btime = value.parse().unwrap_or(0);
                        time_mode = TimeMode::Movetime;
                    }
                }
                "movestogo" => {
                    if let Some(value) = iter.next() {
                        moves_to_go = value.parse().unwrap_or(0);
                    }
                }
                "depth" => {
                    if let Some(value) = iter.next() {
                        depth = value.parse().unwrap_or(0);
                        time_mode = TimeMode::Depth;
                    }
                }
                _ => {}
            }
        }

        if command.contains("movestogo") {
            time_mode = TimeMode::MoveToGo;
        }

        TimeInfo {
            wtime,
            btime,
            winc,
            binc,
            moves_to_go,
            depth,
            time_mode,
        }        
    }


    /// Parse the "position" command and returns a tuple (FEN, Moves)
    pub fn parse_position(&self, uci_token: &str) -> (String, String) {
        let tokens: Vec<&str> = uci_token.split_whitespace().collect();
        let fen;
        let mut moves = String::new();

        if tokens.len() < 2 {
            panic!("RIP Could not parse uci position command");
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
                    panic!("RIP FEN part is missing in uci");
                }
                if let Some(i) = fen_tokens.iter().position(|&x| x == "moves") {
                    fen = fen_tokens[..i].join(" ");
                    moves = fen_tokens[i+1..].join(" ");
                } else {
                    fen = fen_tokens.join(" ");
                }
            },
            _ => {
                panic!("RIP Could not parse uci position command");
            }
        }
        (fen, moves)
    }

    /// Formats a search score as a UCI `score` value: `mate <moves>` for a forced
    /// mate, `cp <centipawns>` otherwise. The score is expected from the perspective
    /// of the side to move, so a positive mate distance means the mover delivers it.
    pub fn format_score(&self, score: i16) -> String {
        if score.abs() > crate::model::MATE_SCORE_THRESHOLD {
            let mate_plies = crate::model::MATE_SCORE - score.abs();
            let mate_moves = (mate_plies + 1) / 2;
            if score > 0 {
                format!("mate {}", mate_moves)
            } else {
                format!("mate -{}", mate_moves)
            }
        } else {
            format!("cp {}", score)
        }
    }

    pub fn get_info_str(&self, search_result: &SearchResult, stats: &Stats) -> String {
        let mut stats = stats.clone();
        let stats = stats.calculate();
        let cp = if search_result.is_white_move { search_result.get_eval() } else { -search_result.get_eval() };
        format!("info depth {} score {} time {} nodes {} nps {} pv {}",
            search_result.calculated_depth,
            self.format_score(cp),
            stats.calc_time_ms,
            stats.created_nodes,
            stats.created_nodes / (stats.calc_time_ms + 1),
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
        let time_info = parser.parse_go(command);
        assert_eq!(31520, time_info.wtime);
        assert_eq!(1410, time_info.btime);
        assert_eq!(TimeMode::HourGlas, time_info.time_mode);

        let parser = UciParserService {};
        let command = "go wtime 31520 btime 1410 winc 100 binc 100";
        let time_info = parser.parse_go(command);
        assert_eq!(31520, time_info.wtime);
        assert_eq!(1410, time_info.btime);
        assert_eq!(100, time_info.winc);
        assert_eq!(100, time_info.binc);
        assert_eq!(TimeMode::HourGlas, time_info.time_mode);

        let parser = UciParserService {};
        let command = "go";
        let time_info = parser.parse_go(command);
        assert_eq!(0, time_info.wtime);
        assert_eq!(0, time_info.btime);
        assert_eq!(0, time_info.winc);
        assert_eq!(0, time_info.binc);
        assert_eq!(TimeMode::None, time_info.time_mode);

        let parser = UciParserService {};
        let command = "go wtime 31520 btime 1410 something extra";
        let time_info = parser.parse_go(command);
        assert_eq!(31520, time_info.wtime);
        assert_eq!(1410, time_info.btime);
        assert_eq!(TimeMode::HourGlas, time_info.time_mode);

        let parser = UciParserService {};
        let command = "go movetime 30000";
        let time_info = parser.parse_go(command);
        assert_eq!(30000, time_info.wtime);
        assert_eq!(30000, time_info.btime);
        assert_eq!(0, time_info.winc);
        assert_eq!(0, time_info.binc);
        assert_eq!(TimeMode::Movetime, time_info.time_mode);

        let parser = UciParserService {};
        let command = "go wtime 15200 btime 14100 movestogo 30";
        let time_info = parser.parse_go(command);
        assert_eq!(15200, time_info.wtime);
        assert_eq!(14100, time_info.btime);
        assert_eq!(0, time_info.winc);
        assert_eq!(0, time_info.binc);
        assert_eq!(30, time_info.moves_to_go);
        assert_eq!(0, time_info.depth);
        assert_eq!(TimeMode::MoveToGo, time_info.time_mode);

        let parser = UciParserService {};
        let command = "go movestogo 30 wtime 15200 btime 14100 ";
        let time_info = parser.parse_go(command);
        assert_eq!(15200, time_info.wtime);
        assert_eq!(14100, time_info.btime);
        assert_eq!(0, time_info.winc);
        assert_eq!(0, time_info.binc);
        assert_eq!(30, time_info.moves_to_go);
        assert_eq!(0, time_info.depth);
        assert_eq!(TimeMode::MoveToGo, time_info.time_mode);

        let parser = UciParserService {};
        let command = "go depth 6";
        let time_info = parser.parse_go(command);
        assert_eq!(0, time_info.wtime);
        assert_eq!(0, time_info.btime);
        assert_eq!(0, time_info.winc);
        assert_eq!(0, time_info.binc);
        assert_eq!(0, time_info.moves_to_go);
        assert_eq!(6, time_info.depth);
        assert_eq!(TimeMode::Depth, time_info.time_mode);
    }

    #[test]
    fn parse_position_command_test() {
        let parser = UciParserService {};

        let uci_token = "position startpos w KQkq - 0 2 moves e2e4 d7d5\n";
        let (fen, moves) = parser.parse_position(&uci_token);
        assert_eq!(INIT_BOARD_FEN, fen);
        assert_eq!("e2e4 d7d5", moves);

        let uci_token = "position startpos\n";
        let (fen, moves) = parser.parse_position(&uci_token);
        assert_eq!(INIT_BOARD_FEN, fen);
        assert_eq!("", moves);

        let uci_token = "position fen rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2 moves e2e4 g8f6\n";
        let (fen, moves) = parser.parse_position(&uci_token);
        assert_eq!("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2", fen);
        assert_eq!("e2e4 g8f6", moves);

        let uci_token = "position fen 2kr1bnr/pppqp1pp/2n5/1B1pPb2/5P2/2P2N2/PP4PP/RNBQK2R b KQ - 4 8 moves Qd1d5\n";
        let (fen, moves) = parser.parse_position(&uci_token);
        assert_eq!("2kr1bnr/pppqp1pp/2n5/1B1pPb2/5P2/2P2N2/PP4PP/RNBQK2R b KQ - 4 8", fen);
        assert_eq!("Qd1d5", moves);

    }

    /// Builds a minimal SearchResult carrying one variant, so the info-string
    /// formatting can be exercised without running a search.
    fn search_result_with(eval: i16, depth: i32, is_white_move: bool) -> SearchResult {
        let mut result = SearchResult::default();
        result.is_white_move = is_white_move;
        result.calculated_depth = depth;
        result.add_variant(crate::model::Variant {
            eval,
            best_move: None,
            move_row: std::collections::VecDeque::new(),
        });
        result
    }

    /// Regression guard: `info depth` used to report `SearchResult::get_depth()`,
    /// which is the length of the principal variation rather than the depth the
    /// iteration actually completed.
    #[test]
    fn info_string_reports_the_completed_search_depth() {
        let parser = UciParserService {};
        let stats = Stats::default();

        let result = search_result_with(42, 11, true);
        let info = parser.get_info_str(&result, &stats);

        assert!(info.starts_with("info depth 11 "),
            "info string must report the completed iteration depth, got: {}", info);
    }

    /// Regression guard: mate scores used to leave the engine as raw centipawns,
    /// and the internal logger converted them with a wrong constant.
    #[test]
    fn mate_scores_are_formatted_as_uci_mate_distances() {
        let parser = UciParserService {};

        // MATE_SCORE - 7 is a mate delivered on the seventh ply, i.e. mate in four.
        assert_eq!("mate 4", parser.format_score(crate::model::MATE_SCORE - 7));
        assert_eq!("mate -4", parser.format_score(-(crate::model::MATE_SCORE - 7)));
        // A mate delivered on the very next ply is mate in one.
        assert_eq!("mate 1", parser.format_score(crate::model::MATE_SCORE - 1));
        // Ordinary evaluations stay centipawns and keep their sign.
        assert_eq!("cp 636", parser.format_score(636));
        assert_eq!("cp -636", parser.format_score(-636));
        assert_eq!("cp 0", parser.format_score(0));
    }

    /// End-to-end guard tying the info string to the score encoding the search
    /// actually produces. Philidor's Legacy is a forced mate in four; Check
    /// Extensions resolve it at nominal depth 5.
    #[test]
    fn info_string_reports_a_real_forced_mate_as_mate_in_four() {
        let service = crate::service::Service::new();
        let mut board = service.fen.set_fen("r6k/6pp/8/6N1/8/1Q6/8/6K1 w - - 0 1");
        let config = crate::config::Config::for_tests();
        let mut stats = Stats::default();

        let (tx_log, _rx_log) = std::sync::mpsc::channel();
        let engine_state = std::sync::Arc::new(crate::model::EngineState {
            stop_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            debug_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            zobrist_table: std::sync::RwLock::new(std::sync::Arc::new(
                crate::zobrist::ZobristTable::with_capacity(100_000))),
            pv_nodes: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0)),
            logger: std::sync::Arc::new(std::sync::RwLock::new(std::sync::Arc::new(|_| {}))),
            log_sender: tx_log,
        });

        let result = service.search.get_moves(
            &mut board, 5, true, &mut stats, &config, &service,
            &engine_state, std::time::Instant::now(), None, None,
        );

        let info = service.uci_parser.get_info_str(&result, &stats);
        assert!(info.contains("score mate 4"),
            "a forced mate in four must be reported as `score mate 4`, got: {}", info);
        assert!(info.starts_with("info depth 5 "),
            "the info string must report the nominal depth, got: {}", info);
    }
}
