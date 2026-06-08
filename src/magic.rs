use std::sync::Once;

#[derive(Clone, Copy)]
pub struct Magic {
    pub mask: u64,
    pub magic: u64,
    pub shift: u8,
    pub offset: usize,
}

static mut BISHOP_MAGICS: [Magic; 64] = [Magic { mask: 0, magic: 0, shift: 0, offset: 0 }; 64];
static mut ROOK_MAGICS: [Magic; 64] = [Magic { mask: 0, magic: 0, shift: 0, offset: 0 }; 64];
static mut BISHOP_TABLE: [u64; 5248] = [0; 5248];
static mut ROOK_TABLE: [u64; 102400] = [0; 102400];

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        unsafe {
            initialize_magics();
        }
    });
}

pub fn get_bishop_attacks(square: usize, occupied: u64) -> u64 {
    init();
    unsafe {
        let magic = &BISHOP_MAGICS[square];
        let index = (((occupied & magic.mask).wrapping_mul(magic.magic)) >> magic.shift) as usize;
        BISHOP_TABLE[magic.offset + index]
    }
}

pub fn get_rook_attacks(square: usize, occupied: u64) -> u64 {
    init();
    unsafe {
        let magic = &ROOK_MAGICS[square];
        let index = (((occupied & magic.mask).wrapping_mul(magic.magic)) >> magic.shift) as usize;
        ROOK_TABLE[magic.offset + index]
    }
}

unsafe fn initialize_magics() {
    unsafe {
        let mut prng = Prng::new(1804289383);
        let bishop_ptr = std::ptr::addr_of_mut!(BISHOP_MAGICS) as *mut Magic;
        let bishop_table_ptr = std::ptr::addr_of_mut!(BISHOP_TABLE) as *mut u64;
        
        // 1. Initialize Bishop Magics & Table
        let mut bishop_offset = 0;
        for sq in 0..64 {
            let mask = bishop_mask(sq);
            let num_bits = mask.count_ones();
            let num_configs = 1 << num_bits;
            let shift = (64 - num_bits) as u8;
            
            let mut occupancies = vec![0u64; num_configs];
            let mut attacks = vec![0u64; num_configs];
            for i in 0..num_configs {
                occupancies[i] = index_to_occupancy(i, num_bits, mask);
                attacks[i] = classical_bishop_attacks(sq, occupancies[i]);
            }
            
            let magic_val = find_magic(sq, num_bits, &occupancies, &attacks, &mut prng);
            bishop_ptr.add(sq).write(Magic {
                mask,
                magic: magic_val,
                shift,
                offset: bishop_offset,
            });
            
            for i in 0..num_configs {
                let idx = (((occupancies[i] & mask).wrapping_mul(magic_val)) >> shift) as usize;
                bishop_table_ptr.add(bishop_offset + idx).write(attacks[i]);
            }
            bishop_offset += num_configs;
        }
        
        // 2. Initialize Rook Magics & Table
        let rook_ptr = std::ptr::addr_of_mut!(ROOK_MAGICS) as *mut Magic;
        let rook_table_ptr = std::ptr::addr_of_mut!(ROOK_TABLE) as *mut u64;
        let mut rook_offset = 0;
        for sq in 0..64 {
            let mask = rook_mask(sq);
            let num_bits = mask.count_ones();
            let num_configs = 1 << num_bits;
            let shift = (64 - num_bits) as u8;
            
            let mut occupancies = vec![0u64; num_configs];
            let mut attacks = vec![0u64; num_configs];
            for i in 0..num_configs {
                occupancies[i] = index_to_occupancy(i, num_bits, mask);
                attacks[i] = classical_rook_attacks(sq, occupancies[i]);
            }
            
            let magic_val = find_magic(sq, num_bits, &occupancies, &attacks, &mut prng);
            rook_ptr.add(sq).write(Magic {
                mask,
                magic: magic_val,
                shift,
                offset: rook_offset,
            });
            
            for i in 0..num_configs {
                let idx = (((occupancies[i] & mask).wrapping_mul(magic_val)) >> shift) as usize;
                rook_table_ptr.add(rook_offset + idx).write(attacks[i]);
            }
            rook_offset += num_configs;
        }
    }
}

fn find_magic(_sq: usize, num_bits: u32, occupancies: &[u64], attacks: &[u64], prng: &mut Prng) -> u64 {
    let num_configs = 1 << num_bits;
    let shift = 64 - num_bits;
    let mut used = vec![0u64; num_configs];
    let mut epoch_map = vec![0u32; num_configs];
    let mut epoch = 0u32;
    
    loop {
        // Generate a sparse magic candidate
        let magic = prng.next() & prng.next() & prng.next();
        
        // Skip magics that have too few set bits
        if magic.count_ones() < 6 {
            continue;
        }
        
        epoch += 1;
        let mut fail = false;
        
        for i in 0..num_configs {
            let idx = ((occupancies[i].wrapping_mul(magic)) >> shift) as usize;
            if epoch_map[idx] == epoch {
                if used[idx] != attacks[i] {
                    fail = true;
                    break;
                }
            } else {
                epoch_map[idx] = epoch;
                used[idx] = attacks[i];
            }
        }
        
        if !fail {
            return magic;
        }
    }
}

