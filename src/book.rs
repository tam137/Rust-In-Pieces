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
        book_map.insert("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", vec!["e2e4", "e2e4", "d2d4", "d2d4", "g1f3", "c2c4"]);

        let e2e4 = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
        let d2d4 = "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1";
        let g1f3 = "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 0 1";
        let c2c4 = "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3 0 1";
        book_map.insert(e2e4, vec!["e7e5", "e7e5", "e7e6", "d7d6", "b8c6", "d7d5"]);
        book_map.insert(d2d4, vec!["e7e6", "d7d5", "d7d5", "d7d6", "g8f6"]);
        book_map.insert(g1f3, vec!["e7e6", "d7d5", "d7d6", "g8f6", "c7c5"]);
        book_map.insert(c2c4, vec!["e7e6", "d7d5", "d7d6", "g8f6", "g8f6", "b8c6", "c7c5", "c7c5"]);


        let e2e4_d7d5 = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2";
        let e2e4_e7e5 = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2";
        let e2e4_d7d6 = "rnbqkbnr/ppp1pppp/3p4/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let e2e4_g8f6 = "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        let e2e4_e7e6 = "rnbqkbnr/pppp1ppp/4p3/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        book_map.insert(e2e4_d7d5, vec!["e4d5", "e4d5", "d2d3"]);
        book_map.insert(e2e4_e7e5, vec!["g1f3", "g1f3", "f1c4", "d2d4", "d2d3"]);
        book_map.insert(e2e4_d7d6, vec!["d2d4", "d2d3", "g1f3", "b1c3", "f1c4", "c2c4", "c2c3"]);
        book_map.insert(e2e4_g8f6, vec!["e4e5", "d2d3", "b1c3", "d1f3"]);    
        book_map.insert(e2e4_e7e6, vec!["d2d4", "d2d3", "g1f3", "b1c3", "f1c4", "c2c4", "c2c3", "e4e5"]);


        let c2c4_d7d5 = "rnbqkbnr/ppp1pppp/8/3p4/2P5/8/PP1PPPPP/RNBQKBNR w KQkq d6 0 2";
        let c2c4_c7c5 = "rnbqkbnr/pp1ppppp/8/2p5/2P5/8/PP1PPPPP/RNBQKBNR w KQkq c6 0 2";
        let c2c4_g8f6 = "rnbqkb1r/pppppppp/5n2/8/2P5/8/PP1PPPPP/RNBQKBNR w KQkq - 0 2";
        book_map.insert(c2c4_d7d5, vec!["d2d4", "c4d5", "e2e3", "b1c3"]);
        book_map.insert(c2c4_c7c5, vec!["d2d4", "b1c3", "e2e3", "e2e4", "d2d3"]);
        book_map.insert(c2c4_g8f6, vec!["d2d4", "d2d4", "b1c3", "e2e3", "d2d3"]);
        

        let d2d4_d7d6 = "rnbqkbnr/ppp1pppp/3p4/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2";
        let d2d4_d7d5 = "rnbqkbnr/ppp1pppp/8/3p4/3P4/8/PPP1PPPP/RNBQKBNR w KQkq d6 0 2";
        let d2d4_e7e6 = "rnbqkbnr/pppp1ppp/4p3/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2";
        let d2d4_g8f6 = "rnbqkb1r/pppppppp/5n2/8/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2";
        book_map.insert(d2d4_d7d6, vec!["c2c4", "e2e4", "e2e3", "g1f3"]);
        book_map.insert(d2d4_d7d5, vec!["c2c4", "c2c4", "c2c3", "e2e3", "c1f4", "g1f3", "b1c3"]);
        book_map.insert(d2d4_e7e6, vec!["c2c4", "c2c3", "e2e3", "c1f4", "g1f3", "b1c3", "e2e4"]);
        book_map.insert(d2d4_g8f6, vec!["c2c4", "c2c4", "c1f4", "c1g5", "g1f3", "e2e3"]);


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