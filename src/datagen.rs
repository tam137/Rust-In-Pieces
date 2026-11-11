//! Self-play training data for the NNUE trainer (Matt-Magie step 102, stage A).
//!
//! Compiled only with the Cargo feature `datagen`; the release binary does not contain it.
//! `suprah datagen --out <file> [OPTIONS]` plays games against itself and writes one line per
//! recorded position in bullet's text format, `fen | score | result`, score in centipawns and
//! result (1.0 / 0.5 / 0.0) both from White's point of view.
//!
//! A game:
//! - opens with 8-12 plies that are random with probability 50 % each (search move otherwise);
//!   they are not recorded, and the game is dropped if the first searched position after them
//!   is already decided (|score| above the opening limit);
//! - continues with a soft node limit per move: iterative deepening stops after the iteration in
//!   which the limit is reached, never inside one;
//! - ends by mate, stalemate, threefold repetition, the fifty-move rule, insufficient material or
//!   the move limit, or is adjudicated: a win when |score| stays at or above the win threshold for
//!   several plies, a draw when |score| stays within the draw band for several plies late enough;
//! - records only quiet positions: not in check, best move neither capture nor promotion, no mate
//!   score.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::config::Config;
use crate::model::{
    BLACK_BISHOP, BLACK_KING, Board, EngineState, GameStatus, MATE_SCORE_THRESHOLD, MoveList,
    SearchContext, Stats, Turn, WHITE_BISHOP, WHITE_KING,
};
use crate::service::Service;

/// Settings of a data generation run.
#[derive(Debug, Clone, PartialEq)]
pub struct DatagenConfig {
    pub out: String,
    pub games: u64,
    pub seed: u64,
    /// Soft node limit per move.
    pub nodes: u64,
    pub random_plies_min: u32,
    pub random_plies_max: u32,
    /// Probability in percent that an opening ply is random.
    pub random_prob: u32,
    /// Drop the game if |score| exceeds this after the opening plies (centipawns).
    pub opening_limit: i16,
    pub win_cp: i16,
    pub win_plies: u32,
    pub draw_cp: i16,
    pub draw_from_move: i32,
    pub draw_plies: u32,
    /// Games are drawn at this full move number.
    pub max_moves: i32,
    pub hash_entries: usize,
    /// Print counters every this many games (0 = only at the end).
    pub report_every: u64,
}

impl Default for DatagenConfig {
    fn default() -> Self {
        Self {
            out: String::new(),
            games: 1_000_000,
            seed: 1,
            nodes: 5_000,
            random_plies_min: 8,
            random_plies_max: 12,
            random_prob: 50,
            opening_limit: 400,
            win_cp: 2_000,
            win_plies: 4,
            draw_cp: 10,
            draw_from_move: 40,
            draw_plies: 8,
            max_moves: 400,
            hash_entries: 1 << 20,
            report_every: 100,
        }
    }
}

pub const USAGE: &str = "\
Suprah self-play data generation (bullet text format, scores and results from White's view)

Usage: suprah datagen --out <FILE> [OPTIONS]

Options:
  --out <FILE>            Output file, appended to
  --games <N>             Games to play [default: 1000000]
  --seed <N>              Random seed; equal seeds give equal data [default: 1]
  --nodes <N>             Soft node limit per move [default: 5000]
  --random-plies <A-B>    Range of opening plies [default: 8-12]
  --random-prob <P>       Percent of opening plies played at random [default: 50]
  --opening-limit <CP>    Drop games with |score| above this after the opening [default: 400]
  --win <CP>/<PLIES>      Adjudicate a win [default: 2000/4]
  --draw <CP>/<PLIES>/<MOVE>  Adjudicate a draw from full move MOVE [default: 10/8/40]
  --max-moves <N>         Draw at this full move number [default: 400]
  --hash <ENTRIES>        Transposition table entries [default: 1048576]
  --report <N>            Print counters every N games, 0 = at the end only [default: 100]
  -h, --help              Show this help
";

