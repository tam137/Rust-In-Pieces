use crate::model::Board;

const NUM_PIECES: usize = 12;
const BOARD_SIZE: usize = 64;

/// Compile-time key material for the Zobrist hash.
///
/// The keys used to be drawn from `StdRng` behind four `once_cell::sync::Lazy` statics. That put a
/// `OnceCell` guard on *every* key access, and because the never-taken initialisation call still
/// clobbers the caller-saved registers, `Board::do_move` carried thirteen such call sites together
/// with the register spills around them. Generating the keys with a `const fn` moves the whole
/// table into `.rodata`: no atomic guard, no branch, no spills, and one copy instead of two.
struct ZobristKeys {
    pieces: [[u64; NUM_PIECES]; BOARD_SIZE],
    white_to_move: u64,
    castling: [u64; 16],
    en_passant: [u64; 8],
}

/// SplitMix64. Chosen over the previous `StdRng` because it is expressible as a `const fn` while
/// still passing BigCrush; the exact key values are irrelevant as long as they are well spread.
/// Returns the advanced state alongside the drawn value.
const fn next_key(state: u64) -> (u64, u64) {
    let state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (state, z ^ (z >> 31))
}

const fn generate_keys() -> ZobristKeys {
    let mut state = 137;

    let mut pieces = [[0u64; NUM_PIECES]; BOARD_SIZE];
    let mut square = 0;
    while square < BOARD_SIZE {
        let mut piece = 0;
        while piece < NUM_PIECES {
            let (next_state, key) = next_key(state);
            state = next_state;
            pieces[square][piece] = key;
            piece += 1;
        }
        square += 1;
    }

    let (next_state, white_to_move) = next_key(state);
    state = next_state;

    let mut castling = [0u64; 16];
    let mut i = 0;
    while i < 16 {
        let (next_state, key) = next_key(state);
        state = next_state;
        castling[i] = key;
        i += 1;
    }

    let mut en_passant = [0u64; 8];
    let mut i = 0;
    while i < 8 {
        let (next_state, key) = next_key(state);
        state = next_state;
        en_passant[i] = key;
        i += 1;
    }

    ZobristKeys { pieces, white_to_move, castling, en_passant }
}

const ZOBRIST_KEYS: ZobristKeys = generate_keys();

pub static ZOBRIST_TABLE: [[u64; NUM_PIECES]; BOARD_SIZE] = ZOBRIST_KEYS.pieces;
pub const WHITE_TO_MOVE: u64 = ZOBRIST_KEYS.white_to_move;
pub static CASTLING_RIGHTS: [u64; 16] = ZOBRIST_KEYS.castling;
pub static EN_PASSANT_FILE: [u64; 8] = ZOBRIST_KEYS.en_passant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TranspositionType {
    Exact,      // PV Node (Exact score)
    LowerBound, // Cut Node (Beta cutoff - score is at least this)
    UpperBound, // All Node (Alpha cutoff - score is at most this)
}

#[derive(Debug, Clone, Copy)]
pub struct TranspositionEntry {
    pub key: u64, // Full 64-bit Zobrist key to prevent index collisions
    pub eval: i16,
    pub best_move: u16,
    pub depth: i8,
    pub entry_type: TranspositionType,
    #[allow(dead_code)]
    pub padding: [u8; 2],
}

impl Default for TranspositionEntry {
    fn default() -> Self {
        Self {
            key: 0,
            eval: 0,
            best_move: 0,
            depth: -1, // -1 signals empty slot
            entry_type: TranspositionType::Exact,
            padding: [0; 2],
        }
    }
}

impl TranspositionEntry {
    #[inline(always)]
    pub fn pack(self) -> u64 {
        let mut val = 0u64;
        val |= (self.eval as u16 as u64) & 0xFFFF;
        val |= ((self.best_move as u64) & 0xFFFF) << 16;
        val |= ((self.depth as u8 as u64) & 0xFF) << 32;
        let type_val = match self.entry_type {
            TranspositionType::Exact => 0,
            TranspositionType::LowerBound => 1,
            TranspositionType::UpperBound => 2,
        } as u64;
        val |= (type_val & 0xFF) << 40;
        val
    }

    #[inline(always)]
    pub fn unpack(key: u64, data: u64) -> Self {
        let eval = (data & 0xFFFF) as u16 as i16;
        let best_move = ((data >> 16) & 0xFFFF) as u16;
        let depth = ((data >> 32) & 0xFF) as u8 as i8;
        let entry_type = match (data >> 40) & 0xFF {
            1 => TranspositionType::LowerBound,
            2 => TranspositionType::UpperBound,
            _ => TranspositionType::Exact,
        };
        Self {
            key,
            eval,
            best_move,
            depth,
            entry_type,
            padding: [0; 2],
        }
    }

