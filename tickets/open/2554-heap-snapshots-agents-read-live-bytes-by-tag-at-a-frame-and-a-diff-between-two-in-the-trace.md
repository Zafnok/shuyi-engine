---
id: "2554"
title: "Heap snapshots agents read: live bytes by tag at a frame, and a diff between two, in run --perf, the MCP perf tool and a running build's trace file"
type: feature
milestone: MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2553", "3065"]
nick_input: none
completed:
---

# 2554 — Heap snapshots agents read

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md),
"perf traces agents can read"):

> "we should also have easily agent-readable / streamable perf traces
> when building in debug -- both timings and heap snapshots"

He put them in the MVP. What the tickets before this one give:

- **3063:** a `FrameRecord` per frame in `App`, and the ADR, which
  defines a heap snapshot: live bytes and counts by memory tag, and a
  diff that compares two snapshots;
- **2553:** the trace agents read, as JSON lines of `PerfLine` (one
  variant, `Frame`): `shuyi run --perf` and `--perf-out`, MCP's `perf`
  tool read from a number, and `SHUYI_PERF=<file>` in a running desktop
  debug build. `PerfSummary` sums it up;
- **3064:** the heap counter (`shuyi-alloc-count`, the global allocator
  of every debug build of a game on `shuyi-app` and of its tool
  program), `Platform::memory`, and each record's heap counts;
- **3065:** memory tags. The counter's `rows()` gives each tag (class,
  name) with its live bytes, live blocks, bytes ever allocated and peak;
  tags are the frame's phases (`load`, `update`, `draw`, `paint`, `app`,
  `perf`), the game's spans, and each asset kind's decode; `other` is
  outside any.

This ticket turns the tags into snapshots and diffs, and puts them in
the trace, so an agent sees what holds the heap and what grew: "update
+96.0 KB, image +24.0 KB between frame 1 and frame 600".

## Nick input

None.

## Scope

**In:**
- **The types, in `shuyi_ui::perf`** (serde, and `JsonSchema` under the
  `schema` feature, `#[non_exhaustive]` with constructors, ADR-0039):
  - `TagRow { class: TagClass, name: Cow<'static, str>, live_bytes: u64,
    live_blocks: u32, allocated_bytes: u64, peak_bytes: u64 }`;
  - `HeapSnapshot { frame: u32, live_bytes: u64, peak_bytes: u64, tags:
    Vec<TagRow>, overflowed: u32 }`: the rows sorted by live bytes (most
    first), then class, then name; `overflowed` the names counted under
    `overflow` (3065);
  - `TagDelta { class, name, live_bytes: i64, live_blocks: i64 }` and
    `HeapDiff { from: u32, to: u32, live_bytes: i64, tags:
    Vec<TagDelta> }`: the tags whose live bytes or blocks changed, a tag
    missing from one snapshot counting as 0 there, sorted by the size of
    the change (most first), then class, then name;
  - `HeapSnapshot::diff(&self, later: &HeapSnapshot) -> HeapDiff`;
  - `PerfLine` gains `Snapshot(HeapSnapshot)` and `Diff(HeapDiff)`.
- **Taking one:**
  - `Platform::heap_snapshot(&mut self) -> Option<HeapSnapshot>`,
    defaulted to `None` (its `frame` left 0), and
    `App::heap_snapshot(&self, platform: &mut impl Platform) ->
    Option<HeapSnapshot>`, which stamps `App::frame_number()`, only
    while debug tools are on; `Harness::heap_snapshot()`;
  - `shuyi_platform::heap`, behind a new `heap` feature with an
    optional dependency on `shuyi-alloc-count` (without `global`; a new
    edge in `tools/layers.ron`): `snapshot(counts, rows) -> HeapSnapshot`,
    the one conversion from the counter's rows, so `shuyi-app` and
    `shuyi-cli` share it. Both turn the feature on, and give
    `heap_snapshot` from the global static when `global::reading()` is
    `Some`;
  - `HeadlessPlatform::with_fake_memory()` (3064) also gives
    snapshots. With *n* the frames it has presented, the rows are
    `Asset`/`image` 49,152 bytes in 3 blocks; `Phase`/`load` 524,288 in
    40; `Phase`/`update` 16,384 × (*n* mod 30) in *n* mod 30;
    `Other`/`other` 475,136 in 12. Their sum is the fake's live bytes on
    the same frame. Each row's peak is the most of its live bytes so
    far, and its bytes ever allocated equal its peak.