/// Parses the arguments after `datagen`; `Ok(None)` asks for the help text.
pub fn parse_args(args: &[String]) -> Result<Option<DatagenConfig>, String> {
    let mut cfg = DatagenConfig::default();
    let mut iter = args.iter();
    while let Some(option) = iter.next() {
        if matches!(option.as_str(), "-h" | "--help") {
            return Ok(None);
        }
        let value = iter
            .next()
            .ok_or_else(|| format!("option {option} needs a value"))?;
        let parts: Vec<&str> = value.split(['/', '-']).collect();
        match option.as_str() {
            "--out" => cfg.out = value.clone(),
            "--games" => cfg.games = number(option, value)?,
            "--seed" => cfg.seed = number(option, value)?,
            "--nodes" => cfg.nodes = number(option, value)?,
            "--random-prob" => cfg.random_prob = number(option, value)?,
            "--opening-limit" => cfg.opening_limit = number(option, value)?,
            "--max-moves" => cfg.max_moves = number(option, value)?,
            "--hash" => cfg.hash_entries = number(option, value)?,
            "--report" => cfg.report_every = number(option, value)?,
            "--random-plies" => {
                let [min, max] = parts[..] else {
                    return Err(format!("{option} expects A-B, got {value:?}"));
                };
                (cfg.random_plies_min, cfg.random_plies_max) =
                    (number(option, min)?, number(option, max)?);
            }
            "--win" => {
                let [cp, plies] = parts[..] else {
                    return Err(format!("{option} expects CP/PLIES, got {value:?}"));
                };
                (cfg.win_cp, cfg.win_plies) = (number(option, cp)?, number(option, plies)?);
            }
            "--draw" => {
                let [cp, plies, from] = parts[..] else {
                    return Err(format!("{option} expects CP/PLIES/MOVE, got {value:?}"));
                };
                (cfg.draw_cp, cfg.draw_plies, cfg.draw_from_move) = (
                    number(option, cp)?,
                    number(option, plies)?,
                    number(option, from)?,
                );
            }
            _ => return Err(format!("unknown option {option}")),
        }
    }

    if cfg.out.is_empty() {
        return Err("no output file given (--out)".to_string());
    }
    if cfg.nodes == 0
        || cfg.games == 0
        || cfg.hash_entries == 0
        || cfg.win_plies == 0
        || cfg.draw_plies == 0
    {
        return Err(
            "--nodes, --games, --hash and the adjudication plies must be at least 1".to_string(),
        );
    }
    if cfg.random_plies_min > cfg.random_plies_max || cfg.random_prob > 100 {
        return Err("--random-plies needs A <= B and --random-prob at most 100".to_string());
    }
    Ok(Some(cfg))
}

fn number<T: std::str::FromStr>(option: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("option {option}: invalid value {value:?}"))
}

/// Result of a finished game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    WhiteWin,
    Draw,
    BlackWin,
}

impl Outcome {
    /// Result label of bullet's text format, from White's point of view.
    pub fn label(self) -> &'static str {
        match self {
            Outcome::WhiteWin => "1.0",
            Outcome::Draw => "0.5",
            Outcome::BlackWin => "0.0",
        }
    }
}

/// One line of training data.
pub fn format_line(fen: &str, score_white: i16, outcome: Outcome) -> String {
    format!("{fen} | {score_white} | {}", outcome.label())
}

/// Whether a searched position is recorded: quiet and with a centipawn score.
pub fn is_recordable(in_check: bool, best_move: &Turn, score_white: i16) -> bool {
    !in_check
        && best_move.capture == 0
        && !best_move.is_promotion()
        && score_white.abs() < MATE_SCORE_THRESHOLD
}

/// Win and draw adjudication over consecutive searched plies.
#[derive(Debug, Clone)]
pub struct Adjudicator {
    win_cp: i16,
    win_plies: u32,
    draw_cp: i16,
    draw_from_move: i32,
    draw_plies: u32,
    /// Positive: plies in a row with White winning, negative: with Black winning.
    win_streak: i32,
    draw_streak: u32,
}

