//! The heap counter's rows as a [`HeapSnapshot`] (ADR-0046 §6), with the
//! `heap` feature: the one conversion a backend that counts the heap
//! makes in its [`Platform::heap_snapshot`](crate::Platform::heap_snapshot),
//! so `shuyi-app` and a game's tool program (`shuyi-cli`) give the same
//! snapshot of the same counts. A backend reads its counter
//! (`shuyi_alloc_count::global::reading` and `rows`, and
//! `shuyi_alloc_count::tags::overflowed`) and hands the counts here.

use crate::alloc_prelude::*;
use alloc::borrow::Cow;

use shuyi_alloc_count::{Counts, TagCounts};
use shuyi_ui::perf::{HeapSnapshot, TagClass, TagRow};

/// The snapshot of the heap counter's `counts` and its `rows` by tag, with
/// `overflowed` names past its table (`tags::overflowed`), at frame 0: a
/// row each, its class read from its number (an unknown one as
/// [`TagClass::Other`]), in the snapshot's order.
pub fn snapshot(counts: Counts, rows: &[TagCounts], overflowed: u32) -> HeapSnapshot {
    let rows: Vec<TagRow> = rows.iter().map(row).collect();
    let mut snapshot = HeapSnapshot::new(counts.live_bytes, counts.peak_bytes, rows);
    snapshot.overflowed = overflowed;
    snapshot
}

/// The counter's row `counts` as a snapshot's.
fn row(counts: &TagCounts) -> TagRow {
    let class = TagClass::from_u8(counts.class).unwrap_or(TagClass::Other);
    let mut row = TagRow::new(
        class,
        Cow::Borrowed(counts.name),
        counts.live_bytes,
        counts.live_blocks,
    );
    row.allocated_bytes = counts.allocated_bytes;
    row.peak_bytes = counts.peak_bytes;
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The counter's row of tag `tag`, `name` of `class`, with `live`
    /// bytes in `blocks`, `allocated` ever and `peak` at the most.
    fn counts(tag: u16, class: u8, name: &'static str, live: u64, blocks: u32) -> TagCounts {
        let mut row = TagCounts::new(tag, class, name);
        row.live_bytes = live;
        row.live_blocks = blocks;
        row.allocated_bytes = live * 3;
        row.peak_bytes = live * 2;
        row
    }

    #[test]
    fn the_counters_rows_become_the_snapshots_in_its_order() {
        let rows = [
            counts(0, 0, "other", 100, 1),
            counts(1, 1, "update", 300, 2),
            counts(2, 3, "image", 500, 4),
            counts(3, 2, "ai", 0, 0),
            counts(4, 9, "strange", 200, 1),
        ];
        let snapshot = snapshot(Counts::new(1100, 4000, 7), &rows, 2);
        assert_eq!(
            (snapshot.frame, snapshot.live_bytes, snapshot.peak_bytes),
            (0, 1100, 4000)
        );
        assert_eq!(snapshot.overflowed, 2);
        let read: Vec<(TagClass, &str, u64, u32)> = snapshot
            .tags
            .iter()
            .map(|r| (r.class, &*r.name, r.live_bytes, r.live_blocks))
            .collect();
        assert_eq!(
            read,
            [
                (TagClass::Asset, "image", 500, 4),
                (TagClass::Phase, "update", 300, 2),
                (TagClass::Other, "strange", 200, 1),
                (TagClass::Other, "other", 100, 1),
                (TagClass::Span, "ai", 0, 0),
            ]
        );
        let image = &snapshot.tags[0];
        assert_eq!((image.allocated_bytes, image.peak_bytes), (1500, 1000));
    }

    #[test]
    fn no_rows_is_a_snapshot_of_the_totals_alone() {
        let snapshot = snapshot(Counts::new(10, 20, 0), &[], 0);
        assert_eq!(snapshot, HeapSnapshot::new(10, 20, Vec::new()));
    }
}
