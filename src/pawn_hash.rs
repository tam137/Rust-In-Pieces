use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct PawnHashEntry {
    pub key: u64,
    pub mg_eval: i16,
    pub eg_eval: i16,
    pub white_passed: u64,
    pub black_passed: u64,
}

#[derive(Debug)]
pub struct AtomicPawnEntry {
    pub key: AtomicU64,
    pub evals: AtomicU32, // mg_eval (16 bits) | (eg_eval (16 bits) << 16)
    pub white_passed: AtomicU64,
    pub black_passed: AtomicU64,
}

#[derive(Debug)]
pub struct PawnHashTable {
    pub table: Vec<AtomicPawnEntry>,
}

impl PawnHashTable {
    pub fn with_capacity(capacity: usize) -> Self {
        let mut table = Vec::with_capacity(capacity.max(1));
        for _ in 0..capacity.max(1) {
            table.push(AtomicPawnEntry {
                key: AtomicU64::new(0),
                evals: AtomicU32::new(0),
                white_passed: AtomicU64::new(0),
                black_passed: AtomicU64::new(0),
            });
        }
        Self { table }
    }

    pub fn probe(&self, hash: u64) -> Option<PawnHashEntry> {
        if self.table.is_empty() {
            return None;
        }
        let index = (hash as usize) % self.table.len();
        let slot = &self.table[index];

        let key1 = slot.key.load(Ordering::Acquire);
        if key1 != hash || key1 == 0 {
            return None;
        }

        let evals = slot.evals.load(Ordering::Relaxed);
        let white_passed = slot.white_passed.load(Ordering::Relaxed);
        let black_passed = slot.black_passed.load(Ordering::Relaxed);

        let key2 = slot.key.load(Ordering::Acquire);
        if key1 == key2 {
            let mg_eval = (evals & 0xFFFF) as u16 as i16;
            let eg_eval = ((evals >> 16) & 0xFFFF) as u16 as i16;
            return Some(PawnHashEntry {
                key: key1,
                mg_eval,
                eg_eval,
                white_passed,
                black_passed,
            });
        }
        None
    }

    pub fn store(&self, hash: u64, mg_eval: i16, eg_eval: i16, white_passed: u64, black_passed: u64) {
        if self.table.is_empty() {
            return;
        }
        let index = (hash as usize) % self.table.len();
        let slot = &self.table[index];

        let evals_val = (mg_eval as u16 as u32) | ((eg_eval as u16 as u32) << 16);

        slot.evals.store(evals_val, Ordering::Relaxed);
        slot.white_passed.store(white_passed, Ordering::Relaxed);
        slot.black_passed.store(black_passed, Ordering::Relaxed);
        slot.key.store(hash, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::Service;
    use crate::zobrist;
    use crate::notation_util::NotationUtil;

    #[test]
    fn test_pawn_hash_table_probe_store() {
        let table = PawnHashTable::with_capacity(16);
        let hash = 0x123456789ABCDEF0;
        assert!(table.probe(hash).is_none());

        table.store(hash, 120, -50, 0x1, 0x2);
        let entry = table.probe(hash).expect("Should find entry");
        assert_eq!(entry.mg_eval, 120);
        assert_eq!(entry.eg_eval, -50);
        assert_eq!(entry.white_passed, 0x1);
        assert_eq!(entry.black_passed, 0x2);
    }

    #[test]
    fn test_pawn_hash_table_concurrency() {
        use std::thread;
        use std::sync::Arc;

        let table = Arc::new(PawnHashTable::with_capacity(256));
        let mut handles = vec![];

        for i in 0..8 {
            let table_clone = Arc::clone(&table);
            handles.push(thread::spawn(move || {
                for offset in 0..1000 {
                    let index = i * 32 + (offset % 32);
                    let key = (index + (offset / 32) * 256) as u64;
                    table_clone.store(key, offset as i16, -(offset as i16), key, key);
                    if let Some(entry) = table_clone.probe(key) {
                        assert_eq!(entry.mg_eval, offset as i16);
                        assert_eq!(entry.eg_eval, -(offset as i16));
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_board_pawn_key_incremental_updates() {
        let service = Service::new();
        // FEN with en passant possibility and pawns
        let mut board = service.fen.set_fen("rnbqkbnr/1ppp1ppp/p7/3Pp3/8/8/PPP1PPPP/RNBQKBNR w KQkq e6 0 3");
        
        let initial_key = board.pawn_key;
        assert_eq!(initial_key, zobrist::gen_pawn_hash(&board));

        // Let's do an en passant capture
        let mut ep_move = NotationUtil::get_turn_from_notation("d5e6");
        ep_move.capture = 20; // Black pawn
        
        let mi = board.do_move(&ep_move);
        assert_eq!(board.pawn_key, zobrist::gen_pawn_hash(&board));

        board.undo_move(&ep_move, mi);
        assert_eq!(board.pawn_key, initial_key);

        // Let's do a pawn promotion
        let mut board_promo = service.fen.set_fen("8/4P3/8/8/8/8/8/k6K w - - 0 1");
        let mut promo_move = NotationUtil::get_turn_from_notation("e7e8q");
        promo_move.promotion = 14;
        
        let initial_promo_key = board_promo.pawn_key;
        assert_eq!(initial_promo_key, zobrist::gen_pawn_hash(&board_promo));

        let mi_promo = board_promo.do_move(&promo_move);
        assert_eq!(board_promo.pawn_key, zobrist::gen_pawn_hash(&board_promo));

        board_promo.undo_move(&promo_move, mi_promo);
        assert_eq!(board_promo.pawn_key, initial_promo_key);
    }
}
