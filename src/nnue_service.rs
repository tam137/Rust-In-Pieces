use std::fs::File;
use std::io::{Read, Result as IoResult};
use crate::model::{
    Board, WHITE_PAWN, WHITE_ROOK, WHITE_KNIGHT, WHITE_BISHOP, WHITE_QUEEN, WHITE_KING,
    BLACK_PAWN, BLACK_ROOK, BLACK_KNIGHT, BLACK_BISHOP, BLACK_QUEEN, BLACK_KING,
};

/// Total feature size per perspective: 64 squares * 6 piece types * 2 piece colors = 768 features.
pub const NNUE_INPUT_SIZE: usize = 768;
/// Size of the accumulator / hidden layer per perspective (White and Black).
pub const NNUE_HIDDEN_SIZE: usize = 256;
/// Number of input king buckets used for feature transformer weights.
pub const NNUE_INPUT_BUCKETS: usize = 10;
/// Number of output layer buckets based on total piece count.
pub const NNUE_OUTPUT_BUCKETS: usize = 8;

/// Quantization scale factor A for activation clipping (SCReLU range [0, QA]).
pub const NNUE_QA: i32 = 255;
/// Quantization scale factor B for output weights.
pub const NNUE_QB: i32 = 64;
/// Internal evaluation scaling factor to map neural output back to Centipawns.
pub const NNUE_SCALE: i32 = 400;

/// Map from square index (0..64) to king bucket index (0..9) for white perspective.
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

/// Holds the loaded NNUE model parameters in heap-allocated boxed arrays.
#[derive(Clone)]
pub struct NNUENetwork {
    /// Feature transformer weights: [input_bucket][feature_index][hidden_neuron]
    pub ft_weights: Box<[[[i16; NNUE_HIDDEN_SIZE]; NNUE_INPUT_SIZE]; NNUE_INPUT_BUCKETS]>,
    /// Feature transformer biases shared across perspectives.
    pub ft_biases: Box<[i16; NNUE_HIDDEN_SIZE]>,
    /// Output layer weights: [output_bucket][2 * NNUE_HIDDEN_SIZE] (concatenated white & black accs).
    pub output_weights: Box<[[i16; 2 * NNUE_HIDDEN_SIZE]; NNUE_OUTPUT_BUCKETS]>,
    /// Output layer biases per output bucket.
    pub output_biases: Box<[i16; NNUE_OUTPUT_BUCKETS]>,
    /// Flag indicating whether weights were successfully loaded from disk.
    pub loaded: bool,
}

impl NNUENetwork {
    /// Creates an empty (zero-initialized) un-loaded NNUE network structure.
    pub fn new_empty() -> Self {
        Self {
            ft_weights: vec![[[0i16; NNUE_HIDDEN_SIZE]; NNUE_INPUT_SIZE]; NNUE_INPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            ft_biases: vec![0i16; NNUE_HIDDEN_SIZE].into_boxed_slice().try_into().unwrap(),
            output_weights: vec![[0i16; 2 * NNUE_HIDDEN_SIZE]; NNUE_OUTPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            output_biases: vec![0i16; NNUE_OUTPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            loaded: false,
        }
    }

    /// Loads binary NNUE network parameters from the specified file path.
    /// Performs strict file-size verification and little-endian i16 parsing.
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open NNUE file '{}': {}", path, e))?;
        let metadata = file.metadata().map_err(|e| format!("Failed to read metadata for '{}': {}", path, e))?;
        
        let expected_data_size = NNUE_INPUT_BUCKETS * NNUE_INPUT_SIZE * NNUE_HIDDEN_SIZE * 2
            + NNUE_HIDDEN_SIZE * 2
            + NNUE_OUTPUT_BUCKETS * 2 * NNUE_HIDDEN_SIZE * 2
            + NNUE_OUTPUT_BUCKETS * 2;
        let expected_file_size = expected_data_size + 48; // Data size + header offset

        if metadata.len() != expected_file_size as u64 {
            return Err(format!(
                "NNUE file size mismatch! Got {} bytes, expected {} bytes",
                metadata.len(),
                expected_file_size
            ));
        }

        let mut net = Self::new_empty();

        // Helper closures to read slice of i16 values from binary stream (Little-Endian)
        let read_i16_slice = |file: &mut File, buf: &mut [i16]| -> IoResult<()> {
            let byte_len = buf.len() * 2;
            let mut byte_buf = vec![0u8; byte_len];
            file.read_exact(&mut byte_buf)?;
            for i in 0..buf.len() {
                buf[i] = i16::from_le_bytes([byte_buf[i * 2], byte_buf[i * 2 + 1]]);
            }
            Ok(())
        };

        // Read feature transformer weights across all input buckets
        for bucket in 0..NNUE_INPUT_BUCKETS {
            for i in 0..NNUE_INPUT_SIZE {
                read_i16_slice(&mut file, &mut net.ft_weights[bucket][i])
                    .map_err(|e| format!("Failed to read feature transformer weights: {}", e))?;
            }
        }

        // Read feature transformer biases
        read_i16_slice(&mut file, net.ft_biases.as_mut_slice())
            .map_err(|e| format!("Failed to read feature transformer biases: {}", e))?;

        // Read output layer weights for each piece-count bucket
        for bucket in 0..NNUE_OUTPUT_BUCKETS {
            read_i16_slice(&mut file, &mut net.output_weights[bucket])
                .map_err(|e| format!("Failed to read output weights for bucket {}: {}", bucket, e))?;
        }

        // Read output layer biases
        read_i16_slice(&mut file, net.output_biases.as_mut_slice())
            .map_err(|e| format!("Failed to read output biases: {}", e))?;

        net.loaded = true;
        Ok(net)
    }
}

/// Accumulator state storing the accumulated feature transformer outputs
/// for White and Black perspectives.
#[derive(Clone)]
pub struct NNUEAccumulator {
    /// Hidden layer activation vector for White perspective.
    pub white: [i16; NNUE_HIDDEN_SIZE],
    /// Hidden layer activation vector for Black perspective.
    pub black: [i16; NNUE_HIDDEN_SIZE],
    /// True if accumulator has been computed for the current board state.
    pub computed: bool,
}

impl NNUEAccumulator {
    /// Creates a zeroed accumulator.
    pub fn new() -> Self {
        Self {
            white: [0; NNUE_HIDDEN_SIZE],
            black: [0; NNUE_HIDDEN_SIZE],
            computed: false,
        }
    }
}

/// Helper struct indicating the king's spatial bucket and horizontal mirroring state.
struct KingBucket {
    index: usize,
    mirrored: bool,
}

/// Determines the king bucket index (0..9) and horizontal mirroring flag for a given king position and perspective.
fn get_king_bucket(king_square: usize, perspective: usize) -> KingBucket {
    // Flip rank for Black perspective (perspective == 1)
    let transformed_sq = if perspective == 1 { king_square ^ 56 } else { king_square };
    KingBucket {
        index: NNUE_INPUT_BUCKET_MAP[transformed_sq],
        mirrored: (transformed_sq % 8) >= 4, // Mirror horizontally if king is on files E-H
    }
}

/// Computes the bucket index and feature index (0..768) for a piece on a given square,
/// accounting for perspective, piece color, and horizontal king symmetry.
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
        // Black perspective: flip colors and vertical ranks
        let mc = piece_color ^ 1;
        let ts = if king_bucket.mirrored { square ^ 63 } else { square ^ 56 };
        (mc, ts)
    } else {
        // White perspective
        let mc = piece_color;
        let ts = if king_bucket.mirrored { square ^ 7 } else { square };
        (mc, ts)
    };

