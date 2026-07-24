use std::fs::File;
use std::io::Read;
use std::path::Path;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::model::Board;

/// 64-bit random matrix used to generate PolyGlot Zobrist keys.
static POLYGLOT_MATRIX: [u64; 64] = [
    0x9D391A2A702951C3, 0x140EE7A3141F323A, 0xDF8A55B2E0573AE6, 0x3EBE02B376662C5E,
    0x2E6FB2B0FCA6F510, 0x9330CD13C47EA651, 0x6794691456B7D505, 0xE9B3FA871788220A,
    0x8797CF49F308FA86, 0xEE73AA9A0AF7E06B, 0x7E1854DCECFD9757, 0xB66FCECEBE06637B,
    0xB8425FCE2D766E41, 0x3CA2D927E298E7B2, 0x32A2613DDE6006BF, 0x489673DF8D63CEEB,
    0x4E7475F2B487A9A3, 0xD4B02E8A6D3B6204, 0xFEF24FDFE21BECA5, 0xDBB8A1EE5963E8EF,
    0x19277A6A0604A671, 0xDFCEAC72EF491873, 0x22DC370E0DF289EB, 0x72EF358F6F4CBE7C,
    0xDE4AE0D780447EFF, 0xD376722E426FE5A6, 0x62957E01CA4DB7F7, 0xEEC2FA8C2D80EB1B,
    0x39E9908F87216A65, 0x633F081180E87F63, 0x4CE70A887A8D7E36, 0x7EAE1E778E29E2F8,
    0x38615A6305FA2E37, 0x82DF0030EEB11FDF, 0x1A284D7C1EC9F2A6, 0xDF36C5611440A56E,
    0x180AAEC6E074C4DC, 0x1A47B2E74B7FDFFA, 0xD56CDA6AE4190DF4, 0xB902DFEE1C8F7966,
    0xB81A02E7618BC912, 0x8A123049F440FDF6, 0x463BE2AF83777598, 0xE7B10C0B7423D07C,
    0xA63255C8F7354924, 0x5821FEE812F4C5DF, 0x4A6B65B0B07D3FDE, 0x3F616DCA2F2F29AA,
    0x91834241B08C4A70, 0x65F93EC0EA0089AA, 0x3D7203E70196FE22, 0xD2919323AE6A2D40,
    0xCA9E3CFAED5B2625, 0xDE6DCE218D2F86B2, 0xEF9A34A6A80EC210, 0xC567EE68D9C08A91,
    0x7940250D4D32B8C3, 0x630E3605634509AD, 0xA26C2A2000000000, 0x6148000000000000,
    0x6184000000000000, 0x0E00000000000000, 0x1C00000000000000, 0x3800000000000000,
];

fn generate_polyglot_table(seed_init: u32) -> Vec<u64> {
    let mut table = Vec::with_capacity(781);
    let mut s = seed_init;
    for _ in 0..781 {
        let mut result = 0u64;
        for i in 0..64 {
            s = s.wrapping_mul(314159269).wrapping_add(453806245);
            if (s >> 31) & 1 == 1 {
                result ^= POLYGLOT_MATRIX[i];
            }
        }
        table.push(result);
    }
    table
}

static POLYGLOT_RANDOM64: Lazy<Vec<u64>> = Lazy::new(|| generate_polyglot_table(0));

/// Represents a single PolyGlot book entry (16 bytes in big-endian).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolyglotEntry {
    pub key: u64,
    pub mov: u16,
    pub weight: u16,
    pub learn: u32,
}