impl Adjudicator {
    pub fn new(cfg: &DatagenConfig) -> Self {
        Self {
            win_cp: cfg.win_cp,
            win_plies: cfg.win_plies,
            draw_cp: cfg.draw_cp,
            draw_from_move: cfg.draw_from_move,
            draw_plies: cfg.draw_plies,
            win_streak: 0,
            draw_streak: 0,
        }
    }

    /// Feeds the score of the position just searched (White's view) at full move `move_number`.
    pub fn observe(&mut self, score_white: i16, move_number: i32) -> Option<Outcome> {
        self.win_streak = if score_white >= self.win_cp {
            self.win_streak.max(0) + 1
        } else if score_white <= -self.win_cp {
            self.win_streak.min(0) - 1
        } else {
            0
        };
        if self.win_streak.unsigned_abs() >= self.win_plies {
            return Some(if self.win_streak > 0 {
                Outcome::WhiteWin
            } else {
                Outcome::BlackWin
            });
        }

        self.draw_streak =
            if move_number >= self.draw_from_move && score_white.abs() <= self.draw_cp {
                self.draw_streak + 1
            } else {
                0
            };
        (self.draw_streak >= self.draw_plies).then_some(Outcome::Draw)
    }
}

/// Dead positions: no sequence of legal moves can mate (K v K, K + minor v K, bishops of one
/// colour only).
pub fn insufficient_material(board: &Board) -> bool {
    /// One colour complex of the board (b1, d1, ... - the light squares).
    const LIGHT_SQUARES: u64 = 0x55AA_55AA_55AA_55AA;
    let kings = board.bitboards[WHITE_KING] | board.bitboards[BLACK_KING];
    let others = board.occupied & !kings;
    if others.count_ones() <= 1 {
        // A lone minor piece cannot mate; any other single piece (pawn, rook, queen) can.
        let minors = board.bitboards[crate::model::WHITE_KNIGHT]
            | board.bitboards[crate::model::BLACK_KNIGHT]
            | board.bitboards[WHITE_BISHOP]
            | board.bitboards[BLACK_BISHOP];
        return others == 0 || others & minors == others;
    }
    let bishops = board.bitboards[WHITE_BISHOP] | board.bitboards[BLACK_BISHOP];
    others == bishops && (bishops & LIGHT_SQUARES == 0 || bishops & !LIGHT_SQUARES == 0)
}

/// A FEN with the given halfmove clock and full move number (the engine's own FEN writer has no
/// halfmove clock).
pub fn fen_with_clocks(fen: &str, halfmove_clock: u32, full_move: u32) -> String {
    let fields: Vec<&str> = fen.split_whitespace().take(4).collect();
    format!("{} {halfmove_clock} {full_move}", fields.join(" "))
}

/// Full move number of the position after `ply` half moves from the start position.
pub fn full_move(ply: u32) -> u32 {
    1 + ply / 2
}

/// Whether the game is over before the side to move searches: mate, stalemate, threefold,
/// fifty-move rule, dead position or move limit.
pub fn game_end(
    board: &Board,
    legal_moves: usize,
    in_check: bool,
    halfmove_clock: u32,
    ply: u32,
    max_moves: i32,
) -> Option<Outcome> {
    // `do_move` marks a threefold repetition; move generation returns nothing afterwards.
    if board.game_status == GameStatus::Draw {
        return Some(Outcome::Draw);
    }
    if legal_moves == 0 {
        return Some(match (in_check, board.white_to_move) {
            (false, _) => Outcome::Draw,
            (true, true) => Outcome::BlackWin,
            (true, false) => Outcome::WhiteWin,
        });
    }
    let move_limit_reached = full_move(ply) as i64 > max_moves as i64;
    (halfmove_clock >= 100 || insufficient_material(board) || move_limit_reached)
        .then_some(Outcome::Draw)
}

/// Counters of a run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Counters {
    pub games: u64,
    pub dropped: u64,
    pub positions: u64,
    pub filtered: u64,
    pub plies: u64,
    pub white_wins: u64,
    pub draws: u64,
    pub black_wins: u64,
}

/// The recorded lines of one game, or `None` when the game was dropped after its opening.
pub struct GameRecord {
    pub lines: Option<Vec<String>>,
    pub plies: u32,
    pub filtered: u32,
    pub outcome: Option<Outcome>,
}

