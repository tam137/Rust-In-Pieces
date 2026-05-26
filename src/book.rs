use std::collections::HashMap;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Debug, Clone)]
pub struct Book {
    pub book_map: HashMap<&'static str, Vec<&'static str>>,
}

impl Book {
    pub fn new() -> Self {
        let mut book_map = HashMap::new();
        book_map.insert("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", vec!["e2e4", "e2e4", "d2d4", "d2d4", "g1f3", "c2c4", "b2b3", "b2b4", "f2f4", "g2g4", "c2c3", "a2a3"]);

        let e2e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let d2d4 = "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1";
        let g1f3 = "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 0 1";
        let c2c4 = "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3 0 1";
        book_map.insert(e2e4, vec!["e7e5", "e7e5", "c7c5", "c7c5", "e7e6", "c7c6", "d7d6", "d7d5"]);
        book_map.insert(d2d4, vec!["e7e6", "d7d5", "d7d5", "d7d6", "g8f6"]);
        book_map.insert(g1f3, vec!["e7e6", "d7d5", "d7d6", "g8f6", "c7c5"]);
        book_map.insert(c2c4, vec!["e7e6", "d7d5", "d7d6", "g8f6", "g8f6", "b8c6", "c7c5", "c7c5"]);


        // Response to e4: Scandinavian, Open Game, Pirc, Alekhine, French, Caro-Kann, Sicilian
        let e2e4_d7d5 = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2";
        let e2e4_e7e5 = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2";
        let e2e4_d7d6 = "rnbqkbnr/ppp1pppp/3p4/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let e2e4_g8f6 = "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let e2e4_e7e6 = "rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let e2e4_c7c6 = "rnbqkbnr/pp1ppppp/2p5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let e2e4_c7c5 = "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2";
        
        book_map.insert(e2e4_d7d5, vec!["e4d5", "e4d5", "d2d3"]);
        book_map.insert(e2e4_e7e5, vec!["g1f3", "g1f3", "f1c4", "d2d4", "d2d3"]);
        book_map.insert(e2e4_d7d6, vec!["d2d4", "d2d3", "g1f3", "b1c3", "f1c4", "c2c4", "c2c3"]);
        book_map.insert(e2e4_g8f6, vec!["e4e5", "d2d3", "b1c3", "d1f3"]);    
        book_map.insert(e2e4_e7e6, vec!["d2d4", "d2d3", "g1f3", "b1c3", "f1c4", "c2c4", "c2c3", "e4e5"]);
        book_map.insert(e2e4_c7c6, vec!["d2d4", "d2d4", "g1f3", "b1c3", "c2c3", "d2d3"]);
        book_map.insert(e2e4_c7c5, vec!["g1f3", "g1f3", "b1c3", "c2c3", "d2d4", "f2f4", "d2d3"]);


        // Continuation after 1.e4 e5 2.Nf3 & 1.e4 c5 2.Nf3 & 1.e4 e6 2.d4 & 1.e4 c6 2.d4
        let e2e4_e7e5_g1f3 = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let e2e4_e7e5_g1f3_b8c6 = "r1bqkbnr/pppp1ppp/2n5/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3";
        let e2e4_c7c5_g1f3 = "rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let e2e4_e7e6_d2d4 = "rnbqkbnr/pppp1ppp/4p3/8/3PP3/8/PPP2PPP/RNBQKBNR b KQkq d3 0 2";
        let e2e4_c7c6_d2d4 = "rnbqkbnr/pp1ppppp/2p5/8/3PP3/8/PPP2PPP/RNBQKBNR b KQkq d3 0 2";

        book_map.insert(e2e4_e7e5_g1f3, vec!["b8c6", "b8c6", "g8f6", "d7d6"]);
        book_map.insert(e2e4_e7e5_g1f3_b8c6, vec!["f1b5", "f1b5", "f1c4", "b1c3", "d2d4"]);
        book_map.insert(e2e4_c7c5_g1f3, vec!["d7d6", "e7e6", "b8c6", "g7g6"]);
        book_map.insert(e2e4_e7e6_d2d4, vec!["d7d5"]);
        book_map.insert(e2e4_c7c6_d2d4, vec!["d7d5"]);


        // Response to c2c4: English Opening
        let c2c4_d7d5 = "rnbqkbnr/ppp1pppp/8/3p4/2P5/8/PP1PPPPP/RNBQKBNR w KQkq d6 0 2";
        let c2c4_c7c5 = "rnbqkbnr/pp1ppppp/8/2p5/2P5/8/PP1PPPPP/RNBQKBNR w KQkq c6 0 2";
        let c2c4_g8f6 = "rnbqkb1r/pppppppp/5n2/8/2P5/8/PP1PPPPP/RNBQKBNR w KQkq - 0 2";
        book_map.insert(c2c4_d7d5, vec!["d2d4", "c4d5", "e2e3", "b1c3"]);
        book_map.insert(c2c4_c7c5, vec!["d2d4", "b1c3", "e2e3", "e2e4", "d2d3"]);
        book_map.insert(c2c4_g8f6, vec!["d2d4", "d2d4", "b1c3", "e2e3", "d2d3"]);
        
        
        // Response to d2d4: Queen's Pawn Games & Queen's Gambit
        let d2d4_d7d6 = "rnbqkbnr/ppp1pppp/3p4/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2";
        let d2d4_d7d5 = "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq d6 0 2";
        let d2d4_e7e6 = "rnbqkbnr/pppp1ppp/4p3/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2";
        let d2d4_g8f6 = "rnbqkb1r/pppppppp/5n2/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2";
        let d2d4_d7d5_c2c4 = "rnbqkbnr/ppp1pppp/8/3p4/2PP4/8/PP2PPPP/RNBQKBNR b KQkq c3 0 2";

        book_map.insert(d2d4_d7d6, vec!["c2c4", "e2e4", "e2e3", "g1f3"]);
        book_map.insert(d2d4_d7d5, vec!["c2c4", "c2c4", "c2c3", "e2e3", "c1f4", "g1f3", "b1c3", "e2e4"]);
        book_map.insert(d2d4_e7e6, vec!["c2c4", "c2c3", "e2e3", "c1f4", "g1f3", "b1c3", "e2e4"]);
        book_map.insert(d2d4_g8f6, vec!["c2c4", "c2c4", "c1f4", "c1g5", "g1f3", "e2e3"]);
        book_map.insert(d2d4_d7d5_c2c4, vec!["e7e6", "e7e6", "c7c6", "d5c4"]);


        // Response to g1f3: Reti Opening
        let g1f3_e7e6_ = "rnbqkbnr/pppp1ppp/4p3/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2";
        let g1f3_d7d5_ = "rnbqkbnr/ppp1pppp/8/3p4/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2";
        let g1f3_d7d6_ = "rnbqkbnr/ppp1pppp/3p4/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2";
        let g1f3_g8f6_ = "rnbqkb1r/pppppppp/5n2/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2";
        let g1f3_c7c5_ = "rnbqkbnr/pp1ppppp/8/2p5/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2";
        book_map.insert(g1f3_e7e6_, vec!["d2d4", "d2d3", "e2e4", "e2e3", "c2c4", "b1c3", "c2c3", "b2b3"]);
        book_map.insert(g1f3_d7d5_, vec!["d2d4", "d2d3", "e2e4", "e2e3", "c2c4", "c2c3", "g2g3"]);
        book_map.insert(g1f3_d7d6_, vec!["d2d4", "d2d3", "e2e4", "e2e3", "c2c4", "b1c3", "c2c3", "g2g3"]);
        book_map.insert(g1f3_g8f6_, vec!["d2d4", "d2d3", "e2e4", "e2e3", "c2c4", "b1c3", "c2c3", "g2g3"]);
        book_map.insert(g1f3_c7c5_, vec!["d2d4", "d2d3", "e2e4", "e2e3", "c2c4", "c2c3", "g2g3"]);
        
        // Dubious / Irregular Openings (Grob, Bird, Larsen, Polish, Saragossa, Anderssen)
        let b1b3 = "rnbqkbnr/pppppppp/8/8/8/1P6/P1PPPPPP/RNBQKBNR b KQkq - 0 1";
        let b1b4 = "rnbqkbnr/pppppppp/8/8/1P6/8/P1PPPPPP/RNBQKBNR b KQkq b3 0 1";
        let f2f4 = "rnbqkbnr/pppppppp/8/8/5P2/8/PPPPP1PP/RNBQKBNR b KQkq f3 0 1";
        let g2g4 = "rnbqkbnr/pppppppp/8/8/6P1/8/PPPPPP1P/RNBQKBNR b KQkq g3 0 1";
        let c2c3 = "rnbqkbnr/pppppppp/8/8/8/2P5/PP1PPPPP/RNBQKBNR b KQkq - 0 1";
        let a2a3 = "rnbqkbnr/pppppppp/8/8/8/P7/1PPPPPPP/RNBQKBNR b KQkq - 0 1";

        book_map.insert(b1b3, vec!["d7d5", "e7e5", "g8f6"]);
        book_map.insert(b1b4, vec!["d7d5", "e7e5", "g8f6"]);
        book_map.insert(f2f4, vec!["d7d5", "e7e5"]);
        book_map.insert(g2g4, vec!["d7d5", "e7e5"]);
        book_map.insert(c2c3, vec!["d7d5", "e7e5"]);
        book_map.insert(a2a3, vec!["d7d5", "e7e5"]);

        let g2g4_d7d5 = "rnbqkbnr/ppp1pppp/8/3p4/6P1/8/PPPPPP1P/RNBQKBNR w KQkq d6 0 2";
        book_map.insert(g2g4_d7d5, vec!["f1g2", "h2h3"]);

        let f2f4_e7e5 = "rnbqkbnr/pppp1ppp/8/4p3/5P2/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 2";
        book_map.insert(f2f4_e7e5, vec!["f4e5"]);

        Book { book_map }
    }


