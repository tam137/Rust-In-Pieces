//! Compatibility tests between the NNUE trainer (`nnue_training/`, bullet) and this engine's
//! inference (Matt-Magie step 102, item 0.7).
//!
//! The network under test is the embedded one, or the file named by `NNUE_TEST_NET`, so every
//! freshly trained network can be checked before it is embedded:
//!
//! ```text
//! NNUE_TEST_NET=<checkpoint>/quantised.bin cargo test --release nnue_compat
//! ```
//!
//! - Colour flip and horizontal mirroring hold by construction for any weights; they guard the
//!   engine's feature transform.
//! - The material signs depend on what the network learned; a network trained with a wrong
//!   perspective, square order or output layout fails them.
//! - `nnue_compat_matches_the_trainer_reference` compares the engine's quantised evaluation with the trainer's
//!   floating-point evaluation of the same checkpoint (`suprah-nnue-trainer --eval-fens`). It is
//!   ignored by default because it needs a reference file:
//!
//! ```text
//! NNUE_TEST_NET=<checkpoint>/quantised.bin NNUE_TEST_REFERENCE=<reference.txt> \
//!     cargo test --release nnue_compat -- --include-ignored
//! ```

use super::*;
use crate::fen_service::FenService;

/// Reference positions shared with the trainer.
const REFERENCE_FENS: &str = include_str!("../nnue_training/reference_fens.txt");

fn test_net() -> NNUENetwork {
    match std::env::var("NNUE_TEST_NET") {
        Ok(path) => {
            NNUENetwork::load_from_file(&path).unwrap_or_else(|e| panic!("NNUE_TEST_NET: {e}"))
        }
        Err(_) => NNUENetwork::load_embedded().expect("embedded network"),
    }
}

fn reference_fens() -> Vec<&'static str> {
    REFERENCE_FENS
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

/// Static evaluation from White's point of view.
fn eval_white(fen: &str, net: &NNUENetwork) -> i32 {
    NNUEService::evaluate(&FenService.set_fen(fen), net) as i32
}

/// Static evaluation from the side to move's point of view.
fn eval_stm(fen: &str, net: &NNUENetwork) -> i32 {
    let white = eval_white(fen, net);
    if fen.split_whitespace().nth(1) == Some("b") {
        -white
    } else {
        white
    }
}

/// Mirrors a FEN vertically and swaps the colours: the same position seen from the other side.
fn colour_flip(fen: &str) -> String {
    let f: Vec<&str> = fen.split_whitespace().collect();
    let swap_case = |s: &str| {
        s.chars()
            .map(|c| {
                if c.is_ascii_uppercase() {
                    c.to_ascii_lowercase()
                } else {
                    c.to_ascii_uppercase()
                }
            })
            .collect::<String>()
    };
    let placement = f[0]
        .split('/')
        .rev()
        .map(swap_case)
        .collect::<Vec<_>>()
        .join("/");
    let side = if f[1] == "w" { "b" } else { "w" };
    let castling = if f[2] == "-" {
        "-".to_string()
    } else {
        let flipped = swap_case(f[2]);
        "KQkq".chars().filter(|c| flipped.contains(*c)).collect()
    };
    let ep = if f[3] == "-" {
        "-".to_string()
    } else {
        let rank = if &f[3][1..] == "3" { '6' } else { '3' };
        format!("{}{rank}", &f[3][..1])
    };
    format!("{placement} {side} {castling} {ep} {} {}", f[4], f[5])
}

/// Mirrors a FEN without castling rights left to right (file a <-> file h).
fn horizontal_mirror(fen: &str) -> String {
    let f: Vec<&str> = fen.split_whitespace().collect();
    assert_eq!(
        f[2], "-",
        "horizontal mirroring needs a position without castling rights"
    );
    let placement = f[0]
        .split('/')
        .map(|rank| rank.chars().rev().collect::<String>())
        .collect::<Vec<_>>()
        .join("/");
    let ep = if f[3] == "-" {
        "-".to_string()
    } else {
        let file = (b'h' - (f[3].as_bytes()[0] - b'a')) as char;
        format!("{file}{}", &f[3][1..])
    };
    format!("{placement} {} - {ep} {} {}", f[1], f[4], f[5])
}

#[test]
fn nnue_compat_fen_helpers_are_involutions() {
    for fen in reference_fens() {
        assert_eq!(colour_flip(&colour_flip(fen)), fen);
        assert_eq!(
            FenService
                .get_fen(&FenService.set_fen(fen))
                .split_whitespace()
                .next(),
            fen.split_whitespace().next()
        );
    }
    assert_eq!(
        colour_flip("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"),
        "rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq e6 0 1"
    );
    assert_eq!(
        horizontal_mirror("8/8/4k3/8/8/3K4/4P3/8 w - - 0 1"),
        "8/8/3k4/8/8/4K3/3P4/8 w - - 0 1"
    );
}

