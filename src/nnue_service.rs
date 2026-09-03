use std::fs::File;
use std::io::Read;
use crate::model::{
    Board, MoveInformation, Turn, WHITE_PAWN, WHITE_ROOK, WHITE_KNIGHT, WHITE_BISHOP, WHITE_QUEEN, WHITE_KING,
    BLACK_PAWN, BLACK_ROOK, BLACK_KNIGHT, BLACK_BISHOP, BLACK_QUEEN, BLACK_KING,
};

pub const NNUE_INPUT_SIZE: usize = 768;   // 64 squares * 6 piece types * 2 colors
pub const NNUE_HIDDEN_SIZE: usize = 256;  // Hidden layer neurons per perspective
pub const NNUE_INPUT_BUCKETS: usize = 10; // King position buckets
pub const NNUE_OUTPUT_BUCKETS: usize = 8; // Output buckets

pub const NNUE_QA: i32 = 255;
pub const NNUE_QB: i32 = 64;
pub const NNUE_SCALE: i32 = 400;

const NNUE_INPUT_BUCKET_MAP: [usize; 64] = [
    0, 1, 2, 3, 3, 2, 1, 0,  // Rank 1
    4, 4, 5, 5, 5, 5, 4, 4,  // Rank 2
    6, 6, 6, 6, 6, 6, 6, 6,  // Rank 3
    7, 7, 7, 7, 7, 7, 7, 7,  // Rank 4
    8, 8, 8, 8, 8, 8, 8, 8,  // Rank 5
    8, 8, 8, 8, 8, 8, 8, 8,  // Rank 6
    9, 9, 9, 9, 9, 9, 9, 9,  // Rank 7
    9, 9, 9, 9, 9, 9, 9, 9,  // Rank 8
];

#[derive(Clone)]
pub struct NNUENetwork {
    pub ft_weights: Box<[[[i16; NNUE_HIDDEN_SIZE]; NNUE_INPUT_SIZE]; NNUE_INPUT_BUCKETS]>,
    pub ft_biases: Box<[i16; NNUE_HIDDEN_SIZE]>,
    pub output_weights: Box<[[i16; 2 * NNUE_HIDDEN_SIZE]; NNUE_OUTPUT_BUCKETS]>,
    pub output_biases: Box<[i16; NNUE_OUTPUT_BUCKETS]>,
    pub loaded: bool,
}

impl NNUENetwork {
    pub const EMBEDDED_NET_BYTES: &'static [u8] = include_bytes!("../eval_models/quantised.bin");

