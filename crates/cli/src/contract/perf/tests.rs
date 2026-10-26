//! Reads of a ring, the summary's spreads and spans, the trace's lines and
//! the text's milliseconds.

use shuyi_ui::perf::{HeapCounts, SpanEvent};

use super::*;

/// Frame `frame`'s record: the frame took `us`, its update half of it, and
/// it ended `spans` (a name and a length each).
fn record(frame: u32, us: u32, spans: &[(&'static str, u32)]) -> FrameRecord {
    let mut record = FrameRecord::new(frame);
    record.times.frame_us = us;
    record.times.update_us = us / 2;
    record.times.draw_us = us / 4;
    record.times.paint_us = us / 8;
    record.times.present_us = us / 16;
    record.spans = spans
        .iter()
        .map(|&(name, us)| SpanEvent::new(name, 0, us, 0))
        .collect();
    record
}

fn ring(capacity: usize, pushed: u32) -> PerfRing {
    let mut ring = PerfRing::new(capacity);
    for frame in 1..=pushed {
        ring.push(record(frame, 1000, &[]));
    }
    ring
}

fn frames(read: &PerfRead) -> Vec<u32> {
    read.records.iter().map(|r| r.frame).collect()
}

#[test]
fn a_read_answers_the_records_since_and_the_number_after_the_last() {
    let ring = ring(10, 6);
    let all = PerfRead::of(&ring, 0, 120);
    assert_eq!(frames(&all), [1, 2, 3, 4, 5, 6]);
    assert_eq!((all.next, all.dropped, all.measured), (6, 0, true));
    let some = PerfRead::of(&ring, 2, 3);
    assert_eq!(frames(&some), [3, 4, 5]);
    assert_eq!(some.next, 5);
    let none = PerfRead::of(&ring, 6, 120);
    assert!(none.records.is_empty(), "{none:?}");
    assert_eq!(none.next, 6);
}

#[test]
fn a_read_past_a_full_ring_counts_what_it_dropped() {
    // Ten kept of twenty-five: numbers 15 to 24.
    let ring = ring(10, 25);
    let read = PerfRead::of(&ring, 0, 120);
    assert_eq!(frames(&read), (16..=25).collect::<Vec<_>>());
    assert_eq!((read.next, read.dropped), (25, 15));
    let later = PerfRead::of(&ring, 12, 4);
    assert_eq!(frames(&later), [16, 17, 18, 19]);
    assert_eq!((later.next, later.dropped), (19, 3));
    let kept = PerfRead::of(&ring, 20, 120);
    assert_eq!(kept.dropped, 0);
}

#[test]
fn a_read_past_the_rings_next_number_reads_nothing_from_0() {
    let read = PerfRead::of(&ring(10, 3), 8, 120);
    assert!(read.records.is_empty(), "{read:?}");
    assert_eq!((read.next, read.dropped, read.measured), (0, 0, true));
}

#[test]
fn a_ring_never_pushed_measured_nothing() {
    let read = PerfRead::of(&PerfRing::new(10), 0, 120);
    assert!(!read.measured);
    assert_eq!(read.next, 0);
    assert_eq!(PerfRead::none(4), {
        let mut none = PerfRead::of(&PerfRing::new(1), 0, 1);
        none.next = 4;
        none
    });
}

#[test]
fn a_summary_spreads_each_part_of_the_frame() {
    let records: Vec<FrameRecord> = (1..=4).map(|f| record(f, f * 1600, &[])).collect();
    let summary = summary(&records).unwrap();
    assert_eq!((summary.frames, summary.dropped), (4, 0));
    assert_eq!(
        summary.frame,
        Spread {
            p50: 3200,
            p99: 6400,
            worst: 6400,
            mean: 4000
        }
    );
    assert_eq!(summary.update.worst, 3200);
    assert_eq!(summary.draw.p50, 800);
    assert_eq!(summary.paint.mean, 500);
    assert_eq!(summary.present.p99, 400);
    assert!(summary.spans.is_empty(), "{:?}", summary.spans);
    assert_eq!(summary.file, None);
}

#[test]
fn a_summary_of_no_records_is_none() {
    assert_eq!(summary(&[]), None);
}

#[test]
fn a_summary_names_the_five_spans_with_the_most_time_ties_by_name() {
    let records = [
        record(1, 1000, &[("ai", 300), ("path", 50), ("e", 10), ("d", 10)]),
        record(2, 1000, &[("ai", 500), ("path", 50), ("c", 10), ("b", 10)]),
        record(3, 1000, &[("a", 10)]),
    ];
    let summary = summary(&records).unwrap();
    let named: Vec<(&str, u32, u64, u32, u32)> = summary
        .spans
        .iter()
        .map(|s| (s.name.as_str(), s.count, s.total_us, s.mean_us, s.worst_us))
        .collect();
    assert_eq!(
        named,
        [
            ("ai", 2, 800, 400, 500),
            ("path", 2, 100, 50, 50),
            ("a", 1, 10, 10, 10),
            ("b", 1, 10, 10, 10),
            ("c", 1, 10, 10, 10),
        ]
    );
}

#[test]
fn a_traces_lines_parse_back_to_its_records() {
    let records = [record(1, 1000, &[("ai", 300)]), record(2, 900, &[])];
    let text = lines(&records);
    assert!(text.ends_with('\n'));
    let parsed: Vec<PerfLine> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let back: Vec<FrameRecord> = parsed
        .into_iter()
        .map(|line| match line {
            PerfLine::Frame(record) => record,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(back, records);
    assert!(text.starts_with(r#"{"kind":"frame","frame":1,"times":{"#));
}

#[test]
fn a_line_with_a_field_it_doesnt_know_is_refused() {
    let line = r#"{"kind":"frame","frame":1,"times":{"update_us":0,"draw_us":0,"paint_us":0,"present_us":0,"frame_us":0},"spans":[],"spans_dropped":0,"gpu":1}"#;
    assert!(serde_json::from_str::<PerfLine>(line).is_err());
    let known = line.replace(r#","gpu":1"#, "");
    assert!(serde_json::from_str::<PerfLine>(&known).is_ok());
}

#[test]
fn a_line_has_its_heap_only_where_it_was_counted() {
    let mut counted = record(2, 900, &[]);
    counted.heap = Some(HeapCounts::new(2048, 4096, 5));
    let text = lines(&[record(1, 1000, &[]), counted.clone()]);
    let (plain, heap) = text.trim_end().split_once('\n').unwrap();
    assert!(!plain.contains("heap"), "{plain}");
    assert!(
        heap.ends_with(r#""heap":{"live_bytes":2048,"peak_bytes":4096,"allocs":5}}"#),
        "{heap}"
    );
    let back: PerfLine = serde_json::from_str(heap).unwrap();
    assert_eq!(back, PerfLine::Frame(counted));
}

/// Frame `frame`'s record with the heap's counts.
fn counted(frame: u32, live: u64, peak: u64, allocs: u32) -> FrameRecord {
    let mut record = record(frame, 1000, &[]);
    record.heap = Some(HeapCounts::new(live, peak, allocs));
    record
}

#[test]
fn the_heaps_summary_is_its_peak_its_last_live_bytes_and_the_allocations_spread() {
    let records = [
        counted(1, 3000, 5000, 4),
        counted(2, 2000, 9000, 0),
        record(3, 1000, &[]),
        counted(4, 2500, 7000, 2),
    ];
    let heap = summary(&records).unwrap().heap.unwrap();
    assert_eq!((heap.peak_bytes, heap.live_bytes), (9000, 2500));
    assert_eq!(
        heap.allocs,
        Spread {
            p50: 2,
            p99: 4,
            worst: 4,
            mean: 2
        }
    );
    let uncounted = [record(1, 1000, &[])];
    assert_eq!(summary(&uncounted).unwrap().heap, None);
}

#[test]
fn the_heap_limits_say_it_wasnt_counted_or_counted_other_threads() {
    let uncounted = [record(1, 1000, &[])];
    let counted = [counted(1, 1, 1, 1)];
    let none: Vec<String> = Vec::new();
    assert_eq!(heap_limits(&[], true, true), none);
    assert_eq!(heap_limits(&uncounted, true, true), [HEAP_UNCOUNTED]);
    assert_eq!(heap_limits(&counted, true, true), [HEAP_THREADS]);
    assert_eq!(heap_limits(&counted, true, false), none);
    assert_eq!(heap_limits(&counted, false, true), none, "a fake's");
    assert!(HEAP_UNCOUNTED.contains("release build, or opted out"));
}

#[test]
fn milliseconds_have_one_decimal_rounded_to_the_nearest_tenth() {
    assert_eq!(millis(0), "0.0");
    assert_eq!(millis(49), "0.0");
    assert_eq!(millis(50), "0.1");
    assert_eq!(millis(1249), "1.2");
    assert_eq!(millis(1250), "1.3");
    assert_eq!(millis(8000), "8.0");
    assert_eq!(millis(123_456), "123.5");
    assert_eq!(millis(u32::MAX), "4294967.3");
}

#[test]
fn the_text_is_one_line_then_the_trace_file() {
    let records = [
        record(1, 1200, &[("ai", 800)]),
        record(2, 3400, &[("ai", 800)]),
    ];
    let mut summary = summary(&records).unwrap();
    assert_eq!(
        text(&summary),
        "perf: 2 frames, frame p50 1.2 ms p99 3.4 ms worst 3.4 ms; span ai 0.8 ms mean\n"
    );
    summary.heap = heap_summary(&[counted(1, 1 << 20, 4_299_162, 3)]);
    assert_eq!(
        text(&summary),
        "perf: 2 frames, frame p50 1.2 ms p99 3.4 ms worst 3.4 ms; span ai 0.8 ms mean; heap \
         peak 4.1 MB\n"
    );
    summary.heap = None;
    summary.spans.clear();
    summary.file = Some("/game/perf.jsonl".to_owned());
    assert_eq!(
        text(&summary),
        "perf: 2 frames, frame p50 1.2 ms p99 3.4 ms worst 3.4 ms\ntrace: /game/perf.jsonl\n"
    );
}

#[test]
fn the_limits_say_why_nothing_was_measured_and_what_was_dropped() {
    assert_eq!(
        dropped_limit(100),
        "the trace keeps the last 600 frames: 100 earlier frame(s) were dropped"
    );
    assert_eq!(unmeasured_limit(0), "nothing was measured: no frame ran");
    assert!(unmeasured_limit(3).contains("without debug tools"));
}