/// One data generation process: the engine with its own tables and random numbers.
pub struct Generator {
    cfg: DatagenConfig,
    config: Config,
    service: Service,
    state: Arc<EngineState>,
    rng: StdRng,
    /// Keeps the engine's log channel open; the search only logs on failures.
    _log: std::sync::mpsc::Receiver<String>,
}

impl Generator {
    pub fn new(cfg: DatagenConfig) -> Self {
        let (log_sender, log) = std::sync::mpsc::channel();
        let state = Arc::new(EngineState {
            stop_flag: Arc::new(AtomicBool::new(false)),
            pondering: AtomicBool::new(false),
            debug_flag: Arc::new(AtomicBool::new(false)),
            zobrist_table: std::sync::RwLock::new(Arc::new(
                crate::zobrist::ZobristTable::with_capacity(cfg.hash_entries),
            )),
            pv_nodes: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            pv_nodes_len: Arc::new(AtomicI32::new(0)),
            logger: Arc::new(std::sync::RwLock::new(Arc::new(|_| {}))),
            log_sender,
            search_tables: std::sync::Mutex::new(crate::model::SearchTables::new()),
        });
        // The playing configuration of the engine, without the UCI info lines.
        let mut config = Config::new();
        config.print_info_string_during_search = false;
        config.max_zobrist_hash_entries = cfg.hash_entries;
        let rng = StdRng::seed_from_u64(cfg.seed);
        Self {
            cfg,
            config,
            service: Service::new(),
            state,
            rng,
            _log: log,
        }
    }

    /// Clears everything a game leaves behind, as `ucinewgame` does.
    fn new_game(&self) {
        self.state.pv_nodes.lock().unwrap().clear();
        self.state
            .pv_nodes_len
            .store(0, std::sync::atomic::Ordering::SeqCst);
        self.service.pawn_table.clear();
        self.state.zobrist_table.read().unwrap().clear();
        self.state.search_tables.lock().unwrap().reset();
    }

    /// Legal moves of the side to move and whether it is in check.
    pub fn legal_moves(&self, board: &mut Board) -> (MoveList, bool) {
        let table = self.state.zobrist_table.read().unwrap().clone();
        let history_table = [[[0i32; 64]; 64]; 2];
        let context = SearchContext {
            zobrist_table: &table,
            stop_flag: &self.state.stop_flag,
            pondering: &self.state.pondering,
            pv_nodes: &self.state.pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
            start_time: std::time::Instant::now(),
            target_time: None,
            root_moves_total: 0,
            root_moves_searched: 0,
            root_depth: 0,
        };
        let mut moves = MoveList::new();
        let mut stats = Stats::default();
        self.service.move_gen.generate_valid_moves_list(
            board,
            &mut stats,
            &self.config,
            &context,
            false,
            &mut moves,
        );
        (moves, self.service.move_gen.is_in_check(board))
    }

    /// Searches the position with the soft node limit; returns the best move and its score from
    /// White's point of view.
    pub fn search(&self, board: &mut Board) -> (Turn, i16) {
        /// Depth cap of the iterative deepening; the node limit ends the search long before.
        const MAX_DEPTH: i32 = 64;

        // The per-`go` preparation of `game_handler.rs`.
        crate::search_diag::new_search_generation();
        {
            let table = self.state.zobrist_table.read().unwrap();
            table.new_search();
            table.set_ways(self.config.tt_cluster_size);
        }
        if self.config.history_age_per_go {
            self.state.search_tables.lock().unwrap().age();
        }

        let mut best: Option<(Turn, i16)> = None;
        let mut prev_score = None;
        let mut nodes = 0u64;
        for depth in 2..=MAX_DEPTH {
            let mut stats = Stats::default();
            let white = board.white_to_move;
            let result = self.service.search.get_moves(
                board,
                depth,
                white,
                &mut stats,
                &self.config,
                &self.service,
                &self.state,
                std::time::Instant::now(),
                None,
                prev_score,
            );
            if result.completed
                && let Some(turn) = result.variants.first().and_then(|v| v.best_move)
            {
                prev_score = Some(result.get_eval());
                best = Some((turn, result.get_eval()));
            }
            nodes += stats.created_nodes as u64;
            if nodes >= self.cfg.nodes {
                break;
            }
        }
        best.expect("the search returned no move in a position with legal moves")
    }

