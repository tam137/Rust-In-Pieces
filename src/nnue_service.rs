use std::fs::File;
use std::io::{Read, Result as IoResult};
use crate::model::{
    Board, WHITE_PAWN, WHITE_ROOK, WHITE_KNIGHT, WHITE_BISHOP, WHITE_QUEEN, WHITE_KING,
    BLACK_PAWN, BLACK_ROOK, BLACK_KNIGHT, BLACK_BISHOP, BLACK_QUEEN, BLACK_KING,
};

pub const NNUE_INPUT_SIZE: usize = 768;   // 64 squares * 6 piece types * 2 colors
pub const NNUE_HIDDEN_SIZE: usize = 256;  // Hidden layer neurons per perspective
pub const NNUE_INPUT_BUCKETS: usize = 10;  // King position buckets
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
    pub fn new_empty() -> Self {
        Self {
            ft_weights: vec![[[0i16; NNUE_HIDDEN_SIZE]; NNUE_INPUT_SIZE]; NNUE_INPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            ft_biases: vec![0i16; NNUE_HIDDEN_SIZE].into_boxed_slice().try_into().unwrap(),
            output_weights: vec![[0i16; 2 * NNUE_HIDDEN_SIZE]; NNUE_OUTPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            output_biases: vec![0i16; NNUE_OUTPUT_BUCKETS].into_boxed_slice().try_into().unwrap(),
            loaded: false,
        }
    }

    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let mut file = File::open(path).map_err(|e| format!("Failed to open NNUE file '{}': {}", path, e))?;
        let metadata = file.metadata().map_err(|e| format!("Failed to read metadata for '{}': {}", path, e))?;
        
        let expected_data_size = NNUE_INPUT_BUCKETS * NNUE_INPUT_SIZE * NNUE_HIDDEN_SIZE * 2
            + NNUE_HIDDEN_SIZE * 2
            + NNUE_OUTPUT_BUCKETS * 2 * NNUE_HIDDEN_SIZE * 2
            + NNUE_OUTPUT_BUCKETS * 2;
        let expected_file_size = expected_data_size + 48;

        if metadata.len() != expected_file_size as u64 {
            return Err(format!(
                "NNUE file size mismatch! Got {} bytes, expected {} bytes",
                metadata.len(),
                expected_file_size
            ));
        }

        let mut net = Self::new_empty();

        let read_i16_slice = |file: &mut File, buf: &mut [i16]| -> IoResult<()> {
            let byte_len = buf.len() * 2;
            let mut byte_buf = vec![0u8; byte_len];
            file.read_exact(&mut byte_buf)?;
            for i in 0..buf.len() {
                buf[i] = i16::from_le_bytes([byte_buf[i * 2], byte_buf[i * 2 + 1]]);
            }
            Ok(())
        };

        for bucket in 0..NNUE_INPUT_BUCKETS {
            for i in 0..NNUE_INPUT_SIZE {
                read_i16_slice(&mut file, &mut net.ft_weights[bucket][i])
                    .map_err(|e| format!("Failed to read feature transformer weights: {}", e))?;
            }
        }

        read_i16_slice(&mut file, net.ft_biases.as_mut_slice())
            .map_err(|e| format!("Failed to read feature transformer biases: {}", e))?;

        for bucket in 0..NNUE_OUTPUT_BUCKETS {
            read_i16_slice(&mut file, &mut net.output_weights[bucket])
                .map_err(|e| format!("Failed to read output weights for bucket {}: {}", bucket, e))?;
        }

        read_i16_slice(&mut file, net.output_biases.as_mut_slice())
            .map_err(|e| format!("Failed to read output biases: {}", e))?;

        net.loaded = true;
        Ok(net)
    }
}

#[derive(Clone)]
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

