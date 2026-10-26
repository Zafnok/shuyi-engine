//! The one counting allocator of a debug build (the `global` feature):
//! a `#[global_allocator]` [`crate::CountingAlloc`] over
//! [`std::alloc::System`], and [`reading`], which reads it.
//!
//! **Why it is here.** A binary has one global allocator: two
//! `#[global_allocator]` statics in it don't link. `shuyi-app` (a game's
//! window) and `shuyi-cli` (its tool program) both count the heap, and a
//! tool program links both. So neither declares the static: both turn on
//! this feature, Cargo unifies it, and the binary gets this one static.
//!
//! **When it counts.** In a debug build (`debug_assertions`), unless the
//! game builds with `--cfg shuyi_no_heap_count`, which a game with a
//! global allocator of its own sets in its `.cargo/config.toml`'s
//! `rustflags` (the crate's docs show how). A release build has no static
//! at all: the program allocates with [`std::alloc::System`] directly,
//! as if this crate weren't there.
//!
//! It counts the whole process: every thread's allocations, not only the
//! game's.

#[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
use std::alloc::System;

#[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
use crate::CountingAlloc;
use crate::Counts;

/// Whether this build counts the heap: a debug build without `--cfg
/// shuyi_no_heap_count`.
pub const COUNTING: bool = cfg!(all(debug_assertions, not(shuyi_no_heap_count)));

#[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
#[global_allocator]
static HEAP: CountingAlloc<System> = CountingAlloc::new(System);

/// The heap's counts now, the calls since the last reading taken; `None`
/// in a build that doesn't count ([`COUNTING`]).
#[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
pub fn reading() -> Option<Counts> {
    Some(HEAP.counts())
}

/// The heap's counts now, the calls since the last reading taken; `None`
/// in a build that doesn't count ([`COUNTING`]).
#[cfg(not(all(debug_assertions, not(shuyi_no_heap_count))))]
pub fn reading() -> Option<Counts> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_debug_test_build_counts_unless_it_opted_out() {
        let counting = cfg!(all(debug_assertions, not(shuyi_no_heap_count)));
        assert_eq!(COUNTING, counting);
        assert_eq!(reading().is_some(), counting);
    }

    /// Other test threads allocate too, so only what holds whatever they
    /// do: a block kept alive is in the live bytes and the peak.
    #[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
    #[test]
    fn the_global_counter_sees_a_block_this_thread_keeps() {
        let block = vec![1u8; 1 << 20];
        let counts = reading().unwrap();
        // The peak only rises (nothing resets the global one), and the
        // block raised it when it was made.
        assert!(counts.live_bytes >= 1 << 20, "{counts:?}");
        assert!(counts.peak_bytes >= 1 << 20, "{counts:?}");
        drop(block);
    }
}
