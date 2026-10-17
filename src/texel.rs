//! The Texel tuner of the hand-crafted evaluation (`task.md` 29.5 E2), feature `texel`.
//!
//! * `texel-extract <out> [--per-game N] [--seed S] <pgn>... [--test <pgn>...]` - quiet positions
//!   from finished games, each traced by the evaluation, into a binary data set. Games from the
//!   files after `--test` form the test set; the rest are split into training and validation by
//!   game.
//! * `texel-check <data>` - rebuilds every evaluation from its trace at the shipped parameters and
//!   compares it with the evaluation the engine computed.
//! * `texel-tune <data> <out-dir> [--epochs E] [--lr L] [--lambda X...] [--free G,...]` - fits the
//!   parameters (Adam, full batch, `K` fixed on the shipped evaluation), writes `theta.txt` and a
//!   report. `--free` names the groups that move - `material`, `tables`, `passers`, `scalars`,
//!   all four by default; the others keep their shipped values. `--tables` names the pieces whose
//!   tables move (`pawn,rook,knight,bishop,queen,king`, all by default).
//! * `texel-export <theta.txt>` - writes the parameters into `src/pst.rs`, `src/config.rs` and
//!   `tuning/parameters.json`; run from the repository root.
//! * `texel-stats <data> [theta.txt]` - on the test set: how often the positional cap fires, the
//!   size of the evaluations, the error; at the shipped parameters or at `theta.txt`.
//! * `texel-lazy <fen file>` - with the compiled parameters, how far `cheap_eval` lies from the
//!   full evaluation on positions sampled from a search (`fen;alpha;beta` per line), against the
//!   lazy margins.
//! * `texel-mirror <fen file>` - every position against its colour mirror: the evaluations must
//!   be exact negatives.
//!
//! The model is the evaluation's own pipeline: the traced sum is tapered, then the positional cap,
//! the opposite-bishop scale and `adjust_eval` are applied as the engine applies them. Positions a
//! mop-up or insufficient material would score are not extracted.
use crate::config::{Aggressiveness, Config};
use crate::eval_trace as et;
use crate::model::Board;
use crate::service::Service;
use std::io::{BufReader, BufWriter, Read, Write};

pub fn run(args: &[String]) {
    match args[0].as_str() {
        "texel-extract" => extract(&args[1..]),
        "texel-check" => check(&args[1]),
        "texel-tune" => tune(&args[1..]),
        "texel-export" => export_file(&args[1]),
        "texel-stats" => stats(&args[1], args.get(2).map(String::as_str)),
        "texel-lazy" => lazy(&args[1]),
        "texel-mirror" => mirror(&args[1]),
        other => panic!("unknown texel command {other}"),
    }
}

fn hce_config() -> Config {
    let mut config = Config::new();
    config.use_nnue = false;
    config.print_eval_per_figure = false;
    config
}

// ---------------------------------------------------------------------------------------------
// The data set

/// One traced position, as the tuner holds it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sample {
    /// 0 Black won, 1 draw, 2 White won.
    pub result: u8,
    /// 0 training, 1 validation, 2 test.
    pub set: u8,
    pub phase: u8,
    pub opposite_bishops: bool,
    /// The evaluation the engine computed, White-positive.
    pub eval: i16,
    /// What the per-unit integer tapers rounded away at the shipped parameters.
    pub taper_residual: f32,
    pub constant: (f32, f32),
    pub pieces: Vec<(u16, i8)>,
    pub terms: Vec<(u16, f32, f32)>,
}

impl Sample {
    fn label(&self) -> f64 {
        self.result as f64 / 2.0
    }
}

/// Traces one position. `None` for positions the model does not cover.
pub fn trace_position(service: &Service, config: &Config, theta0: &[f64], board: &Board) -> Option<Sample> {
    if crate::eval_service::EvalService::is_insufficient_material(board) || has_bare_king(board) {
        return None;
    }
    let pawn_table = crate::pawn_hash::PawnHashTable::new(16);
    et::begin(theta0);
    let eval = service.eval.calc_eval_known_check(board, config, &service.move_gen, &pawn_table,
        i16::MIN + 1, i16::MAX - 1, 0, false);
    let trace = et::finish();
    let pre_cap = trace.pre_cap.expect("the trace saw no positional cap");
    assert_eq!(trace.final_eval, Some(eval as i32), "the trace did not see the final evaluation");
    assert_eq!(trace.tapered_sum, pre_cap, "a contribution to the evaluation bypassed the trace units");
    let phase = phase_of(board);
    let mut sample = Sample {
        result: 1,
        set: 0,
        phase: phase as u8,
        opposite_bishops: opposite_bishops(board),
        eval,
        taper_residual: 0.0,
        constant: (trace.constant.0 as f32, trace.constant.1 as f32),
        pieces: trace.pieces.clone(),
        terms: trace.terms.iter().map(|&(i, m, e)| (i as u16, m as f32, e as f32)).collect(),
    };
    let (mg, eg) = sums(&sample, theta0);
    sample.taper_residual = (pre_cap as f64 - taper(mg, eg, phase)) as f32;
    Some(sample)
}

fn has_bare_king(board: &Board) -> bool {
    use crate::model::{BLACK_KING, WHITE_KING};
    let white = board.white_pieces & !board.bitboards[WHITE_KING];
    let black = board.black_pieces & !board.bitboards[BLACK_KING];
    white == 0 || black == 0
}

fn phase_of(board: &Board) -> i16 {
    let b = &board.bitboards;
    let n = (b[2] | b[8]).count_ones() + (b[3] | b[9]).count_ones() + 2 * (b[1] | b[7]).count_ones()
        + 4 * (b[4] | b[10]).count_ones();
    (n.min(24) * 255 / 24) as i16
}

fn opposite_bishops(board: &Board) -> bool {
    use crate::model::*;
    let (wb, bb) = (board.bitboards[WHITE_BISHOP], board.bitboards[BLACK_BISHOP]);
    if wb.count_ones() != 1 || bb.count_ones() != 1 {
        return false;
    }
    let others = board.bitboards[WHITE_KNIGHT] | board.bitboards[WHITE_ROOK] | board.bitboards[WHITE_QUEEN]
        | board.bitboards[BLACK_KNIGHT] | board.bitboards[BLACK_ROOK] | board.bitboards[BLACK_QUEEN];
    if others != 0 {
        return false;
    }
    let colour = |sq: u32| (sq % 8 + sq / 8) % 2;
    colour(wb.trailing_zeros()) != colour(bb.trailing_zeros())
}

// ---------------------------------------------------------------------------------------------
// The model

#[inline(always)]
fn taper(mg: f64, eg: f64, phase: i16) -> f64 {
    (mg * phase as f64 + eg * (256 - phase) as f64) / 256.0
}

/// The untapered middlegame and endgame sums of a sample at `theta`.
#[inline(always)]
fn sums(s: &Sample, theta: &[f64]) -> (f64, f64) {
    let (mut mg, mut eg) = (s.constant.0 as f64, s.constant.1 as f64);
    for &(i, cm, ce) in &s.terms {
        mg += theta[i as usize] * cm as f64;
        eg += theta[i as usize] * ce as f64;
    }
    for &(code, sign) in &s.pieces {
        let (piece, visual) = ((code / 64) as usize, (code % 64) as usize);
        let sign = sign as f64;
        mg += sign * (theta[et::MATERIAL_MG + piece] + theta[et::TABLE_MG + piece * 64 + visual]);
        eg += sign * (theta[et::MATERIAL_EG + piece] + theta[et::TABLE_EG + piece * 64 + visual]);
    }
    (mg, eg)
}

/// The material sums of a sample at `theta`, as the positional cap reads them.
#[inline(always)]
fn material_sums(s: &Sample, theta: &[f64]) -> (f64, f64) {
    let (mut mg, mut eg) = (0.0, 0.0);
    for &(code, sign) in &s.pieces {
        let piece = (code / 64) as usize;
        if piece < 5 {
            mg += sign as f64 * theta[et::MATERIAL_MG + piece];
            eg += sign as f64 * theta[et::MATERIAL_EG + piece];
        }
    }
    (mg, eg)
}

/// The fixed part of the pipeline: the cap, the opposite-bishop scale and `adjust_eval`.
#[derive(Clone, Copy)]
pub struct Pipeline {
    cap: Option<(f64, f64)>,
    opposite_bishops_scale: f64,
    max_mult: f64,
}

impl Pipeline {
    pub fn of(config: &Config) -> Self {
        let cap = match config.aggressiveness {
            Aggressiveness::Normal => 150.0,
            Aggressiveness::Aggressive => 250.0,
            Aggressiveness::HighAggressive => 400.0,
        };
        let damping = if config.positional_cap_damping > 0 { config.positional_cap_damping as f64 } else { 1.0 };
        Pipeline {
            cap: if config.enable_positional_cap { Some((cap, damping)) } else { None },
            opposite_bishops_scale: config.opposite_bishops_draw_scale as f64 / 100.0,
            max_mult: config.max_eval_mult as f64,
        }
    }

