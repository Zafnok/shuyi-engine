//! The fake clock, the ring's order, capacity, dropped count and sequence
//! numbers, and the context's spans: their depth, their order and the cap.

use proptest::prelude::*;

use super::*;
use crate::toy;

fn record(n: u32) -> FrameRecord {
    FrameRecord::new(n)
}

fn frames<'a>(records: impl Iterator<Item = &'a FrameRecord>) -> Vec<u32> {
    records.map(|r| r.frame).collect()
}

#[test]
fn the_fake_clock_adds_a_hundred_times_its_read_mod_seven_plus_one() {
    let mut clock = PerfClock::fake();
    let times: Vec<u64> = (0..9).map(|_| clock.read()).collect();
    assert_eq!(times, [100, 300, 600, 1000, 1500, 2100, 2800, 2900, 3100]);
    let PerfClock::Fake(fake) = clock else {
        panic!("a fake clock")
    };
    assert_eq!(fake.reads(), 9);
    assert_eq!(FakeClock::new().reads(), 0);
}

#[test]
fn a_real_clock_reads_its_function() {
    let mut clock = PerfClock::Real(|| 42);
    assert_eq!(clock.read(), 42);
}

#[test]
fn the_ring_keeps_the_last_records_in_order_and_counts_the_dropped() {
    let mut ring = PerfRing::new(PERF_FRAMES);
    for n in 0..700 {
        ring.push(record(n));
    }
    assert_eq!(ring.records().len(), 600);
    assert_eq!(ring.dropped(), 100);
    assert_eq!(frames(ring.records()), (100..700).collect::<Vec<_>>());
    assert_eq!(ring.next_seq(), 700);
}

#[test]
fn a_ring_of_nothing_counts_every_record_dropped() {
    let mut ring = PerfRing::new(0);
    ring.push(record(1));
    assert_eq!(ring.records().len(), 0);
    assert_eq!((ring.dropped(), ring.next_seq()), (1, 1));
    assert_eq!(ring.since(0).len(), 0);
}

#[test]
fn since_gives_the_records_after_a_seq_once_the_ring_has_wrapped() {
    let mut ring = PerfRing::new(3);
    for n in 0..10 {
        ring.push(record(n));
    }
    // Kept: 7, 8, 9 (sequence numbers 7 to 9).
    assert_eq!(frames(ring.since(0)), [7, 8, 9]);
    assert_eq!(frames(ring.since(8)), [8, 9]);
    assert_eq!(frames(ring.since(10)), Vec::<u32>::new());
    assert_eq!(frames(ring.since(u64::MAX)), Vec::<u32>::new());
}

#[test]
fn a_record_reads_back_from_its_text() {
    let mut record = FrameRecord::new(3);
    record.times.frame_us = 16_000;
    record.spans.push(SpanEvent::new("ai", 10, 20, 1));
    record.spans_dropped = 2;
    let text = ron::to_string(&record).expect("a record writes");
    let back: FrameRecord = ron::from_str(&text).expect("and reads back");
    assert_eq!(back, record);
    assert!(ron::from_str::<FrameTimes>("(update_us: 1, nope: 2)").is_err());
}

#[test]
fn a_span_with_no_clock_calls_its_closure_once_and_records_nothing() {
    let mut ctx = toy::ctx();
    let mut calls = 0;
    assert_eq!(ctx.begin_perf_frame(), None);
    let got = ctx.span("ai", |_| {
        calls += 1;
        7
    });
    assert_eq!((got, calls), (7, 1));
    assert_eq!(ctx.end_perf_frame(), None);
}

#[test]
fn a_span_outside_a_measured_frame_reads_nothing() {
    let mut ctx = toy::ctx();
    ctx.set_perf_clock(Some(PerfClock::fake()));
    ctx.span("ai", |_| ());
    let Some(PerfClock::Fake(fake)) = ctx.perf_clock() else {
        panic!("the fake clock")
    };
    assert_eq!(fake.reads(), 0);
}