#[test]
fn nnue_compat_colour_flip_negates_the_score() {
    let net = test_net();
    for fen in reference_fens() {
        let flipped = colour_flip(fen);
        assert_eq!(
            eval_white(fen, &net),
            -eval_white(&flipped, &net),
            "{fen} vs {flipped}"
        );
    }
}

#[test]
fn nnue_compat_horizontal_mirror_keeps_the_score() {
    let net = test_net();
    let mut checked = 0;
    for fen in reference_fens()
        .into_iter()
        .filter(|f| f.split_whitespace().nth(2) == Some("-"))
    {
        let mirrored = horizontal_mirror(fen);
        assert_eq!(
            eval_white(fen, &net),
            eval_white(&mirrored, &net),
            "{fen} vs {mirrored}"
        );
        checked += 1;
    }
    assert!(
        checked >= 20,
        "too few positions without castling rights: {checked}"
    );
}

#[test]
fn nnue_compat_extra_material_has_the_right_sign() {
    // (FEN, minimum advantage for White in centipawns). Output buckets 0, 1, 3, 5 and 7.
    // The test is about the sign: a wrong perspective or square order gives the opposite sign,
    // and the three positions with pawns or pieces left demand a clear lead. Bare-king endings
    // are rare in self-play that adjudicates early; there only the sign is required (a test
    // network on such data scored K+Q v K at +215 and K+R v K at +50).
    const SIGN_MARGIN: i32 = 1;
    let white_ahead = [
        ("4k3/8/8/8/8/8/8/3QK3 w - - 0 1", SIGN_MARGIN),
        ("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", SIGN_MARGIN),
        ("4k3/pppp4/8/8/8/8/PPPP4/3QK3 w - - 0 1", 300),
        ("r3k3/ppp2ppp/2n5/8/8/2N5/PPP2PPP/R2QK2R w - - 0 1", 300),
        ("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1", 300),
    ];
    let net = test_net();
    for (fen, margin) in white_ahead {
        for side in ["w", "b"] {
            let position = fen.replacen(" w ", &format!(" {side} "), 1);
            let score = eval_white(&position, &net);
            assert!(
                score >= margin,
                "{position}: {score} cp, expected at least +{margin}"
            );
            let flipped = colour_flip(&position);
            let score = eval_white(&flipped, &net);
            assert!(
                score <= -margin,
                "{flipped}: {score} cp, expected at most -{margin}"
            );
        }
    }
}

#[test]
fn nnue_compat_output_bucket_matches_the_trainer() {
    // bullet `MaterialCount<8>`: (occupied squares - 2) / ceil(32 / 8).
    for fen in reference_fens() {
        let board = FenService.set_fen(fen);
        let pieces = fen
            .split_whitespace()
            .next()
            .unwrap()
            .chars()
            .filter(char::is_ascii_alphabetic)
            .count();
        assert_eq!(
            NNUEService::get_output_bucket(&board),
            (pieces - 2) / 32usize.div_ceil(NNUE_OUTPUT_BUCKETS),
            "{fen}"
        );
    }
}

/// Largest accepted difference between the engine's quantised and the trainer's floating-point
/// evaluation: `ABS` centipawns plus `REL` of the trainer's value. Rounding the 2 x hidden output
/// weights to steps of 1/QB alone moves a score by tens of centipawns; test networks trained on
/// 5 M positions differed by up to 23 cp, also at scores near zero. Pipeline errors (sign,
/// mirroring, transposition, scale) differ by hundreds.
const REFERENCE_TOLERANCE_ABS: f64 = 40.0;
const REFERENCE_TOLERANCE_REL: f64 = 0.05;

#[test]
#[ignore = "needs NNUE_TEST_NET and NNUE_TEST_REFERENCE from `suprah-nnue-trainer --eval-fens`"]
fn nnue_compat_matches_the_trainer_reference() {
    let path = std::env::var("NNUE_TEST_REFERENCE").expect("NNUE_TEST_REFERENCE is not set");
    assert!(
        std::env::var("NNUE_TEST_NET").is_ok(),
        "NNUE_TEST_NET is not set"
    );
    let reference = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let net = test_net();

    let mut compared = 0;
    let mut worst = 0.0f64;
    for line in reference.lines().filter(|l| l.contains('|')) {
        let (fen, cp) = line.rsplit_once('|').unwrap();
        let (fen, expected) = (fen.trim(), cp.trim().parse::<f64>().unwrap());
        let actual = eval_stm(fen, &net) as f64;
        let diff = (actual - expected).abs();
        worst = worst.max(diff);
        assert!(
            diff <= REFERENCE_TOLERANCE_ABS + REFERENCE_TOLERANCE_REL * expected.abs(),
            "{fen}: engine {actual} cp, trainer {expected:.1} cp"
        );
        compared += 1;
    }
    assert_eq!(
        compared,
        reference_fens().len(),
        "reference file does not cover all reference positions"
    );
    println!("{compared} positions, largest difference {worst:.1} cp");
}