    /// The evaluation at `theta`, and its derivatives with respect to the tapered sum and to the
    /// tapered material.
    #[inline(always)]
    fn eval(&self, s: &Sample, theta: &[f64]) -> (f64, f64, f64) {
        let phase = s.phase as i16;
        let (mg, eg) = sums(s, theta);
        let pre = taper(mg, eg, phase) + s.taper_residual as f64;
        let (mut d_pre, mut d_mat) = (1.0, 0.0);
        let mut eval = pre;
        if let Some((cap, damping)) = self.cap {
            let (mm, me) = material_sums(s, theta);
            let material = taper(mm, me, phase);
            let positional = pre - material;
            if positional.abs() > cap {
                eval = material + positional.signum() * (cap + (positional.abs() - cap) / damping);
                d_pre = 1.0 / damping;
                d_mat = 1.0 - 1.0 / damping;
            }
        }
        if s.opposite_bishops {
            eval *= self.opposite_bishops_scale;
            d_pre *= self.opposite_bishops_scale;
            d_mat *= self.opposite_bishops_scale;
        }
        if phase + 100 < 255 && eval.abs() >= 200.0 {
            let mult = (255.0 / (phase + 100) as f64).min(self.max_mult);
            eval *= mult;
            d_pre *= mult;
            d_mat *= mult;
        }
        (eval, d_pre, d_mat)
    }
}

fn sigmoid(k: f64, e: f64) -> f64 {
    1.0 / (1.0 + (-k * e).exp())
}

fn mse(samples: &[Sample], set: u8, k: f64, pipeline: &Pipeline, theta: &[f64]) -> f64 {
    let (mut sum, mut n) = (0.0, 0usize);
    for s in samples.iter().filter(|s| s.set == set) {
        let d = s.label() - sigmoid(k, pipeline.eval(s, theta).0);
        sum += d * d;
        n += 1;
    }
    sum / n.max(1) as f64
}

fn fit_k(samples: &[Sample], pipeline: &Pipeline, theta: &[f64]) -> f64 {
    let (mut a, mut b) = (0.0005f64, 0.02f64);
    let g = 0.618_033_988_75;
    for _ in 0..50 {
        let c = b - g * (b - a);
        let d = a + g * (b - a);
        if mse(samples, 0, c, pipeline, theta) < mse(samples, 0, d, pipeline, theta) { b = d; } else { a = c; }
    }
    (a + b) / 2.0
}

/// The gradient of the training error at `theta`, summed over `samples` (not divided).
fn gradient(samples: &[Sample], k: f64, pipeline: &Pipeline, theta: &[f64], grad: &mut [f64]) {
    for s in samples.iter().filter(|s| s.set == 0) {
        let (eval, d_pre, d_mat) = pipeline.eval(s, theta);
        let p = sigmoid(k, eval);
        let g = -2.0 * (s.label() - p) * p * (1.0 - p) * k;
        let w_mg = s.phase as f64 / 256.0;
        let w_eg = 1.0 - w_mg;
        let a = g * d_pre;
        for &(i, cm, ce) in &s.terms {
            grad[i as usize] += a * (cm as f64 * w_mg + ce as f64 * w_eg);
        }
        let b = g * d_mat;
        for &(code, sign) in &s.pieces {
            let (piece, visual) = ((code / 64) as usize, (code % 64) as usize);
            let sign = sign as f64;
            grad[et::TABLE_MG + piece * 64 + visual] += a * sign * w_mg;
            grad[et::TABLE_EG + piece * 64 + visual] += a * sign * w_eg;
            grad[et::MATERIAL_MG + piece] += (a + if piece < 5 { b } else { 0.0 }) * sign * w_mg;
            grad[et::MATERIAL_EG + piece] += (a + if piece < 5 { b } else { 0.0 }) * sign * w_eg;
        }
    }
}

/// Scalars that depend on nothing but a piece and its square. The piece-square table expresses
/// each of them exactly, so fitting both leaves a direction the error cannot see, and the two drift
/// apart without bound (first run: the rook on the seventh 33 -> 474 against table entries near
/// -500). They keep their shipped values and the tables carry their part.
const PIECE_SQUARE_SCALARS: &[et::P] = &[
    et::P::PawnCentered, et::P::PawnOnLastRankBonus, et::P::PawnOnBeforeLastRankBonus,
    et::P::PawnOnBeforeBeforeLastRankBonus, et::P::PawnUndevelopedMalus, et::P::KnightOnRimMalus,
    et::P::KnightCentered, et::P::UndevelopedKnightMalus, et::P::UndevelopedBishopMalus,
    et::P::UndevelopedKingMalus, et::P::KingTrappAtBaselineMalus, et::P::RookOnSeventh,
];

/// The parameter groups `texel-tune --free` can set free; the others keep their shipped values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Groups {
    material: bool,
    tables: bool,
    passers: bool,
    scalars: bool,
    /// Whose tables move when `tables` is free, in `WHITE_PAWN..=WHITE_KING` order.
    table_pieces: [bool; 6],
}

/// `material,tables,passers,scalars`, any subset, in any order.
fn parse_groups(list: &str) -> Groups {
    let mut groups = Groups { material: false, tables: false, passers: false, scalars: false, table_pieces: [true; 6] };
    for name in list.split(',') {
        match name.trim() {
            "material" => groups.material = true,
            "tables" => groups.tables = true,
            "passers" => groups.passers = true,
            "scalars" => groups.scalars = true,
            other => panic!("unknown parameter group {other}"),
        }
    }
    groups
}

/// `pawn,rook,knight,bishop,queen,king`, any subset: the pieces whose tables move.
fn parse_table_pieces(list: &str) -> [bool; 6] {
    let mut pieces = [false; 6];
    for name in list.split(',') {
        let p = ["pawn", "rook", "knight", "bishop", "queen", "king"].iter().position(|&n| n == name.trim())
            .unwrap_or_else(|| panic!("unknown piece {name}"));
        pieces[p] = true;
    }
    pieces
}

/// Which parameters move: every scalar but the piece-square ones, the passed-pawn table on ranks
/// 2 to 7, material without the king, every table square a piece can stand on - each only if its
/// group is free.
fn trainable_for(groups: &Groups) -> Vec<bool> {
    let mut t = vec![false; et::N_PARAMS];
    if groups.scalars {
        for v in t.iter_mut().take(et::N_SCALARS) {
            *v = true;
        }
        for &p in PIECE_SQUARE_SCALARS {
            t[p as usize] = false;
        }
    }
    if groups.passers {
        for r in 1..7 {
            t[et::PASSER_MG + r] = true;
            t[et::PASSER_EG + r] = true;
        }
    }
    if groups.material {
        for p in 0..5 {
            t[et::MATERIAL_MG + p] = true;
            t[et::MATERIAL_EG + p] = true;
        }
    }
    if groups.tables {
        for p in (0..6).filter(|&p| groups.table_pieces[p]) {
            for visual in 0..64 {
                let pawn_rank_ok = p != 0 || (8..56).contains(&visual);
                t[et::TABLE_MG + p * 64 + visual] = pawn_rank_ok;
                t[et::TABLE_EG + p * 64 + visual] = pawn_rank_ok;
            }
        }
    }
    t
}

/// Every group free, as step 26 (`task.md` 29.6) tuned.
#[cfg(test)]
fn trainable() -> Vec<bool> {
    trainable_for(&parse_groups("material,tables,passers,scalars"))
}

/// The mean of each table over the squares its piece can stand on.
fn table_means(theta: &[f64], trainable: &[bool]) -> Vec<f64> {
    let mut means = Vec::with_capacity(12);
    for base in [et::TABLE_MG, et::TABLE_EG] {
        for p in 0..6 {
            let range = base + p * 64..base + p * 64 + 64;
            let n = range.clone().filter(|&i| trainable[i]).count() as f64;
            // A held table has no squares to average; its mean is not used.
            means.push(if n > 0.0 { range.filter(|&i| trainable[i]).map(|i| theta[i]).sum::<f64>() / n } else { 0.0 });
        }
    }
    means
}