#[test]
fn nested_spans_have_depths_and_end_inner_first() {
    let mut ctx = toy::ctx();
    ctx.set_perf_clock(Some(PerfClock::fake()));
    // Reads: 100 the frame, 300 outer, 600 inner, 1000 inner's end, 1500
    // outer's end.
    assert_eq!(ctx.begin_perf_frame(), Some(100));
    ctx.span("outer", |ctx| ctx.span("inner", |_| ()));
    let record = ctx.end_perf_frame().expect("a measured frame");
    assert_eq!(
        record.spans,
        [
            SpanEvent::new("inner", 500, 400, 1),
            SpanEvent::new("outer", 200, 1200, 0),
        ]
    );
    assert_eq!(record.spans_dropped, 0);
    assert_eq!(ctx.end_perf_frame(), None, "the frame has ended");
}

#[test]
fn spans_past_the_cap_are_counted_not_kept() {
    let mut ctx = toy::ctx();
    ctx.set_perf_clock(Some(PerfClock::fake()));
    ctx.begin_perf_frame();
    for _ in 0..=MAX_SPANS {
        ctx.span("step", |_| ());
    }
    let record = ctx.end_perf_frame().expect("a measured frame");
    assert_eq!(record.spans.len(), MAX_SPANS);
    assert_eq!(record.spans_dropped, 1);
    // The next frame starts again.
    ctx.begin_perf_frame();
    ctx.span("step", |_| ());
    let record = ctx.end_perf_frame().expect("a measured frame");
    assert_eq!((record.spans.len(), record.spans_dropped), (1, 0));
}

#[test]
fn taking_the_clock_away_mid_frame_ends_it_with_nothing() {
    let mut ctx = toy::ctx();
    ctx.set_perf_clock(Some(PerfClock::fake()));
    ctx.begin_perf_frame();
    ctx.span("ai", |ctx| ctx.set_perf_clock(None));
    assert_eq!(ctx.end_perf_frame(), None);
}

/// What a test does to a ring: push a record, or read what is new.
#[derive(Debug, Clone)]
enum Op {
    Push,
    Read,
}

proptest! {
    #[test]
    fn a_reader_that_keeps_next_seq_gets_every_kept_record_once_in_order(
        capacity in 1usize..6,
        ops in prop::collection::vec(prop_oneof![Just(Op::Push), Just(Op::Read)], 0..60),
    ) {
        let mut ring = PerfRing::new(capacity);
        let mut pushed = 0u32;
        let mut read: Vec<u32> = Vec::new();
        let mut seq = 0;
        for op in ops.iter().chain([Op::Read].iter()) {
            match op {
                Op::Push => {
                    ring.push(record(pushed));
                    pushed += 1;
                }
                Op::Read => {
                    let new = frames(ring.since(seq));
                    let kept_new: Vec<u32> = frames(ring.records())
                        .into_iter()
                        .filter(|f| u64::from(*f) >= seq)
                        .collect();
                    prop_assert_eq!(&new, &kept_new);
                    read.extend(new);
                    seq = ring.next_seq();
                }
            }
        }
        prop_assert!(read.windows(2).all(|w| w[0] < w[1]));
        let capacity = u64::try_from(capacity).unwrap_or(0);
        prop_assert_eq!(ring.dropped(), u64::from(pushed).saturating_sub(capacity));
    }
}

#[test]
fn spread_of_no_values_is_none() {
    assert_eq!(spread([]), None);
}

#[test]
fn spread_of_one_value_is_that_value() {
    let one = Spread {
        p50: 7,
        p99: 7,
        worst: 7,
        mean: 7,
    };
    assert_eq!(spread([7]), Some(one));
}

#[test]
fn spread_of_two_values_takes_the_nearest_rank() {
    // Rank ⌈50 × 2 / 100⌉ = 1 and ⌈99 × 2 / 100⌉ = 2; the mean 4.5 rounds down.
    let two = Spread {
        p50: 3,
        p99: 6,
        worst: 6,
        mean: 4,
    };
    assert_eq!(spread([6, 3]), Some(two));
}

