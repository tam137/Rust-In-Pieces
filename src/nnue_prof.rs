//! Matt-Magie step 79, phase A: what the network accumulator costs the search.
//!
//! Compiled out unless the `nnue-prof` Cargo feature is on, so the release binary is unchanged.
//! With it, the search counts processor cycles (`rdtsc`) in the accumulator updates, the bucket
//! refreshes inside them, full recomputations and the output layer, against the whole search; and
//! it counts the accumulator steps (an update or the null move's copy into a child slot) whose slot
//! no evaluation ever needed. A slot is *needed* when the network evaluates at it or at any slot
//! below it on the current path - which is exactly the set a lazy accumulator would compute, so the
//! unneeded steps are what lazy child accumulators (review item N2) would save.
//!
//! The counters are cumulative over the process, as `search_diag`'s are, and the search runs on
//! one thread, so they are thread-local cells.

#[cfg(feature = "nnue-prof")]
mod imp {
    use std::cell::Cell;

    pub const SLOTS: usize = crate::search_service::MAX_PLY;

    thread_local! {
        pub static UPDATE_CYCLES: Cell<u64> = const { Cell::new(0) };
        pub static UPDATES: Cell<u64> = const { Cell::new(0) };
        pub static REFRESH_CYCLES: Cell<u64> = const { Cell::new(0) };
        pub static REFRESHES: Cell<u64> = const { Cell::new(0) };
        pub static FULL_CYCLES: Cell<u64> = const { Cell::new(0) };
        pub static FULLS: Cell<u64> = const { Cell::new(0) };
        pub static EVAL_CYCLES: Cell<u64> = const { Cell::new(0) };
        pub static EVALS: Cell<u64> = const { Cell::new(0) };
        pub static NULL_COPIES: Cell<u64> = const { Cell::new(0) };
        pub static SEARCH_CYCLES: Cell<u64> = const { Cell::new(0) };
        pub static STEPS: Cell<u64> = const { Cell::new(0) };
        pub static UNNEEDED: Cell<u64> = const { Cell::new(0) };
        pub static FINNY_FIRST: Cell<u64> = const { Cell::new(0) };
        pub static FINNY_CACHED: Cell<u64> = const { Cell::new(0) };
        pub static FINNY_FEATURES: Cell<u64> = const { Cell::new(0) };
        pub static WRITTEN: [Cell<bool>; SLOTS] = const { [const { Cell::new(false) }; SLOTS] };
        pub static NEEDED: [Cell<bool>; SLOTS] = const { [const { Cell::new(false) }; SLOTS] };
    }

    pub fn add(cell: &'static std::thread::LocalKey<Cell<u64>>, value: u64) {
        cell.with(|c| c.set(c.get() + value));
    }

    pub fn read(cell: &'static std::thread::LocalKey<Cell<u64>>) -> u64 {
        cell.with(|c| c.get())
    }
}

/// A monotonic tick counter, or 0 without the feature: the time-stamp counter on x86_64, the
/// generic timer's virtual count (`cntvct_el0`, a fixed frequency, see [`tick_frequency`]) on
/// aarch64. Its resolution is coarser than one update; summed over millions of calls the
/// quantisation averages out.
#[inline(always)]
pub fn cycles() -> u64 {
    #[cfg(all(feature = "nnue-prof", target_arch = "x86_64"))]
    {
        // SAFETY: `rdtsc` has no preconditions on x86_64.
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    #[cfg(all(feature = "nnue-prof", target_arch = "aarch64"))]
    {
        let ticks: u64;
        // SAFETY: reading the virtual count is permitted at EL0 on Linux.
        unsafe { core::arch::asm!("isb", "mrs {}, cntvct_el0", out(reg) ticks, options(nomem, nostack)) };
        ticks
    }
    #[cfg(not(all(feature = "nnue-prof", any(target_arch = "x86_64", target_arch = "aarch64"))))]
    {
        0
    }
}

/// Ticks per second of [`cycles`] on aarch64 (`cntfrq_el0`); 0 elsewhere.
#[allow(dead_code)]
pub fn tick_frequency() -> u64 {
    #[cfg(all(feature = "nnue-prof", target_arch = "aarch64"))]
    {
        let freq: u64;
        // SAFETY: reading the counter frequency is permitted at EL0 on Linux.
        unsafe { core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq, options(nomem, nostack)) };
        freq
    }
    #[cfg(not(all(feature = "nnue-prof", target_arch = "aarch64")))]
    {
        0
    }
}