    #[inline(always)]
    pub fn compress_move(turn: Option<crate::model::Turn>) -> u16 {
        if let Some(t) = turn {
            let from = t.from as u16;
            let to = t.to as u16;
            let promo_type = match t.promotion % 10 {
                4 => 1, // Queen
                1 => 2, // Rook
                3 => 3, // Bishop
                2 => 4, // Knight
                _ => 0,
            } as u16;
            
            let mut val = 0u16;
            val |= to & 0x3F;
            val |= (from & 0x3F) << 6;
            val |= (promo_type & 0x07) << 12;
            val |= 1 << 15;
            val
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn decompress_move(&self, board: &crate::model::Board) -> Option<crate::model::Turn> {
        let val = self.best_move;
        if val == 0 || (val & (1 << 15)) == 0 {
            return None;
        }
        let to = (val & 0x3F) as u8;
        let from = ((val >> 6) & 0x3F) as u8;
        let promo_type = ((val >> 12) & 0x07) as u8;
        
        let promotion = if promo_type != 0 {
            let offset = if board.white_to_move { 10 } else { 20 };
            match promo_type {
                1 => offset + 4, // Queen
                2 => offset + 1, // Rook
                3 => offset + 3, // Bishop
                4 => offset + 2, // Knight
                _ => 0,
            }
        } else {
            0
        };

        let mut capture = board.mailbox[to as usize];
        if capture == 0 {
            let moved_piece = board.mailbox[from as usize];
            if (moved_piece == 10 || moved_piece == 20) && (to as i8 == board.field_for_en_passante) {
                capture = if board.white_to_move { 20 } else { 10 };
            }
        }

        Some(crate::model::Turn {
            from,
            to,
            capture,
            promotion,
            order: 0,
            gives_check: false,
            eval: 0,
            has_hashed_eval: false,
            rank: 0,
        })
    }
}

#[derive(Debug)]
pub struct AtomicEntry {
    pub key: std::sync::atomic::AtomicU64,
    pub data: std::sync::atomic::AtomicU64,
}

#[derive(Debug)]
pub struct ZobristTable {
    pub table: Vec<AtomicEntry>,
}

impl ZobristTable {

    pub fn with_capacity(capacity: usize) -> Self {
        let mut table = Vec::with_capacity(capacity.max(1));
        let default_entry = TranspositionEntry::default();
        let default_key = default_entry.key;
        let default_data = default_entry.pack();
        for _ in 0..capacity.max(1) {
            table.push(AtomicEntry {
                key: std::sync::atomic::AtomicU64::new(default_key),
                data: std::sync::atomic::AtomicU64::new(default_data),
            });
        }
        Self { table }
    }

    /// Maps a Zobrist key onto a slot without a hardware division.
    ///
    /// `hash % len` compiled to a real 64-bit `div`, which sits on the dependency chain *ahead* of
    /// the slot load and therefore delays the start of what is almost always a DRAM miss. The
    /// multiply-shift (Lemire) reduction below is a single widening multiply, keeps arbitrary
    /// (non-power-of-two) table sizes so the UCI `Hash` option is unaffected, and consumes the high
    /// key bits - which are as well distributed as the low ones for a Zobrist hash.
    #[inline(always)]
    pub fn slot_index(&self, hash: u64) -> usize {
        ((hash as u128 * self.table.len() as u128) >> 64) as usize
    }

    /// Pulls the slot for `hash` into L1 ahead of the probe.
    ///
    /// A default-sized table is far larger than the last-level cache, so nearly every probe misses
    /// to DRAM. Issuing the request as soon as the child key is known - `do_move` computes it
    /// before it touches the board - overlaps that latency with move generation and evaluation.
    #[inline(always)]
    pub fn prefetch(&self, hash: u64) {
        #[cfg(target_arch = "x86_64")]
        {
            let slot = unsafe { self.table.as_ptr().add(self.slot_index(hash)) };
            unsafe { std::arch::x86_64::_mm_prefetch(slot as *const i8, std::arch::x86_64::_MM_HINT_T0) };
        }
        #[cfg(target_arch = "aarch64")]
        {
            let slot = unsafe { self.table.as_ptr().add(self.slot_index(hash)) };
            unsafe { std::arch::aarch64::_prefetch(slot as *const i8, std::arch::aarch64::_PREFETCH_READ, std::arch::aarch64::_PREFETCH_LOCALITY3) };
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            let _ = hash;
        }
    }

    #[inline]
    pub fn get_entry(&self, hash: u64) -> Option<TranspositionEntry> {
        let slot = &self.table[self.slot_index(hash)];

        let key1 = slot.key.load(std::sync::atomic::Ordering::Acquire);
        if key1 != hash {
            return None;
        }
        let data = slot.data.load(std::sync::atomic::Ordering::Relaxed);
        let key2 = slot.key.load(std::sync::atomic::Ordering::Acquire);

        if key1 == key2 {
            let entry = TranspositionEntry::unpack(key1, data);
            if entry.depth != -1 {
                return Some(entry);
            }
        }
        None
    }

    pub fn insert_entry(&self, hash: u64, entry: TranspositionEntry) {
        let slot = &self.table[self.slot_index(hash)];

        let key1 = slot.key.load(std::sync::atomic::Ordering::Relaxed);
        let data1 = slot.data.load(std::sync::atomic::Ordering::Relaxed);
        let existing = TranspositionEntry::unpack(key1, data1);

        // Replacement policy:
        // 1. existing.depth == -1: The slot is empty, always store the new entry.
        // 2. existing.key == hash: Same position. Overwrite if new search is at least as deep.
        // 3. existing.key != hash: Hash collision (slot contention).
        //    - If new entry is QS (entry.depth <= 0) and existing is Main Search (existing.depth >= 1):
        //      REJECT write to protect deep PV/interior search tree nodes from TT cache pollution.
        //    - Otherwise: replace (standard collision aging / main search overwrites).
        let should_replace = if existing.depth == -1 {
            true
        } else if existing.key == hash {
            entry.depth >= existing.depth
        } else {
            !(entry.depth <= 0 && existing.depth >= 1)
        };

        if should_replace {
            // Invalidate the key before writing data to prevent a "torn read" by another thread.
            slot.key.store(!hash, std::sync::atomic::Ordering::Release);
            slot.data.store(entry.pack(), std::sync::atomic::Ordering::Release);
            // Restore the correct key after data is written.
            slot.key.store(hash, std::sync::atomic::Ordering::Release);
        }
    }

    pub fn _size(&self) -> usize {
        self.table.iter()
            .map(|slot| {
                let key = slot.key.load(std::sync::atomic::Ordering::Relaxed);
                let data = slot.data.load(std::sync::atomic::Ordering::Relaxed);
                TranspositionEntry::unpack(key, data)
            })
            .filter(|e| e.depth != -1)
            .count()
    }

    /// Returns every slot to the empty state.
    ///
    /// Both words have to be reset. `insert_entry` recognises an empty slot by `depth == -1`, which
    /// lives in `data`; resetting only `key` left the previous depth behind, so after `ucinewgame`
    /// every slot that had held a `depth >= 1` entry kept rejecting quiescence writes under the
    /// collision rule until a main-search store happened to land on it. A stale `data` also let a
    /// probe for the (reserved) key `0` read back the evicted entry.
    pub fn clear(&self) {
        let default_entry = TranspositionEntry::default();
        let default_key = default_entry.key;
        let default_data = default_entry.pack();
        for slot in &self.table {
            slot.key.store(default_key, std::sync::atomic::Ordering::Relaxed);
            slot.data.store(default_data, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// True when the en passant target square `ep` can actually be taken by `capturing_white`.
///
/// The key used to be mixed in for every double push, which split otherwise identical positions
/// into two table entries and cost transposition hits for nothing. Folding the key in only when a
/// capture exists makes the two positions share a slot again.
///
/// The capturing pawn stands beside the pawn that made the double push, so the victim square is one
/// rank behind `ep` from the capturer's point of view. Only the enemy pawn bitboard is read, which
/// is why this is equally valid on the position *before* the double push - that move is never a
/// capture and so cannot change it. Pins are ignored, exactly as in the pseudo-legal convention
/// used elsewhere; the predicate only has to be consistent between full and incremental hashing.
#[inline(always)]
pub fn ep_capture_available(board: &Board, ep: i8, capturing_white: bool) -> bool {
    let ep = ep as usize;
    let (victim_square, attackers) = if capturing_white {
        (ep - 8, board.bitboards[crate::model::WHITE_PAWN])
    } else {
        (ep + 8, board.bitboards[crate::model::BLACK_PAWN])
    };

    let file = ep % 8;
    let mut neighbours = 0u64;
    if file > 0 {
        neighbours |= 1u64 << (victim_square - 1);
    }
    if file < 7 {
        neighbours |= 1u64 << (victim_square + 1);
    }
    attackers & neighbours != 0
}

pub fn gen_hash(board: &Board) -> u64 {
    let mut hash = 0u64;
    if board.white_to_move {
        hash ^= WHITE_TO_MOVE;
    }
    let castle_index = (if board.white_possible_to_castle_short { 1 } else { 0 })
        | (if board.white_possible_to_castle_long { 2 } else { 0 })
        | (if board.black_possible_to_castle_short { 4 } else { 0 })
        | (if board.black_possible_to_castle_long { 8 } else { 0 });
    hash ^= CASTLING_RIGHTS[castle_index];
    if board.field_for_en_passante >= 0
        && ep_capture_available(board, board.field_for_en_passante, board.white_to_move)
    {
        let file = (board.field_for_en_passante % 8) as usize;
        hash ^= EN_PASSANT_FILE[file];
    }
    for piece_idx in 0..12 {
        let mut bb = board.bitboards[piece_idx];
        while bb != 0 {
            let square = bb.trailing_zeros() as usize;
            hash ^= ZOBRIST_TABLE[square][piece_idx];
            bb &= bb - 1; // Clear least significant set bit
        }
    }
    hash
}

/// Hash of the position produced by a null move: the side to move flips and the en passant square
/// is given up. Must be called on the position *before* the null move is applied.
///
/// Kept here rather than inline in the search so that the en passant condition can never drift
/// apart from the one in [`gen_hash`] and [`calc_incremental_hash`] - a mismatch would not corrupt
/// the board (the search restores the hash wholesale) but would make the whole subtree below the
/// null move probe and store under a key that belongs to no position.
#[inline(always)]
pub fn null_move_hash(board: &Board) -> u64 {
    let mut hash = board.cached_hash ^ WHITE_TO_MOVE;
    if board.field_for_en_passante >= 0
        && ep_capture_available(board, board.field_for_en_passante, board.white_to_move)
    {
        hash ^= EN_PASSANT_FILE[(board.field_for_en_passante % 8) as usize];
    }
    hash
}

pub fn gen_pawn_hash(board: &Board) -> u64 {
    let mut hash = 0u64;
    let mut wp = board.bitboards[crate::model::WHITE_PAWN];
    while wp != 0 {
        let square = wp.trailing_zeros() as usize;
        hash ^= ZOBRIST_TABLE[square][crate::model::WHITE_PAWN];
        wp &= wp - 1;
    }
    let mut bp = board.bitboards[crate::model::BLACK_PAWN];
    while bp != 0 {
        let square = bp.trailing_zeros() as usize;
        hash ^= ZOBRIST_TABLE[square][crate::model::BLACK_PAWN];
        bp &= bp - 1;
    }
    hash
}

#[inline(always)]
pub fn get_zobrist_val(square: usize, piece_idx: usize) -> u64 {
    ZOBRIST_TABLE[square][piece_idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zobrist_entry_size_test() {
        assert_eq!(std::mem::size_of::<AtomicEntry>(), 16, "AtomicEntry must be exactly 16 bytes");
        assert_eq!(std::mem::size_of::<TranspositionEntry>(), 16, "TranspositionEntry must be exactly 16 bytes");

        let entries_count: usize = 100_000_000;
        let total_bytes = entries_count * std::mem::size_of::<AtomicEntry>();
        assert_eq!(total_bytes, 1_600_000_000, "100M entries must equal 1.6 GB (1,600,000,000 bytes)");
    }

    /// Two keys that land in the same slot of a two-entry table, and two that land in the other.
    /// The values are asserted rather than assumed so that the collision tests below keep testing
    /// collisions if the index function is ever changed again.
    const SLOT_A_1: u64 = 0;
    const SLOT_A_2: u64 = 2;
    const SLOT_A_3: u64 = 4;
    const SLOT_B_1: u64 = 1 << 63;
    const SLOT_B_2: u64 = (1 << 63) | 1;

    fn assert_slot_layout(table: &ZobristTable) {
        assert_eq!(table.slot_index(SLOT_A_1), table.slot_index(SLOT_A_2));
        assert_eq!(table.slot_index(SLOT_A_1), table.slot_index(SLOT_A_3));
        assert_eq!(table.slot_index(SLOT_B_1), table.slot_index(SLOT_B_2));
        assert_ne!(table.slot_index(SLOT_A_1), table.slot_index(SLOT_B_1));
    }

    fn entry(key: u64, eval: i16, depth: i8, entry_type: TranspositionType) -> TranspositionEntry {
        TranspositionEntry { key, eval, depth, entry_type, best_move: 0, padding: [0; 2] }
    }

    #[test]
    fn zobrist_replacement_policy_test() {
        let table = ZobristTable::with_capacity(2);
        assert_slot_layout(&table);

        table.insert_entry(SLOT_A_1, entry(SLOT_A_1, 100, 3, TranspositionType::Exact));
        let ret = table.get_entry(SLOT_A_1).unwrap();
        assert_eq!(ret.eval, 100);
        assert_eq!(ret.depth, 3);

        table.insert_entry(SLOT_A_2, entry(SLOT_A_2, 200, 5, TranspositionType::Exact));
        assert!(table.get_entry(SLOT_A_1).is_none());
        let ret2 = table.get_entry(SLOT_A_2).unwrap();
        assert_eq!(ret2.eval, 200);
        assert_eq!(ret2.depth, 5);

        table.insert_entry(SLOT_A_3, entry(SLOT_A_3, 400, 2, TranspositionType::Exact));
        let ret3 = table.get_entry(SLOT_A_3).unwrap();
        assert_eq!(ret3.eval, 400);
        assert_eq!(ret3.depth, 2);
        assert!(table.get_entry(SLOT_A_2).is_none());

        // Same position, shallower search: the deeper entry stands.
        table.insert_entry(SLOT_A_3, entry(SLOT_A_3, 150, 1, TranspositionType::Exact));
        let kept = table.get_entry(SLOT_A_3).unwrap();
        assert_eq!(kept.eval, 400);
        assert_eq!(kept.depth, 2);
    }

    #[test]
    fn zobrist_qs_tt_collision_protection_test() {
        let table = ZobristTable::with_capacity(2);
        assert_slot_layout(&table);

        // 1. Store a deep main-search entry (depth = 6).
        table.insert_entry(SLOT_A_1, entry(SLOT_A_1, 250, 6, TranspositionType::Exact));
        assert_eq!(table.get_entry(SLOT_A_1).unwrap().depth, 6);

        // 2. A quiescence entry (depth = 0) colliding on the same slot must be rejected.
        table.insert_entry(SLOT_A_2, entry(SLOT_A_2, 50, 0, TranspositionType::LowerBound));

        let preserved = table.get_entry(SLOT_A_1);
        assert!(preserved.is_some(), "Deep main search entry must not be evicted by QS collision");
        assert_eq!(preserved.unwrap().depth, 6);
        assert_eq!(preserved.unwrap().eval, 250);
        assert!(table.get_entry(SLOT_A_2).is_none(), "Colliding QS entry must not be present");

        // 3. A main-search entry (depth = 8) may take the slot.
        table.insert_entry(SLOT_A_2, entry(SLOT_A_2, 300, 8, TranspositionType::Exact));
        assert!(table.get_entry(SLOT_A_1).is_none(), "Deeper main search entry replaces collision");
        assert_eq!(table.get_entry(SLOT_A_2).unwrap().depth, 8);

        // 4. A quiescence entry fits into an empty slot.
        table.insert_entry(SLOT_B_1, entry(SLOT_B_1, 75, 0, TranspositionType::UpperBound));
        assert_eq!(table.get_entry(SLOT_B_1).unwrap().depth, 0);

        // 5. Another quiescence entry colliding there may replace it - the rule only protects
        //    main-search depth.
        table.insert_entry(SLOT_B_2, entry(SLOT_B_2, 80, 0, TranspositionType::Exact));
        assert!(table.get_entry(SLOT_B_1).is_none());
        assert_eq!(table.get_entry(SLOT_B_2).unwrap().depth, 0);
    }

    #[test]
    fn zobrist_move_compression_test() {
        let board = crate::service::Service::new().fen.set_init_board();

        let original_move = crate::model::Turn {
            from: 12,
            to: 28,
            capture: 0,
            promotion: 0,
            order: 0,
            gives_check: false,
            eval: 0,
            has_hashed_eval: false,
            rank: 0,
        };
        let compressed = TranspositionEntry::compress_move(Some(original_move));
        assert_ne!(compressed, 0);

        let entry = TranspositionEntry {
            key: 12345,
            eval: 0,
            best_move: compressed,
            depth: 3,
            entry_type: TranspositionType::Exact,
            padding: [0; 2],
        };
        let decompressed = entry.decompress_move(&board).unwrap();
        assert_eq!(decompressed.from, original_move.from);
        assert_eq!(decompressed.to, original_move.to);
        assert_eq!(decompressed.promotion, original_move.promotion);
        assert_eq!(decompressed.capture, original_move.capture);
        assert_eq!(decompressed, original_move);

        let board_promo = crate::service::Service::new().fen.set_fen("7k/P7/8/8/8/8/8/7K w - - 0 1");
        let original_promo = crate::model::Turn {
            from: 48,
            to: 56,
            capture: 0,
            promotion: 14,
            order: 0,
            gives_check: false,
            eval: 0,
            has_hashed_eval: false,
            rank: 0,
        };
        let compressed_promo = TranspositionEntry::compress_move(Some(original_promo));
        let entry_promo = TranspositionEntry {
            key: 12345,
            eval: 0,
            best_move: compressed_promo,
            depth: 3,
            entry_type: TranspositionType::Exact,
            padding: [0; 2],
        };
        let decompressed_promo = entry_promo.decompress_move(&board_promo).unwrap();
        assert_eq!(decompressed_promo.from, original_promo.from);
        assert_eq!(decompressed_promo.to, original_promo.to);
        assert_eq!(decompressed_promo.promotion, original_promo.promotion);
        assert_eq!(decompressed_promo, original_promo);

        let board_ep = crate::service::Service::new().fen.set_fen("rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
        let original_ep = crate::model::Turn {
            from: 36,
            to: 43,
            capture: 20,
            promotion: 0,
            order: 0,
            gives_check: false,
            eval: 0,
            has_hashed_eval: false,
            rank: 0,
        };
        let compressed_ep = TranspositionEntry::compress_move(Some(original_ep));
        let entry_ep = TranspositionEntry {
            key: 12345,
            eval: 0,
            best_move: compressed_ep,
            depth: 3,
            entry_type: TranspositionType::Exact,
            padding: [0; 2],
        };
        let decompressed_ep = entry_ep.decompress_move(&board_ep).unwrap();
        assert_eq!(decompressed_ep.from, original_ep.from);
        assert_eq!(decompressed_ep.to, original_ep.to);
        assert_eq!(decompressed_ep.capture, original_ep.capture);
        assert_eq!(decompressed_ep, original_ep);

        let compressed_none = TranspositionEntry::compress_move(None);
        assert_eq!(compressed_none, 0);
        let entry_none = TranspositionEntry {
            key: 12345,
            eval: 0,
            best_move: compressed_none,
            depth: 3,
            entry_type: TranspositionType::Exact,
            padding: [0; 2],
        };
        assert!(entry_none.decompress_move(&board).is_none());
    }

    #[test]
    fn zobrist_lock_free_concurrency_test() {
        use std::sync::Arc;
        use std::thread;

        let table = Arc::new(ZobristTable::with_capacity(100));
        let mut handles = vec![];

        for _ in 0..8 {
            let table_clone = Arc::clone(&table);
            handles.push(thread::spawn(move || {
                for i in 0..1000 {
                    // Spread over the table: the reduction consumes the high bits, so small
                    // consecutive keys would all share slot 0.
                    let key = ((i % 10) as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                    let depth = (i % 10) as i8;
                    let entry = TranspositionEntry {
                        key,
                        eval: i as i16,
                        best_move: 0,
                        depth,
                        entry_type: TranspositionType::Exact,
                        padding: [0; 2],
                    };
                    table_clone.insert_entry(key, entry);
                    if let Some(ret) = table_clone.get_entry(key) {
                        assert_eq!(ret.key, key);
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }


    #[test]
    fn zobrist_clear_resets_key_and_data_test() {
        let table = ZobristTable::with_capacity(2);
        assert_slot_layout(&table);

        table.insert_entry(SLOT_A_1, entry(SLOT_A_1, 250, 6, TranspositionType::Exact));
        table.insert_entry(SLOT_B_1, entry(SLOT_B_1, 120, 4, TranspositionType::Exact));
        assert_eq!(table.get_entry(SLOT_A_1).unwrap().depth, 6);
        assert_eq!(table._size(), 2);

        table.clear();

        // `SLOT_A_1` is the reserved empty key, so a stale `data` word would hand the evicted
        // entry straight back to the next probe.
        assert!(table.get_entry(SLOT_A_1).is_none(), "a cleared slot must not answer a probe");
        assert!(table.get_entry(SLOT_B_1).is_none(), "a cleared slot must not answer a probe");
        assert_eq!(table._size(), 0, "no slot may count as occupied after clear()");

        // A cleared slot counts as empty, so a quiescence entry has to fit even though the slot
        // previously held a deep main-search entry.
        table.insert_entry(SLOT_A_2, entry(SLOT_A_2, 50, 0, TranspositionType::LowerBound));
        assert!(
            table.get_entry(SLOT_A_2).is_some(),
            "a QS write into a cleared slot must not be blocked by a depth left behind by clear()"
        );
    }

    #[test]
    fn zobrist_slot_index_is_in_range_test() {
        for capacity in [1usize, 2, 3, 17, 64, 1000, 4096] {
            let table = ZobristTable::with_capacity(capacity);
            assert_eq!(table.table.len(), capacity);
            for hash in [0u64, 1, 2, 3, u64::MAX, u64::MAX - 1, u64::MAX / 2, 1 << 63, 0x9E37_79B9_7F4A_7C15] {
                let index = table.slot_index(hash);
                assert!(index < capacity, "slot_index({hash}) = {index} is out of range for {capacity}");
            }
        }
    }

    #[test]
    fn zobrist_slot_index_spreads_over_the_table_test() {
        // The reduction consumes the high key bits, so a run of well-mixed keys has to reach every
        // slot instead of piling up in one region.
        let table = ZobristTable::with_capacity(256);
        let mut seen = [false; 256];
        let mut state = 1u64;
        for _ in 0..20_000 {
            let (next_state, key) = next_key(state);
            state = next_state;
            seen[table.slot_index(key)] = true;
        }
        assert!(seen.iter().all(|&hit| hit), "every slot must be reachable by the reduction");
    }

    #[test]
    fn zobrist_const_keys_are_distinct_and_non_zero_test() {
        let mut keys = Vec::with_capacity(BOARD_SIZE * NUM_PIECES + 1 + 16 + 8);
        for square in 0..BOARD_SIZE {
            for piece in 0..NUM_PIECES {
                keys.push(ZOBRIST_TABLE[square][piece]);
            }
        }
        keys.push(WHITE_TO_MOVE);
        keys.extend_from_slice(&CASTLING_RIGHTS);
        keys.extend_from_slice(&EN_PASSANT_FILE);

        assert!(keys.iter().all(|&key| key != 0), "a zero key would be invisible to the hash");

        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total, "all Zobrist keys must be pairwise distinct");
    }

    #[test]
    fn zobrist_prefetch_is_safe_test() {
        // The prefetch computes a raw slot pointer, so the smallest table and a non-power-of-two
        // size are the cases worth pinning down.
        for capacity in [1usize, 2, 3, 1000] {
            let table = ZobristTable::with_capacity(capacity);
            for hash in [0u64, 1, u64::MAX, u64::MAX / 3, 1 << 63] {
                table.prefetch(hash);
            }
        }
    }

    #[test]
    fn zobrist_en_passant_key_only_when_capturable_test() {
        let fen_service = crate::fen_service::FenService;

        // Black has just played d7d5 and White's e5 pawn can take on d6.
        let capturable = fen_service.set_fen("rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
        assert!(ep_capture_available(&capturable, capturable.field_for_en_passante, capturable.white_to_move));
        let mut capturable_without_ep = capturable.clone();
        capturable_without_ep.field_for_en_passante = -1;
        assert_ne!(
            gen_hash(&capturable), gen_hash(&capturable_without_ep),
            "a takeable en passant square must change the hash"
        );

        // After 1.e4 no black pawn stands beside e4, so the square is unusable and must not split
        // this position off from the one reached without the double push.
        let idle = fen_service.set_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        assert!(!ep_capture_available(&idle, idle.field_for_en_passante, idle.white_to_move));
        let mut idle_without_ep = idle.clone();
        idle_without_ep.field_for_en_passante = -1;
        assert_eq!(
            gen_hash(&idle), gen_hash(&idle_without_ep),
            "an en passant square nobody can use must not change the hash"
        );
    }

    #[test]
    fn zobrist_incremental_en_passant_matches_full_hash_test() {
        use crate::notation_util::NotationUtil;
        let fen_service = crate::fen_service::FenService;

        // White double push with a black pawn waiting on d4: the key belongs in the hash.
        let mut white_with_taker = fen_service.set_fen("rnbqkbnr/ppp1pppp/8/8/3p4/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let e2e4 = NotationUtil::get_turn_from_notation("e2e4");
        let before = white_with_taker.cached_hash;
        let move_information = white_with_taker.do_move(&e2e4);
        assert_eq!(white_with_taker.field_for_en_passante, 20);
        assert_eq!(
            white_with_taker.cached_hash, gen_hash(&white_with_taker),
            "incremental hash must match the full hash while the en passant key is present"
        );
        let mut ep_stripped = white_with_taker.clone();
        ep_stripped.field_for_en_passante = -1;
        assert_ne!(gen_hash(&white_with_taker), gen_hash(&ep_stripped));
        white_with_taker.undo_move(&e2e4, move_information);
        assert_eq!(white_with_taker.cached_hash, before);

        // Same push without a taker: the key must stay out, incrementally and fully.
        let mut white_without_taker = fen_service.set_init_board();
        let before = white_without_taker.cached_hash;
        let move_information = white_without_taker.do_move(&e2e4);
        assert_eq!(white_without_taker.field_for_en_passante, 20);
        assert_eq!(
            white_without_taker.cached_hash, gen_hash(&white_without_taker),
            "incremental hash must match the full hash while the en passant key is absent"
        );
        white_without_taker.undo_move(&e2e4, move_information);
        assert_eq!(white_without_taker.cached_hash, before);

        // Mirrored: Black double push with a white pawn waiting on e5.
        let mut black_with_taker = fen_service.set_fen("rnbqkbnr/pppppppp/8/4P3/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
        let d7d5 = NotationUtil::get_turn_from_notation("d7d5");
        let before = black_with_taker.cached_hash;
        let move_information = black_with_taker.do_move(&d7d5);
        assert_eq!(black_with_taker.field_for_en_passante, 43);
        assert_eq!(black_with_taker.cached_hash, gen_hash(&black_with_taker));
        let mut ep_stripped = black_with_taker.clone();
        ep_stripped.field_for_en_passante = -1;
        assert_ne!(
            gen_hash(&black_with_taker), gen_hash(&ep_stripped),
            "the mirrored case must mix the key in as well"
        );
        black_with_taker.undo_move(&d7d5, move_information);
        assert_eq!(black_with_taker.cached_hash, before);
    }

    #[test]
    fn zobrist_null_move_hash_matches_full_hash_test() {
        let fen_service = crate::fen_service::FenService;

        for position in [
            // Usable en passant square: the key is in the hash, so the null move has to remove it.
            "rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
            // Unusable one: the key was never mixed in, so the null move must leave it alone.
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            // No en passant square at all.
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        ] {
            let mut board = fen_service.set_fen(position);
            assert_eq!(board.cached_hash, gen_hash(&board));

            // Exactly the sequence `SearchService::minimax` performs for a null move.
            board.cached_hash = null_move_hash(&board);
            board.white_to_move = !board.white_to_move;
            board.field_for_en_passante = -1;

            assert_eq!(
                board.cached_hash, gen_hash(&board),
                "the null move hash must agree with the full hash for '{position}'"
            );
        }
    }

    #[test]
    fn zobrist_castling_rights_hash_test() {
        let fen_service = crate::fen_service::FenService;
        let board1 = fen_service.set_init_board();
        let mut board2 = fen_service.set_init_board();

        let hash1 = gen_hash(&board1);

        // Remove White Kingside castling right
        board2.white_possible_to_castle_short = false;
        let hash2 = gen_hash(&board2);

        assert_ne!(hash1, hash2, "Positions differing in castling rights must have distinct Zobrist hashes");
    }

    #[test]
    fn zobrist_en_passant_hash_test() {
        let fen_service = crate::fen_service::FenService;
        let board1 = fen_service.set_fen("rnbqkbnr/ppp1pp1p/6p1/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
        let mut board2 = board1.clone();

        let hash1 = gen_hash(&board1);

        // Clear en passant square
        board2.field_for_en_passante = -1;
        let hash2 = gen_hash(&board2);

        assert_ne!(hash1, hash2, "Positions differing in en passant target field must have distinct Zobrist hashes");
    }
}


#[inline(always)]
pub fn calc_incremental_hash(board: &Board, turn: &crate::model::Turn) -> u64 {
    let mut hash = board.cached_hash;
    let from = turn.from as usize;
    let to = turn.to as usize;
    let moved_piece = board.mailbox[from];
    let moved_bb_idx = Board::piece_to_bb_idx(moved_piece);

    // 1. Swap turn
    hash ^= WHITE_TO_MOVE;

    // 2. Remove the old en passant file, on the same condition under which it was mixed in. The
    // board still carries the position the key was created for, so the predicate sees the very
    // state `gen_hash` would.
    if board.field_for_en_passante >= 0
        && ep_capture_available(board, board.field_for_en_passante, board.white_to_move)
    {
        let file = (board.field_for_en_passante % 8) as usize;
        hash ^= EN_PASSANT_FILE[file];
    }
    // Set the new en passant file, again only when the opponent can actually take. The double push
    // itself is never a capture, so the enemy pawn bitboard read here is already the one the next
    // position will have.
    let double_push = (moved_piece == 10 && from / 8 == 1 && to / 8 == 3)
        || (moved_piece == 20 && from / 8 == 6 && to / 8 == 4);
    if double_push {
        let new_ep = if moved_piece == 10 { from + 8 } else { from - 8 };
        if ep_capture_available(board, new_ep as i8, !board.white_to_move) {
            hash ^= EN_PASSANT_FILE[new_ep % 8];
        }
    }

    // 3. Remove old castling rights
    let old_castle_index = (if board.white_possible_to_castle_short { 1 } else { 0 })
        | (if board.white_possible_to_castle_long { 2 } else { 0 })
        | (if board.black_possible_to_castle_short { 4 } else { 0 })
        | (if board.black_possible_to_castle_long { 8 } else { 0 });
    hash ^= CASTLING_RIGHTS[old_castle_index];

    // Compute new castling rights based on from/to
    let mut w_short = board.white_possible_to_castle_short;
    let mut w_long = board.white_possible_to_castle_long;
    let mut b_short = board.black_possible_to_castle_short;
    let mut b_long = board.black_possible_to_castle_long;
    
    match from {
        56 => b_long = false,
        63 => b_short = false,
        60 => { b_long = false; b_short = false; }
        0 => w_long = false,
        7 => w_short = false,
        4 => { w_long = false; w_short = false; }
        _ => {}
    }
    match to {
        56 => b_long = false,
        63 => b_short = false,
        0 => w_long = false,
        7 => w_short = false,
        _ => {}
    }
    let new_castle_index = (if w_short { 1 } else { 0 })
        | (if w_long { 2 } else { 0 })
        | (if b_short { 4 } else { 0 })
        | (if b_long { 8 } else { 0 });
    hash ^= CASTLING_RIGHTS[new_castle_index];

    // 4. Move piece
    hash ^= ZOBRIST_TABLE[from][moved_bb_idx]; // remove from 'from'
    if turn.is_promotion() {
        let promo_bb_idx = Board::piece_to_bb_idx(turn.promotion);
        hash ^= ZOBRIST_TABLE[to][promo_bb_idx]; // add promotion piece to 'to'
    } else {
        hash ^= ZOBRIST_TABLE[to][moved_bb_idx]; // add moved piece to 'to'
    }

    // 5. Handle capture
    let mut actual_capture = turn.capture;
    if actual_capture == 0 {
        let piece_at_to = board.mailbox[to];
        if piece_at_to != 0 && (10..=15).contains(&piece_at_to) != board.white_to_move {
            actual_capture = piece_at_to;
        } else if (moved_piece == 10 || moved_piece == 20) && (to as i8 == board.field_for_en_passante) {
            actual_capture = if board.white_to_move { 20 } else { 10 };
        }
    }
    if actual_capture != 0 {
        let is_en_passant = (moved_piece == 10 || moved_piece == 20) && (to as i8 == board.field_for_en_passante);
        let capture_sq = if is_en_passant {
            if board.white_to_move { to - 8 } else { to + 8 }
        } else {
            to
        };
        let capture_bb_idx = Board::piece_to_bb_idx(actual_capture);
        hash ^= ZOBRIST_TABLE[capture_sq][capture_bb_idx];
    }

    // 6. Handle rook movement in castling
    if moved_piece == 15 || moved_piece == 25 {
        let is_castling = (to as i8 - from as i8).abs() == 2;
        if is_castling {
            match to {
                6 => {
                    hash ^= ZOBRIST_TABLE[7][crate::model::WHITE_ROOK];
                    hash ^= ZOBRIST_TABLE[5][crate::model::WHITE_ROOK];
                }
                2 => {
                    hash ^= ZOBRIST_TABLE[0][crate::model::WHITE_ROOK];
                    hash ^= ZOBRIST_TABLE[3][crate::model::WHITE_ROOK];
                }
                62 => {
                    hash ^= ZOBRIST_TABLE[63][crate::model::BLACK_ROOK];
                    hash ^= ZOBRIST_TABLE[61][crate::model::BLACK_ROOK];
                }
                58 => {
                    hash ^= ZOBRIST_TABLE[56][crate::model::BLACK_ROOK];
                    hash ^= ZOBRIST_TABLE[59][crate::model::BLACK_ROOK];
                }
                _ => {}
            }
        }
    }

    hash
}