- **`run --perf`** (2553):
  - the session (`Session::heap_snapshot(&mut self) ->
    Option<HeapSnapshot>`, by default `None`; `Headless` over the
    Harness) takes a snapshot once the game has started, before the
    script, and one after the last frame;
  - with `--perf-out`, the file ends with the two `snapshot` lines and
    their `diff`;
  - `PerfSummary` gains `heap_tags`, the last snapshot's first 10 rows,
    and `heap_growth`, the diff's first 10 rows;
  - `run::text` adds a line: `heap grew 120.0 KB from frame 0 to 600:
    update +96.0 KB, image +24.0 KB, ai +4.0 KB` (the three largest
    changes, with 3064's `bytes_text`), or `heap shrank …`, or nothing
    when nothing changed;
  - without a counter there is no snapshot, and 3064's limit says why.
- **MCP's `perf` tool** (2553):
  - input `snapshot: bool`: take one now. The session keeps the last 8
    by frame, and the answer lists their frames;
  - input `diff_from: Option<u32>`: the frame of a kept snapshot. The
    answer has the diff from it to the snapshot taken now, or to the
    newest kept one when `snapshot` is false. A frame that isn't kept
    is wrong arguments (2509's code), and the error lists the kept
    frames;
  - the answer gains `snapshot`, `diff` and `kept`.
- **The trace file of a running build** (2553's `SHUYI_PERF`): a
  snapshot line on its first frame, then every 600 frames, each after
  the first followed by its diff from the one before, so a file shows
  what grows as a person plays.
- **The contract's version** raised by one, with its line in
  `versions.json`, and the schemas regenerated.
- **Docs** (the `document` skill): `docs/contract.md`'s *Perf traces*:
  what a snapshot and a diff hold, the tags' classes and the phases'
  names, `other` and `overflow`, that a tag is where a block was
  allocated or last grown, and how to find what grows (two snapshots
  and a diff, from `run` or MCP).
- **Either order with 2555:** if 2555 is done, the link's `Perf`
  request gains `snapshot: bool`: the backend takes it after the frame
  (as the link's screenshot is taken after `present`) and answers it
  with the records; `shuyi attach` gains `--perf-snapshot`; and MCP's
  `perf` takes snapshots and diffs over the link, the session keeping
  them on the `shuyi` side. If not, 2555 adds them (its own clause says
  so).

**Out (do not do):**
- the tags themselves (3065), and the heap counts per frame (3064);
- the debug view's `memory` channel (2368, parked);
- call stacks, single blocks and the largest blocks (3063's ADR);
- a snapshot written to a file of its own, or loaded back: the trace's
  lines are the snapshots;
- a leak check that fails a test: a follow-up if a game asks (a test
  can already take two snapshots with the Harness and compare them).

## Implementation steps

1. Read 3063's ADR, `shuyi_ui::perf`, 2553's `run --perf`, MCP `perf`
   tool and trace file, 3064's and 3065's code, and
   `docs/contract.md`'s *Perf traces*.
2. Add the types and `diff`, with unit tests.
3. Add `Platform::heap_snapshot`, `App::heap_snapshot`,
   `Harness::heap_snapshot`, `shuyi_platform::heap`, and the fake's
   snapshots.
4. Add the snapshots to `run --perf`, the MCP tool and the trace file.
5. Regenerate the schemas, raise the version, write the docs, run the
   gates.

## Acceptance criteria

- [ ] With the fake memory, `run --perf --perf-out` on a game whose
      Harness starts on `with_fake_memory()` ends its file with two
      snapshot lines and a diff, and the file and the summary are
      committed snapshots.
- [ ] `diff` on known snapshots: a tag only in the later one, a tag
      only in the earlier one, a tag unchanged (left out), and the
      order by the size of the change.
- [ ] MCP's scripted client takes a snapshot, steps 30 frames, takes
      another with `diff_from` the first, and reads a diff; a
      `diff_from` that isn't kept answers wrong arguments with the kept
      frames.
- [ ] `cargo shuyi run --perf` on Hello, Grid with the real counter
      answers `heap_tags` with `load` or an asset kind among the first
      rows.
- [ ] `shuyi-app`'s trace writer, given 1,201 frames, writes snapshots
      on frames 1, 601 and 1201 and two diffs (a test over the writer).
- [ ] The schemas are regenerated and `versions.json` has the new
      version.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `diff`; the rows' order; the fake's snapshots on frames 0, 1,
  29 and 30 and their sum; `snapshot(counts, rows)`; the text line.
- Integration: `run --perf` with the fake (snapshots) and with the real
  counter; the MCP client with snapshots and a diff; the trace writer's
  snapshot lines.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