#[test]
fn spread_of_a_hundred_values_ranks_them_sorted() {
    // 1 to 100, shuffled: rank 50 is 50, rank 99 is 99, the mean 50.5.
    let values = (1..=100u32).map(|v| (v * 37) % 100 + 1);
    let hundred = Spread {
        p50: 50,
        p99: 99,
        worst: 100,
        mean: 50,
    };
    assert_eq!(spread(values), Some(hundred));
}

#[test]
fn spread_of_equal_values_is_that_value_everywhere() {
    let same = Spread {
        p50: 400,
        p99: 400,
        worst: 400,
        mean: 400,
    };
    assert_eq!(spread([400; 9]), Some(same));
}

#[test]
fn bytes_read_in_the_largest_unit_with_a_tenth_rounded_down() {
    let texts: Vec<String> = [
        0,
        1023,
        1024,
        1024 * 1024 - 1,
        3_355_444,
        1 << 30,
        5 << 40,
        u64::MAX,
    ]
    .into_iter()
    .map(bytes_text)
    .collect();
    assert_eq!(
        texts,
        [
            "0 B",
            "1023 B",
            "1.0 KB",
            "1023.9 KB",
            "3.2 MB",
            "1.0 GB",
            "5120.0 GB",
            "17179869183.9 GB",
        ]
    );
}

#[test]
fn a_new_record_has_no_heap_counts_and_a_reading_holds_its_own() {
    assert_eq!(FrameRecord::new(3).heap, None);
    let heap = HeapCounts::new(1, 2, 3);
    assert_eq!((heap.live_bytes, heap.peak_bytes, heap.allocs), (1, 2, 3));
    assert_eq!(MemoryReading::new(heap).heap, heap);
}

#[test]
fn a_tag_class_is_its_number_and_back() {
    let classes = [
        TagClass::Other,
        TagClass::Phase,
        TagClass::Span,
        TagClass::Asset,
    ];
    for (n, class) in (0u8..).zip(classes) {
        assert_eq!(class as u8, n);
        assert_eq!(TagClass::from_u8(n), Some(class));
    }
    assert_eq!(TagClass::from_u8(4), None);
    assert_eq!(TagClass::from_u8(u8::MAX), None);
}

#[test]
fn a_tag_class_writes_as_a_word() {
    let text = ron::to_string(&vec![TagClass::Phase, TagClass::Asset]).expect("classes write");
    assert_eq!(text, "[phase,asset]");
    let back: Vec<TagClass> = ron::from_str(&text).expect("and read back");
    assert_eq!(back, [TagClass::Phase, TagClass::Asset]);
}

/// A snapshot of `frame` with `rows`, its live bytes their sum.
fn snapshot(frame: u32, rows: Vec<TagRow>) -> HeapSnapshot {
    let live = rows.iter().map(|r| r.live_bytes).sum();
    let mut snapshot = HeapSnapshot::new(live, live, rows);
    snapshot.frame = frame;
    snapshot
}

/// The rows' names, in order.
fn names(rows: &[TagRow]) -> Vec<&str> {
    rows.iter().map(|r| &*r.name).collect()
}

#[test]
fn a_snapshots_rows_go_by_live_bytes_then_class_then_name() {
    let snapshot = snapshot(
        1,
        vec![
            TagRow::new(TagClass::Other, "other", 100, 1),
            TagRow::new(TagClass::Span, "draw", 300, 2),
            TagRow::new(TagClass::Phase, "update", 300, 1),
            TagRow::new(TagClass::Phase, "draw", 300, 1),
            TagRow::new(TagClass::Asset, "image", 500, 3),
            TagRow::new(TagClass::Phase, "load", 0, 0),
        ],
    );
    assert_eq!(
        names(&snapshot.tags),
        ["image", "draw", "update", "draw", "other", "load"]
    );
    assert_eq!(snapshot.tags[1].class, TagClass::Phase);
    assert_eq!(snapshot.tags[3].class, TagClass::Span);
    assert_eq!((snapshot.live_bytes, snapshot.overflowed), (1500, 0));
    let row = &snapshot.tags[0];
    assert_eq!((row.allocated_bytes, row.peak_bytes), (500, 500));
}