/// Keeps every table's mean at `means`: whatever a step added to a table uniformly is the piece's
/// material and moves there. Material plus table, what the evaluation sums, is unchanged; only
/// the split the positional cap reads is fixed. The king's material cancels and does not move.
fn recentre_tables(theta: &mut [f64], trainable: &[bool], means: &[f64]) {
    // With material held there is nowhere to move a shift to, and the tables keep whatever mean
    // the error gives them; a held table has no shift to move.
    if !trainable[et::MATERIAL_MG] {
        return;
    }
    let now = table_means(theta, trainable);
    for (k, (base, material)) in [(et::TABLE_MG, et::MATERIAL_MG), (et::TABLE_EG, et::MATERIAL_EG)].into_iter().enumerate() {
        for p in 0..6 {
            if !(base + p * 64..base + p * 64 + 64).any(|i| trainable[i]) {
                continue;
            }
            let shift = now[k * 6 + p] - means[k * 6 + p];
            for i in base + p * 64..base + p * 64 + 64 {
                if trainable[i] {
                    theta[i] -= shift;
                }
            }
            if p < 5 {
                theta[material + p] += shift;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Binary data set

fn write_sample(w: &mut impl Write, s: &Sample) -> std::io::Result<()> {
    w.write_all(&[s.result, s.set, s.phase, s.opposite_bishops as u8])?;
    w.write_all(&s.eval.to_le_bytes())?;
    w.write_all(&s.taper_residual.to_le_bytes())?;
    w.write_all(&s.constant.0.to_le_bytes())?;
    w.write_all(&s.constant.1.to_le_bytes())?;
    w.write_all(&[s.pieces.len() as u8, s.terms.len() as u8])?;
    for &(code, sign) in &s.pieces {
        w.write_all(&code.to_le_bytes())?;
        w.write_all(&[sign as u8])?;
    }
    for &(i, m, e) in &s.terms {
        w.write_all(&i.to_le_bytes())?;
        w.write_all(&m.to_le_bytes())?;
        w.write_all(&e.to_le_bytes())?;
    }
    Ok(())
}

pub fn read_samples(path: &str) -> Vec<Sample> {
    let mut bytes = Vec::new();
    BufReader::new(std::fs::File::open(path).unwrap()).read_to_end(&mut bytes).unwrap();
    let mut at = 0usize;
    let mut take = |n: usize| { let s = &bytes[at..at + n]; at += n; s.to_vec() };
    let magic = take(8);
    assert_eq!(&magic, b"TEXEL001", "not a texel data set");
    let count = u64::from_le_bytes(take(8).try_into().unwrap()) as usize;
    let mut samples = Vec::with_capacity(count);
    let f32_of = |b: Vec<u8>| f32::from_le_bytes(b.try_into().unwrap());
    for _ in 0..count {
        let h = take(4);
        let eval = i16::from_le_bytes(take(2).try_into().unwrap());
        let taper_residual = f32_of(take(4));
        let constant = (f32_of(take(4)), f32_of(take(4)));
        let n = take(2);
        let mut pieces = Vec::with_capacity(n[0] as usize);
        for _ in 0..n[0] {
            let code = u16::from_le_bytes(take(2).try_into().unwrap());
            pieces.push((code, take(1)[0] as i8));
        }
        let mut terms = Vec::with_capacity(n[1] as usize);
        for _ in 0..n[1] {
            let i = u16::from_le_bytes(take(2).try_into().unwrap());
            terms.push((i, f32_of(take(4)), f32_of(take(4))));
        }
        samples.push(Sample { result: h[0], set: h[1], phase: h[2], opposite_bishops: h[3] != 0, eval,
            taper_residual, constant, pieces, terms });
    }
    samples
}

// ---------------------------------------------------------------------------------------------
// texel-extract

fn token_to_uci(tok: &str, white: bool) -> Option<String> {
    let t: String = tok.chars().filter(|c| !matches!(c, '+' | '#' | '!' | '?')).collect();
    if t == "0-0" || t == "O-O" { return Some(if white { "e1g1" } else { "e8g8" }.into()); }
    if t == "0-0-0" || t == "O-O-O" { return Some(if white { "e1c1" } else { "e8c8" }.into()); }
    let t = t.trim_start_matches(['K', 'Q', 'R', 'B', 'N']);
    if let Some(eq) = t.find('=') {
        let promo = t[eq + 1..].chars().next()?.to_ascii_lowercase();
        let body = &t[..eq];
        let from_rank = if white { '7' } else { '2' };
        return if let Some(x) = body.find('x') {
            Some(format!("{}{}{}{}", body.chars().next()?, from_rank, &body[x + 1..], promo))
        } else if body.len() == 2 {
            Some(format!("{}{}{}{}", &body[..1], from_rank, body, promo))
        } else if body.len() == 4 {
            Some(format!("{}{}", body, promo))
        } else {
            None
        };
    }
    let t: String = t.chars().filter(|&c| c != 'x' && c != '-').collect();
    if t.len() == 4 { Some(t) } else { None }
}

/// A capture the side to move wins material with, by the engine's own SEE.
fn has_winning_capture(service: &Service, config: &Config, board: &Board) -> bool {
    let white = board.white_to_move;
    let mut enemies = if white { board.black_pieces } else { board.white_pieces };
    enemies &= !(board.bitboards[crate::model::WHITE_KING] | board.bitboards[crate::model::BLACK_KING]);
    while enemies != 0 {
        let sq = enemies.trailing_zeros() as u8;
        let mut attackers = service.move_gen.get_attackers_mask(board, !white, sq, board.occupied);
        while attackers != 0 {
            let from = attackers.trailing_zeros() as u8;
            let turn = crate::model::Turn::new(from, sq, board.get_piece_at(sq), 0, false, 0);
            if service.search.see(board, &turn, config, &service.move_gen) > 0 {
                return true;
            }
            attackers &= attackers - 1;
        }
        enemies &= enemies - 1;
    }
    false
}

/// The FENs of the quiet positions of one game.
fn quiet_positions(service: &Service, config: &Config, moves: &str, min_ply: usize) -> Option<Vec<String>> {
    let mut board = service.fen.set_init_board();
    let mut prev_capture = false;
    let mut out = Vec::new();
    for (ply, tok) in moves.split_whitespace().filter(|t| !t.ends_with('.') && !t.contains("...")).enumerate() {
        if matches!(tok, "1-0" | "0-1" | "1/2-1/2" | "*") {
            break;
        }
        let white = board.white_to_move;
        let uci = token_to_uci(tok, white)?;
        let turn = crate::notation_util::NotationUtil::get_turn_from_notation(&uci);
        let mover = board.get_piece_at(turn.from);
        if mover == 0 || (white && mover >= 20) || (!white && mover < 20) {
            return None;
        }
        let victim = board.get_piece_at(turn.to);
        let en_passant = (mover == 10 || mover == 20) && turn.from % 8 != turn.to % 8 && victim == 0;
        let capture = victim != 0 || en_passant;
        if ply >= min_ply && !capture && turn.promotion == 0 && !prev_capture
            && !service.move_gen.is_in_check(&board) && !has_winning_capture(service, config, &board)
        {
            out.push(service.fen.get_fen(&board));
        }
        board.do_move(&turn);
        prev_capture = capture;
    }
    Some(out)
}

fn games_of(path: &str) -> Vec<(u8, String)> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut games = Vec::new();
    let mut result: Option<u8> = None;
    let mut moves = String::new();
    let flush = |result: &mut Option<u8>, moves: &mut String, games: &mut Vec<(u8, String)>| {
        if let Some(r) = result.take() && !moves.trim().is_empty() {
            games.push((r, std::mem::take(moves)));
        }
        moves.clear();
    };
    for line in text.lines() {
        if line.starts_with("[Event ") {
            flush(&mut result, &mut moves, &mut games);
        } else if let Some(r) = line.strip_prefix("[Result \"") {
            result = match r.trim_end_matches("\"]") { "1-0" => Some(2), "0-1" => Some(0), "1/2-1/2" => Some(1), _ => None };
        } else if !line.starts_with('[') {
            moves.push(' ');
            moves.push_str(line);
        }
    }
    flush(&mut result, &mut moves, &mut games);
    games
}

fn extract(args: &[String]) {
    use rand::{SeedableRng, seq::SliceRandom};
    let out = &args[0];
    let (mut per_game, mut seed, mut test_mode) = (10usize, 20261016u64, false);
    let mut files: Vec<(String, bool)> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--per-game" => { per_game = args[i + 1].parse().unwrap(); i += 1; }
            "--seed" => { seed = args[i + 1].parse().unwrap(); i += 1; }
            "--test" => test_mode = true,
            path => files.push((path.to_string(), test_mode)),
        }
        i += 1;
    }
    let service = Service::new();
    let config = hce_config();
    let theta0 = et::theta_of(&config);
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut samples: Vec<Sample> = Vec::new();
    let (mut games, mut rejected, mut excluded) = (0usize, 0usize, 0usize);
    let mut worst_unit = 0.0f64;
    let mut units_over_one = 0usize;
    let mut units = 0usize;
    for (path, is_test) in &files {
        for (result, moves) in games_of(path) {
            let Some(mut fens) = quiet_positions(&service, &config, &moves, 16) else { rejected += 1; continue };
            games += 1;
            fens.shuffle(&mut rng);
            for fen in fens.into_iter().take(per_game) {
                let board = service.fen.set_fen(&fen);
                et::begin(&theta0);
                // Unit-level check of the hooks: what the coefficients predict against the sums.
                let pawn_table = crate::pawn_hash::PawnHashTable::new(16);
                service.eval.calc_eval_known_check(&board, &config, &service.move_gen, &pawn_table,
                    i16::MIN + 1, i16::MAX - 1, 0, false);
                let trace = et::finish();
                for u in &trace.units {
                    let r = (u.actual.0 - u.predicted.0).abs().max((u.actual.1 - u.predicted.1).abs());
                    units += 1;
                    if r > 1.0 {
                        units_over_one += 1;
                        if r > worst_unit {
                            worst_unit = r;
                            eprintln!("unit residual {r} at {fen}: {u:?}");
                        }
                    }
                }
                let Some(mut sample) = trace_position(&service, &config, &theta0, &board) else { excluded += 1; continue };
                sample.result = result;
                sample.set = if *is_test { 2 } else if games % 10 == 0 { 1 } else { 0 };
                samples.push(sample);
            }
        }
        eprintln!("{path}: {games} games so far, {} positions", samples.len());
    }
    let mut w = BufWriter::new(std::fs::File::create(out).unwrap());
    w.write_all(b"TEXEL001").unwrap();
    w.write_all(&(samples.len() as u64).to_le_bytes()).unwrap();
    for s in &samples {
        write_sample(&mut w, s).unwrap();
    }
    let by_set = |k: u8| samples.iter().filter(|s| s.set == k).count();
    println!("games {games} (rejected {rejected}), positions {} (train {}, validation {}, test {}), excluded {excluded}",
        samples.len(), by_set(0), by_set(1), by_set(2));
    println!("units {units}, residual above 1 cp {units_over_one}, worst {worst_unit:.2}");
}

// ---------------------------------------------------------------------------------------------
// texel-check

fn check(path: &str) {
    let samples = read_samples(path);
    let config = hce_config();
    let pipeline = Pipeline::of(&config);
    let theta0 = et::theta_of(&config);
    let mut diffs: Vec<f64> = samples.iter().map(|s| (pipeline.eval(s, &theta0).0 - s.eval as f64).abs()).collect();
    diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = diffs.len();
    let within3 = diffs.iter().filter(|&&d| d <= 3.0).count();
    println!("{n} positions: mean |model - engine| {:.3} cp, median {:.2}, 99.5% {:.2}, max {:.2}; within 3 cp {:.3}%",
        diffs.iter().sum::<f64>() / n as f64, diffs[n / 2], diffs[n * 995 / 1000], diffs[n - 1],
        100.0 * within3 as f64 / n as f64);
    // The engine rounds the cap's excess, the opposite-bishop scale and `adjust_eval` to integers
    // where the model does not, and `adjust_eval` multiplies whatever was rounded before it by up
    // to `max_eval_mult`. Split by whether it fired, so that rounding is not read as a wrong hook.
    for (label, adjusted) in [("adjust_eval idle", false), ("adjust_eval fired", true)] {
        let part: Vec<f64> = samples.iter()
            .filter(|s| (s.phase as i16 + 100 < 255 && s.eval.abs() >= 200) == adjusted)
            .map(|s| (pipeline.eval(s, &theta0).0 - s.eval as f64).abs()).collect();
        let m = part.len().max(1) as f64;
        println!("  {label}: {} positions, mean {:.3} cp, within 3 cp {:.3}%", part.len(),
            part.iter().sum::<f64>() / m, 100.0 * part.iter().filter(|&&d| d <= 3.0).count() as f64 / m);
    }
}

// ---------------------------------------------------------------------------------------------
// texel-stats, texel-lazy

fn read_theta(path: &str) -> Vec<f64> {
    let theta: Vec<f64> = std::fs::read_to_string(path).unwrap().lines().map(|l| l.parse().unwrap()).collect();
    assert_eq!(theta.len(), et::N_PARAMS, "theta has the wrong length");
    theta
}

fn stats(data: &str, theta_path: Option<&str>) {
    let samples = read_samples(data);
    let config = hce_config();
    let pipeline = Pipeline::of(&config);
    let theta0 = et::theta_of(&config);
    let theta = theta_path.map(read_theta).unwrap_or_else(|| theta0.clone());
    let k = fit_k(&samples, &pipeline, &theta0);
    let test: Vec<&Sample> = samples.iter().filter(|s| s.set == 2).collect();
    let (mut capped, mut abs_eval, mut abs_positional) = (0usize, 0.0, 0.0);
    for s in &test {
        let (mg, eg) = sums(s, &theta);
        let pre = taper(mg, eg, s.phase as i16) + s.taper_residual as f64;
        let (mm, me) = material_sums(s, &theta);
        let positional = pre - taper(mm, me, s.phase as i16);
        if positional.abs() > 150.0 {
            capped += 1;
        }
        abs_positional += positional.abs();
        abs_eval += pipeline.eval(s, &theta).0.abs();
    }
    let n = test.len() as f64;
    println!("test positions {}: cap fires {:.1}%, mean |positional| {:.1}, mean |eval| {:.1}, MSE {:.6} (K {k:.6})",
        test.len(), 100.0 * capped as f64 / n, abs_positional / n, abs_eval / n, mse(&samples, 2, k, &pipeline, &theta));
    let names = ["pawn", "rook", "knight", "bishop", "queen", "king"];
    let mut line = String::from("tables, roughness / range:");
    for (label, base) in [("mg", et::TABLE_MG), ("eg", et::TABLE_EG)] {
        for (p, name) in names.iter().enumerate() {
            let (rough, range) = table_shape(&theta, base, p);
            line.push_str(&format!(" {label} {name} {rough:.1}/{range:.0};"));
        }
    }
    let (rough, range) = tables_shape(&theta);
    println!("{line} mean {rough:.1}/{range:.0}");
}

fn lazy(path: &str) {
    let service = Service::new();
    let config = hce_config();
    let table = crate::pawn_hash::PawnHashTable::new(1 << 16);
    let mut diffs: Vec<i32> = Vec::new();
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let board = service.fen.set_fen(line.split(';').next().unwrap());
        if crate::eval_service::EvalService::is_insufficient_material(&board) {
            continue;
        }
        let full = service.eval.calc_eval_known_check(&board, &config, &service.move_gen, &table,
            i16::MIN + 1, i16::MAX - 1, 0, false);
        let cheap = service.eval.cheap_eval(&board, &config, &table);
        diffs.push((full as i32 - cheap as i32).abs());
    }
    let n = diffs.len() as f64;
    let share = |t: i32| 100.0 * diffs.iter().filter(|&&d| d > t).count() as f64 / n;
    println!("{} positions: mean |full - cheap| {:.1} cp, above 120 (QS margin) {:.2}%, above 180 (search margin) {:.2}%",
        diffs.len(), diffs.iter().map(|&d| d as f64).sum::<f64>() / n, share(120), share(180));
}