    pub fn new_empty() -> Self {
        Self {
            ft_weights: vec![[[0i16; NNUE_HIDDEN_SIZE]; NNUE_INPUT_SIZE]; NNUE_INPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            ft_biases: vec![0i16; NNUE_HIDDEN_SIZE].into_boxed_slice().try_into().unwrap(),
            output_weights: vec![[0i16; 2 * NNUE_HIDDEN_SIZE]; NNUE_OUTPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            output_biases: vec![0i16; NNUE_OUTPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            loaded: false,
        }
    }

    pub fn load_embedded() -> Result<Self, String> {
        Self::load_from_bytes(Self::EMBEDDED_NET_BYTES)
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let expected_data_size = NNUE_INPUT_BUCKETS * NNUE_INPUT_SIZE * NNUE_HIDDEN_SIZE * 2
            + NNUE_HIDDEN_SIZE * 2
            + NNUE_OUTPUT_BUCKETS * 2 * NNUE_HIDDEN_SIZE * 2
            + NNUE_OUTPUT_BUCKETS * 2;
        let expected_file_size = expected_data_size + 48;

        if bytes.len() != expected_file_size {
            return Err(format!(
                "NNUE byte size mismatch! Got {} bytes, expected {} bytes",
                bytes.len(),
                expected_file_size
            ));
        }

        let mut net = Self::new_empty();
        let mut offset = 0;

        let mut read_i16_slice = |buf: &mut [i16]| -> Result<(), String> {
            let byte_len = buf.len() * 2;
            if offset + byte_len > bytes.len() {
                return Err("Unexpected end of NNUE buffer".to_string());
            }
            for (i, item) in buf.iter_mut().enumerate() {
                let idx = offset + i * 2;
                *item = i16::from_le_bytes([bytes[idx], bytes[idx + 1]]);
            }
            offset += byte_len;
            Ok(())
        };

        for bucket in 0..NNUE_INPUT_BUCKETS {
            for i in 0..NNUE_INPUT_SIZE {
                read_i16_slice(&mut net.ft_weights[bucket][i])
                    .map_err(|e| format!("Failed to read feature transformer weights: {}", e))?;
            }
        }

        read_i16_slice(net.ft_biases.as_mut_slice())
            .map_err(|e| format!("Failed to read feature transformer biases: {}", e))?;

        for bucket in 0..NNUE_OUTPUT_BUCKETS {
            read_i16_slice(&mut net.output_weights[bucket])
                .map_err(|e| format!("Failed to read output weights for bucket {}: {}", bucket, e))?;
        }

        read_i16_slice(net.output_biases.as_mut_slice())
            .map_err(|e| format!("Failed to read output biases: {}", e))?;

        net.loaded = true;
        Ok(net)
    }

    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open NNUE file '{}': {}", path, e))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(|e| format!("Failed to read NNUE file '{}': {}", path, e))?;
        Self::load_from_bytes(&bytes)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct NNUEAccumulator {
    pub white: [i16; NNUE_HIDDEN_SIZE],
    pub black: [i16; NNUE_HIDDEN_SIZE],
    pub computed: bool,
}

impl NNUEAccumulator {
    pub fn new() -> Self {
        Self {
            white: [0; NNUE_HIDDEN_SIZE],
            black: [0; NNUE_HIDDEN_SIZE],
            computed: false,
        }
    }
}

struct KingBucket {
    index: usize,
    mirrored: bool,
}

fn get_king_bucket(king_square: usize, perspective: usize) -> KingBucket {
    let transformed_sq = if perspective == 1 { king_square ^ 56 } else { king_square };
    KingBucket {
        index: NNUE_INPUT_BUCKET_MAP[transformed_sq],
        mirrored: (transformed_sq % 8) >= 4,
    }
}

fn get_feature_index(
    perspective: usize,
    piece_type: usize,
    piece_color: usize,
    square: usize,
    king_bucket: &KingBucket,
) -> (usize, usize) {
    let color_stride = 64 * 6;
    let piece_stride = 64;

    let (mapped_color, transformed_square) = if perspective == 1 {
        let mc = piece_color ^ 1;
        let ts = if king_bucket.mirrored { square ^ 63 } else { square ^ 56 };
        (mc, ts)
    } else {
        let mc = piece_color;
        let ts = if king_bucket.mirrored { square ^ 7 } else { square };
        (mc, ts)
    };

    let input_idx = mapped_color * color_stride + piece_type * piece_stride + transformed_square;
    (king_bucket.index, input_idx)
}

#[inline(always)]
fn add_weights(acc: &mut [i16; NNUE_HIDDEN_SIZE], weights: &[i16; NNUE_HIDDEN_SIZE]) {
    for i in 0..NNUE_HIDDEN_SIZE {
        acc[i] += weights[i];
    }
}

#[inline(always)]
fn sub_weights(acc: &mut [i16; NNUE_HIDDEN_SIZE], weights: &[i16; NNUE_HIDDEN_SIZE]) {
    for i in 0..NNUE_HIDDEN_SIZE {
        acc[i] -= weights[i];
    }
}

#[inline(always)]
pub fn piece_to_type_and_color(piece: u8) -> (usize, usize) {
    let color = if piece >= 20 { 1 } else { 0 };
    let piece_type = match piece % 10 {
        0 => 0, // Pawn
        1 => 3, // Rook
        2 => 1, // Knight
        3 => 2, // Bishop
        4 => 4, // Queen
        5 => 5, // King
        _ => 0,
    };
    (piece_type, color)
}

pub struct NNUEService;

impl NNUEService {
    pub fn compute_accumulator(board: &Board, net: &NNUENetwork) -> NNUEAccumulator {
        let mut acc = NNUEAccumulator::new();
        if !net.loaded {
            return acc;
        }

        let white_king_sq = board.bitboards[WHITE_KING].trailing_zeros() as usize;
        let black_king_sq = board.bitboards[BLACK_KING].trailing_zeros() as usize;

        if white_king_sq >= 64 || black_king_sq >= 64 {
            return acc;
        }

        let white_bucket = get_king_bucket(white_king_sq, 0);
        let black_bucket = get_king_bucket(black_king_sq, 1);

        acc.white.copy_from_slice(net.ft_biases.as_ref());
        acc.black.copy_from_slice(net.ft_biases.as_ref());

        let pieces_list = [
            (board.bitboards[WHITE_PAWN], 0, 0),
            (board.bitboards[WHITE_KNIGHT], 1, 0),
            (board.bitboards[WHITE_BISHOP], 2, 0),
            (board.bitboards[WHITE_ROOK], 3, 0),
            (board.bitboards[WHITE_QUEEN], 4, 0),
            (board.bitboards[WHITE_KING], 5, 0),
            (board.bitboards[BLACK_PAWN], 0, 1),
            (board.bitboards[BLACK_KNIGHT], 1, 1),
            (board.bitboards[BLACK_BISHOP], 2, 1),
            (board.bitboards[BLACK_ROOK], 3, 1),
            (board.bitboards[BLACK_QUEEN], 4, 1),
            (board.bitboards[BLACK_KING], 5, 1),
        ];

        for (mut bb, piece_type, piece_color) in pieces_list {
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                bb &= bb - 1;

                let (w_bucket, w_input) = get_feature_index(0, piece_type, piece_color, sq, &white_bucket);
                let w_weights = &net.ft_weights[w_bucket][w_input];
                for i in 0..NNUE_HIDDEN_SIZE {
                    acc.white[i] += w_weights[i];
                }

                let (b_bucket, b_input) = get_feature_index(1, piece_type, piece_color, sq, &black_bucket);
                let b_weights = &net.ft_weights[b_bucket][b_input];
                for i in 0..NNUE_HIDDEN_SIZE {
                    acc.black[i] += b_weights[i];
                }
            }
        }

        acc.computed = true;
        acc
    }

    pub fn get_output_bucket(board: &Board) -> usize {
        let mut piece_count = 0;
        for i in 0..12 {
            piece_count += board.bitboards[i].count_ones() as usize;
        }
        let bucket = (piece_count as i32 - 2) / 4;
        bucket.clamp(0, (NNUE_OUTPUT_BUCKETS - 1) as i32) as usize
    }

    fn screlu_dot(acc: &[i16; NNUE_HIDDEN_SIZE], weights: &[i16]) -> i32 {
        let mut sum: i32 = 0;
        for i in 0..NNUE_HIDDEN_SIZE {
            let val = acc[i] as i32;
            let clamped = val.clamp(0, NNUE_QA);
            sum += clamped * clamped * (weights[i] as i32);
        }
        sum
    }

    pub fn refresh_perspective(
        board: &Board,
        net: &NNUENetwork,
        perspective: usize,
        out: &mut [i16; NNUE_HIDDEN_SIZE],
    ) {
        if !net.loaded {
            return;
        }

        let king_sq = if perspective == 0 {
            board.bitboards[WHITE_KING].trailing_zeros() as usize
        } else {
            board.bitboards[BLACK_KING].trailing_zeros() as usize
        };

        if king_sq >= 64 {
            return;
        }

        let king_bucket = get_king_bucket(king_sq, perspective);
        out.copy_from_slice(net.ft_biases.as_ref());

        let pieces_list = [
            (board.bitboards[WHITE_PAWN], 0, 0),
            (board.bitboards[WHITE_KNIGHT], 1, 0),
            (board.bitboards[WHITE_BISHOP], 2, 0),
            (board.bitboards[WHITE_ROOK], 3, 0),
            (board.bitboards[WHITE_QUEEN], 4, 0),
            (board.bitboards[WHITE_KING], 5, 0),
            (board.bitboards[BLACK_PAWN], 0, 1),
            (board.bitboards[BLACK_KNIGHT], 1, 1),
            (board.bitboards[BLACK_BISHOP], 2, 1),
            (board.bitboards[BLACK_ROOK], 3, 1),
            (board.bitboards[BLACK_QUEEN], 4, 1),
            (board.bitboards[BLACK_KING], 5, 1),
        ];

        for (mut bb, piece_type, piece_color) in pieces_list {
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                bb &= bb - 1;

                let (bucket, input_idx) = get_feature_index(perspective, piece_type, piece_color, sq, &king_bucket);
                let weights = &net.ft_weights[bucket][input_idx];
                for i in 0..NNUE_HIDDEN_SIZE {
                    out[i] += weights[i];
                }
            }
        }
    }

    pub fn update_accumulator(
        parent: &NNUEAccumulator,
        board: &Board,
        turn: &Turn,
        move_info: &MoveInformation,
        net: &NNUENetwork,
    ) -> NNUEAccumulator {
        if !net.loaded {
            return NNUEAccumulator::new();
        }
        if !parent.computed {
            return Self::compute_accumulator(board, net);
        }

        let white_king_sq = board.bitboards[WHITE_KING].trailing_zeros() as usize;
        let black_king_sq = board.bitboards[BLACK_KING].trailing_zeros() as usize;

        if white_king_sq >= 64 || black_king_sq >= 64 {
            return Self::compute_accumulator(board, net);
        }

        let moved_piece = move_info.moved_piece;
        let from = turn.from as usize;
        let to = turn.to as usize;

        let is_castling = (moved_piece == 15 || moved_piece == 25) && (to as i8 - from as i8).abs() == 2;
        let is_en_passant = (moved_piece == 10 || moved_piece == 20) && (to as i8 == move_info.en_passante);
        let is_promotion = turn.is_promotion();

        let (removed, removed_count, added, added_count) = if is_castling {
            let mut rem = [(0u8, 0usize); 2];
            let mut add = [(0u8, 0usize); 2];
            rem[0] = (moved_piece, from);
            add[0] = (moved_piece, to);
            let mut rc = 1;
            let mut ac = 1;

            let (rook_from, rook_to, rook_piece) = match to {
                6 => (7, 5, 11),    // White short
                2 => (0, 3, 11),    // White long
                62 => (63, 61, 21), // Black short
                58 => (56, 59, 21), // Black long
                _ => (0, 0, 0),
            };
            if rook_piece != 0 {
                rem[1] = (rook_piece, rook_from);
                add[1] = (rook_piece, rook_to);
                rc = 2;
                ac = 2;
            }
            (rem, rc, add, ac)
        } else if is_en_passant {
            let mut rem = [(0u8, 0usize); 2];
            let mut add = [(0u8, 0usize); 2];
            rem[0] = (moved_piece, from);
            add[0] = (moved_piece, to);
            let (victim_sq, victim_piece) = if moved_piece == 10 {
                (to - 8, 20)
            } else {
                (to + 8, 10)
            };
            rem[1] = (victim_piece, victim_sq);
            (rem, 2, add, 1)
        } else if is_promotion {
            let mut rem = [(0u8, 0usize); 2];
            let mut add = [(0u8, 0usize); 2];
            rem[0] = (moved_piece, from);
            add[0] = (turn.promotion, to);
            let rc = if move_info.capture != 0 {
                rem[1] = (move_info.capture, to);
                2
            } else {
                1
            };
            (rem, rc, add, 1)
        } else {
            let mut rem = [(0u8, 0usize); 2];
            let mut add = [(0u8, 0usize); 2];
            rem[0] = (moved_piece, from);
            add[0] = (moved_piece, to);
            let rc = if move_info.capture != 0 {
                rem[1] = (move_info.capture, to);
                2
            } else {
                1
            };
            (rem, rc, add, 1)
        };

        let mut new_acc = NNUEAccumulator::new();

        // White perspective
        let white_bucket = get_king_bucket(white_king_sq, 0);
        if moved_piece == 15 {
            let old_bucket = get_king_bucket(from, 0);
            if old_bucket.index != white_bucket.index || old_bucket.mirrored != white_bucket.mirrored {
                Self::refresh_perspective(board, net, 0, &mut new_acc.white);
            } else {
                new_acc.white = parent.white;
                for i in 0..removed_count {
                    let (p, sq) = removed[i];
                    let (pt, pc) = piece_to_type_and_color(p);
                    let (b, idx) = get_feature_index(0, pt, pc, sq, &white_bucket);
                    sub_weights(&mut new_acc.white, &net.ft_weights[b][idx]);
                }
                for i in 0..added_count {
                    let (p, sq) = added[i];
                    let (pt, pc) = piece_to_type_and_color(p);
                    let (b, idx) = get_feature_index(0, pt, pc, sq, &white_bucket);
                    add_weights(&mut new_acc.white, &net.ft_weights[b][idx]);
                }
            }
        } else {
            new_acc.white = parent.white;
            for i in 0..removed_count {
                let (p, sq) = removed[i];
                let (pt, pc) = piece_to_type_and_color(p);
                let (b, idx) = get_feature_index(0, pt, pc, sq, &white_bucket);
                sub_weights(&mut new_acc.white, &net.ft_weights[b][idx]);
            }
            for i in 0..added_count {
                let (p, sq) = added[i];
                let (pt, pc) = piece_to_type_and_color(p);
                let (b, idx) = get_feature_index(0, pt, pc, sq, &white_bucket);
                add_weights(&mut new_acc.white, &net.ft_weights[b][idx]);
            }
        }

        // Black perspective
        let black_bucket = get_king_bucket(black_king_sq, 1);
        if moved_piece == 25 {
            let old_bucket = get_king_bucket(from, 1);
            if old_bucket.index != black_bucket.index || old_bucket.mirrored != black_bucket.mirrored {
                Self::refresh_perspective(board, net, 1, &mut new_acc.black);
            } else {
                new_acc.black = parent.black;
                for i in 0..removed_count {
                    let (p, sq) = removed[i];
                    let (pt, pc) = piece_to_type_and_color(p);
                    let (b, idx) = get_feature_index(1, pt, pc, sq, &black_bucket);
                    sub_weights(&mut new_acc.black, &net.ft_weights[b][idx]);
                }
                for i in 0..added_count {
                    let (p, sq) = added[i];
                    let (pt, pc) = piece_to_type_and_color(p);
                    let (b, idx) = get_feature_index(1, pt, pc, sq, &black_bucket);
                    add_weights(&mut new_acc.black, &net.ft_weights[b][idx]);
                }
            }
        } else {
            new_acc.black = parent.black;
            for i in 0..removed_count {
                let (p, sq) = removed[i];
                let (pt, pc) = piece_to_type_and_color(p);
                let (b, idx) = get_feature_index(1, pt, pc, sq, &black_bucket);
                sub_weights(&mut new_acc.black, &net.ft_weights[b][idx]);
            }
            for i in 0..added_count {
                let (p, sq) = added[i];
                let (pt, pc) = piece_to_type_and_color(p);
                let (b, idx) = get_feature_index(1, pt, pc, sq, &black_bucket);
                add_weights(&mut new_acc.black, &net.ft_weights[b][idx]);
            }
        }

        new_acc.computed = true;
        new_acc
    }

    pub fn evaluate_with_accumulator(board: &Board, net: &NNUENetwork, acc: &NNUEAccumulator) -> i16 {
        if !net.loaded || !acc.computed {
            return 0;
        }

        let output_bucket = Self::get_output_bucket(board);
        let (us_acc, them_acc) = if board.white_to_move {
            (&acc.white, &acc.black)
        } else {
            (&acc.black, &acc.white)
        };

        let weights = &net.output_weights[output_bucket];
        let us_weights = &weights[0..NNUE_HIDDEN_SIZE];
        let them_weights = &weights[NNUE_HIDDEN_SIZE..2 * NNUE_HIDDEN_SIZE];

        let mut output = Self::screlu_dot(us_acc, us_weights) + Self::screlu_dot(them_acc, them_weights);
        output /= NNUE_QA;
        output += net.output_biases[output_bucket] as i32;

        let eval = (output * NNUE_SCALE) / (NNUE_QA * NNUE_QB);
        let final_eval = eval.clamp(i16::MIN as i32, i16::MAX as i32) as i16;

        if board.white_to_move {
            final_eval
        } else {
            -final_eval
        }
    }

    pub fn evaluate(board: &Board, net: &NNUENetwork) -> i16 {
        if !net.loaded {
            return 0;
        }

        let acc = Self::compute_accumulator(board, net);
        Self::evaluate_with_accumulator(board, net, &acc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen_service::FenService;

    #[test]
    fn test_nnue_model_loading_and_evaluation() {
        let net_result = NNUENetwork::load_from_file("eval_models/quantised.bin");
        assert!(net_result.is_ok(), "Failed to load quantised.bin: {:?}", net_result.err());
        let net = net_result.unwrap();
        assert!(net.loaded);

        let fen_service = FenService;
        let board = fen_service.set_fen(crate::model::INIT_BOARD_FEN);
        let eval = NNUEService::evaluate(&board, &net);
        // Startpos evaluation should be reasonable (close to 0 cp)
        assert!(eval.abs() < 200, "Startpos NNUE eval should be reasonable, got: {}", eval);
    }

    #[test]
    fn test_nnue_model_loading_missing_file() {
        let missing_path = "eval_models/non_existent_model_file.bin";
        let net_result = NNUENetwork::load_from_file(missing_path);
        assert!(net_result.is_err());
        let err_msg = net_result.err().unwrap();
        assert!(
            err_msg.contains("Failed to open NNUE file 'eval_models/non_existent_model_file.bin'"),
            "Error message should mention missing file, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_incremental_accumulator_quiet_and_captures() {
        let net = NNUENetwork::load_from_file("eval_models/quantised.bin").unwrap();
        let fen_service = FenService;
        let mut board = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let parent_acc = NNUEService::compute_accumulator(&board, &net);

        // 1. e2-e4 (quiet pawn move)
        let turn1 = crate::notation_util::NotationUtil::get_turn_from_notation("e2e4");
        let mi1 = board.do_move(&turn1);
        let inc_acc1 = NNUEService::update_accumulator(&parent_acc, &board, &turn1, &mi1, &net);
        let comp_acc1 = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc1, comp_acc1, "Accumulator divergence on quiet pawn move e2e4");

        // 2. d7-d5 (quiet pawn move for Black)
        let turn2 = crate::notation_util::NotationUtil::get_turn_from_notation("d7d5");
        let mi2 = board.do_move(&turn2);
        let inc_acc2 = NNUEService::update_accumulator(&inc_acc1, &board, &turn2, &mi2, &net);
        let comp_acc2 = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc2, comp_acc2, "Accumulator divergence on quiet pawn move d7d5");

        // 3. e4 x d5 (capture)
        let turn3 = crate::notation_util::NotationUtil::get_turn_from_notation("e4d5");
        let mi3 = board.do_move(&turn3);
        let inc_acc3 = NNUEService::update_accumulator(&inc_acc2, &board, &turn3, &mi3, &net);
        let comp_acc3 = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc3, comp_acc3, "Accumulator divergence on capture e4xd5");
    }

    #[test]
    fn test_incremental_accumulator_en_passant() {
        let net = NNUENetwork::load_from_file("eval_models/quantised.bin").unwrap();
        let fen_service = FenService;
        // Position where White can play en passant: White pawn on e5, Black plays d7-d5
        let mut board = fen_service.set_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2");
        let parent_acc = NNUEService::compute_accumulator(&board, &net);

        let ep_turn = crate::notation_util::NotationUtil::get_turn_from_notation("e5d6");
        let mi = board.do_move(&ep_turn);
        let inc_acc = NNUEService::update_accumulator(&parent_acc, &board, &ep_turn, &mi, &net);
        let comp_acc = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc, comp_acc, "Accumulator divergence on En Passant e5xd6");
    }

    #[test]
    fn test_incremental_accumulator_castling() {
        let net = NNUENetwork::load_from_file("eval_models/quantised.bin").unwrap();
        let fen_service = FenService;
        // White can castle short, Black can castle long
        let mut board = fen_service.set_fen("r3k2r/pppq1ppp/2np1n2/2b1p1B1/2B1P1b1/2NP1N2/PPPQ1PPP/R3K2R w KQkq - 4 7");
        let parent_acc = NNUEService::compute_accumulator(&board, &net);

        // White castles short: e1g1
        let castle_white = crate::notation_util::NotationUtil::get_turn_from_notation("e1g1");
        let mi_w = board.do_move(&castle_white);
        let inc_acc_w = NNUEService::update_accumulator(&parent_acc, &board, &castle_white, &mi_w, &net);
        let comp_acc_w = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc_w, comp_acc_w, "Accumulator divergence on White short castling");

        // Black castles long: e8c8
        let castle_black = crate::notation_util::NotationUtil::get_turn_from_notation("e8c8");
        let mi_b = board.do_move(&castle_black);
        let inc_acc_b = NNUEService::update_accumulator(&inc_acc_w, &board, &castle_black, &mi_b, &net);
        let comp_acc_b = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc_b, comp_acc_b, "Accumulator divergence on Black long castling");
    }

    #[test]
    fn test_incremental_accumulator_promotions() {
        let net = NNUENetwork::load_from_file("eval_models/quantised.bin").unwrap();
        let fen_service = FenService;
        // White pawn on e7, can promote to Queen or capture and promote
        let mut board = fen_service.set_fen("r1bqk2r/ppppPppp/2n5/8/8/5N2/PPPP1PPP/RNBQKB1R w KQkq - 1 6");
        let parent_acc = NNUEService::compute_accumulator(&board, &net);

        // 1. Capture promotion: e7xd8q
        let promo_capture = crate::notation_util::NotationUtil::get_turn_from_notation("e7d8q");
        let mi = board.do_move(&promo_capture);
        let inc_acc = NNUEService::update_accumulator(&parent_acc, &board, &promo_capture, &mi, &net);
        let comp_acc = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc, comp_acc, "Accumulator divergence on promotion with capture e7xd8q");

        // Undo move and test underpromotion without capture
        board.undo_move(&promo_capture, mi);
        // Change position slightly to test quiet promotion
        let mut board2 = fen_service.set_fen("4k3/4P3/8/8/8/8/8/4K3 w - - 0 1");
        let parent_acc2 = NNUEService::compute_accumulator(&board2, &net);
        let promo_quiet = crate::notation_util::NotationUtil::get_turn_from_notation("e7e8n");
        let mi2 = board2.do_move(&promo_quiet);
        let inc_acc2 = NNUEService::update_accumulator(&parent_acc2, &board2, &promo_quiet, &mi2, &net);
        let comp_acc2 = NNUEService::compute_accumulator(&board2, &net);
        assert_eq!(inc_acc2, comp_acc2, "Accumulator divergence on quiet knight underpromotion e7e8n");
    }

    #[test]
    fn test_incremental_accumulator_king_moves_and_bucket_changes() {
        let net = NNUENetwork::load_from_file("eval_models/quantised.bin").unwrap();
        let fen_service = FenService;
        // King walks across squares: e1 -> d1 (mirror change / bucket test), d1 -> c1, etc.
        let mut board = fen_service.set_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1");
        let parent_acc = NNUEService::compute_accumulator(&board, &net);

        let turn1 = crate::notation_util::NotationUtil::get_turn_from_notation("e1d1");
        let mi1 = board.do_move(&turn1);
        let inc_acc1 = NNUEService::update_accumulator(&parent_acc, &board, &turn1, &mi1, &net);
        let comp_acc1 = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc1, comp_acc1, "Accumulator divergence on King move e1-d1");

        let turn2 = crate::notation_util::NotationUtil::get_turn_from_notation("e8d8");
        let mi2 = board.do_move(&turn2);
        let inc_acc2 = NNUEService::update_accumulator(&inc_acc1, &board, &turn2, &mi2, &net);
        let comp_acc2 = NNUEService::compute_accumulator(&board, &net);
        assert_eq!(inc_acc2, comp_acc2, "Accumulator divergence on Black King move e8-d8");
    }

    #[test]
    fn test_incremental_accumulator_long_move_sequence() {
        let net = NNUENetwork::load_from_file("eval_models/quantised.bin").unwrap();
        let fen_service = FenService;
        let mut board = fen_service.set_fen(crate::model::INIT_BOARD_FEN);
        let mut current_acc = NNUEService::compute_accumulator(&board, &net);

        // 20-ply standard opening sequence (Ruy Lopez: Morphy Defense)
        let moves = [
            "e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6", "b5a4", "g8f6",
            "e1g1", "b7b5", "a4b3", "f8e7", "f1e1", "d7d6", "c2c3", "e8g8",
            "h2h3", "c6b8", "d2d4", "b8d7",
        ];

        for mv in moves {
            let turn = crate::notation_util::NotationUtil::get_turn_from_notation(mv);
            let mi = board.do_move(&turn);
            current_acc = NNUEService::update_accumulator(&current_acc, &board, &turn, &mi, &net);
            let comp_acc = NNUEService::compute_accumulator(&board, &net);
            assert_eq!(current_acc, comp_acc, "Accumulator divergence at move {}", mv);
        }
    }

    #[test]
    fn test_incremental_accumulator_kiwipete_tree() {
        let net = NNUENetwork::load_from_file("eval_models/quantised.bin").unwrap();
        let service = crate::service::Service::new();
        let config = crate::config::Config::for_tests();
        let zobrist_table = crate::zobrist::ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = crate::model::SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
            start_time: std::time::Instant::now(),
            target_time: None,
            root_moves_total: 0,
            root_moves_searched: 0,
            root_depth: 0,
        };

        let mut board = service.fen.set_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        let parent_acc = NNUEService::compute_accumulator(&board, &net);

        let mut moves = crate::model::MoveList::new();
        service.move_gen.generate_valid_moves_list(&mut board, &mut crate::model::Stats::new(), &config, &context, true, &mut moves);

        // Test all moves in Kiwipete (captures, quiet, en passant, castling)
        for i in 0..moves.len {
            let turn = moves.moves[i];
            let mi = board.do_move(&turn);
            let inc_acc = NNUEService::update_accumulator(&parent_acc, &board, &turn, &mi, &net);
            let comp_acc = NNUEService::compute_accumulator(&board, &net);
            assert_eq!(inc_acc, comp_acc, "Kiwipete depth 1 divergence on move {:?}", turn);

            // Also test depth 2 for the first 5 branches
            if i < 5 {
                let mut d2_moves = crate::model::MoveList::new();
                service.move_gen.generate_valid_moves_list(&mut board, &mut crate::model::Stats::new(), &config, &context, true, &mut d2_moves);
                for j in 0..d2_moves.len {
                    let d2_turn = d2_moves.moves[j];
                    let mi2 = board.do_move(&d2_turn);
                    let inc_acc2 = NNUEService::update_accumulator(&inc_acc, &board, &d2_turn, &mi2, &net);
                    let comp_acc2 = NNUEService::compute_accumulator(&board, &net);
                    assert_eq!(inc_acc2, comp_acc2, "Kiwipete depth 2 divergence on {:?} -> {:?}", turn, d2_turn);
                    board.undo_move(&d2_turn, mi2);
                }
            }

            board.undo_move(&turn, mi);
        }
    }
}

