use std::fs::File;
use std::io::Read;
use crate::model::{
    Board, MoveInformation, Turn, WHITE_PAWN, WHITE_ROOK, WHITE_KNIGHT, WHITE_BISHOP, WHITE_QUEEN, WHITE_KING,
    BLACK_PAWN, BLACK_ROOK, BLACK_KNIGHT, BLACK_BISHOP, BLACK_QUEEN, BLACK_KING,
};

pub const NNUE_INPUT_SIZE: usize = 768;   // 64 squares * 6 piece types * 2 colors
/// Hidden layer neurons per perspective: 512 since v0.60.0-NNUE, or 256 with the Cargo feature
/// `hidden256` (the width of every release before; for comparison builds).
pub const NNUE_HIDDEN_SIZE: usize = if cfg!(feature = "hidden256") { 256 } else { 512 };
pub const NNUE_INPUT_BUCKETS: usize = 10; // King position buckets
pub const NNUE_OUTPUT_BUCKETS: usize = 8; // Output buckets

pub const NNUE_QA: i32 = 255;
pub const NNUE_QB: i32 = 64;
pub const NNUE_SCALE: i32 = 400;

/// Largest output weight magnitude for which `clamp(v) * w`, `v` clamped to `[0, NNUE_QA]`, fits
/// an `i16` (255 * 128 = 32640). Nets beyond it are refused at load time, so the forward pass can
/// take that product in 16 bits. bullet clips the output layer to +-1.98, which quantises to +-127.
const MAX_OUTPUT_WEIGHT: u16 = (i16::MAX as i32 / NNUE_QA) as u16;

/// Neurons per 32-bit partial sum of the SCReLU dot product (see `NNUEService::screlu_dot`).
const SCRELU_CHUNK: usize = 256;
const _: () = assert!(NNUE_HIDDEN_SIZE % SCRELU_CHUNK == 0, "the hidden size must be a multiple of 256");
const _: () = assert!(
    (NNUE_QA as i64) * (NNUE_QA as i64) * (MAX_OUTPUT_WEIGHT as i64) * (SCRELU_CHUNK as i64) <= i32::MAX as i64,
    "one chunk of the SCReLU dot product must fit 32 bits"
);

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

/// `T` on a 64-byte boundary. Every row of the network is `2 * NNUE_HIDDEN_SIZE` bytes, a multiple
/// of 64, so a cache-line aligned array of rows starts each row on a line of its own (eight lines
/// at 256 neurons instead of nine), and no vector load splits a line.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C, align(64))]
pub struct CacheAligned<T>(pub T);

impl<T> std::ops::Deref for CacheAligned<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::DerefMut for CacheAligned<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A zeroed, cache-line aligned `T` on the heap, built in place: the feature transformer is 3.9 MB
/// and must never pass through the stack.
fn zeroed_aligned_box<T: Copy>() -> Box<CacheAligned<T>> {
    // SAFETY: only instantiated with arrays of `i16`, for which all-zero bytes are a valid value.
    unsafe { Box::<CacheAligned<T>>::new_zeroed().assume_init() }
}

#[derive(Clone)]
pub struct NNUENetwork {
    pub ft_weights: Box<CacheAligned<[[[i16; NNUE_HIDDEN_SIZE]; NNUE_INPUT_SIZE]; NNUE_INPUT_BUCKETS]>>,
    pub ft_biases: Box<CacheAligned<[i16; NNUE_HIDDEN_SIZE]>>,
    /// Per output bucket, the side to move's half `[0]` and the other side's `[1]`; the file stores
    /// them in that order, so this is the file's layout split at `NNUE_HIDDEN_SIZE`.
    pub output_weights: Box<CacheAligned<[[[i16; NNUE_HIDDEN_SIZE]; 2]; NNUE_OUTPUT_BUCKETS]>>,
    pub output_biases: Box<[i16; NNUE_OUTPUT_BUCKETS]>,
    pub loaded: bool,
}

impl NNUENetwork {
    #[cfg(feature = "hidden256")]
    pub const EMBEDDED_NET_BYTES: &'static [u8] = include_bytes!("../eval_models/quantised256.bin");
    #[cfg(not(feature = "hidden256"))]
    pub const EMBEDDED_NET_BYTES: &'static [u8] = include_bytes!("../eval_models/quantised.bin");