/// The colour mirror of a FEN: ranks reversed, colours and the side to move swapped.
fn mirror_fen(fen: &str) -> String {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    let swap = |c: char| if c.is_ascii_uppercase() { c.to_ascii_lowercase() } else { c.to_ascii_uppercase() };
    let ranks: Vec<String> = parts[0].split('/').rev().map(|r| r.chars().map(swap).collect()).collect();
    let side = if parts[1] == "w" { "b" } else { "w" };
    let castling: String = if parts[2] == "-" { "-".into() } else {
        let mut v: Vec<char> = parts[2].chars().map(swap).collect();
        v.sort_by_key(|c| (c.is_ascii_lowercase(), *c));
        v.into_iter().collect()
    };
    let ep = if parts[3] == "-" { "-".to_string() } else {
        format!("{}{}", &parts[3][..1], 9 - parts[3][1..2].parse::<u32>().unwrap())
    };
    format!("{} {} {} {} {} {}", ranks.join("/"), side, castling, ep, parts.get(4).unwrap_or(&"0"), parts.get(5).unwrap_or(&"1"))
}

fn mirror(path: &str) {
    let service = Service::new();
    let config = hce_config();
    let (mut n, mut asymmetric) = (0usize, 0usize);
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let fen = line.split(';').next().unwrap();
        let eval = |fen: &str| service.eval.calc_eval_known_check(&service.fen.set_fen(fen), &config, &service.move_gen,
            &crate::pawn_hash::PawnHashTable::new(16), i16::MIN + 1, i16::MAX - 1, 0, false);
        let (a, b) = (eval(fen), eval(&mirror_fen(fen)));
        n += 1;
        if a != -b {
            asymmetric += 1;
            if asymmetric <= 10 {
                println!("asymmetric: {fen} {a} / {b}");
            }
        }
    }
    println!("{n} positions, {asymmetric} asymmetric");
}

