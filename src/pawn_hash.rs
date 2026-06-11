use std::cell::Cell;

/// Entry for the lock-free Pawn Hash Table.
/// `Copy` and `Clone` are derived to allow array initialization.
#[derive(Clone, Copy)]
pub struct PawnEntry {
    pub pawn_hash: u64,
    pub mg: i16,
    pub eg: i16,
}

/// A fast, lock-free, single-threaded hash table for caching pawn structure evaluations.
/// Utilizes interior mutability (`std::cell::Cell`) to allow caching during immutable search traversal.
pub struct PawnHashTable {
    entries: Vec<Cell<PawnEntry>>,
    pub capacity: usize,
    mask: usize,
}

impl PawnHashTable {
    /// Initializes a new Pawn Hash Table given a specific number of entries.
    pub fn new(mut capacity: usize) -> Self {
        
        // Ensure capacity is a power of two for fast bitwise masking
        if capacity.count_ones() != 1 {
            capacity = capacity.next_power_of_two() / 2;
        }
        
        // Failsafe minimum
        if capacity == 0 {
            capacity = 1;
        }

        let mask = capacity - 1;

        // Initialize with default empty cells
        let entries = vec![Cell::new(PawnEntry { pawn_hash: 0, mg: 0, eg: 0 }); capacity];

        PawnHashTable {
            entries,
            capacity,
            mask,
        }
    }

    /// Primarily used for testing: Initializes with a fixed number of entries.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut actual_capacity = capacity;
        if actual_capacity.count_ones() != 1 {
            actual_capacity = actual_capacity.next_power_of_two() / 2;
        }
        if actual_capacity == 0 {
            actual_capacity = 1;
        }
        
        let mask = actual_capacity - 1;
        let entries = vec![Cell::new(PawnEntry { pawn_hash: 0, mg: 0, eg: 0 }); actual_capacity];

        PawnHashTable {
            entries,
            capacity: actual_capacity,
            mask,
        }
    }

    /// Fetches the pawn structure scores if the hash matches.
    pub fn get(&self, pawn_hash: u64) -> Option<(i16, i16)> {
        let index = (pawn_hash as usize) & self.mask;
        let entry = self.entries[index].get();
        if entry.pawn_hash == pawn_hash {
            Some((entry.mg, entry.eg))
        } else {
            None
        }
    }

    /// Caches the evaluated pawn structure scores.
    pub fn store(&self, pawn_hash: u64, mg: i16, eg: i16) {
        let index = (pawn_hash as usize) & self.mask;
        self.entries[index].set(PawnEntry {
            pawn_hash,
            mg,
            eg,
        });
    }

    /// Clears the entire table.
    pub fn clear(&self) {
        for entry in &self.entries {
            entry.set(PawnEntry { pawn_hash: 0, mg: 0, eg: 0 });
        }
    }
}