    pub fn new_empty() -> Self {
        Self {
            ft_weights: zeroed_aligned_box(),
            ft_biases: zeroed_aligned_box(),
            output_weights: zeroed_aligned_box(),
            output_biases: Box::new([0i16; NNUE_OUTPUT_BUCKETS]),
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
        // The trainer pads the file to a multiple of 64 bytes (`expected_quantised_bytes`).
        let expected_file_size = expected_data_size.next_multiple_of(64);

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

        read_i16_slice(&mut net.ft_biases.0)
            .map_err(|e| format!("Failed to read feature transformer biases: {}", e))?;

        for bucket in 0..NNUE_OUTPUT_BUCKETS {
            for half in 0..2 {
                read_i16_slice(&mut net.output_weights[bucket][half])
                    .map_err(|e| format!("Failed to read output weights for bucket {}: {}", bucket, e))?;
            }
        }

        read_i16_slice(net.output_biases.as_mut_slice())
            .map_err(|e| format!("Failed to read output biases: {}", e))?;

        if let Some(&w) = net.output_weights.iter().flatten().flatten().find(|w| w.unsigned_abs() > MAX_OUTPUT_WEIGHT) {
            return Err(format!(
                "NNUE output weight {} is outside +-{}: the SCReLU product clamp(v) * w would overflow 16 bits",
                w, MAX_OUTPUT_WEIGHT
            ));
        }

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

/// Default value of the `NnueModelPath` option. It selects the embedded network, as does an empty path.
pub const DEFAULT_MODEL_PATH: &str = "eval_models/quantised.bin";

/// Training-run name of the embedded network, kept next to it in `eval_models/quantised.name`.
#[cfg(feature = "hidden256")]
const EMBEDDED_NET_NAME: &str = include_str!("../eval_models/quantised256.name");
#[cfg(not(feature = "hidden256"))]
const EMBEDDED_NET_NAME: &str = include_str!("../eval_models/quantised.name");

/// True when `model_path` selects the embedded network rather than a file.
pub fn uses_embedded_net(model_path: &str) -> bool {
    model_path.is_empty() || model_path == DEFAULT_MODEL_PATH
}

/// 32-bit FNV-1a over the network file: tells two networks apart even when their names match.
pub fn net_fingerprint(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5u32, |hash, &byte| (hash ^ byte as u32).wrapping_mul(0x0100_0193))
}

/// One line naming the network that `model_path` selects, for the UCI `info string` after `uci` and
/// after `setoption name NnueModelPath`: name, origin and fingerprint.
pub fn describe_net(model_path: &str) -> String {
    if uses_embedded_net(model_path) {
        return format!("NNUE net {} (embedded, fnv {:08x})",
            EMBEDDED_NET_NAME.trim(), net_fingerprint(NNUENetwork::EMBEDDED_NET_BYTES));
    }
    // The path as given: a trainer checkpoint is always `quantised.bin`, its folder names the run.
    match std::fs::read(model_path) {
        Ok(bytes) => format!("NNUE net {} (file, fnv {:08x})", model_path, net_fingerprint(&bytes)),
        Err(e) => format!("NNUE net {} (file not readable: {})", model_path, e),
    }
}

/// Both perspectives, each a whole number of cache lines from a boundary, so every slot of the search's
/// stack starts on one too.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C, align(64))]
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

#[inline(always)]
fn get_king_bucket(king_square: usize, perspective: usize) -> KingBucket {
    let transformed_sq = if perspective == 1 { king_square ^ 56 } else { king_square };
    KingBucket {
        index: NNUE_INPUT_BUCKET_MAP[transformed_sq],
        mirrored: (transformed_sq % 8) >= 4,
    }
}

#[inline(always)]
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

/// The feature transformer row of `piece` on `sq` for one perspective. Forced inline: as a closure
/// body it was compiled out of line, one call per changed feature ahead of every vector loop.
#[inline(always)]
fn feature_weights<'a>(
    net: &'a NNUENetwork,
    perspective: usize,
    piece: u8,
    sq: usize,
    king_bucket: &KingBucket,
) -> &'a [i16; NNUE_HIDDEN_SIZE] {
    let (pt, pc) = piece_to_type_and_color(piece);
    let (bucket, idx) = get_feature_index(perspective, pt, pc, sq, king_bucket);
    &net.ft_weights[bucket][idx]
}

/// `out = parent - removed + added` for one perspective in one pass (Matt-Magie step 79, N1). The
/// terms are taken in the order the separate passes took them - every removed feature subtracted,
/// then every added one added - so each value is the one they produced.
#[inline(always)]
fn apply_delta(
    out: &mut [i16; NNUE_HIDDEN_SIZE],
    parent: &[i16; NNUE_HIDDEN_SIZE],
    perspective: usize,
    removed: &[(u8, usize)],
    added: &[(u8, usize)],
    king_bucket: &KingBucket,
    net: &NNUENetwork,
) {
    let weights = |&(piece, sq): &(u8, usize)| feature_weights(net, perspective, piece, sq, king_bucket);
    match (removed, added) {
        ([r0], [a0]) => {
            let (r0, a0) = (weights(r0), weights(a0));
            for i in 0..NNUE_HIDDEN_SIZE {
                out[i] = parent[i] - r0[i] + a0[i];
            }
        }
        ([r0, r1], [a0]) => {
            let (r0, r1, a0) = (weights(r0), weights(r1), weights(a0));
            for i in 0..NNUE_HIDDEN_SIZE {
                out[i] = parent[i] - r0[i] - r1[i] + a0[i];
            }
        }
        ([r0, r1], [a0, a1]) => {
            let (r0, r1, a0, a1) = (weights(r0), weights(r1), weights(a0), weights(a1));
            for i in 0..NNUE_HIDDEN_SIZE {
                out[i] = parent[i] - r0[i] - r1[i] + a0[i] + a1[i];
            }
        }
        _ => {
            *out = *parent;
            for feature in removed {
                sub_weights(out, weights(feature));
            }
            for feature in added {
                add_weights(out, weights(feature));
            }
        }
    }
}

/// Feature piece type and colour per board piece code (`10..=15` White, `20..=25` Black, units
/// pawn, rook, knight, bishop, queen, king), indexed by the code: one load instead of a division.
const PIECE_FEATURES: [(u8, u8); 32] = {
    let mut table = [(0u8, 0u8); 32];
    let types = [0u8, 3, 1, 2, 4, 5];
    let mut unit = 0;
    while unit < 6 {
        table[10 + unit] = (types[unit], 0);
        table[20 + unit] = (types[unit], 1);
        unit += 1;
    }
    table
};

