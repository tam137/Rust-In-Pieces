use std::collections::HashMap;
use rand::seq::SliceRandom; // Um zufällige Einträge auszuwählen
use rand::thread_rng;

#[derive(Debug, Clone)]
pub struct Book {
    pub book_map: HashMap<&'static str, Vec<&'static str>>,
}

impl Book {
    pub fn new() -> Self {
        let mut book_map = HashMap::new();
        book_map.insert("", vec!["e2e4", "d2d4", "g1c3", "c2c4", "b1c3"]);

        book_map.insert("e2e4", vec!["e7e5", "d7d6", "g8f6", "b8c6"]);
        book_map.insert("g1f3", vec!["e7e6", "d7d5", "d7d6", "g8f6"]);
        book_map.insert("b1c3", vec!["e7e6", "e7e5", "d7d6", "g8f6"]);
        book_map.insert("d2d4", vec!["e7e6", "d7d5", "g8f6", "b8c6"]);

        Book { book_map }
    }


    pub fn get_random_book_move(&self, made_moves: &str) -> &str {
        if let Some(moves) = self.book_map.get(made_moves) {
            let mut rng = thread_rng();
            return moves.choose(&mut rng).copied().unwrap_or("");
        }
        ""
    }
}