// ---------------------------------------------------------------------------------------------
// texel-tune

/// Every pair of king-adjacent squares of every table and phase, both of them movable under `mask`
/// - the pawn tables' back ranks take no part, as no pawn stands there.
fn smoothness_pairs(mask: &[bool]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for base in [et::TABLE_MG, et::TABLE_EG] {
        for p in 0..6 {
            for row in 0..8i32 {
                for file in 0..8i32 {
                    for (dr, df) in [(0, 1), (1, 0), (1, 1), (1, -1)] {
                        let (r2, f2) = (row + dr, file + df);
                        if !(0..8).contains(&r2) || !(0..8).contains(&f2) {
                            continue;
                        }
                        let i = base + p * 64 + (row * 8 + file) as usize;
                        let j = base + p * 64 + (r2 * 8 + f2) as usize;
                        if mask[i] && mask[j] {
                            pairs.push((i, j));
                        }
                    }
                }
            }
        }
    }
    pairs
}

/// The sum of squared jumps between the neighbouring squares of `pairs`.
#[cfg(test)]
fn smoothness_penalty(theta: &[f64], pairs: &[(usize, usize)]) -> f64 {
    pairs.iter().map(|&(i, j)| (theta[i] - theta[j]).powi(2)).sum()
}

/// Adds the gradient of `s * smoothness_penalty` to `grad`.
fn add_smoothness_gradient(theta: &[f64], pairs: &[(usize, usize)], s: f64, grad: &mut [f64]) {
    for &(i, j) in pairs {
        let g = 2.0 * s * (theta[i] - theta[j]);
        grad[i] += g;
        grad[j] -= g;
    }
}

/// A table's mean jump between king-adjacent squares and its range, over the squares its piece
/// can stand on: the roughness and the size `task.md` 29.7 measured.
fn table_shape(theta: &[f64], base: usize, piece: usize) -> (f64, f64) {
    let rows = if piece == 0 { 1..7 } else { 0..8 };
    let table = &theta[base + piece * 64..base + piece * 64 + 64];
    let (mut jumps, mut n) = (0.0, 0usize);
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for row in rows.clone() {
        for file in 0..8i32 {
            let v = table[(row * 8 + file) as usize];
            lo = lo.min(v);
            hi = hi.max(v);
            for (dr, df) in [(0, 1), (1, 0), (1, 1), (1, -1)] {
                let (r2, f2) = (row + dr, file + df);
                if rows.contains(&r2) && (0..8).contains(&f2) {
                    jumps += (v - table[(r2 * 8 + f2) as usize]).abs();
                    n += 1;
                }
            }
        }
    }
    (jumps / n as f64, hi - lo)
}

/// Mean roughness and mean range over the twelve tables.
fn tables_shape(theta: &[f64]) -> (f64, f64) {
    let (mut rough, mut range) = (0.0, 0.0);
    for base in [et::TABLE_MG, et::TABLE_EG] {
        for p in 0..6 {
            let (r, g) = table_shape(theta, base, p);
            rough += r;
            range += g;
        }
    }
    (rough / 12.0, range / 12.0)
}

/// Adam over the full training batch from `theta0`, moving the parameters of `mask` only, with L2
/// towards `theta0` on the tables and `smooth` times the squared jumps between neighbouring squares. Returns the final vector and the best one on the validation
/// set as `(error, vector, epoch)`, checked every 50 epochs and at the end.
#[allow(clippy::too_many_arguments)]
fn run_epochs(samples: &[Sample], k: f64, pipeline: &Pipeline, theta0: &[f64], mask: &[bool], lambda: f64,
              smooth: f64, epochs: usize, lr: f64, threads: usize) -> (Vec<f64>, (f64, Vec<f64>, usize)) {
    let train_n = samples.iter().filter(|s| s.set == 0).count() as f64;
    let pairs = if smooth > 0.0 { smoothness_pairs(mask) } else { Vec::new() };
    let means0 = table_means(theta0, mask);
    let chunk = samples.len().div_ceil(threads).max(1);
    let mut theta = theta0.to_vec();
    let (mut m1, mut m2) = (vec![0.0; et::N_PARAMS], vec![0.0; et::N_PARAMS]);
    let mut best_val = (f64::MAX, theta.clone(), 0usize);
    for epoch in 1..=epochs {
        let partials: Vec<Vec<f64>> = std::thread::scope(|scope| {
            let handles: Vec<_> = samples.chunks(chunk).map(|part| {
                let theta = &theta;
                scope.spawn(move || {
                    let mut g = vec![0.0; et::N_PARAMS];
                    gradient(part, k, pipeline, theta, &mut g);
                    g
                })
            }).collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let mut grad = vec![0.0; et::N_PARAMS];
        for part in partials {
            for (g, p) in grad.iter_mut().zip(part) {
                *g += p;
            }
        }
        for j in 0..et::N_PARAMS {
            // The penalty holds only the tables: a square few positions reach has almost no
            // gradient of its own and would otherwise follow Adam's normalised noise.
            let penalty = if j >= et::TABLE_MG { 2.0 * lambda * (theta[j] - theta0[j]) } else { 0.0 };
            grad[j] = if mask[j] { grad[j] / train_n + penalty } else { 0.0 };
        }
        add_smoothness_gradient(&theta, &pairs, smooth, &mut grad);
        for j in 0..et::N_PARAMS {
            if !mask[j] { continue; }
            m1[j] = 0.9 * m1[j] + 0.1 * grad[j];
            m2[j] = 0.999 * m2[j] + 0.001 * grad[j] * grad[j];
            let mh = m1[j] / (1.0 - 0.9f64.powi(epoch as i32));
            let vh = m2[j] / (1.0 - 0.999f64.powi(epoch as i32));
            theta[j] -= lr * mh / (vh.sqrt() + 1e-12);
        }
        recentre_tables(&mut theta, mask, &means0);
        if epoch % 50 == 0 || epoch == epochs {
            let val = mse(samples, 1, k, pipeline, &theta);
            if val < best_val.0 {
                best_val = (val, theta.clone(), epoch);
            }
            println!("smooth {smooth:e} lambda {lambda:e} epoch {epoch}: validation {val:.6} (best {:.6} at {})", best_val.0, best_val.2);
        }
    }
    (theta, best_val)
}

fn tune(args: &[String]) {
    let data = &args[0];
    let out_dir = std::path::Path::new(&args[1]);
    std::fs::create_dir_all(out_dir).unwrap();
    let (mut epochs, mut lr, mut lambdas, mut threads) = (2000usize, 1.0f64, vec![0.0f64], 4usize);
    let mut groups = parse_groups("material,tables,passers,scalars");
    let mut smooths = vec![0.0f64];
    let mut table_pieces: Option<[bool; 6]> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--epochs" => { epochs = args[i + 1].parse().unwrap(); i += 1; }
            "--lr" => { lr = args[i + 1].parse().unwrap(); i += 1; }
            "--threads" => { threads = args[i + 1].parse().unwrap(); i += 1; }
            "--lambda" => { lambdas = args[i + 1].split(',').map(|x| x.parse().unwrap()).collect(); i += 1; }
            "--free" => { groups = parse_groups(&args[i + 1]); i += 1; }
            "--smooth" => { smooths = args[i + 1].split(',').map(|x| x.parse().unwrap()).collect(); i += 1; }
            "--tables" => { table_pieces = Some(parse_table_pieces(&args[i + 1])); i += 1; }
            other => panic!("unknown option {other}"),
        }
        i += 1;
    }
    if let Some(pieces) = table_pieces {
        groups.table_pieces = pieces;
    }
    let samples = read_samples(data);
    let config = hce_config();
    let pipeline = Pipeline::of(&config);
    let theta0 = et::theta_of(&config);
    let k = fit_k(&samples, &pipeline, &theta0);
    let mut report = String::new();
    let log = |line: String, report: &mut String| { println!("{line}"); report.push_str(&line); report.push('\n'); };
    let counts: Vec<usize> = (0..3).map(|set| samples.iter().filter(|s| s.set == set).count()).collect();
    log(format!("positions: train {}, validation {}, test {}; K = {k:.6}; free {groups:?}", counts[0], counts[1], counts[2]), &mut report);
    let base: Vec<f64> = (0..3).map(|set| mse(&samples, set, k, &pipeline, &theta0)).collect();
    log(format!("shipped: train {:.6}, validation {:.6}, test {:.6}", base[0], base[1], base[2]), &mut report);
    let mask = trainable_for(&groups);
    let shape0 = tables_shape(&theta0);
    log(format!("shipped tables: mean roughness {:.1}, mean range {:.0}", shape0.0, shape0.1), &mut report);
    let mut best: Option<(f64, Vec<f64>, f64)> = None;
    for (&smooth, &lambda) in smooths.iter().flat_map(|s| lambdas.iter().map(move |l| (s, l))) {
        let (_, best_val) = run_epochs(&samples, k, &pipeline, &theta0, &mask, lambda, smooth, epochs, lr, threads);
        let rounded: Vec<f64> = best_val.1.iter().map(|v| v.round()).collect();
        let val = mse(&samples, 1, k, &pipeline, &rounded);
        let test = mse(&samples, 2, k, &pipeline, &rounded);
        let shape = tables_shape(&rounded);
        // Every fit is kept: the choice between them is made by the search's time, not here.
        let name = format!("theta_{smooth:e}_{lambda:e}.txt");
        std::fs::write(out_dir.join(&name), rounded.iter().map(|v| format!("{v}\n")).collect::<String>()).unwrap();
        log(format!("smooth {smooth:e} lambda {lambda:e}: best epoch {}, validation {:.6} (rounded {val:.6}), test {test:.6} ({:+.1}e-6), roughness {:.1}, range {:.0} -> {name}",
            best_val.2, best_val.0, (test - base[2]) * 1e6, shape.0, shape.1), &mut report);
        if best.as_ref().is_none_or(|b| val < b.0) {
            best = Some((val, rounded, lambda));
        }
    }
    let (_, theta, lambda) = best.unwrap();
    let tuned: Vec<f64> = (0..3).map(|set| mse(&samples, set, k, &pipeline, &theta)).collect();
    log(format!("chosen lambda {lambda:e}: train {:.6} ({:+.1}e-6), validation {:.6} ({:+.1}e-6), test {:.6} ({:+.1}e-6)",
        tuned[0], (tuned[0] - base[0]) * 1e6, tuned[1], (tuned[1] - base[1]) * 1e6, tuned[2], (tuned[2] - base[2]) * 1e6), &mut report);
    log(String::new(), &mut report);
    for (j, (name, _, _)) in et::SCALARS.iter().enumerate() {
        log(format!("{name:<40} {:>6} -> {:>6}", theta0[j], theta[j]), &mut report);
    }
    for r in 0..8 {
        log(format!("passed_pawn_bonus rank {}                 mg {:>5} -> {:>5}   eg {:>5} -> {:>5}", r + 1,
            theta0[et::PASSER_MG + r], theta[et::PASSER_MG + r], theta0[et::PASSER_EG + r], theta[et::PASSER_EG + r]), &mut report);
    }
    let names = ["pawn", "rook", "knight", "bishop", "queen", "king"];
    for (p, name) in names.iter().enumerate() {
        log(format!("material {name:<8}                        mg {:>5} -> {:>5}   eg {:>5} -> {:>5}", theta0[et::MATERIAL_MG + p],
            theta[et::MATERIAL_MG + p], theta0[et::MATERIAL_EG + p], theta[et::MATERIAL_EG + p]), &mut report);
    }
    std::fs::write(out_dir.join("report.txt"), &report).unwrap();
    std::fs::write(out_dir.join("theta.txt"), theta.iter().map(|v| format!("{v}\n")).collect::<String>()).unwrap();
    std::fs::write(out_dir.join("pst_block.rs"), pst_block(&theta)).unwrap();
}