#[inline(always)]
pub fn piece_to_type_and_color(piece: u8) -> (usize, usize) {
    let (piece_type, color) = PIECE_FEATURES[(piece & 31) as usize];
    (piece_type as usize, color as usize)
}

/// The twelve piece bitboards in board order, with the piece type and colour the features use.
const FEATURE_PIECES: [(usize, usize, usize); 12] = [
    (WHITE_PAWN, 0, 0), (WHITE_KNIGHT, 1, 0), (WHITE_BISHOP, 2, 0),
    (WHITE_ROOK, 3, 0), (WHITE_QUEEN, 4, 0), (WHITE_KING, 5, 0),
    (BLACK_PAWN, 0, 1), (BLACK_KNIGHT, 1, 1), (BLACK_BISHOP, 2, 1),
    (BLACK_ROOK, 3, 1), (BLACK_QUEEN, 4, 1), (BLACK_KING, 5, 1),
];

/// One cached perspective of the Finny table: the accumulator last built for this king bucket and
/// the piece bitboards it was built for.
#[derive(Copy, Clone)]
struct FinnyEntry {
    acc: [i16; NNUE_HIDDEN_SIZE],
    pieces: [u64; 12],
    valid: bool,
}

/// The Finny table (Matt-Magie step 82; Stockfish's accumulator caches, SleepMind's `c042109`): per
/// perspective and king bucket, mirrored or not, the accumulator last built there. A king that
/// crosses a bucket boundary no longer rebuilds its perspective from every piece but from the cached
/// one, adding and removing only the pieces that changed since. `i16` sums wrap and are exact in
/// any order, so the result is the full rebuild bit for bit.
pub struct FinnyTable {
    entries: [[[FinnyEntry; 2]; NNUE_INPUT_BUCKETS]; 2],
}

impl FinnyTable {
    pub fn new() -> Self {
        let empty = FinnyEntry { acc: [0; NNUE_HIDDEN_SIZE], pieces: [0; 12], valid: false };
        Self { entries: [[[empty; 2]; NNUE_INPUT_BUCKETS]; 2] }
    }
}

/// The search's accumulator per ply and the Finny table that serves its king-bucket rebuilds. A
/// slot reads and writes as `acc_stack[ply]`.
pub struct AccStack {
    slots: [NNUEAccumulator; crate::search_service::MAX_PLY],
    pub finny: FinnyTable,
}

impl AccStack {
    /// The stack lives on the search thread's stack: about 164 KB at 256 neurons and 315 KB at 512,
    /// well inside the thread's 2 MB. On the heap it measured 0.5 % slower (Matt-Magie step 103).
    pub fn new() -> Self {
        Self { slots: [NNUEAccumulator::new(); crate::search_service::MAX_PLY], finny: FinnyTable::new() }
    }

    /// The child at `child_idx` after `turn`, from the slot at `ply_idx` (step 79, N1), its
    /// king-bucket rebuilds served by the Finny table. The two slots differ except at the ply
    /// ceiling, where the clamped index makes them one; the parent is then copied first.
    #[inline(always)]
    pub fn update_child(&mut self, ply_idx: usize, child_idx: usize, board: &Board, turn: &Turn,
        move_info: &MoveInformation, net: &NNUENetwork) {
        if child_idx > ply_idx {
            let (lower, upper) = self.slots.split_at_mut(child_idx);
            NNUEService::update_accumulator_into(&lower[ply_idx], &mut upper[0], board, turn, move_info, net,
                &mut self.finny);
        } else {
            let parent = self.slots[ply_idx];
            NNUEService::update_accumulator_into(&parent, &mut self.slots[child_idx], board, turn, move_info, net,
                &mut self.finny);
        }
    }
}

impl std::ops::Index<usize> for AccStack {
    type Output = NNUEAccumulator;
    fn index(&self, ply: usize) -> &NNUEAccumulator {
        &self.slots[ply]
    }
}

impl std::ops::IndexMut<usize> for AccStack {
    fn index_mut(&mut self, ply: usize) -> &mut NNUEAccumulator {
        &mut self.slots[ply]
    }
}

pub struct NNUEService;

impl NNUEService {
    pub fn compute_accumulator(board: &Board, net: &NNUENetwork) -> NNUEAccumulator {
        let start = crate::nnue_prof::cycles();
        let acc = Self::compute_accumulator_inner(board, net);
        crate::nnue_prof::record_full(start);
        acc
    }

    fn compute_accumulator_inner(board: &Board, net: &NNUENetwork) -> NNUEAccumulator {
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

        acc.white.copy_from_slice(&net.ft_biases.0);
        acc.black.copy_from_slice(&net.ft_biases.0);

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
        let piece_count = board.occupied.count_ones();
        let bucket = (piece_count as i32 - 2) / 4;
        bucket.clamp(0, (NNUE_OUTPUT_BUCKETS - 1) as i32) as usize
    }

