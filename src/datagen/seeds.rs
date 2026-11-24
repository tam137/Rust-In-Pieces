//! Seeds of a data generation run.
//!
//! A run has one base seed, random unless the configuration fixes it, and every thread derives its
//! own seed from it with the SplitMix64 finaliser. `base + thread`, the old rule of
//! `scripts/run_datagen.sh`, made two runs with nearby base seeds share threads and play the same
//! games; the mixed seeds of two different bases only meet by chance.

/// The SplitMix64 increment (2^64 / golden ratio).
const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// The SplitMix64 finaliser: a bijection on `u64` that spreads every input bit over the output.
pub fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Seed of thread `thread` (1-based, as the `part_<i>.txt` files) of a run with base seed `base`.
pub fn thread_seed(base: u64, thread: usize) -> u64 {
    mix64(base.wrapping_add((thread as u64).wrapping_mul(GOLDEN_GAMMA)))
}

/// A random base seed below 2^63, so it can be written back into the configuration as a TOML
/// integer to reproduce the run.
pub fn random_base_seed() -> u64 {
    rand::random::<u64>() >> 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn thread_seeds_are_deterministic_and_distinct() {
        assert_eq!(thread_seed(1000, 3), thread_seed(1000, 3));
        let seeds: HashSet<u64> = (1..=64).map(|t| thread_seed(1000, t)).collect();
        assert_eq!(seeds.len(), 64);
    }

    #[test]
    fn nearby_base_seeds_share_no_thread_seed() {
        // The old rule `base + i` gave runs 1000 and 1005 seven common seeds out of twelve.
        let a: HashSet<u64> = (1..=12).map(|t| thread_seed(1000, t)).collect();
        let b: HashSet<u64> = (1..=12).map(|t| thread_seed(1005, t)).collect();
        assert!(a.is_disjoint(&b));
        let consecutive: HashSet<u64> = (0..200u64)
            .flat_map(|base| (1..=16).map(move |t| thread_seed(base, t)))
            .collect();
        assert_eq!(consecutive.len(), 200 * 16);
    }

    #[test]
    fn random_base_seeds_fit_a_toml_integer() {
        for _ in 0..1000 {
            assert!(random_base_seed() <= i64::MAX as u64);
        }
    }

    #[test]
    fn mix64_matches_the_reference_splitmix64_output() {
        // First output of the reference SplitMix64 generator seeded with 0: mix64(0 + gamma).
        assert_eq!(mix64(GOLDEN_GAMMA), 0xE220_A839_7B1D_CDAF);
    }
}