// ---------------------------------------------------------------------------------------------
// texel-export

const BLOCK_BEGIN: &str = "// BEGIN TEXEL GENERATED";
const BLOCK_END: &str = "// END TEXEL GENERATED";

/// The generated block of `src/pst.rs` for `theta`, exactly as the file holds it.
pub fn pst_block(theta: &[f64]) -> String {
    let int = |v: f64| v.round() as i64;
    let mut out = String::new();
    out.push_str("// BEGIN TEXEL GENERATED - written by `suprah texel-export` (feature `texel`); re-tune, do not edit.\n");
    out.push_str("/// Material per piece in `WHITE_PAWN..=WHITE_KING` order: pawn, rook, knight, bishop, queen, king.\n");
    for (label, base) in [("MG", et::MATERIAL_MG), ("EG", et::MATERIAL_EG)] {
        let values: Vec<String> = (0..6).map(|p| int(theta[base + p]).to_string()).collect();
        out.push_str(&format!("pub const MATERIAL_{label}: [i16; 6] = [{}];\n", values.join(", ")));
    }
    let names = ["pawn", "rook", "knight", "bishop", "queen", "king"];
    for (label, base, phase) in [("MG", et::TABLE_MG, "middlegame"), ("EG", et::TABLE_EG, "endgame")] {
        out.push_str(&format!("/// Piece-square tables without material, from White's side, rank 8 first ({phase}).\n"));
        out.push_str(&format!("pub const TABLE_{label}: [[i16; 64]; 6] = [\n"));
        for (p, name) in names.iter().enumerate() {
            out.push_str(&format!("    [ // {name}\n"));
            for row in 0..8 {
                let cells: Vec<String> = (0..8).map(|c| format!("{:4},", int(theta[base + p * 64 + row * 8 + c]))).collect();
                out.push_str(&format!("        {}\n", cells.join(" ")));
            }
            out.push_str("    ],\n");
        }
        out.push_str("];\n");
    }
    out.push_str("// END TEXEL GENERATED");
    out
}

/// `text` with its generated block replaced by `block`.
pub fn replace_block(text: &str, block: &str) -> String {
    let start = text.find(BLOCK_BEGIN).expect("no generated block");
    let end = text.find(BLOCK_END).expect("no end of the generated block") + BLOCK_END.len();
    format!("{}{}{}", &text[..start], block, &text[end..])
}

/// `config.rs` with the defaults of `Config::new_raw` set to `theta`.
pub fn replace_config_defaults(text: &str, theta: &[f64]) -> String {
    let mut out = text.to_string();
    for (j, (name, _, _)) in et::SCALARS.iter().enumerate() {
        let key = format!("\n            {name}: ");
        let at = out.find(&key).unwrap_or_else(|| panic!("no default for {name}")) + key.len();
        let end = at + out[at..].find(',').unwrap();
        out.replace_range(at..end, &(theta[j].round() as i64).to_string());
    }
    for (name, base) in [("passed_pawn_bonus_mg", et::PASSER_MG), ("passed_pawn_bonus_eg", et::PASSER_EG)] {
        let key = format!("\n            {name}: [");
        let at = out.find(&key).unwrap_or_else(|| panic!("no default for {name}")) + key.len();
        let end = at + out[at..].find(']').unwrap();
        let values: Vec<String> = (0..8).map(|r| (theta[base + r].round() as i64).to_string()).collect();
        out.replace_range(at..end, &values.join(", "));
    }
    out
}

/// `parameters.json` with every registered scalar at `theta`, its range widened to hold it.
pub fn replace_registered_values(text: &str, theta: &[f64]) -> String {
    let mut out = text.to_string();
    for (j, (name, _, _)) in et::SCALARS.iter().enumerate() {
        let key = format!("\"{name}\": {{");
        let Some(start) = out.find(&key) else { continue };
        let end = start + out[start..].find('}').unwrap();
        let body = out[start..end].to_string();
        let field = |f: &str| -> f64 {
            let k = format!("\"{f}\": ");
            let at = body.find(&k).unwrap() + k.len();
            let stop = at + body[at..].find([',', '\n']).unwrap();
            body[at..stop].trim().parse().unwrap()
        };
        let value = theta[j].round() as i64;
        let mut new_body = String::new();
        for line in body.split_inclusive('\n') {
            let trimmed = line.trim_start();
            let indent = &line[..line.len() - trimmed.len()];
            if trimmed.starts_with("\"value\":") {
                new_body.push_str(&format!("{indent}\"value\": {value},\n"));
            } else if trimmed.starts_with("\"min\":") && (value as f64) < field("min") {
                new_body.push_str(&format!("{indent}\"min\": {value},\n"));
            } else if trimmed.starts_with("\"max\":") && (value as f64) > field("max") {
                new_body.push_str(&format!("{indent}\"max\": {value}\n"));
            } else {
                new_body.push_str(line);
            }
        }
        out.replace_range(start..end, &new_body);
    }
    out
}