    /// `sum(clamp(v)^2 * w)`, taken as `(clamp(v) * w) * clamp(v)`: the first product in 16 bits,
    /// exact because the loader refuses any `|w| > MAX_OUTPUT_WEIGHT`, the second widened to 32.
    /// That is the shape of `pmaddwd` / `smlal`, eight or more lanes of i16 per instruction instead
    /// of two 32-bit multiplies per neuron, and its value is the exact formula's bit for bit.
    ///
    /// The 32-bit lanes hold at most `255^2 * 128 * 256 = 2_130_739_200 < i32::MAX` per 256 neurons;
    /// their wrapping sums are exact modulo 2^32 and therefore exact. With 256 neurons that is the
    /// whole row and the result stays in 32 bits, the code of every release up to v0.55.0-NNUE (any edit of this
    /// hot path moved the search speed by 1-2 % through code layout alone, Matt-Magie step 103).
    #[cfg(feature = "hidden256")]
    fn screlu_dot(acc: &[i16; NNUE_HIDDEN_SIZE], weights: &[i16; NNUE_HIDDEN_SIZE]) -> i32 {
        let mut sum: i32 = 0;
        for i in 0..NNUE_HIDDEN_SIZE {
            let clamped = acc[i].clamp(0, NNUE_QA as i16);
            let product = clamped.wrapping_mul(weights[i]);
            sum = sum.wrapping_add(product as i32 * clamped as i32);
        }
        sum
    }