    let input_idx = mapped_color * color_stride + piece_type * piece_stride + transformed_square;
    (king_bucket.index, input_idx)
}

/// Service providing NNUE accumulator evaluation and position scoring.
pub struct NNUEService;

impl NNUEService {
    /// Computes the feature transformer accumulator state for both White and Black perspectives.
    pub fn compute_accumulator(board: &Board, net: &NNUENetwork) -> NNUEAccumulator {
        let mut acc = NNUEAccumulator::new();
        if !net.loaded {
            return acc;
        }

        // Locate kings for both sides
        let white_king_sq = board.bitboards[WHITE_KING].trailing_zeros() as usize;
        let black_king_sq = board.bitboards[BLACK_KING].trailing_zeros() as usize;

        if white_king_sq >= 64 || black_king_sq >= 64 {
            return acc;
        }

        let white_bucket = get_king_bucket(white_king_sq, 0);
        let black_bucket = get_king_bucket(black_king_sq, 1);

        // Initialize accumulators with feature transformer biases
        acc.white.copy_from_slice(net.ft_biases.as_ref());
        acc.black.copy_from_slice(net.ft_biases.as_ref());

        // List all piece bitboards with (bitboard, piece_type, piece_color)
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

        // Accumulate feature weights for every active piece on the board
        for (mut bb, piece_type, piece_color) in pieces_list {
            while bb != 0 {
                let sq = bb.trailing_zeros() as usize;
                bb &= bb - 1;

                // White perspective accumulation
                let (w_bucket, w_input) = get_feature_index(0, piece_type, piece_color, sq, &white_bucket);
                let w_weights = &net.ft_weights[w_bucket][w_input];
                for i in 0..NNUE_HIDDEN_SIZE {
                    acc.white[i] += w_weights[i];
                }

                // Black perspective accumulation
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

    /// Determines the output bucket index (0..7) based on the total piece count on the board.
    pub fn get_output_bucket(board: &Board) -> usize {
        let mut piece_count = 0;
        for i in 0..12 {
            piece_count += board.bitboards[i].count_ones() as usize;
        }
        let bucket = (piece_count as i32 - 2) / 4;
        bucket.clamp(0, (NNUE_OUTPUT_BUCKETS - 1) as i32) as usize
    }

    /// Computes dot product with Square Clipped ReLU (SCReLU) activation: sum(clamp(acc, 0, QA)^2 * weight).
    /// Clamps accumulator values to [0, NNUE_QA] (0..255), squares them, and multiplies by output weights.
    fn screlu_dot(acc: &[i16; NNUE_HIDDEN_SIZE], weights: &[i16]) -> i32 {
        let mut sum: i32 = 0;
        for i in 0..NNUE_HIDDEN_SIZE {
            let val = acc[i] as i32;
            // SCReLU Clamping: limit raw accumulated activations to valid quantization range [0, 255]
            let clamped = val.clamp(0, NNUE_QA);
            // Square activation (clamped^2) and accumulate weighted output
            sum += clamped * clamped * (weights[i] as i32);
        }
        sum
    }

    /// Evaluates the board position using NNUE forward pass and returns score in Centipawns relative to side to move.
    pub fn evaluate(board: &Board, net: &NNUENetwork) -> i16 {
        if !net.loaded {
            return 0;
        }

        let acc = Self::compute_accumulator(board, net);
        if !acc.computed {
            return 0;
        }

        let output_bucket = Self::get_output_bucket(board);
        let (us_acc, them_acc) = if board.white_to_move {
            (&acc.white, &acc.black)
        } else {
            (&acc.black, &acc.white)
        };

        // Output weights split: [0..256] for side to move (us), [256..512] for opponent (them)
        let weights = &net.output_weights[output_bucket];
        let us_weights = &weights[0..NNUE_HIDDEN_SIZE];
        let them_weights = &weights[NNUE_HIDDEN_SIZE..2 * NNUE_HIDDEN_SIZE];

        // Perform SCReLU dot products for both perspectives and combine them
        let mut output = Self::screlu_dot(us_acc, us_weights) + Self::screlu_dot(them_acc, them_weights);

        // Normalize sum by NNUE_QA (255) to compensate for squaring factor (clamped^2 / QA)
        output /= NNUE_QA;

        // Add output layer base bias for the selected output bucket
        output += net.output_biases[output_bucket] as i32;

        // Rescale raw quantized neural score to standard Centipawns: (output * NNUE_SCALE) / (NNUE_QA * NNUE_QB)
        let eval = (output * NNUE_SCALE) / (NNUE_QA * NNUE_QB);
        let final_eval = eval.clamp(i16::MIN as i32, i16::MAX as i32) as i16;

        // Return score relative to side to move (positive = favorable for side to move)
        if board.white_to_move {
            final_eval
        } else {
            -final_eval
        }
    }

    /// Extracts detailed position telemetry (accumulators, SCReLU activations, buckets, eval) for visualizer report.
    pub fn extract_telemetry_frame(board: &Board, net: &NNUENetwork, ply: usize, fen: &str, move_uci: &str) -> Option<crate::visualizer_service::NNUEMoveFrame> {
        if !net.loaded {
            return None;
        }

        let acc = Self::compute_accumulator(board, net);
        if !acc.computed {
            return None;
        }

        let white_king_sq = board.bitboards[WHITE_KING].trailing_zeros() as usize;
        let black_king_sq = board.bitboards[BLACK_KING].trailing_zeros() as usize;
        let w_bucket = get_king_bucket(white_king_sq, 0).index;
        let b_bucket = get_king_bucket(black_king_sq, 1).index;
        let output_bucket = Self::get_output_bucket(board);

        let eval_cp = Self::evaluate(board, net);

        let white_screlu: Vec<i32> = acc.white.iter().map(|&v| {
            let clamped = v.clamp(0, NNUE_QA as i16);
            clamped as i32 * clamped as i32
        }).collect();

        let black_screlu: Vec<i32> = acc.black.iter().map(|&v| {
            let clamped = v.clamp(0, NNUE_QA as i16);
            clamped as i32 * clamped as i32
        }).collect();

        let weights = &net.output_weights[output_bucket];
        let (w_weights_slice, b_weights_slice) = if board.white_to_move {
            (&weights[0..NNUE_HIDDEN_SIZE], &weights[NNUE_HIDDEN_SIZE..2 * NNUE_HIDDEN_SIZE])
        } else {
            (&weights[NNUE_HIDDEN_SIZE..2 * NNUE_HIDDEN_SIZE], &weights[0..NNUE_HIDDEN_SIZE])
        };

        let white_weights = w_weights_slice.to_vec();
        let black_weights = b_weights_slice.to_vec();

        let white_contrib: Vec<i32> = white_screlu.iter().zip(white_weights.iter())
            .map(|(&s, &w)| s * w as i32)
            .collect();

        let black_contrib: Vec<i32> = black_screlu.iter().zip(black_weights.iter())
            .map(|(&s, &w)| s * w as i32)
            .collect();

        Some(crate::visualizer_service::NNUEMoveFrame {
            ply,
            fen: fen.to_string(),
            move_uci: move_uci.to_string(),
            white_acc: acc.white.to_vec(),
            black_acc: acc.black.to_vec(),
            white_screlu,
            black_screlu,
            white_weights,
            black_weights,
            white_contrib,
            black_contrib,
            eval_cp,
            white_king_bucket: w_bucket,
            black_king_bucket: b_bucket,
            output_bucket,
            side_to_move: if board.white_to_move { "w".to_string() } else { "b".to_string() },
        })
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
}