impl PolyglotEntry {
    /// Decodes the 16-bit PolyGlot move integer into standard algebraic notation (e.g. "e2e4", "e7e8q").
    pub fn to_algebraic(&self) -> String {
        let to_file = (self.mov & 0x07) as u8;
        let to_rank = ((self.mov >> 3) & 0x07) as u8;
        let from_file = ((self.mov >> 6) & 0x07) as u8;
        let from_rank = ((self.mov >> 9) & 0x07) as u8;
        let promotion = ((self.mov >> 12) & 0x07) as u8;

        let from_char = (b'a' + from_file) as char;
        let from_num = (b'1' + from_rank) as char;
        let to_char = (b'a' + to_file) as char;
        let to_num = (b'1' + to_rank) as char;

        let promo_char = match promotion {
            1 => "n",
            2 => "b",
            3 => "r",
            4 => "q",
            _ => "",
        };

        format!("{}{}{}{}{}", from_char, from_num, to_char, to_num, promo_char)
    }
}

/// Maps Suprah internal piece representation to PolyGlot piece index (0..11).
fn map_piece_to_polyglot(piece: u8) -> Option<usize> {
    match piece {
        10 => Some(0),  // White Pawn
        20 => Some(1),  // Black Pawn
        12 => Some(2),  // White Knight
        22 => Some(3),  // Black Knight
        13 => Some(4),  // White Bishop
        23 => Some(5),  // Black Bishop
        11 => Some(6),  // White Rook
        21 => Some(7),  // Black Rook
        14 => Some(8),  // White Queen
        24 => Some(9),  // Black Queen
        15 => Some(10), // White King
        25 => Some(11), // Black King
        _ => None,
    }
}

/// Checks if an enemy pawn exists on adjacent files that can capture en-passant on `ep_sq`.
fn enemy_pawns_can_capture_ep(board: &Board, ep_sq: usize) -> bool {
    let file = ep_sq % 8;
    if board.white_to_move {
        // White is to move. EP target square is on 6th rank (e.g. e6=44).
        // White pawns (10) are on 5th rank.
        if file > 0 && ep_sq >= 9 && board.mailbox[ep_sq - 9] == 10 {
            return true;
        }
        if file < 7 && ep_sq >= 7 && board.mailbox[ep_sq - 7] == 10 {
            return true;
        }
    } else {
        // Black is to move. EP target square is on 3rd rank (e.g. e3=20).
        // Black pawns (20) are on 4th rank.
        if file > 0 && ep_sq + 7 < 64 && board.mailbox[ep_sq + 7] == 20 {
            return true;
        }
        if file < 7 && ep_sq + 9 < 64 && board.mailbox[ep_sq + 9] == 20 {
            return true;
        }
    }
    false
}

/// Computes the 64-bit PolyGlot Zobrist hash key for a given Board position.
pub fn polyglot_key(board: &Board) -> u64 {
    let mut key = 0u64;

    // 1. Pieces on board
    for rank in 0..8 {
        for file in 0..8 {
            let sq = rank * 8 + file;
            let piece = board.mailbox[sq];
            if piece != 0 {
                if let Some(pg_piece) = map_piece_to_polyglot(piece) {
                    let pg_sq = (7 - rank) * 8 + file;
                    let offset = 64 * pg_piece + pg_sq;
                    let val = POLYGLOT_RANDOM64[offset];
                    key ^= val;
                }
            }
        }
    }

    // 2. Castling rights
    if board.white_possible_to_castle_short {
        key ^= POLYGLOT_RANDOM64[768];
    }
    if board.white_possible_to_castle_long {
        key ^= POLYGLOT_RANDOM64[769];
    }
    if board.black_possible_to_castle_short {
        key ^= POLYGLOT_RANDOM64[770];
    }
    if board.black_possible_to_castle_long {
        key ^= POLYGLOT_RANDOM64[771];
    }

    // 3. En Passant square
    if board.field_for_en_passante >= 0 {
        let ep_sq = board.field_for_en_passante as usize;
        let file = ep_sq % 8;
        if enemy_pawns_can_capture_ep(board, ep_sq) {
            key ^= POLYGLOT_RANDOM64[772 + file];
        }
    }

    // 4. Side to move (Black turn)
    if !board.white_to_move {
        key ^= POLYGLOT_RANDOM64[780];
    }

    key
}