    pub fn get_random_book_move(&self, fen: &str) -> &str {
        if let Some(moves) = self.book_map.get(fen) {
            let mut rng = thread_rng();
            return moves.choose(&mut rng).copied().unwrap_or("");
        }
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::Service;
    use crate::config::Config;
    use crate::model::{MoveList, DataMap, SearchContext, Stats};
    use crate::zobrist::ZobristTable;

    #[test]
    fn test_all_book_moves_are_legal() {
        let service = Service::new();
        let fen_service = service.fen;
        let book = Book::new();
        
        let config = Config::for_tests();
        let zobrist_table = ZobristTable::with_capacity(1);
        let stop_flag = std::sync::atomic::AtomicBool::new(false);
        let pv_nodes = std::sync::Mutex::new(std::collections::HashMap::new());
        let history_table = [[0u32; 64]; 64];
        let context = SearchContext {
            zobrist_table: &zobrist_table,
            stop_flag: &stop_flag,
            pv_nodes: &pv_nodes,
            killer_moves: [None; 2],
            history_table: &history_table,
            counter_move: None,
        };

        for (fen, moves) in book.book_map.iter() {
            // Load the FEN onto a board. If it is invalid, this will panic.
            let mut board = fen_service.set_fen(fen);
            
            // Generate all legal moves for the current active player.
            let local_map = DataMap::new();
            let mut move_list = MoveList::new();
            service.move_gen.generate_valid_moves_list(&mut board, &mut Stats::new(), &config, &context, &local_map, &mut move_list);

            // Convert all generated moves to algebraic strings.
            let mut legal_moves = Vec::new();
            for m in move_list.as_slice() {
                legal_moves.push(m.to_algebraic());
            }

            // Verify that every suggested book move is in the list of legal moves.
            for &m in moves.iter() {
                assert!(
                    legal_moves.contains(&m.to_string()),
                    "Illegal book move '{}' found for FEN '{}'. Legal moves are: {:?}",
                    m,
                    fen,
                    legal_moves
                );
            }
        }
    }
}