/// One incremental update since `start`; `refreshed` when a perspective was rebuilt because the
/// king crossed a bucket boundary.
#[inline(always)]
#[allow(unused_variables)]
pub fn record_update(start: u64, refreshed: bool) {
    #[cfg(feature = "nnue-prof")]
    {
        let spent = cycles().wrapping_sub(start);
        imp::add(&imp::UPDATE_CYCLES, spent);
        imp::add(&imp::UPDATES, 1);
        if refreshed {
            imp::add(&imp::REFRESH_CYCLES, spent);
            imp::add(&imp::REFRESHES, 1);
        }
    }
}

/// One full recomputation since `start`.
#[inline(always)]
#[allow(unused_variables)]
pub fn record_full(start: u64) {
    #[cfg(feature = "nnue-prof")]
    {
        imp::add(&imp::FULL_CYCLES, cycles().wrapping_sub(start));
        imp::add(&imp::FULLS, 1);
    }
}

/// One output-layer evaluation since `start`.
#[inline(always)]
#[allow(unused_variables)]
pub fn record_eval(start: u64) {
    #[cfg(feature = "nnue-prof")]
    {
        imp::add(&imp::EVAL_CYCLES, cycles().wrapping_sub(start));
        imp::add(&imp::EVALS, 1);
    }
}

/// An accumulator step into `slot`: an update, or (`null_copy`) the null move's copy. The slot's
/// previous occupant is retired here, counted as unneeded if nothing ever evaluated at or below it.
#[inline(always)]
#[allow(unused_variables)]
pub fn step_into(slot: usize, null_copy: bool) {
    #[cfg(feature = "nnue-prof")]
    {
        if slot >= imp::SLOTS {
            return;
        }
        imp::WRITTEN.with(|w| imp::NEEDED.with(|n| {
            if w[slot].get() && !n[slot].get() {
                imp::add(&imp::UNNEEDED, 1);
            }
            w[slot].set(true);
            n[slot].set(false);
        }));
        imp::add(&imp::STEPS, 1);
        if null_copy {
            imp::add(&imp::NULL_COPIES, 1);
        }
    }
}

/// One king-bucket rebuild through the Finny table (Matt-Magie step 82): a first use of the
/// bucket (a full rebuild), or a cached one that added and removed `features` features.
#[inline(always)]
#[allow(unused_variables)]
pub fn record_finny(first_use: bool, features: u64) {
    #[cfg(feature = "nnue-prof")]
    {
        if first_use {
            imp::add(&imp::FINNY_FIRST, 1);
        } else {
            imp::add(&imp::FINNY_CACHED, 1);
            imp::add(&imp::FINNY_FEATURES, features);
        }
    }
}

/// The network evaluates at `slot`: that slot and every slot below it on the path are needed.
#[inline(always)]
#[allow(unused_variables)]
pub fn needed_at(slot: usize) {
    #[cfg(feature = "nnue-prof")]
    {
        imp::NEEDED.with(|n| {
            let mut q = slot.min(imp::SLOTS - 1);
            while q >= 1 && !n[q].get() {
                n[q].set(true);
                q -= 1;
            }
        });
    }
}

/// End of one search that started at `start`: retires the slots still on the stack and adds the
/// search's cycles.
#[inline(always)]
#[allow(unused_variables)]
pub fn end_search(start: u64) {
    #[cfg(feature = "nnue-prof")]
    {
        imp::add(&imp::SEARCH_CYCLES, cycles().wrapping_sub(start));
        imp::WRITTEN.with(|w| imp::NEEDED.with(|n| {
            for slot in 0..imp::SLOTS {
                if w[slot].get() && !n[slot].get() {
                    imp::add(&imp::UNNEEDED, 1);
                }
                w[slot].set(false);
                n[slot].set(false);
            }
        }));
    }
}

/// Prints the cumulative counters on stderr as one `SEARCHDIAGNNUE` line.
pub fn dump() {
    #[cfg(feature = "nnue-prof")]
    {
        use imp::*;
        eprintln!(
            "SEARCHDIAGNNUE tick_hz={} search_cycles={} update_cycles={} updates={} refresh_cycles={} refreshes={} \
             full_cycles={} fulls={} eval_cycles={} evals={} steps={} null_copies={} unneeded={} \
             finny_first={} finny_cached={} finny_features={}",
            tick_frequency(), read(&SEARCH_CYCLES), read(&UPDATE_CYCLES), read(&UPDATES), read(&REFRESH_CYCLES),
            read(&REFRESHES), read(&FULL_CYCLES), read(&FULLS), read(&EVAL_CYCLES), read(&EVALS),
            read(&STEPS), read(&NULL_COPIES), read(&UNNEEDED),
            read(&FINNY_FIRST), read(&FINNY_CACHED), read(&FINNY_FEATURES),
        );
    }
}