fn export_file(theta_path: &str) {
    let theta = read_theta(theta_path);
    let pst = std::fs::read_to_string("src/pst.rs").unwrap();
    std::fs::write("src/pst.rs", replace_block(&pst, &pst_block(&theta))).unwrap();
    let config = std::fs::read_to_string("src/config.rs").unwrap();
    std::fs::write("src/config.rs", replace_config_defaults(&config, &theta)).unwrap();
    let params = std::fs::read_to_string("tuning/parameters.json").unwrap();
    std::fs::write("tuning/parameters.json", replace_registered_values(&params, &theta)).unwrap();
    println!("written: src/pst.rs, src/config.rs, tuning/parameters.json");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval_trace::P;

    const POSITIONS: &[&str] = &[
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r1bq1rk1/pp2bppp/2n1pn2/3p4/2PP4/2N1PN2/PP3PPP/R2QKB1R w KQ - 0 8",
        "8/5k2/3b4/3P4/2K5/8/5B2/8 w - - 0 60",
        "6k1/5ppp/8/3P4/8/8/r4PPP/1R4K1 b - - 0 40",
        "2r3k1/pp3ppp/8/3Pp3/8/8/PP3PPP/2R3K1 w - - 0 30",
        "r4rk1/1pp2ppp/p1n5/4P3/1P1q4/P1N5/2Q2PPP/R4RK1 b - - 0 20",
        "8/8/4k3/8/4P3/2r5/4K3/5R2 w - - 0 50",
        "6k1/5ppp/8/8/8/8/5PPP/3RR1K1 w - - 0 40",
        "6k1/5pp1/8/8/8/2q5/5PP1/3R2K1 w - - 0 45",
    ];

    fn traced(fen: &str) -> (Sample, i16) {
        let service = Service::new();
        let config = hce_config();
        let board = service.fen.set_fen(fen);
        let eval = service.eval.calc_eval_known_check(&board, &config, &service.move_gen,
            &crate::pawn_hash::PawnHashTable::new(16), i16::MIN + 1, i16::MAX - 1, 0, false);
        let sample = trace_position(&service, &config, &et::theta_of(&config), &board).expect("excluded");
        (sample, eval)
    }

    /// Every unit's coefficients predict its sums to integer rounding, and the model rebuilds the
    /// engine's evaluation at the shipped parameters.
    #[test]
    fn test_the_trace_rebuilds_the_evaluation() {
        let service = Service::new();
        let config = hce_config();
        let theta0 = et::theta_of(&config);
        let pipeline = Pipeline::of(&config);
        for fen in POSITIONS {
            let board = service.fen.set_fen(fen);
            et::begin(&theta0);
            service.eval.calc_eval_known_check(&board, &config, &service.move_gen,
                &crate::pawn_hash::PawnHashTable::new(16), i16::MIN + 1, i16::MAX - 1, 0, false);
            for u in et::finish().units {
                assert!((u.actual.0 - u.predicted.0).abs() <= 1.0 && (u.actual.1 - u.predicted.1).abs() <= 1.0,
                    "{fen}: a unit the hooks do not explain: {u:?}");
            }
            let (sample, eval) = traced(fen);
            let model = pipeline.eval(&sample, &theta0).0;
            assert!((model - eval as f64).abs() <= 3.0, "{fen}: model {model} against engine {eval}");
        }
    }

    /// The trace is a measuring device: switching it on must not change a single evaluation.
    #[test]
    fn test_tracing_does_not_change_the_evaluation() {
        let service = Service::new();
        let config = hce_config();
        for fen in POSITIONS {
            let board = service.fen.set_fen(fen);
            let table = crate::pawn_hash::PawnHashTable::new(16);
            let plain = service.eval.calc_eval_known_check(&board, &config, &service.move_gen, &table,
                i16::MIN + 1, i16::MAX - 1, 0, false);
            et::begin(&et::theta_of(&config));
            let traced = service.eval.calc_eval_known_check(&board, &config, &service.move_gen, &table,
                i16::MIN + 1, i16::MAX - 1, 0, false);
            et::finish();
            assert_eq!(plain, traced, "{fen}");
        }
    }

    /// The model's evaluation follows a change of parameters the way the engine does: the
    /// coefficients are derivatives, not a replay of the shipped numbers.
    #[test]
    fn test_the_model_follows_a_changed_configuration() {
        let service = Service::new();
        let base = hce_config();
        let mut changed = hce_config();
        changed.bishop_mobility_factor += 4;
        changed.king_pawn_shield_kingside -= 10;
        changed.passed_pawn_bonus_eg[5] += 60;
        changed.your_turn_bonus += 7;
        let pipeline = Pipeline::of(&base);
        let theta_changed = et::theta_of(&changed);
        for fen in POSITIONS {
            let board = service.fen.set_fen(fen);
            let engine = service.eval.calc_eval_known_check(&board, &changed, &service.move_gen,
                &crate::pawn_hash::PawnHashTable::new(16), i16::MIN + 1, i16::MAX - 1, 0, false);
            let (sample, _) = traced(fen);
            let model = pipeline.eval(&sample, &theta_changed).0;
            assert!((model - engine as f64).abs() <= 3.0, "{fen}: model {model} against engine {engine}");
        }
    }

    /// The analytic gradient equals central finite differences.
    #[test]
    fn test_the_gradient_matches_finite_differences() {
        let config = hce_config();
        let pipeline = Pipeline::of(&config);
        let theta0 = et::theta_of(&config);
        let samples: Vec<Sample> = POSITIONS.iter().enumerate().map(|(i, fen)| {
            let mut s = traced(fen).0;
            s.result = (i % 3) as u8;
            s
        }).collect();
        let k = 0.004;
        let mut grad = vec![0.0; et::N_PARAMS];
        gradient(&samples, k, &pipeline, &theta0, &mut grad);
        let loss = |theta: &[f64]| -> f64 {
            samples.iter().map(|s| { let d = s.label() - sigmoid(k, pipeline.eval(s, theta).0); d * d }).sum()
        };
        let probes = [P::BishopMobilityFactor as usize, P::KingPawnShieldKingside as usize, P::YourTurnBonus as usize,
            et::passer_eg(5), et::MATERIAL_MG + 2, et::MATERIAL_EG + 1, et::TABLE_MG + 64 * 2 + 18, et::TABLE_EG + 64 * 5 + 6];
        for &j in &probes {
            let h = 0.01;
            let (mut up, mut down) = (theta0.clone(), theta0.clone());
            up[j] += h;
            down[j] -= h;
            let numeric = (loss(&up) - loss(&down)) / (2.0 * h);
            assert!((numeric - grad[j]).abs() <= 1e-6 + 1e-3 * numeric.abs(), "parameter {j}: {numeric} against {}", grad[j]);
        }
    }

    /// The export of the shipped parameters reproduces today's files byte for byte.
    #[test]
    fn test_exporting_the_shipped_parameters_changes_no_file() {
        let theta0 = et::theta_of(&hce_config());
        let pst = std::fs::read_to_string("src/pst.rs").unwrap();
        assert_eq!(replace_block(&pst, &pst_block(&theta0)), pst);
        let config = std::fs::read_to_string("src/config.rs").unwrap();
        assert_eq!(replace_config_defaults(&config, &theta0), config);
        let params = std::fs::read_to_string("tuning/parameters.json").unwrap();
        assert_eq!(replace_registered_values(&params, &theta0), params);
    }

    /// Re-centring moves a table's uniform shift into material: every square's material plus table
    /// value, which is what the evaluation sums, stays put, and the table means return to theirs.
    #[test]
    fn test_recentring_keeps_what_the_evaluation_sums() {
        let mask = trainable();
        let theta0 = et::theta_of(&hce_config());
        let means0 = table_means(&theta0, &mask);
        let mut theta = theta0.clone();
        for (j, v) in theta.iter_mut().enumerate().skip(et::TABLE_MG) {
            if mask[j] {
                *v += 7.0 + (j % 13) as f64;
            }
        }
        let sum = |t: &[f64], material: usize, table: usize, p: usize, sq: usize| t[material + p] + t[table + p * 64 + sq];
        let before: Vec<f64> = (0..5).flat_map(|p| (8..56).map(move |sq| (p, sq)))
            .map(|(p, sq)| sum(&theta, et::MATERIAL_MG, et::TABLE_MG, p, sq)).collect();
        recentre_tables(&mut theta, &mask, &means0);
        let after: Vec<f64> = (0..5).flat_map(|p| (8..56).map(move |sq| (p, sq)))
            .map(|(p, sq)| sum(&theta, et::MATERIAL_MG, et::TABLE_MG, p, sq)).collect();
        for (a, b) in before.iter().zip(&after) {
            assert!((a - b).abs() < 1e-9);
        }
        for (a, b) in table_means(&theta, &mask).iter().zip(&means0) {
            assert!((a - b).abs() < 1e-9);
        }
        for &p in PIECE_SQUARE_SCALARS {
            assert!(!mask[p as usize], "{p:?} is a piece-square term and must stay fixed");
        }
    }

    /// `--free material,tables` moves material and tables and nothing else: a few real epochs
    /// leave every scalar and the passed-pawn table where they were.
    #[test]
    fn test_only_the_freed_groups_move() {
        let config = hce_config();
        let pipeline = Pipeline::of(&config);
        let theta0 = et::theta_of(&config);
        let samples: Vec<Sample> = POSITIONS.iter().enumerate().map(|(i, fen)| {
            let mut s = traced(fen).0;
            s.result = (i % 3) as u8;
            s
        }).collect();
        let mask = trainable_for(&parse_groups("material,tables"));
        let theta = run_epochs(&samples, 0.004, &pipeline, &theta0, &mask, 0.0, 0.0, 5, 1.0, 1).0;
        for j in 0..et::TABLE_MG {
            let group_is_free = (et::MATERIAL_MG..et::TABLE_MG).contains(&j);
            if !group_is_free {
                assert_eq!(theta[j], theta0[j], "parameter {j} moved although its group is held");
            }
        }
        assert!((et::MATERIAL_MG..et::N_PARAMS).any(|j| theta[j] != theta0[j]), "nothing free moved");
        assert_eq!(trainable_for(&parse_groups("material,tables,passers,scalars")), trainable());
    }

    /// `--free material` alone: the tables are held, so their means have nowhere to go and the
    /// re-centring must leave everything finite and the tables untouched.
    #[test]
    fn test_material_alone_moves_only_material() {
        let config = hce_config();
        let pipeline = Pipeline::of(&config);
        let theta0 = et::theta_of(&config);
        let samples: Vec<Sample> = POSITIONS.iter().enumerate().map(|(i, fen)| {
            let mut s = traced(fen).0;
            s.result = (i % 3) as u8;
            s
        }).collect();
        let mask = trainable_for(&parse_groups("material"));
        let theta = run_epochs(&samples, 0.004, &pipeline, &theta0, &mask, 0.0, 0.0, 5, 1.0, 1).0;
        assert!(theta.iter().all(|v| v.is_finite()), "a parameter left the reals");
        for j in 0..et::N_PARAMS {
            if !(et::MATERIAL_MG..et::TABLE_MG).contains(&j) {
                assert_eq!(theta[j], theta0[j], "parameter {j} moved although only material is free");
            }
        }
        assert!((et::MATERIAL_MG..et::TABLE_MG).any(|j| theta[j] != theta0[j]), "material did not move");
    }

    /// The smoothness penalty's gradient equals central finite differences of the penalty.
    #[test]
    fn test_the_smoothness_gradient_matches_finite_differences() {
        let mask = trainable_for(&parse_groups("material,tables"));
        let pairs = smoothness_pairs(&mask);
        let mut theta = et::theta_of(&hce_config());
        for (j, v) in theta.iter_mut().enumerate().skip(et::TABLE_MG) {
            *v += ((j * 37) % 23) as f64 - 11.0;
        }
        let s = 1e-3;
        let mut grad = vec![0.0; et::N_PARAMS];
        add_smoothness_gradient(&theta, &pairs, s, &mut grad);
        for j in [et::TABLE_MG + 64 * 2 + 27, et::TABLE_EG + 64 * 5 + 6, et::TABLE_MG + 9, et::TABLE_EG + 64 * 4 + 63] {
            let h = 0.01;
            let (mut up, mut down) = (theta.clone(), theta.clone());
            up[j] += h;
            down[j] -= h;
            let numeric = (s * smoothness_penalty(&up, &pairs) - s * smoothness_penalty(&down, &pairs)) / (2.0 * h);
            assert!((numeric - grad[j]).abs() <= 1e-9 + 1e-6 * numeric.abs(), "parameter {j}: {numeric} against {}", grad[j]);
        }
    }

    /// A table without jumps between neighbours feels no penalty and no pull; the pawn table's
    /// back ranks, where no pawn stands, take no part.
    #[test]
    fn test_a_flat_table_feels_no_smoothness() {
        let mask = trainable_for(&parse_groups("material,tables"));
        let pairs = smoothness_pairs(&mask);
        let mut theta = et::theta_of(&hce_config());
        for p in 0..6 {
            for sq in 0..64 {
                theta[et::TABLE_MG + p * 64 + sq] = 10.0 * p as f64;
                theta[et::TABLE_EG + p * 64 + sq] = -7.0 * p as f64;
            }
        }
        theta[et::TABLE_MG + 3] = 500.0;
        assert_eq!(smoothness_penalty(&theta, &pairs), 0.0);
        let mut grad = vec![0.0; et::N_PARAMS];
        add_smoothness_gradient(&theta, &pairs, 1.0, &mut grad);
        assert!(grad.iter().all(|&g| g == 0.0));
        assert!(pairs.iter().all(|&(i, j)| mask[i] && mask[j]));
    }

    /// A strong penalty smooths a noisy start within a few epochs; without it the same epochs
    /// leave the roughness about where it was.
    #[test]
    fn test_a_strong_smoothness_lowers_the_roughness() {
        let config = hce_config();
        let pipeline = Pipeline::of(&config);
        let mut theta0 = et::theta_of(&config);
        for (j, v) in theta0.iter_mut().enumerate().skip(et::TABLE_MG) {
            *v += ((j * 37) % 41) as f64 - 20.0;
        }
        let samples: Vec<Sample> = POSITIONS.iter().enumerate().map(|(i, fen)| {
            let mut s = traced(fen).0;
            s.result = (i % 3) as u8;
            s
        }).collect();
        let mask = trainable_for(&parse_groups("material,tables"));
        let rough = |t: &[f64]| (0..6).map(|p| table_shape(t, et::TABLE_MG, p).0).sum::<f64>();
        let smooth = run_epochs(&samples, 0.004, &pipeline, &theta0, &mask, 0.0, 1e-2, 20, 1.0, 1).0;
        let plain = run_epochs(&samples, 0.004, &pipeline, &theta0, &mask, 0.0, 0.0, 20, 1.0, 1).0;
        assert!(rough(&smooth) < 0.8 * rough(&theta0), "{} against {}", rough(&smooth), rough(&theta0));
        assert!(rough(&plain) > 0.9 * rough(&theta0), "{} against {}", rough(&plain), rough(&theta0));
    }

    /// `--tables rook,knight,bishop,queen`: the pawn and king tables stay exactly where they were
    /// - a held table has no squares to average, so the re-centring must leave it and its
    /// material alone - while the four named tables move and every value stays finite.
    #[test]
    fn test_only_the_named_tables_move() {
        let config = hce_config();
        let pipeline = Pipeline::of(&config);
        let theta0 = et::theta_of(&config);
        let samples: Vec<Sample> = POSITIONS.iter().enumerate().map(|(i, fen)| {
            let mut s = traced(fen).0;
            s.result = (i % 3) as u8;
            s
        }).collect();
        let mut groups = parse_groups("material,tables");
        groups.table_pieces = parse_table_pieces("rook,knight,bishop,queen");
        let mask = trainable_for(&groups);
        let theta = run_epochs(&samples, 0.004, &pipeline, &theta0, &mask, 0.0, 0.0, 5, 1.0, 1).0;
        assert!(theta.iter().all(|v| v.is_finite()), "a parameter left the reals");
        for base in [et::TABLE_MG, et::TABLE_EG] {
            for held in [0, 5] {
                let range = base + held * 64..base + held * 64 + 64;
                assert_eq!(theta[range.clone()], theta0[range], "the table of piece {held} moved");
            }
            for free in 1..5 {
                let range = base + free * 64..base + free * 64 + 64;
                assert_ne!(theta[range.clone()], theta0[range], "the table of piece {free} did not move");
            }
        }
        assert_eq!(parse_table_pieces("pawn,rook,knight,bishop,queen,king"), [true; 6]);
    }

    #[test]
    fn test_the_data_set_round_trips() {
        let samples: Vec<Sample> = POSITIONS.iter().map(|fen| traced(fen).0).collect();
        let path = std::env::temp_dir().join(format!("texel-roundtrip-{}.bin", std::process::id()));
        {
            let mut w = BufWriter::new(std::fs::File::create(&path).unwrap());
            w.write_all(b"TEXEL001").unwrap();
            w.write_all(&(samples.len() as u64).to_le_bytes()).unwrap();
            for s in &samples {
                write_sample(&mut w, s).unwrap();
            }
        }
        let read = read_samples(path.to_str().unwrap());
        std::fs::remove_file(&path).unwrap();
        assert_eq!(read, samples);
    }

    #[test]
    fn test_pgn_tokens_become_uci_moves() {
        assert_eq!(token_to_uci("Ng1f3", true).as_deref(), Some("g1f3"));
        assert_eq!(token_to_uci("e2e4", true).as_deref(), Some("e2e4"));
        assert_eq!(token_to_uci("0-0", false).as_deref(), Some("e8g8"));
        assert_eq!(token_to_uci("0-0-0", true).as_deref(), Some("e1c1"));
        assert_eq!(token_to_uci("a8=Q", true).as_deref(), Some("a7a8q"));
        assert_eq!(token_to_uci("exd1=N", false).as_deref(), Some("e2d1n"));
    }
}
