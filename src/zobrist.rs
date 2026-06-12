use rand::{RngCore, rngs::StdRng, SeedableRng};
use once_cell::sync::Lazy;

use crate::model::Board;

const NUM_PIECES: usize = 12;
const BOARD_SIZE: usize = 64;

static ZOBRIST_DATA: Lazy<([[u64; NUM_PIECES]; BOARD_SIZE], u64)> = Lazy::new(|| {
    let mut rng = StdRng::seed_from_u64(137);
    let mut table = [[0u64; NUM_PIECES]; BOARD_SIZE];

    for row in table.iter_mut() {
        for val in row.iter_mut() {
            *val = rng.next_u64();
        }
    }

    let white_to_move = rng.next_u64();

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

    pub fn get_entry(&self, hash: &u64) -> Option<TranspositionEntry> {
        let index = (*hash as usize) % self.table.len();
        let slot = &self.table[index];

        let key1 = slot.key.load(std::sync::atomic::Ordering::Acquire);
        if key1 != *hash {
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
        let index = (hash as usize) % self.table.len();
        let slot = &self.table[index];

        let key1 = slot.key.load(std::sync::atomic::Ordering::Relaxed);
        let data1 = slot.data.load(std::sync::atomic::Ordering::Relaxed);
        let existing = TranspositionEntry::unpack(key1, data1);

        if existing.depth == -1 || existing.key == hash || entry.depth >= existing.depth {
            slot.data.store(entry.pack(), std::sync::atomic::Ordering::Release);
            slot.key.store(hash, std::sync::atomic::Ordering::Release);
        }
    }

    pub fn get_eval_for_hash(&self, hash: &u64) -> Option<i16> {
        self.get_entry(hash).map(|e| e.eval)
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

    pub fn clear(&self) {
        let default_entry = TranspositionEntry::default();
        let default_key = default_entry.key;
        for slot in &self.table {
            slot.key.store(default_key, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

pub fn gen_hash(board: &Board) -> u64 {
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

pub fn get_zobrist_val(square: usize, piece_idx: usize) -> u64 {
    ZOBRIST_TABLE[square][piece_idx]
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
                    let key = (i % 10) as u64;
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
                    if let Some(ret) = table_clone.get_entry(&key) {
                        assert_eq!(ret.key, key);
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}