#[test]
fn a_diff_counts_a_tag_missing_from_one_snapshot_as_nothing_there() {
    let earlier = snapshot(
        1,
        vec![
            TagRow::new(TagClass::Phase, "load", 4096, 4),
            TagRow::new(TagClass::Phase, "update", 1024, 1),
            TagRow::new(TagClass::Other, "other", 512, 2),
        ],
    );
    let later = snapshot(
        600,
        vec![
            TagRow::new(TagClass::Phase, "load", 4096, 4),
            TagRow::new(TagClass::Phase, "update", 99_328, 7),
            TagRow::new(TagClass::Span, "ai", 4096, 1),
            TagRow::new(TagClass::Asset, "image", 24_576, 3),
        ],
    );
    let diff = earlier.diff(&later);
    assert_eq!((diff.from, diff.to), (1, 600));
    assert_eq!(diff.live_bytes, 132_096 - 5632);
    assert_eq!(
        diff.tags,
        [
            // Only in both, grown.
            TagDelta::new(TagClass::Phase, "update", 98_304, 6),
            // Only in the later one.
            TagDelta::new(TagClass::Asset, "image", 24_576, 3),
            TagDelta::new(TagClass::Span, "ai", 4096, 1),
            // Only in the earlier one. `load` is unchanged: left out.
            TagDelta::new(TagClass::Other, "other", -512, -2),
        ]
    );
    let back = later.diff(&earlier);
    assert_eq!(
        (back.from, back.to, back.live_bytes),
        (600, 1, 5632 - 132_096)
    );
    assert_eq!(
        back.tags[0],
        TagDelta::new(TagClass::Phase, "update", -98_304, -6)
    );
}

#[test]
fn a_diff_goes_by_the_size_of_the_change_then_class_then_name() {
    let earlier = snapshot(
        0,
        vec![
            TagRow::new(TagClass::Phase, "update", 10_000, 10),
            TagRow::new(TagClass::Span, "b", 100, 1),
        ],
    );
    let later = snapshot(
        30,
        vec![
            TagRow::new(TagClass::Phase, "update", 0, 0),
            TagRow::new(TagClass::Span, "b", 100, 2),
            TagRow::new(TagClass::Span, "a", 10_000, 1),
            TagRow::new(TagClass::Asset, "image", 20_000, 1),
        ],
    );
    let diff = earlier.diff(&later);
    let order: Vec<(&str, i64)> = diff.tags.iter().map(|t| (&*t.name, t.live_bytes)).collect();
    // A shrink as large as a growth sorts with it, by class then name; a
    // tag whose blocks alone changed is kept, last.
    assert_eq!(
        order,
        [
            ("image", 20_000),
            ("update", -10_000),
            ("a", 10_000),
            ("b", 0)
        ]
    );
    assert_eq!(diff.tags[3].live_blocks, 1);
    assert_eq!(earlier.diff(&earlier), HeapDiff::new(0, 0));
}

#[test]
fn a_change_past_i64_is_at_its_end() {
    let (small, huge) = (
        snapshot(0, Vec::new()),
        HeapSnapshot::new(u64::MAX, 0, Vec::new()),
    );
    assert_eq!(small.diff(&huge).live_bytes, i64::MAX);
    assert_eq!(huge.diff(&small).live_bytes, i64::MIN);
}

#[test]
fn snapshot_and_diff_lines_read_back_from_their_text() {
    let earlier = snapshot(1, vec![TagRow::new(TagClass::Asset, "image", 10, 1)]);
    let mut later = snapshot(2, vec![TagRow::new(TagClass::Span, "ai", 30, 2)]);
    later.overflowed = 3;
    let lines = [
        PerfLine::Snapshot(later.clone()),
        PerfLine::Diff(earlier.diff(&later)),
    ];
    for line in lines {
        let text = ron::to_string(&line).expect("a line writes");
        let back: PerfLine = ron::from_str(&text).expect("and reads back");
        assert_eq!(back, line);
    }
}
