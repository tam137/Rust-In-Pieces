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

        // Extended deeper lines
        let e2e4_e7e5_g1f3_b8c6_f1b5 = "r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 0 3";
        let e2e4_c7c5_g1f3_d7d6_d2d4_c5d4_f3d4_g8f6_b1c3 = "rnbqkb1r/pp2pppp/3p1n2/8/3NP3/2N5/PPP2PPP/R1BQKB1R b KQkq - 0 5";
        let e2e4_e7e6_d2d4_d7d5_b1c3 = "rnbqkbnr/ppp2ppp/4p3/3p4/3PP3/2N5/PPP2PPP/R1BQKBNR b KQkq - 0 3";
        let e2e4_c7c6_d2d4_d7d5_b1c3 = "rnbqkbnr/pp2pppp/2p5/3p4/3PP3/2N5/PPP2PPP/R1BQKBNR b KQkq - 0 3";
        let d2d4_d7d5_c2c4_e7e6_b1c3_g8f6 = "rnbqkb1r/ppp2ppp/4pn2/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 0 4";

        book_map.insert(e2e4_e7e5_g1f3_b8c6_f1b5, vec!["a7a6", "g8f6"]);
        book_map.insert(e2e4_c7c5_g1f3_d7d6_d2d4_c5d4_f3d4_g8f6_b1c3, vec!["a7a6", "e7e6"]);
        book_map.insert(e2e4_e7e6_d2d4_d7d5_b1c3, vec!["g8f6", "f8b4", "d5e4"]);
        book_map.insert(e2e4_c7c6_d2d4_d7d5_b1c3, vec!["d5e4"]);
        book_map.insert(d2d4_d7d5_c2c4_e7e6_b1c3_g8f6, vec!["c1g5", "g1f3", "c4d5"]);

        // Solid additions for Black and White
        book_map.insert("rnbqkb1r/ppp2ppp/4pn2/3p4/2PP4/2N2N2/PP2PPPP/R1BQKB1R b KQkq - 0 4", vec!["f8e7", "f8b4"]);
        book_map.insert("rnbqkbnr/pp2pppp/2p5/8/3PN3/8/PPP2PPP/R1BQKBNR b KQkq - 0 4", vec!["c8f5", "b8d7", "g8f6"]);
        book_map.insert("rnbqk2r/ppp1bppp/4pn2/3p2B1/2PP4/2N5/PP2PPPP/R2QKBNR w KQkq - 1 5", vec!["e2e3", "g1f3"]);
        book_map.insert("rnbqkbnr/ppp2ppp/8/3p4/3P4/8/PPP2PPP/RNBQKBNR w KQkq - 0 4", vec!["g1f3", "f1d3"]);

        // --- NEW VARIATIONS START ---

        // Italian Game (Giuoco Piano) & Two Knights Defense
        let e2e4_e7e5_g1f3_b8c6_f1c4_f8c5 = "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4";
        let e2e4_e7e5_g1f3_b8c6_f1c4_g8f6 = "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4";

        book_map.insert(e2e4_e7e5_g1f3_b8c6_f1c4_f8c5, vec!["c2c3", "d2d3", "e1g1"]);
        book_map.insert(e2e4_e7e5_g1f3_b8c6_f1c4_g8f6, vec!["d2d3", "d2d4", "f3g5"]);

        // Ruy Lopez (Spanish Opening)
        let e2e4_e7e5_g1f3_b8c6_f1b5_a7a6_b5a4 = "r1bqkbnr/1ppp1ppp/p1n5/4p3/B3P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 0 4";
        let e2e4_e7e5_g1f3_b8c6_f1b5_a7a6_b5a4_g8f6_e1g1 = "r1bqkb1r/1ppp1ppp/p1n2n2/4p3/B3P3/5N2/PPPP1PPP/RNBQ1RK1 b kq - 0 5";
        let e2e4_e7e5_g1f3_b8c6_f1b5_g8f6 = "r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4";

        book_map.insert(e2e4_e7e5_g1f3_b8c6_f1b5_a7a6_b5a4, vec!["g8f6"]);
        book_map.insert(e2e4_e7e5_g1f3_b8c6_f1b5_a7a6_b5a4_g8f6_e1g1, vec!["f8e7", "f6e4"]);
        book_map.insert(e2e4_e7e5_g1f3_b8c6_f1b5_g8f6, vec!["e1g1", "d2d3"]);

        // Sicilian Defense (Alapin, Closed, Sveshnikov/Classical, French)
        let e2e4_c7c5_c2c3 = "rnbqkbnr/pp1ppppp/8/2p5/4P3/2P5/PP1P1PPP/RNBQKBNR b KQkq - 0 2";
        let e2e4_c7c5_b1c3 = "rnbqkbnr/pp1ppppp/8/2p5/4P3/2N5/PPPP1PPP/R1BQKBNR b KQkq - 0 2";
        let e2e4_c7c5_g1f3_b8c6 = "r1bqkbnr/pp1ppppp/2n5/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 3";
        let e2e4_c7c5_g1f3_e7e6 = "rnbqkbnr/pp1p1ppp/4p3/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 3";

        book_map.insert(e2e4_c7c5_c2c3, vec!["d7d5", "g8f6"]);
        book_map.insert(e2e4_c7c5_b1c3, vec!["b8c6", "g7g6", "d7d6"]);
        book_map.insert(e2e4_c7c5_g1f3_b8c6, vec!["d2d4", "b1c3", "f1b5"]);
        book_map.insert(e2e4_c7c5_g1f3_e7e6, vec!["d2d4", "c2c3", "b1c3"]);

        // French Defense (Advance & Exchange)
        let e2e4_e7e6_d2d4_d7d5_e4e5 = "rnbqkbnr/ppp2ppp/4p3/3pP3/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3";
        let e2e4_e7e6_d2d4_d7d5_e4d5 = "rnbqkbnr/ppp2ppp/4p3/3P4/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3";

        book_map.insert(e2e4_e7e6_d2d4_d7d5_e4e5, vec!["c7c5"]);
        book_map.insert(e2e4_e7e6_d2d4_d7d5_e4d5, vec!["e6d5"]);

        // Caro-Kann Defense (Advance & Exchange)
        let e2e4_c7c6_d2d4_d7d5_e4e5 = "rnbqkbnr/pp2pppp/2p5/3pP3/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3";
        let e2e4_c7c6_d2d4_d7d5_e4d5 = "rnbqkbnr/pp2pppp/2p5/3P4/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3";

        book_map.insert(e2e4_c7c6_d2d4_d7d5_e4e5, vec!["c8f5", "c6c5"]);
        book_map.insert(e2e4_c7c6_d2d4_d7d5_e4d5, vec!["c6d5"]);

        // Queen's Gambit (Declined & Accepted) & Slav Defense
        let d2d4_d7d5_c2c4_e7e6_b1c3 = "rnbqkbnr/ppp2ppp/4p3/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR b KQkq - 0 3";
        let d2d4_d7d5_c2c4_c7c6 = "rnbqkbnr/pp2pppp/2p5/3p4/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3";
        let d2d4_d7d5_c2c4_d5c4 = "rnbqkbnr/ppp1pppp/8/8/2pP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3";

        book_map.insert(d2d4_d7d5_c2c4_e7e6_b1c3, vec!["g8f6", "c7c6"]);
        book_map.insert(d2d4_d7d5_c2c4_c7c6, vec!["g1f3", "b1c3"]);
        book_map.insert(d2d4_d7d5_c2c4_d5c4, vec!["g1f3", "e2e3", "e2e4"]);

        // Indian Defenses (King's Indian & Grünfeld)
        let d2d4_g8f6_c2c4_g7g6 = "rnbqkb1r/pppppp1p/5np1/8/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3";
        let d2d4_g8f6_c2c4_g7g6_b1c3_f8g7 = "rnbqk2r/ppppppbp/5np1/8/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 0 4";
        let d2d4_g8f6_c2c4_g7g6_b1c3_d7d5 = "rnbqkb1r/ppp1pp1p/5np1/3p4/2PP4/2N5/PP2PPPP/R1BQKBNR w KQkq d6 0 4";

        book_map.insert(d2d4_g8f6_c2c4_g7g6, vec!["b1c3", "g1f3"]);
        book_map.insert(d2d4_g8f6_c2c4_g7g6_b1c3_f8g7, vec!["e2e4"]);
        book_map.insert(d2d4_g8f6_c2c4_g7g6_b1c3_d7d5, vec!["c4d5", "g1f3"]);

        // Nimzo-Indian Defense & Playable Exotic Openings (Budapest, Benoni)
        let d2d4_g8f6_c2c4 = "rnbqkb1r/pppppppp/5n2/8/2PP4/8/PP2PPPP/RNBQKBNR b KQkq c3 0 2";
        let d2d4_g8f6_c2c4_e7e6 = "rnbqkb1r/pppp1ppp/4pn2/8/2PP4/8/PP2PPPP/RNBQKBNR w KQkq - 0 3";
        let d2d4_g8f6_c2c4_e7e5 = "rnbqkb1r/pppp1ppp/5n2/4p3/2PP4/8/PP2PPPP/RNBQKBNR w KQkq e6 0 3";
        let d2d4_g8f6_c2c4_c7c5 = "rnbqkb1r/pp1ppppp/5n2/2p5/2PP4/8/PP2PPPP/RNBQKBNR w KQkq c6 0 3";
        let d2d4_g8f6_c2c4_e7e6_b1c3 = "rnbqkb1r/pppp1ppp/4pn2/8/2PP4/2N5/PP2PPPP/R1BQKBNR b KQkq - 0 3";
        let d2d4_g8f6_c2c4_e7e6_b1c3_f8b4 = "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N5/PP2PPPP/R1BQKBNR w KQkq - 0 4";
        let d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_e2e3 = "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N1P3/PP3PPP/R1BQKBNR b KQkq - 0 4";
        let d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_d1c2 = "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N5/PPQ1PPPP/R1B1KBNR b KQkq - 0 4";
        let d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_g1f3 = "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/2N2N2/PP2PPPP/R1BQKB1R b KQkq - 0 4";
        let d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_d1b3 = "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/1QN5/PP2PPPP/R1B1KBNR b KQkq - 0 4";
        let d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_c1g5 = "rnbqk2r/pppp1ppp/4pn2/6B1/1bPP4/2N5/PP2PPPP/R2QKBNR b KQkq - 0 4";
        let d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_a2a3 = "rnbqk2r/pppp1ppp/4pn2/8/1bPP4/P1N5/1P2PPPP/R1BQKBNR b KQkq - 0 4";
        let d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_a2a3_b4c3_b2c3 = "rnbqk2r/pppp1ppp/4pn2/8/2PP4/P1P5/4PPPP/R1BQKBNR b KQkq - 0 5";

        book_map.insert(d2d4_g8f6_c2c4, vec!["e7e6", "g7g6", "e7e5", "c7c5"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6, vec!["b1c3", "g1f3", "g2g3"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e5, vec!["d4e5"]);
        book_map.insert(d2d4_g8f6_c2c4_c7c5, vec!["d4d5"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3, vec!["f8b4"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3_f8b4, vec!["e2e3", "d1c2", "g1f3", "d1b3", "c1g5", "a2a3"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_e2e3, vec!["e8g8", "c7c5", "b7b6"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_d1c2, vec!["e8g8", "d7d5", "c7c5"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_g1f3, vec!["c7c5", "b7b6", "e8g8"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_d1b3, vec!["c7c5"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_c1g5, vec!["h7h6", "c7c5"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_a2a3, vec!["b4c3"]);
        book_map.insert(d2d4_g8f6_c2c4_e7e6_b1c3_f8b4_a2a3_b4c3_b2c3, vec!["c7c5", "b7b6", "e8g8"]);

        // --- NEW VARIATIONS END ---
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
    use crate::model::{MoveList, SearchContext, Stats};
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
        start_time: std::time::Instant::now(),
                            target_time: None,
                            root_moves_total: 0,
                            root_moves_searched: 0,
                        };

        for (fen, moves) in book.book_map.iter() {
            // Load the FEN onto a board. If it is invalid, this will panic.
            let mut board = fen_service.set_fen(fen);
            
            // Generate all legal moves for the current active player.

            let mut move_list = MoveList::new();
            service.move_gen.generate_valid_moves_list(&mut board, &mut Stats::new(), &config, &context, true, false, &mut move_list);

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