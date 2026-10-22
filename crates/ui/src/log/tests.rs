//! The ring's order, capacity, dropped count and sequence numbers, the
//! filter, and the names of levels and categories.

use proptest::prelude::*;

use super::*;

fn line(n: u32) -> LogLine {
    LogLine {
        frame: n,
        level: LogLevel::Info,
        category: LogCategory::Game,
        text: format!("line {n}"),
    }
}

fn frames<'a>(lines: impl Iterator<Item = &'a LogLine>) -> Vec<u32> {
    lines.map(|l| l.frame).collect()
}

#[test]
fn levels_and_categories_read_back_from_their_names() {
    for level in LogLevel::ALL {
        assert_eq!(LogLevel::from_name(level.name()), Some(level));
    }
    for category in LogCategory::ALL {
        assert_eq!(LogCategory::from_name(category.name()), Some(category));
    }
    assert_eq!(LogLevel::Warn.name(), "warn");
    assert_eq!(LogCategory::Storage.name(), "storage");
    assert_eq!(LogLevel::from_name("Warn"), None);
    assert_eq!(LogCategory::from_name("nope"), None);
}

#[test]
fn levels_are_ordered_from_the_most_serious() {
    assert!(LogLevel::Error < LogLevel::Warn);
    assert!(LogLevel::Warn < LogLevel::Info);
    assert!(LogLevel::Info < LogLevel::Debug);
}

#[test]
fn the_ring_keeps_the_last_lines_in_order_and_counts_the_dropped() {
    let mut ring = LogRing::new(LOG_LINES);
    for n in 0..600 {
        ring.push(line(n));
    }
    assert_eq!(ring.lines().len(), 500);
    assert_eq!(ring.dropped(), 100);
    assert_eq!(frames(ring.lines()), (100..600).collect::<Vec<_>>());
    assert_eq!(ring.next_seq(), 600);
    ring.add_dropped(3);
    assert_eq!(ring.dropped(), 103);
}

#[test]
fn a_ring_of_nothing_counts_every_line_dropped() {
    let mut ring = LogRing::new(0);
    ring.push(line(1));
    assert_eq!(ring.lines().len(), 0);
    assert_eq!((ring.dropped(), ring.next_seq()), (1, 1));
    assert_eq!(ring.since(0).len(), 0);
}

#[test]
fn take_empties_the_ring_and_sequence_numbers_go_on() {
    let mut ring = LogRing::new(2);
    for n in 0..3 {
        ring.push(line(n));
    }
    let (lines, dropped) = ring.take();
    assert_eq!(frames(lines.iter()), [1, 2]);
    assert_eq!(dropped, 1);
    assert_eq!((ring.lines().len(), ring.dropped()), (0, 0));
    assert_eq!(ring.next_seq(), 3);
    ring.push(line(3));
    assert_eq!(frames(ring.since(3)), [3]);
    assert_eq!(ring.since(4).len(), 0);
}

#[test]
fn since_gives_the_lines_after_a_seq_once_the_ring_has_wrapped() {
    let mut ring = LogRing::new(3);
    for n in 0..10 {
        ring.push(line(n));
    }
    // Kept: 7, 8, 9 (sequence numbers 7 to 9).
    assert_eq!(frames(ring.since(0)), [7, 8, 9]);
    assert_eq!(frames(ring.since(8)), [8, 9]);
    assert_eq!(frames(ring.since(10)), Vec::<u32>::new());
    assert_eq!(frames(ring.since(u64::MAX)), Vec::<u32>::new());
}

#[test]
fn the_filter_passes_a_level_and_more_serious_per_category() {
    let filter = LogFilter::default();
    assert!(filter.allows(LogCategory::Audio, LogLevel::Info));
    assert!(filter.allows(LogCategory::Audio, LogLevel::Error));
    assert!(!filter.allows(LogCategory::Audio, LogLevel::Debug));
    let filter = LogFilter::new(LogLevel::Warn)
        .with(LogCategory::Input, LogLevel::Info)
        .with(LogCategory::Input, LogLevel::Debug);
    assert!(!filter.allows(LogCategory::Audio, LogLevel::Info));
    assert!(filter.allows(LogCategory::Audio, LogLevel::Warn));
    assert!(
        filter.allows(LogCategory::Input, LogLevel::Debug),
        "the last says"
    );
}

/// What a test does to a ring: push a line, or read what is new.
#[derive(Debug, Clone)]
enum Op {
    Push,
    Read,
}

proptest! {
    #[test]
    fn a_reader_that_keeps_next_seq_gets_every_kept_line_once_in_order(
        capacity in 1usize..6,
        ops in prop::collection::vec(prop_oneof![Just(Op::Push), Just(Op::Read)], 0..60),
    ) {
        let mut ring = LogRing::new(capacity);
        let mut pushed = 0u32;
        let mut read: Vec<u32> = Vec::new();
        let mut seq = 0;
        for op in ops.iter().chain([Op::Read].iter()) {
            match op {
                Op::Push => {
                    ring.push(line(pushed));
                    pushed += 1;
                }
                Op::Read => {
                    let new = frames(ring.since(seq));
                    // Whatever is new is still kept, and newer than all read.
                    prop_assert!(new.windows(2).all(|w| w[0] < w[1]));
                    prop_assert!(read.last().is_none_or(|last| new.first().is_none_or(|f| f > last)));
                    // Nothing kept and pushed after the last read is missed.
                    let kept_new: Vec<u32> = frames(ring.lines())
                        .into_iter()
                        .filter(|f| u64::from(*f) >= seq)
                        .collect();
                    prop_assert_eq!(&new, &kept_new);
                    read.extend(new);
                    seq = ring.next_seq();
                }
            }
        }
        // Each line was read at most once, in the order pushed.
        prop_assert!(read.windows(2).all(|w| w[0] < w[1]));
        prop_assert_eq!(ring.dropped(), pushed.saturating_sub(u32::try_from(capacity).unwrap_or(0)));
    }
}
