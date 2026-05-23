use rand::{Rng, rngs::StdRng, SeedableRng};
use once_cell::sync::Lazy;

use crate::model::Board;

const NUM_PIECES: usize = 12;
const BOARD_SIZE: usize = 64;

static ZOBRIST_DATA: Lazy<([[u64; NUM_PIECES]; BOARD_SIZE], u64)> = Lazy::new(|| {
    let mut rng = StdRng::seed_from_u64(137);
    let mut table = [[0u64; NUM_PIECES]; BOARD_SIZE];

    for i in 0..BOARD_SIZE {
        for piece_index in 0..NUM_PIECES {
            table[i][piece_index] = rng.gen();
        }
    }

    let white_to_move = rng.gen();

    (table, white_to_move)
});

static ZOBRIST_TABLE: Lazy<[[u64; NUM_PIECES]; BOARD_SIZE]> = Lazy::new(|| ZOBRIST_DATA.0);
static WHITE_TO_MOVE: Lazy<u64> = Lazy::new(|| ZOBRIST_DATA.1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TranspositionType {
    Exact,      // PV Node (Exact score)
    LowerBound, // Cut Node (Beta cutoff - score is at least this)
    UpperBound, // All Node (Alpha cutoff - score is at most this)
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TranspositionEntry {
    pub key: u64, // Full 64-bit Zobrist key to prevent index collisions
    pub eval: i16,
    pub best_move: u16,
    pub depth: i8,
    pub entry_type: TranspositionType,
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
            gives_check: false,
            eval: 0,
            hash: 0,
            has_hashed_eval: false,
            rank: 0,
        })
    }
}

#[derive(Debug)]
pub struct ZobristTable {
    pub table: std::sync::RwLock<Vec<TranspositionEntry>>,
}

impl ZobristTable {
    pub fn new() -> Self {
        Self::with_capacity(10_000_000)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            table: std::sync::RwLock::new(vec![TranspositionEntry::default(); capacity.max(1)]),
        }
    }

    pub fn get_entry(&self, hash: &u64) -> Option<TranspositionEntry> {
        let table = self.table.read().unwrap();
        let index = (*hash as usize) % table.len();
        let entry = table[index];
        if entry.depth != -1 && entry.key == *hash {
            Some(entry)
        } else {
            None
        }
    }

    pub fn insert_entry(&self, hash: u64, entry: TranspositionEntry) {
        let mut table = self.table.write().unwrap();
        let index = (hash as usize) % table.len();
        let existing = &mut table[index];
        // Depth-Preferred replacement policy (always overwrite if key is identical)
        if existing.depth == -1 || existing.key == hash || entry.depth >= existing.depth {
            *existing = entry;
        }
    }

    pub fn get_eval_for_hash(&self, hash: &u64) -> Option<i16> {
        let table = self.table.read().unwrap();
        let index = (*hash as usize) % table.len();
        let entry = table[index];
        if entry.depth != -1 && entry.key == *hash {
            Some(entry.eval)
        } else {
            None
        }
    }

    pub fn _size(&self) -> usize {
        let table = self.table.read().unwrap();
        table.iter().filter(|e| e.depth != -1).count()
    }
}

pub fn gen(board: &Board) -> u64 {
    let mut hash = 0u64;
    if board.white_to_move {
        hash ^= *WHITE_TO_MOVE;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zobrist_replacement_policy_test() {
        let table = ZobristTable::with_capacity(2);

        let entry1 = TranspositionEntry {
            key: 0,
            eval: 100,
            depth: 3,
            entry_type: TranspositionType::Exact,
            best_move: 0,
            padding: [0; 2],
        };
        table.insert_entry(0, entry1);
        let ret = table.get_entry(&0).unwrap();
        assert_eq!(ret.eval, 100);
        assert_eq!(ret.depth, 3);

        let entry2 = TranspositionEntry {
            key: 2,
            eval: 200,
            depth: 5,
            entry_type: TranspositionType::Exact,
            best_move: 0,
            padding: [0; 2],
        };
        table.insert_entry(2, entry2);
        assert!(table.get_entry(&0).is_none());
        let ret2 = table.get_entry(&2).unwrap();
        assert_eq!(ret2.eval, 200);
        assert_eq!(ret2.depth, 5);

        let entry3 = TranspositionEntry {
            key: 4,
            eval: 400,
            depth: 2,
            entry_type: TranspositionType::Exact,
            best_move: 0,
            padding: [0; 2],
        };
        table.insert_entry(4, entry3);
        assert!(table.get_entry(&4).is_none());
        let ret_kept = table.get_entry(&2).unwrap();
        assert_eq!(ret_kept.eval, 200);

        let entry4 = TranspositionEntry {
            key: 2,
            eval: 150,
            depth: 1,
            entry_type: TranspositionType::Exact,
            best_move: 0,
            padding: [0; 2],
        };
        table.insert_entry(2, entry4);
        let ret_overwritten = table.get_entry(&2).unwrap();
        assert_eq!(ret_overwritten.eval, 150);
        assert_eq!(ret_overwritten.depth, 1);
    }

    #[test]
    fn zobrist_move_compression_test() {
        let board = crate::service::Service::new().fen.set_init_board();

        let original_move = crate::model::Turn {
            from: 12,
            to: 28,
            capture: 0,
            promotion: 0,
            gives_check: false,
            eval: 0,
            hash: 0,
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
            gives_check: false,
            eval: 0,
            hash: 0,
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
            gives_check: false,
            eval: 0,
            hash: 0,
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
}