    /// The 512-neuron network (the default since v0.60.0-NNUE) sums 32-bit chunks of `SCRELU_CHUNK` neurons, each exact as above, in
    /// 64 bits: one perspective of 512 saturated neurons already exceeds 32 bits.
    #[cfg(not(feature = "hidden256"))]
    fn screlu_dot(acc: &[i16; NNUE_HIDDEN_SIZE], weights: &[i16; NNUE_HIDDEN_SIZE]) -> i64 {
        let (acc_chunks, _) = acc.as_chunks::<SCRELU_CHUNK>();
        let (weight_chunks, _) = weights.as_chunks::<SCRELU_CHUNK>();
        let mut total: i64 = 0;
        for (acc, weights) in acc_chunks.iter().zip(weight_chunks) {
            let mut sum: i32 = 0;
            for i in 0..SCRELU_CHUNK {
                let clamped = acc[i].clamp(0, NNUE_QA as i16);
                let product = clamped.wrapping_mul(weights[i]);
                sum = sum.wrapping_add(product as i32 * clamped as i32);
            }
            total += sum as i64;
        }
        total
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
        out.copy_from_slice(&net.ft_biases.0);

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

    /// [`Self::refresh_perspective`] from the Finny table: the cached accumulator of this king bucket,
    /// moved to the board by the pieces that differ, or a full rebuild that fills the entry the first
    /// time a bucket is used.
    pub fn refresh_perspective_cached(
        board: &Board,
        net: &NNUENetwork,
        perspective: usize,
        out: &mut [i16; NNUE_HIDDEN_SIZE],
        finny: &mut FinnyTable,
    ) {
        let king = if perspective == 0 { WHITE_KING } else { BLACK_KING };
        let king_sq = board.bitboards[king].trailing_zeros() as usize;
        if !net.loaded || king_sq >= 64 {
            Self::refresh_perspective(board, net, perspective, out);
            return;
        }
        let bucket = get_king_bucket(king_sq, perspective);
        let entry = &mut finny.entries[perspective][bucket.index][bucket.mirrored as usize];
        if !entry.valid {
            Self::refresh_perspective(board, net, perspective, &mut entry.acc);
            for (pieces, &(bb, _, _)) in entry.pieces.iter_mut().zip(FEATURE_PIECES.iter()) {
                *pieces = board.bitboards[bb];
            }
            entry.valid = true;
            crate::nnue_prof::record_finny(true, 0);
        } else {
            #[cfg(feature = "nnue-prof")]
            let mut changed = 0u64;
            for (pieces, &(bb, piece_type, color)) in entry.pieces.iter_mut().zip(FEATURE_PIECES.iter()) {
                #[cfg(feature = "nnue-prof")]
                {
                    changed += ((*pieces) ^ board.bitboards[bb]).count_ones() as u64;
                }
                let (old, new) = (*pieces, board.bitboards[bb]);
                let mut removed = old & !new;
                while removed != 0 {
                    let sq = removed.trailing_zeros() as usize;
                    removed &= removed - 1;
                    let (b, idx) = get_feature_index(perspective, piece_type, color, sq, &bucket);
                    sub_weights(&mut entry.acc, &net.ft_weights[b][idx]);
                }
                let mut added = new & !old;
                while added != 0 {
                    let sq = added.trailing_zeros() as usize;
                    added &= added - 1;
                    let (b, idx) = get_feature_index(perspective, piece_type, color, sq, &bucket);
                    add_weights(&mut entry.acc, &net.ft_weights[b][idx]);
                }
                *pieces = new;
            }
            #[cfg(feature = "nnue-prof")]
            crate::nnue_prof::record_finny(false, changed);
        }
        *out = entry.acc;
    }

    /// [`Self::update_accumulator_into`] returning the child by value, for the tests.
    #[cfg(test)]
    pub fn update_accumulator(
        parent: &NNUEAccumulator,
        board: &Board,
        turn: &Turn,
        move_info: &MoveInformation,
        net: &NNUENetwork,
    ) -> NNUEAccumulator {
        let mut out = NNUEAccumulator::new();
        Self::update_accumulator_into(parent, &mut out, board, turn, move_info, net, &mut FinnyTable::new());
        out
    }

    /// The child's accumulator after `turn`, written into `out` (Matt-Magie step 79, review item
    /// N1): each perspective in one pass, `parent - removed + added`, in the order the separate
    /// copy, subtract and add passes used, so the values are bit-identical to them - only the
    /// zeroing, the separate passes and the return by value are gone. A king that crosses a bucket
    /// boundary rebuilds its perspective from the board, as before.
    pub fn update_accumulator_into(
        parent: &NNUEAccumulator,
        out: &mut NNUEAccumulator,
        board: &Board,
        turn: &Turn,
        move_info: &MoveInformation,
        net: &NNUENetwork,
        finny: &mut FinnyTable,
    ) {
        let start = crate::nnue_prof::cycles();
        let mut refreshed = false;
        Self::update_accumulator_into_inner(parent, out, board, turn, move_info, net, finny, &mut refreshed);
        crate::nnue_prof::record_update(start, refreshed);
    }

    fn update_accumulator_into_inner(
        parent: &NNUEAccumulator,
        out: &mut NNUEAccumulator,
        board: &Board,
        turn: &Turn,
        move_info: &MoveInformation,
        net: &NNUENetwork,
        finny: &mut FinnyTable,
        refreshed: &mut bool,
    ) {
        if !net.loaded {
            *out = NNUEAccumulator::new();
            return;
        }
        if !parent.computed {
            *out = Self::compute_accumulator_inner(board, net);
            return;
        }

        let white_king_sq = board.bitboards[WHITE_KING].trailing_zeros() as usize;
        let black_king_sq = board.bitboards[BLACK_KING].trailing_zeros() as usize;

        if white_king_sq >= 64 || black_king_sq >= 64 {
            *out = Self::compute_accumulator_inner(board, net);
            return;
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

        let removed = &removed[..removed_count];
        let added = &added[..added_count];

        // White perspective
        let white_bucket = get_king_bucket(white_king_sq, 0);
        let white_refresh = moved_piece == 15 && {
            let old_bucket = get_king_bucket(from, 0);
            old_bucket.index != white_bucket.index || old_bucket.mirrored != white_bucket.mirrored
        };
        if white_refresh {
            Self::refresh_perspective_cached(board, net, 0, &mut out.white, finny);
            *refreshed = true;
        } else {
            apply_delta(&mut out.white, &parent.white, 0, removed, added, &white_bucket, net);
        }

        // Black perspective
        let black_bucket = get_king_bucket(black_king_sq, 1);
        let black_refresh = moved_piece == 25 && {
            let old_bucket = get_king_bucket(from, 1);
            old_bucket.index != black_bucket.index || old_bucket.mirrored != black_bucket.mirrored
        };
        if black_refresh {
            Self::refresh_perspective_cached(board, net, 1, &mut out.black, finny);
            *refreshed = true;
        } else {
            apply_delta(&mut out.black, &parent.black, 1, removed, added, &black_bucket, net);
        }

        out.computed = true;
    }

    pub fn evaluate_with_accumulator(board: &Board, net: &NNUENetwork, acc: &NNUEAccumulator) -> i16 {
        let start = crate::nnue_prof::cycles();
        let eval = Self::evaluate_with_accumulator_inner(board, net, acc);
        crate::nnue_prof::record_eval(start);
        eval
    }

    fn evaluate_with_accumulator_inner(board: &Board, net: &NNUENetwork, acc: &NNUEAccumulator) -> i16 {
        if !net.loaded || !acc.computed {
            return 0;
        }

        let output_bucket = Self::get_output_bucket(board);
        let (us_acc, them_acc) = if board.white_to_move {
            (&acc.white, &acc.black)
        } else {
            (&acc.black, &acc.white)
        };

        let [us_weights, them_weights] = &net.output_weights[output_bucket];

        let final_eval = Self::output_layer(Self::screlu_dot(us_acc, us_weights),
            Self::screlu_dot(them_acc, them_weights), net.output_biases[output_bucket]);

        if board.white_to_move {
            final_eval
        } else {
            -final_eval
        }
    }

    /// The output neuron from the two dot products, in centipawns for the side to move. With 256
    /// neurons in 32 bits as in every release up to v0.55.0-NNUE; a saturated network (every neuron at `QA`,
    /// every weight at the bound) would overflow the sum of both perspectives, which no trained
    /// network comes near (it would be an evaluation beyond 200 000 cp).
    #[cfg(feature = "hidden256")]
    #[inline(always)]
    fn output_layer(us: i32, them: i32, bias: i16) -> i16 {
        let mut output = us + them;
        output /= NNUE_QA;
        output += bias as i32;
        let eval = (output * NNUE_SCALE) / (NNUE_QA * NNUE_QB);
        eval.clamp(i16::MIN as i32, i16::MAX as i32) as i16
    }

    /// The 512-neuron network in 64 bits throughout, free of overflow for any weights the loader accepts
    /// (test `a_saturated_network_evaluates_to_the_clamp_bound_without_overflow`). The bound is
    /// symmetric so that the negation for Black cannot overflow either.
    #[cfg(not(feature = "hidden256"))]
    #[inline(always)]
    fn output_layer(us: i64, them: i64, bias: i16) -> i16 {
        let mut output = us + them;
        output /= NNUE_QA as i64;
        output += bias as i64;
        let eval = (output * NNUE_SCALE as i64) / (NNUE_QA * NNUE_QB) as i64;
        eval.clamp(-(i16::MAX as i64), i16::MAX as i64) as i16
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

    /// The file of the embedded network, for the tests that load it from disk.
    const EMBEDDED_NET_FILE: &str =
        if cfg!(feature = "hidden256") { "eval_models/quantised256.bin" } else { "eval_models/quantised.bin" };

    #[test]
    fn test_nnue_model_loading_and_evaluation() {
        let net_result = NNUENetwork::load_from_file(EMBEDDED_NET_FILE);
        assert!(net_result.is_ok(), "Failed to load quantised.bin: {:?}", net_result.err());
        let net = net_result.unwrap();
        assert!(net.loaded);

        let fen_service = FenService;
        let board = fen_service.set_fen(crate::model::INIT_BOARD_FEN);
        let eval = NNUEService::evaluate(&board, &net);
        // Startpos evaluation should be reasonable (close to 0 cp)
        assert!(eval.abs() < 200, "Startpos NNUE eval should be reasonable, got: {}", eval);
    }

    /// Byte offset of the output weights in a network file: after the feature transformer weights
    /// and biases.
    const OUTPUT_WEIGHTS_OFFSET: usize = (NNUE_INPUT_BUCKETS * NNUE_INPUT_SIZE * NNUE_HIDDEN_SIZE + NNUE_HIDDEN_SIZE) * 2;

    fn embedded_net_with_output_weight(value: i16) -> Vec<u8> {
        let mut bytes = NNUENetwork::EMBEDDED_NET_BYTES.to_vec();
        let at = OUTPUT_WEIGHTS_OFFSET + 2 * (3 * 2 * NNUE_HIDDEN_SIZE + 17);
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
        bytes
    }

    /// The SCReLU dot product as `clamp(v)^2 * w` in 64 bits, the formula the fast path must match.
    fn screlu_dot_reference(acc: &[i16; NNUE_HIDDEN_SIZE], weights: &[i16; NNUE_HIDDEN_SIZE]) -> i64 {
        let mut sum: i64 = 0;
        for i in 0..NNUE_HIDDEN_SIZE {
            let clamped = (acc[i] as i64).clamp(0, NNUE_QA as i64);
            sum += clamped * clamped * weights[i] as i64;
        }
        sum
    }

    /// The fast path multiplies `clamp(v) * w` in 16 bits, which is exact only while
    /// `255 * |w| <= i16::MAX`: a net with an output weight of 128 loads, one of 129 does not.
    #[test]
    fn a_net_whose_output_weights_overflow_the_16_bit_product_is_rejected() {
        assert!(NNUENetwork::load_from_bytes(&embedded_net_with_output_weight(128)).is_ok());
        assert!(NNUENetwork::load_from_bytes(&embedded_net_with_output_weight(-128)).is_ok());
        for bad in [129, -129, i16::MAX, i16::MIN] {
            let err = NNUENetwork::load_from_bytes(&embedded_net_with_output_weight(bad)).err()
                .unwrap_or_else(|| panic!("output weight {bad} must be rejected"));
            assert!(err.contains("output weight"), "unexpected error: {err}");
        }
    }

    /// The 16-bit product SCReLU equals `clamp(v)^2 * w` in 64 bits for every accumulator value
    /// class - below 0, the clamp bounds, above 255, the i16 extremes - and the weight bounds.
    #[test]
    fn the_screlu_dot_equals_the_exact_reference() {
        let mut state = 0x5EED_5C2E_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let edges_acc = [i16::MIN, -1, 0, 1, 254, 255, 256, i16::MAX];
        let edges_w = [-128i16, -127, -1, 0, 1, 127, 128];
        for round in 0..2000 {
            let mut acc = [0i16; NNUE_HIDDEN_SIZE];
            let mut weights = [0i16; NNUE_HIDDEN_SIZE];
            for i in 0..NNUE_HIDDEN_SIZE {
                let r = next();
                acc[i] = if round % 4 == 0 { edges_acc[(r % 8) as usize] } else { (r as i16) % 600 - 150 };
                weights[i] = if round % 3 == 0 { edges_w[((r >> 20) % 7) as usize] } else { ((r >> 32) % 257) as i16 - 128 };
            }
            assert_eq!(NNUEService::screlu_dot(&acc, &weights) as i64, screlu_dot_reference(&acc, &weights), "round {round}");
        }
        let full = [NNUE_QA as i16; NNUE_HIDDEN_SIZE];
        for w in [128i16, -128] {
            assert_eq!(NNUEService::screlu_dot(&full, &[w; NNUE_HIDDEN_SIZE]) as i64, screlu_dot_reference(&full, &[w; NNUE_HIDDEN_SIZE]));
        }
    }

    /// Only the 512-neuron build is hardened (see `output_layer`).
    /// The worst case of the output layer: every neuron of both perspectives saturated at `QA` and
    /// every output weight at the bound +-128. The dot products then exceed 32 bits (per perspective
    /// from 512 neurons on, both summed already at 256), so the sum, the bias and the scaling run in
    /// 64 bits and the result is clamped to `+-i16::MAX`, which negates for Black without overflow.
    #[test]
    #[cfg(not(feature = "hidden256"))]
    fn a_saturated_network_evaluates_to_the_clamp_bound_without_overflow() {
        let fen_service = FenService;
        let white = fen_service.set_fen(crate::model::INIT_BOARD_FEN);
        let black = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
        let mut acc = NNUEAccumulator::new();
        acc.white = [NNUE_QA as i16; NNUE_HIDDEN_SIZE];
        acc.black = [NNUE_QA as i16; NNUE_HIDDEN_SIZE];
        acc.computed = true;
        for weight in [128i16, -128] {
            let mut net = NNUENetwork::new_empty();
            for bucket in net.output_weights.iter_mut() {
                *bucket = [[weight; NNUE_HIDDEN_SIZE]; 2];
            }
            net.loaded = true;
            let sign = weight.signum();
            assert_eq!(NNUEService::evaluate_with_accumulator(&white, &net, &acc), sign * i16::MAX, "w {weight}");
            assert_eq!(NNUEService::evaluate_with_accumulator(&black, &net, &acc), -sign * i16::MAX, "b {weight}");
        }
    }

    /// Every row the vector loops read starts on a cache line: the feature transformer
    /// rows, the biases, both output halves and both accumulator perspectives, also in the stack.
    #[test]
    fn the_network_rows_and_the_accumulators_are_cache_line_aligned() {
        let net = NNUENetwork::load_embedded().unwrap();
        let aligned = |p: *const i16| p as usize % 64 == 0;
        for bucket in [0, 7, NNUE_INPUT_BUCKETS - 1] {
            for input in [0, 1, 383, NNUE_INPUT_SIZE - 1] {
                assert!(aligned(net.ft_weights[bucket][input].as_ptr()), "ft row {bucket}/{input}");
            }
        }
        assert!(aligned(net.ft_biases.as_ptr()));
        for bucket in 0..NNUE_OUTPUT_BUCKETS {
            for half in 0..2 {
                assert!(aligned(net.output_weights[bucket][half].as_ptr()), "output {bucket}/{half}");
            }
        }
        let stack = Box::new(AccStack::new());
        for ply in [0, 1, 2, crate::search_service::MAX_PLY - 1] {
            assert!(aligned(stack[ply].white.as_ptr()) && aligned(stack[ply].black.as_ptr()), "ply {ply}");
        }
    }

    #[test]
    fn the_fingerprint_is_fnv1a() {
        assert_eq!(net_fingerprint(b""), 0x811c_9dc5);
        assert_eq!(net_fingerprint(b"a"), 0xe40c_292c);
        assert_eq!(net_fingerprint(b"foobar"), 0xbf9c_f968);
    }

    #[test]
    fn the_default_and_the_empty_path_select_the_embedded_net() {
        assert!(uses_embedded_net(DEFAULT_MODEL_PATH));
        assert!(uses_embedded_net(""));
        assert!(!uses_embedded_net("nets/other.bin"));
    }

    #[test]
    fn the_embedded_net_is_described_by_its_name_and_fingerprint() {
        let name = EMBEDDED_NET_NAME.trim();
        assert!(!name.is_empty() && !name.contains('\n'), "quantised.name must hold one name");
        let expected = format!("NNUE net {} (embedded, fnv {:08x})", name,
            net_fingerprint(NNUENetwork::EMBEDDED_NET_BYTES));
        assert_eq!(describe_net(DEFAULT_MODEL_PATH), expected);
        assert_eq!(describe_net(""), expected);
    }

    #[test]
    fn a_net_file_is_described_by_its_path_and_fingerprint() {
        let path = std::env::temp_dir().join(format!("rip_describe_net_{}.bin", std::process::id()));
        std::fs::write(&path, b"foobar").unwrap();
        let path = path.to_str().unwrap();
        let described = describe_net(path);
        std::fs::remove_file(path).unwrap();
        assert_eq!(described, format!("NNUE net {} (file, fnv bf9cf968)", path));
        assert!(describe_net("eval_models/non_existent_model_file.bin").starts_with(
            "NNUE net eval_models/non_existent_model_file.bin (file not readable: "));
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

    /// Matt-Magie step 79, N1: after every legal move of every position of random games - quiet
    /// moves, captures, castling, en passant, promotions, king moves across a bucket boundary - the
    /// child accumulator written in one pass equals the one computed from the board, bit for bit.
    #[test]
    fn the_one_pass_update_equals_a_full_computation_after_every_move() {
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
        let (mut moves_checked, mut refreshes, mut castles, mut promotions, mut en_passant) = (0, 0, 0, 0, 0);
        // One Finny table across all positions (Matt-Magie step 82): its entries go stale between
        // unrelated positions, so every bucket rebuild after the first also checks a large difference.
        let mut finny = FinnyTable::new();
        for mut board in crate::eval_service::tests::random_positions(16, 120, 0x5EED_0079) {
            let parent = NNUEService::compute_accumulator(&board, &net);
            for turn in crate::move_gen_service::tests::generate_valid_moves_list(&mut board) {
                let mi = board.do_move(&turn);
                let mut child = NNUEAccumulator::new();
                child.white[0] = 12345; // a stale slot: every value has to be written
                NNUEService::update_accumulator_into(&parent, &mut child, &board, &turn, &mi, &net, &mut finny);
                assert_eq!(child, NNUEService::compute_accumulator(&board, &net),
                    "{} diverges", turn.to_algebraic());
                moves_checked += 1;
                let king = mi.moved_piece == 15 || mi.moved_piece == 25;
                if king && (turn.to as i8 - turn.from as i8).abs() == 2 { castles += 1; }
                if king { refreshes += 1; }
                if turn.is_promotion() { promotions += 1; }
                if (mi.moved_piece == 10 || mi.moved_piece == 20) && turn.to as i8 == mi.en_passante { en_passant += 1; }
                board.undo_move(&turn, mi);
            }
        }
        assert!(moves_checked > 1000, "only {moves_checked} moves checked");
        assert!(castles > 0 && promotions > 0 && refreshes > 0,
            "the sample must hold castling ({castles}), promotions ({promotions}) and king moves ({refreshes}); en passant {en_passant}");
    }

    /// Matt-Magie step 82: a rebuild from the Finny table equals the full rebuild bit for bit - the
    /// first use of a bucket, the same bucket again a few moves later (a small difference) and after
    /// an unrelated position (a large one), for both perspectives, castled kings included.
    #[test]
    fn a_finny_rebuild_equals_a_full_rebuild() {
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
        let mut finny = FinnyTable::new();
        let (mut checks, mut first_uses) = (0, 0);
        for board in crate::eval_service::tests::random_positions(12, 120, 0x5EED_0082) {
            for perspective in [0, 1] {
                let king = if perspective == 0 { WHITE_KING } else { BLACK_KING };
                let bucket = get_king_bucket(board.bitboards[king].trailing_zeros() as usize, perspective);
                if !finny.entries[perspective][bucket.index][bucket.mirrored as usize].valid {
                    first_uses += 1;
                }
                let mut cached = [0i16; NNUE_HIDDEN_SIZE];
                let mut full = [0i16; NNUE_HIDDEN_SIZE];
                NNUEService::refresh_perspective_cached(&board, &net, perspective, &mut cached, &mut finny);
                NNUEService::refresh_perspective(&board, &net, perspective, &mut full);
                assert_eq!(cached, full, "perspective {perspective}");
                checks += 1;
            }
        }
        assert!(checks > 500 && first_uses > 4 && checks > first_uses * 20,
            "{checks} rebuilds, {first_uses} first uses of a bucket");
    }

    /// The largest differences a Finny entry can see, in both directions, on the same king squares:
    /// a full board after bare kings (30 features added) and bare kings after a full board (30
    /// removed), then a position that adds and removes at once.
    #[test]
    fn a_finny_rebuild_equals_a_full_rebuild_for_the_largest_differences() {
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
        let fen = FenService;
        let sequence = [
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1",
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1",
            "1q2k1r1/2p2p1p/1n1p2b1/pP1Pp3/Q1B1P1n1/2N2N2/P4PPP/2R1K2R w - - 0 1",
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
        ];
        let mut finny = FinnyTable::new();
        for fen_str in sequence {
            let board = fen.set_fen(fen_str);
            for perspective in [0, 1] {
                let mut cached = [0i16; NNUE_HIDDEN_SIZE];
                let mut full = [0i16; NNUE_HIDDEN_SIZE];
                NNUEService::refresh_perspective_cached(&board, &net, perspective, &mut cached, &mut finny);
                NNUEService::refresh_perspective(&board, &net, perspective, &mut full);
                assert_eq!(cached, full, "{fen_str}, perspective {perspective}");
            }
        }
    }

    /// The piece table returns what the `% 10` decoding returned for every piece code on the board.
    #[test]
    fn the_piece_table_decodes_every_piece_code() {
        let decode = |piece: u8| {
            let piece_type = match piece % 10 { 0 => 0, 1 => 3, 2 => 1, 3 => 2, 4 => 4, 5 => 5, _ => 0 };
            (piece_type, if piece >= 20 { 1 } else { 0 })
        };
        for piece in (10..=15).chain(20..=25) {
            assert_eq!(piece_to_type_and_color(piece), decode(piece), "piece {piece}");
        }
    }

    /// The output bucket from the occupancy equals the one from the twelve piece bitboards, after
    /// every move of random games, captures and promotions included.
    #[test]
    fn the_output_bucket_counts_every_piece_on_the_board() {
        let reference = |board: &Board| {
            let count: i32 = board.bitboards.iter().map(|bb| bb.count_ones() as i32).sum();
            ((count - 2) / 4).clamp(0, NNUE_OUTPUT_BUCKETS as i32 - 1) as usize
        };
        let mut checked = 0;
        for mut board in crate::eval_service::tests::random_positions(16, 120, 0x5EED_0B0C) {
            assert_eq!(NNUEService::get_output_bucket(&board), reference(&board));
            for turn in crate::move_gen_service::tests::generate_valid_moves_list(&mut board) {
                let mi = board.do_move(&turn);
                assert_eq!(NNUEService::get_output_bucket(&board), reference(&board), "{}", turn.to_algebraic());
                board.undo_move(&turn, mi);
                checked += 1;
            }
        }
        assert!(checked > 1000, "only {checked} moves checked");
    }

    #[test]
    fn test_incremental_accumulator_quiet_and_captures() {
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
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
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
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
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
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
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
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
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
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
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
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
        let net = NNUENetwork::load_from_file(EMBEDDED_NET_FILE).unwrap();
        let service = crate::service::Service::new();
        let config = crate::config::Config::for_tests();
        let zobrist_table = crate::zobrist::ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pondering = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[[0i32; 64]; 64]; 2];
        let context = crate::model::SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pondering: &pondering,
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


#[cfg(test)]
#[path = "nnue_compat_tests.rs"]
mod compat_tests;