    /// Plays one game from the start position.
    pub fn play_game(&mut self) -> GameRecord {
        self.new_game();
        let mut board = self.service.fen.set_init_board();
        let random_plies = self
            .rng
            .gen_range(self.cfg.random_plies_min..=self.cfg.random_plies_max);
        let mut adjudicator = Adjudicator::new(&self.cfg);
        let mut positions: Vec<(String, i16)> = Vec::new();
        let (mut ply, mut halfmove_clock, mut filtered) = (0u32, 0u32, 0u32);

        let outcome = loop {
            let (moves, in_check) = self.legal_moves(&mut board);
            if let Some(outcome) = game_end(
                &board,
                moves.len,
                in_check,
                halfmove_clock,
                ply,
                self.cfg.max_moves,
            ) {
                break outcome;
            }

            let turn = if ply < random_plies {
                if self.rng.gen_range(0..100) < self.cfg.random_prob {
                    moves.moves[self.rng.gen_range(0..moves.len)]
                } else {
                    self.search(&mut board).0
                }
            } else {
                let (turn, score) = self.search(&mut board);
                if ply == random_plies && score.abs() > self.cfg.opening_limit {
                    return GameRecord {
                        lines: None,
                        plies: ply,
                        filtered,
                        outcome: None,
                    };
                }
                if is_recordable(in_check, &turn, score) {
                    let fen = self.service.fen.get_fen(&board);
                    positions.push((fen_with_clocks(&fen, halfmove_clock, full_move(ply)), score));
                } else {
                    filtered += 1;
                }
                if let Some(outcome) = adjudicator.observe(score, full_move(ply) as i32) {
                    break outcome;
                }
                turn
            };

            let pawn_move = matches!(board.mailbox[turn.from as usize], 10 | 20);
            halfmove_clock = if pawn_move || turn.capture != 0 {
                0
            } else {
                halfmove_clock + 1
            };
            board.do_move(&turn);
            ply += 1;
        };

        let lines = positions
            .iter()
            .map(|(fen, score)| format_line(fen, *score, outcome))
            .collect();
        GameRecord {
            lines: Some(lines),
            plies: ply,
            filtered,
            outcome: Some(outcome),
        }
    }
}

impl Counters {
    fn add(&mut self, game: &GameRecord) {
        self.plies += game.plies as u64;
        self.filtered += game.filtered as u64;
        let Some(lines) = &game.lines else {
            self.dropped += 1;
            return;
        };
        self.games += 1;
        self.positions += lines.len() as u64;
        match game.outcome {
            Some(Outcome::WhiteWin) => self.white_wins += 1,
            Some(Outcome::Draw) => self.draws += 1,
            Some(Outcome::BlackWin) => self.black_wins += 1,
            None => {}
        }
    }

    fn report(&self, seconds: f64) -> String {
        let games = self.games.max(1) as f64;
        let searched = (self.positions + self.filtered).max(1) as f64;
        format!(
            "games {} | dropped {} | positions {} | filtered {:.1} % | W/D/L {:.1}/{:.1}/{:.1} % | \
             plies/game {:.0} | {:.0} pos/s",
            self.games,
            self.dropped,
            self.positions,
            100.0 * self.filtered as f64 / searched,
            100.0 * self.white_wins as f64 / games,
            100.0 * self.draws as f64 / games,
            100.0 * self.black_wins as f64 / games,
            self.plies as f64 / (self.games + self.dropped).max(1) as f64,
            self.positions as f64 / seconds.max(1e-9),
        )
    }
}

