//! The one counting allocator of a debug build (the `global` feature):
//! a `#[global_allocator]` [`crate::CountingAlloc::with_tags`] over
//! [`std::alloc::System`], and [`reading`] and [`rows`], which read it.
//! What it counts goes under the tags the program enters
//! ([`crate::tags`]): `shuyi-app` and `shuyi-cli` enter the frame's
//! phases, the game's spans and each asset kind's decode.
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
use crate::{CountingAlloc, Tagged};
use crate::{Counts, TagCounts};

/// Whether this build counts the heap: a debug build without `--cfg
/// shuyi_no_heap_count`.
pub const COUNTING: bool = cfg!(all(debug_assertions, not(shuyi_no_heap_count)));

#[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
#[global_allocator]
static HEAP: CountingAlloc<System, Tagged> = CountingAlloc::with_tags(System);

/// The heap's counts now, the calls since the last reading taken; `None`
/// in a build that doesn't count ([`COUNTING`]).
pub fn reading() -> Option<Counts> {
    COUNTING.then(read)
}

/// The heap's counts by tag now ([`CountingAlloc::rows`](crate::CountingAlloc::rows));
/// `None` in a build that doesn't count ([`COUNTING`]).
pub fn rows() -> Option<Vec<TagCounts>> {
    COUNTING.then(read_rows)
}

// What `reading` and `rows` read, by build: two names each, so mutation
// testing (which runs a debug build) can leave out the ones it never
// compiles.
#[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
use {counted as read, counted_rows as read_rows};
#[cfg(not(all(debug_assertions, not(shuyi_no_heap_count))))]
use {uncounted as read, uncounted_rows as read_rows};

/// The global counter's counts.
#[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
fn counted() -> Counts {
    HEAP.counts()
}

/// The global counter's counts by tag.
#[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
fn counted_rows() -> Vec<TagCounts> {
    HEAP.rows()
}

/// Never called: this build has no counter, so [`COUNTING`] is false.
#[cfg(not(all(debug_assertions, not(shuyi_no_heap_count))))]
fn uncounted() -> Counts {
    Counts::default()
}

/// Never called, as [`uncounted`].
#[cfg(not(all(debug_assertions, not(shuyi_no_heap_count))))]
fn uncounted_rows() -> Vec<TagCounts> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_debug_test_build_counts_unless_it_opted_out() {
        let counting = cfg!(all(debug_assertions, not(shuyi_no_heap_count)));
        assert_eq!(COUNTING, counting);
        assert_eq!(reading().is_some(), counting);
        assert_eq!(rows().is_some(), counting);
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

    /// The tag of `name`, a span's, unique to the test that asks.
    #[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
    fn span(name: &'static str) -> u16 {
        crate::tags::tag(2, name)
    }

    /// The global counter's row of `tag`, if it has one.
    #[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
    fn row(tag: u16) -> Option<TagCounts> {
        rows().unwrap().into_iter().find(|r| r.tag == tag)
    }

    /// A list made under one tag and grown under another counts under the
    /// second. Its tags are this test's, so other threads never add to
    /// them.
    #[cfg(all(debug_assertions, not(shuyi_no_heap_count)))]
    #[test]
    fn a_list_grown_under_another_tag_counts_under_it() {
        use crate::tags::{enter, leave};
        let (made, grown) = (span("global-test-made"), span("global-test-grown"));
        let before = enter(made);
        let mut list: Vec<u8> = Vec::with_capacity(1000);
        enter(grown);
        list.reserve_exact(4000);
        leave(before);
        let capacity = u64::try_from(list.capacity()).unwrap();
        let made_row = row(made).unwrap();
        assert_eq!((made_row.live_bytes, made_row.live_blocks), (0, 0));
        assert_eq!(made_row.allocated_bytes, 1000);
        assert_eq!(made_row.peak_bytes, 1000);
        let grown_row = row(grown).unwrap();
        assert_eq!((grown_row.live_bytes, grown_row.live_blocks), (capacity, 1));
        assert_eq!(grown_row.name, "global-test-grown");
        assert_eq!(grown_row.class, 2);
        drop(list);
        assert_eq!(row(grown).unwrap().live_bytes, 0);
    }
}
