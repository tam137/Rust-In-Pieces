use std::collections::HashMap;
use rand::Rng;

pub struct OpeningBook<'a> {
    book: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> OpeningBook<'a> {

    pub fn new() -> Self {
        let mut book = HashMap::new();
        book.insert("", vec!["e2e4", "d2d4", "c2c4", "g1f3", "e2e3", "a2a3", "b2b3"]);
        book.insert("e2e3", vec!["d7d5", "e7e5", "c7c5", "c7c6", "g8f6", "f7f5", "e7e6"]);
        book.insert("e2e4", vec!["d7d5", "e7e5", "c7c5", "c7c6"]);
        book.insert("d2d4", vec!["de7d5", "e7e6", "g8f6", "c7c6", "c7c5"]);
        book.insert("c2c4", vec!["c7c5", "c7c6", "g8f5", "e7e5", "e7e6", "d7d6"]);
        book.insert("g1f3", vec!["d7d5", "e7e5", "c7c5", "g8f6", "e7e6", "c7c6"]);
        book.insert("a2a3", vec!["d7d5", "e7e5", "c7c5", "g8f6", "e7e6", "c7c6", "f7f5"]);
        book.insert("b2b3", vec!["d7d5", "e7e5", "c7c5", "g8f6", "e7e6"]);
        Self { book }
    }

    pub fn get_opening_move(&self, path: &str) -> &str {
        match self.book.get(path) {
            Some(moves) => {
                let mut rng = rand::thread_rng();
                let random_index = rng.gen_range(0..moves.len());
                let ret = moves[random_index].clone();
                ret
            }
            None => {
                ""
            }
        }
    }
}