fn bishop_mask(square: usize) -> u64 {
    let mut mask = 0u64;
    let r = (square / 8) as i32;
    let f = (square % 8) as i32;
    
    let (mut tr, mut tf) = (r + 1, f + 1);
    while tr < 7 && tf < 7 {
        mask |= 1u64 << (tr * 8 + tf);
        tr += 1;
        tf += 1;
    }
    let (mut tr, mut tf) = (r + 1, f - 1);
    while tr < 7 && tf > 0 {
        mask |= 1u64 << (tr * 8 + tf);
        tr += 1;
        tf -= 1;
    }
    let (mut tr, mut tf) = (r - 1, f + 1);
    while tr > 0 && tf < 7 {
        mask |= 1u64 << (tr * 8 + tf);
        tr -= 1;
        tf += 1;
    }
    let (mut tr, mut tf) = (r - 1, f - 1);
    while tr > 0 && tf > 0 {
        mask |= 1u64 << (tr * 8 + tf);
        tr -= 1;
        tf -= 1;
    }
    mask
}

fn rook_mask(square: usize) -> u64 {
    let mut mask = 0u64;
    let r = (square / 8) as i32;
    let f = (square % 8) as i32;
    
    for tr in (r + 1)..7 {
        mask |= 1u64 << (tr * 8 + f);
    }
    for tr in 1..r {
        mask |= 1u64 << (tr * 8 + f);
    }
    for tf in (f + 1)..7 {
        mask |= 1u64 << (r * 8 + tf);
    }
    for tf in 1..f {
        mask |= 1u64 << (r * 8 + tf);
    }
    mask
}

fn index_to_occupancy(index: usize, num_bits: u32, mask: u64) -> u64 {
    let mut occupancy = 0u64;
    let mut temp_mask = mask;
    for i in 0..num_bits {
        let sq = temp_mask.trailing_zeros() as u8;
        temp_mask &= temp_mask - 1;
        if (index & (1 << i)) != 0 {
            occupancy |= 1u64 << sq;
        }
    }
    occupancy
}

fn classical_bishop_attacks(square: usize, occupied: u64) -> u64 {
    let mut attacks = 0u64;
    let r = (square / 8) as i32;
    let f = (square % 8) as i32;
    
    let (mut tr, mut tf) = (r + 1, f + 1);
    while tr < 8 && tf < 8 {
        let sq = tr * 8 + tf;
        attacks |= 1u64 << sq;
        if (occupied & (1u64 << sq)) != 0 { break; }
        tr += 1;
        tf += 1;
    }
    let (mut tr, mut tf) = (r + 1, f - 1);
    while tr < 8 && tf >= 0 {
        let sq = tr * 8 + tf;
        attacks |= 1u64 << sq;
        if (occupied & (1u64 << sq)) != 0 { break; }
        tr += 1;
        tf -= 1;
    }
    let (mut tr, mut tf) = (r - 1, f + 1);
    while tr >= 0 && tf < 8 {
        let sq = tr * 8 + tf;
        attacks |= 1u64 << sq;
        if (occupied & (1u64 << sq)) != 0 { break; }
        tr -= 1;
        tf += 1;
    }
    let (mut tr, mut tf) = (r - 1, f - 1);
    while tr >= 0 && tf >= 0 {
        let sq = tr * 8 + tf;
        attacks |= 1u64 << sq;
        if (occupied & (1u64 << sq)) != 0 { break; }
        tr -= 1;
        tf -= 1;
    }
    attacks
}

fn classical_rook_attacks(square: usize, occupied: u64) -> u64 {
    let mut attacks = 0u64;
    let r = (square / 8) as i32;
    let f = (square % 8) as i32;
    
    for tr in (r + 1)..8 {
        let sq = tr * 8 + f;
        attacks |= 1u64 << sq;
        if (occupied & (1u64 << sq)) != 0 { break; }
    }
    for tr in (0..r).rev() {
        let sq = tr * 8 + f;
        attacks |= 1u64 << sq;
        if (occupied & (1u64 << sq)) != 0 { break; }
    }
    for tf in (f + 1)..8 {
        let sq = r * 8 + tf;
        attacks |= 1u64 << sq;
        if (occupied & (1u64 << sq)) != 0 { break; }
    }
    for tf in (0..f).rev() {
        let sq = r * 8 + tf;
        attacks |= 1u64 << sq;
        if (occupied & (1u64 << sq)) != 0 { break; }
    }
    attacks
}

struct Prng {
    state: u64,
}

impl Prng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    
    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_attacks_match_classical() {
        let mut prng = Prng::new(123456789);
        for sq in 0..64 {
            // Generate some random occupancies and compare
            for _ in 0..100 {
                let occupied = prng.next();
                
                let magic_bishop = get_bishop_attacks(sq, occupied);
                let classical_bishop = classical_bishop_attacks(sq, occupied);
                assert_eq!(magic_bishop, classical_bishop, "Bishop mismatch at square {} with occupied {}", sq, occupied);

                let magic_rook = get_rook_attacks(sq, occupied);
                let classical_rook = classical_rook_attacks(sq, occupied);
                assert_eq!(magic_rook, classical_rook, "Rook mismatch at square {} with occupied {}", sq, occupied);
            }
        }
    }
}