/// Plays `cfg.games` recorded games and appends their positions to `cfg.out`.
pub fn run(cfg: DatagenConfig) -> std::io::Result<Counters> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&cfg.out)?;
    let mut out = std::io::BufWriter::new(file);
    let start = std::time::Instant::now();
    let (games, report_every) = (cfg.games, cfg.report_every);
    let mut generator = Generator::new(cfg);
    let mut counters = Counters::default();

    while counters.games < games {
        let game = generator.play_game();
        if let Some(lines) = &game.lines {
            for line in lines {
                writeln!(out, "{line}")?;
            }
            // Whole games only: an interrupted run never leaves half a game behind.
            out.flush()?;
        }
        counters.add(&game);
        if game.lines.is_some() && report_every > 0 && counters.games % report_every == 0 {
            eprintln!("{}", counters.report(start.elapsed().as_secs_f64()));
        }
    }
    eprintln!("done: {}", counters.report(start.elapsed().as_secs_f64()));
    Ok(counters)
}

/// Entry point of `suprah datagen <args>`; returns the process exit code.
pub fn main(args: &[String]) -> i32 {
    let cfg = match parse_args(args) {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            print!("{USAGE}");
            return 0;
        }
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            return 2;
        }
    };
    eprintln!("datagen: {cfg:?}");
    match run(cfg) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen_service::FenService;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn quiet_move() -> Turn {
        crate::notation_util::NotationUtil::get_turn_from_notation("e2e4")
    }

    #[test]
    fn arguments_parse_with_defaults_and_reject_nonsense() {
        let cfg = parse_args(&args(&["--out", "d.txt"])).unwrap().unwrap();
        assert_eq!(
            (cfg.out.as_str(), cfg.nodes, cfg.win_cp, cfg.win_plies),
            ("d.txt", 5_000, 2_000, 4)
        );
        let cfg = parse_args(&args(&[
            "--out",
            "d.txt",
            "--games",
            "7",
            "--seed",
            "3",
            "--nodes",
            "800",
            "--random-plies",
            "6-10",
            "--random-prob",
            "100",
            "--win",
            "1500/3",
            "--draw",
            "5/6/30",
            "--max-moves",
            "300",
            "--hash",
            "4096",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!((cfg.games, cfg.seed, cfg.nodes), (7, 3, 800));
        assert_eq!(
            (cfg.random_plies_min, cfg.random_plies_max, cfg.random_prob),
            (6, 10, 100)
        );
        assert_eq!(
            (
                cfg.win_cp,
                cfg.win_plies,
                cfg.draw_cp,
                cfg.draw_plies,
                cfg.draw_from_move
            ),
            (1500, 3, 5, 6, 30)
        );
        assert_eq!((cfg.max_moves, cfg.hash_entries), (300, 4096));
        assert_eq!(parse_args(&args(&["--help"])), Ok(None));
        for bad in [
            &["--games", "5"][..],
            &["--out", "d.txt", "--nodes", "0"],
            &["--out", "d.txt", "--random-plies", "12-8"],
            &["--out", "d.txt", "--random-prob", "101"],
            &["--out", "d.txt", "--win", "2000"],
            &["--out", "d.txt", "--bogus", "1"],
            &["--out", "d.txt", "--nodes"],
        ] {
            assert!(parse_args(&args(bad)).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn lines_carry_score_and_result_from_whites_view() {
        let fen = "4k3/8/8/8/8/8/8/3QK3 b - - 0 1";
        assert_eq!(
            format_line(fen, 950, Outcome::WhiteWin),
            format!("{fen} | 950 | 1.0")
        );
        assert_eq!(
            format_line(fen, -12, Outcome::Draw),
            format!("{fen} | -12 | 0.5")
        );
        assert_eq!(
            format_line(fen, -700, Outcome::BlackWin),
            format!("{fen} | -700 | 0.0")
        );
    }

    #[test]
    fn only_quiet_positions_with_centipawn_scores_are_recorded() {
        let quiet = quiet_move();
        let mut capture = quiet;
        capture.capture = 21;
        let mut promotion = crate::notation_util::NotationUtil::get_turn_from_notation("a7a8q");
        promotion.capture = 0;
        assert!(promotion.is_promotion());

        assert!(is_recordable(false, &quiet, 35));
        assert!(!is_recordable(true, &quiet, 35));
        assert!(!is_recordable(false, &capture, 35));
        assert!(!is_recordable(false, &promotion, 35));
        assert!(!is_recordable(false, &quiet, MATE_SCORE_THRESHOLD));
        assert!(!is_recordable(false, &quiet, -MATE_SCORE_THRESHOLD - 5));
    }

    #[test]
    fn a_win_needs_the_threshold_on_consecutive_plies() {
        let cfg = DatagenConfig::default();
        let mut adj = Adjudicator::new(&cfg);
        assert_eq!(adj.observe(2100, 30), None);
        assert_eq!(adj.observe(2500, 31), None);
        assert_eq!(adj.observe(1999, 31), None); // streak broken
        for _ in 0..3 {
            assert_eq!(adj.observe(2000, 32), None);
        }
        assert_eq!(adj.observe(3000, 33), Some(Outcome::WhiteWin));

        let mut adj = Adjudicator::new(&cfg);
        assert_eq!(adj.observe(-2500, 20), None);
        assert_eq!(adj.observe(2500, 20), None); // sign change restarts
        for _ in 0..3 {
            assert_eq!(adj.observe(-2600, 21), None);
        }
        assert_eq!(adj.observe(-2600, 22), Some(Outcome::BlackWin));
    }

    #[test]
    fn a_draw_needs_a_small_score_late_enough_on_consecutive_plies() {
        let cfg = DatagenConfig::default();
        let mut adj = Adjudicator::new(&cfg);
        for _ in 0..20 {
            assert_eq!(adj.observe(0, 39), None); // too early
        }
        for _ in 0..7 {
            assert_eq!(adj.observe(-10, 40), None);
        }
        assert_eq!(adj.observe(11, 41), None); // streak broken
        for _ in 0..7 {
            assert_eq!(adj.observe(10, 42), None);
        }
        assert_eq!(adj.observe(3, 43), Some(Outcome::Draw));
    }

    #[test]
    fn dead_positions_are_recognised() {
        let dead = [
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/2N1K3 w - - 0 1",
            "4kb2/8/8/8/8/8/8/4K3 b - - 0 1",
            "2b1k3/8/8/8/8/8/8/3BK3 w - - 0 1", // c8 and d1 are both light squares
        ];
        let alive = [
            "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/R3K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/1NN1K3 w - - 0 1",
            "3bk3/8/8/8/8/8/8/3BK3 w - - 0 1", // d8 dark, d1 light
            "4k3/8/8/8/8/8/8/1N1BK3 w - - 0 1",
        ];
        for fen in dead {
            assert!(
                insufficient_material(&FenService.set_fen(fen)),
                "{fen} should be dead"
            );
        }
        for fen in alive {
            assert!(
                !insufficient_material(&FenService.set_fen(fen)),
                "{fen} is not dead"
            );
        }
    }

    fn small(seed: u64) -> DatagenConfig {
        DatagenConfig {
            out: "unused".to_string(),
            seed,
            nodes: 300,
            hash_entries: 1 << 14,
            ..DatagenConfig::default()
        }
    }

    #[test]
    fn clocks_replace_the_last_two_fen_fields() {
        let fen = "4k3/8/8/8/8/8/8/3QK3 b - - 0 1";
        assert_eq!(
            fen_with_clocks(fen, 13, 57),
            "4k3/8/8/8/8/8/8/3QK3 b - - 13 57"
        );
        assert_eq!(
            (full_move(0), full_move(1), full_move(2), full_move(17)),
            (1, 1, 2, 9)
        );
    }

    #[test]
    fn games_end_by_the_rules() {
        let generator = Generator::new(small(1));
        let verdict = |fen: &str, halfmove: u32, ply: u32| {
            let mut board = FenService.set_fen(fen);
            let (moves, in_check) = generator.legal_moves(&mut board);
            game_end(&board, moves.len, in_check, halfmove, ply, 400)
        };
        // Fool's mate: White is mated.
        assert_eq!(
            verdict(
                "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
                0,
                4
            ),
            Some(Outcome::BlackWin)
        );
        // Black is mated.
        assert_eq!(
            verdict("6Qk/6K1/8/8/8/8/8/8 b - - 0 1", 0, 60),
            Some(Outcome::WhiteWin)
        );
        assert_eq!(
            verdict("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", 0, 60),
            Some(Outcome::Draw)
        ); // stalemate
        assert_eq!(
            verdict("4k3/8/8/8/8/8/8/2N1K3 w - - 0 1", 0, 60),
            Some(Outcome::Draw)
        ); // dead
        assert_eq!(
            verdict("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", 100, 160),
            Some(Outcome::Draw)
        ); // fifty moves
        assert_eq!(verdict("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", 99, 160), None);
        assert_eq!(verdict("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", 0, 798), None); // move 400
        assert_eq!(
            verdict("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", 0, 800),
            Some(Outcome::Draw)
        ); // move 401
    }

    #[test]
    fn search_scores_are_from_whites_view() {
        let generator = Generator::new(small(1));
        for (fen, white_ahead) in [
            ("4k3/8/8/8/8/8/8/3QK3 w - - 0 1", true),
            ("4k3/8/8/8/8/8/8/3QK3 b - - 0 1", true),
            ("3qk3/8/8/8/8/8/8/4K3 w - - 0 1", false),
            ("3qk3/8/8/8/8/8/8/4K3 b - - 0 1", false),
        ] {
            let (_, score) = generator.search(&mut FenService.set_fen(fen));
            assert_eq!(score > 300, white_ahead, "{fen}: {score}");
            assert_eq!(score < -300, !white_ahead, "{fen}: {score}");
        }
    }

    fn play(seed: u64, games: usize) -> Vec<String> {
        let mut generator = Generator::new(small(seed));
        let mut lines = Vec::new();
        let mut played = 0;
        while played < games {
            if let Some(game) = generator.play_game().lines {
                lines.extend(game);
                played += 1;
            }
        }
        lines
    }

    #[test]
    fn games_are_recorded_consistently_and_reproducibly() {
        let lines = play(7, 2);
        assert!(lines.len() > 20, "only {} positions", lines.len());
        let min_ply = DatagenConfig::default().random_plies_min;
        let results: std::collections::HashSet<&str> = lines
            .iter()
            .map(|l| l.rsplit(" | ").next().unwrap())
            .collect();
        assert!(results.len() <= 2, "one result per game: {results:?}");
        for line in &lines {
            let fields: Vec<&str> = line.split(" | ").collect();
            assert_eq!(fields.len(), 3, "{line}");
            let fen = fields[0];
            let score: i16 = fields[1].parse().unwrap();
            assert!(score.abs() < MATE_SCORE_THRESHOLD);
            assert!(["1.0", "0.5", "0.0"].contains(&fields[2]), "{line}");
            let board = FenService.set_fen(fen);
            assert_eq!(
                FenService.get_fen(&board).split(' ').next(),
                fen.split(' ').next(),
                "{fen}"
            );
            let f: Vec<&str> = fen.split(' ').collect();
            let ply = 2 * (f[5].parse::<u32>().unwrap() - 1) + u32::from(f[1] == "b");
            assert!(ply >= min_ply, "opening ply {ply} recorded: {line}");
        }
        assert_eq!(lines, play(7, 2), "same seed, same data");
        assert_ne!(lines, play(8, 2), "other seed, other data");
    }

    #[test]
    fn a_run_appends_whole_games_and_counts_them() {
        let path =
            std::env::temp_dir().join(format!("suprah-datagen-test-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let cfg = DatagenConfig {
            out: path.to_string_lossy().into_owned(),
            games: 2,
            report_every: 0,
            ..small(3)
        };
        let counters = run(cfg.clone()).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(counters.games, 2);
        assert_eq!(
            counters.white_wins + counters.draws + counters.black_wins,
            2
        );
        assert_eq!(text.lines().count() as u64, counters.positions);
        run(cfg).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap().lines().count() as u64,
            2 * counters.positions
        );
        std::fs::remove_file(&path).unwrap();
    }
}