/// PolyGlot Book Reader.
#[derive(Debug, Clone)]
pub struct PolyglotBook {
    entries: Vec<PolyglotEntry>,
}

impl PolyglotBook {
    /// Loads a PolyGlot `.bin` file into memory.
    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let num_entries = buffer.len() / 16;
        let mut entries = Vec::with_capacity(num_entries);

        for chunk in buffer.chunks_exact(16) {
            let key = u64::from_be_bytes(chunk[0..8].try_into().unwrap());
            let mov = u16::from_be_bytes(chunk[8..10].try_into().unwrap());
            let weight = u16::from_be_bytes(chunk[10..12].try_into().unwrap());
            let learn = u32::from_be_bytes(chunk[12..16].try_into().unwrap());

            entries.push(PolyglotEntry {
                key,
                mov,
                weight,
                learn,
            });
        }

        Ok(Self { entries })
    }

    /// Creates an in-memory PolyGlotBook from a slice of entries.
    pub fn from_entries(entries: Vec<PolyglotEntry>) -> Self {
        Self { entries }
    }

    /// Finds all matching PolyGlot entries for a given Board position.
    pub fn find_entries(&self, board: &Board) -> &[PolyglotEntry] {
        let key = polyglot_key(board);
        let pos = match self.entries.binary_search_by_key(&key, |e| e.key) {
            Ok(idx) => idx,
            Err(_) => return &[],
        };

        // Binary search lands on any matching entry. Find start index.
        let mut start = pos;
        while start > 0 && self.entries[start - 1].key == key {
            start -= 1;
        }

        // Find end index.
        let mut end = pos + 1;
        while end < self.entries.len() && self.entries[end].key == key {
            end += 1;
        }

        &self.entries[start..end]
    }

    /// Selects a move weighted randomly from matching book entries.
    pub fn get_random_book_move(&self, board: &Board) -> String {
        let matching = self.find_entries(board);
        if matching.is_empty() {
            return String::new();
        }

        let mut rng = thread_rng();
        if let Ok(choice) = matching.choose_weighted(&mut rng, |e| e.weight as u32) {
            choice.to_algebraic()
        } else if let Some(first) = matching.first() {
            first.to_algebraic()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::Service;

    #[test]
    fn test_polyglot_key_consistency() {
        let service = Service::new();
        let board1 = service.fen.set_init_board();
        let board2 = service.fen.set_init_board();

        let key1 = polyglot_key(&board1);
        let key2 = polyglot_key(&board2);

        assert_eq!(key1, key2, "PolyGlot key must be deterministic for identical positions");
        assert_ne!(key1, 0, "PolyGlot key should not be 0 for initial board");

        let board_e4 = service.fen.set_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
        let key_e4 = polyglot_key(&board_e4);
        assert_ne!(key1, key_e4, "PolyGlot key must change after 1.e4");
    }

    #[test]
    fn test_polyglot_move_decoding() {
        // e2e4: from_file=4 (e), from_rank=1 (2), to_file=4 (e), to_rank=3 (4)
        // mov = (0 << 12) | (1 << 9) | (4 << 6) | (3 << 3) | 4 = 796
        let entry = PolyglotEntry {
            key: 0x463b96181691fc9c,
            mov: 796,
            weight: 10,
            learn: 0,
        };
        assert_eq!(entry.to_algebraic(), "e2e4");
    }

    #[test]
    fn test_polyglot_in_memory_book_lookup() {
        let service = Service::new();
        let board = service.fen.set_init_board();
        let key = polyglot_key(&board);

        let entries = vec![
            PolyglotEntry {
                key,
                mov: 796, // e2e4
                weight: 100,
                learn: 0,
            }
        ];
        let book = PolyglotBook::from_entries(entries);
        let mov = book.get_random_book_move(&board);
        assert_eq!(mov, "e2e4");
    }
}
