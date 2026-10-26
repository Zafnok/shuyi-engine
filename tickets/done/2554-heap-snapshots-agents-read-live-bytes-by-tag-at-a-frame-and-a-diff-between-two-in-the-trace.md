---
id: "2554"
title: "Heap snapshots agents read: live bytes by tag at a frame, and a diff between two, in run --perf, the MCP perf tool and a running build's trace file"
type: feature
milestone: MVP
model: sonnet-5
effort: high
status: done
blocked_by: ["2553", "3065"]
nick_input: none
completed: 2026-10-26
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
- **From 3065 (done):** a row is `shuyi_alloc_count::TagCounts { tag,
  class: u8, name: &'static str, live_bytes, live_blocks: u32,
  allocated_bytes, peak_bytes }`, from `CountingAlloc::rows`, and for
  the global static from `global::rows()` (`None` in a build that
  doesn't count); `tags::overflowed()` is the snapshot's `overflowed`.
  A class is a number that `TagClass::from_u8` reads (`Other` is 0,
  `tags::OTHER_CLASS`, the class of `other` and `overflow`). Names
  compare by class and text, so a span named `draw` is a tag apart
  from the phase. A `realloc` adds its new size to the current tag's
  bytes ever allocated. `App` enters `perf` every frame, around its
  record; tests hand fake scopes to `HeadlessPlatform::with_scopes` or
  `Harness::set_scopes`.

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
- **From 2553 (done):** the pieces this ticket grows are
  `crates/cli/src/contract/perf.rs` (`PerfSummary`, `summary`, `lines`,
  the limits), `contract/run.rs`'s `perf_of` and `run_at` (the trace
  file's writing; `run` is `run_at` from the working folder),
  `crates/cli/src/mcp.rs`'s `PerfInput` and `PerfOutput` with the host's
  `perf_read` (`mcp/host.rs`), and `crates/app/src/perf_file.rs`
  (`write_lines`, `PerfFile::write`, called after each frame in
  `shuyi_app::run`). `PerfLine` has one variant, `Frame`; the toy's
  session (`shuyi_cli::toy::ToyTools::session`) runs on
  `PerfClock::fake()`, and its summary and trace are snapshots in
  `crates/cli/tests/it/snapshots/`.
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
- **The skill pack:** 2553's `shuyi-perf` skill gains a section, "What
  holds the heap, and what grew": when to use it (the heap grows as the
  game plays, a stage or a target runs out of memory); reading
  `heap_tags` and `heap_growth` from `run --perf`; taking a snapshot
  and a diff in an MCP session (`perf` with `snapshot` and
  `diff_from`), with the wrong-arguments code its error gives; what a
  tag's class says (a phase, a span, an asset kind, `other`); and
  marking the suspect code with a span to give its bytes a tag of their
  own. Its description names the heap too, and its `metadata.version`
  is raised. Its worked example checks that `heap_tags` and
  `heap_growth` are there, and that a `diff_from` that isn't kept
  answers its code.
- **Either order with 2555:** if 2555 is done, the link's `Perf`
  request gains `snapshot: bool`: the backend takes it after the frame
  (as the link's screenshot is taken after `present`) and answers it
  with the records; `shuyi attach` gains `--perf-snapshot`, which
  `shuyi-perf`'s section on a running build names; and MCP's `perf`
  takes snapshots and diffs over the link, the session keeping them on
  the `shuyi` side. If not, 2555 adds them (its own clause says so).

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
5. Regenerate the schemas, raise the version, write the docs and the
   skill's section, run the gates.

## Acceptance criteria

- [x] With the fake memory, `run --perf --perf-out` on a game whose
      Harness starts on `with_fake_memory()` ends its file with two
      snapshot lines and a diff, and the file and the summary are
      committed snapshots.
- [x] `diff` on known snapshots: a tag only in the later one, a tag
      only in the earlier one, a tag unchanged (left out), and the
      order by the size of the change.
- [x] MCP's scripted client takes a snapshot, steps 30 frames, takes
      another with `diff_from` the first, and reads a diff; a
      `diff_from` that isn't kept answers wrong arguments with the kept
      frames.
- [x] `cargo shuyi run --perf` on Hello, Grid with the real counter
      answers `heap_tags` with `load` or an asset kind among the first
      rows.
- [x] `shuyi-app`'s trace writer, given 1,201 frames, writes snapshots
      on frames 1, 601 and 1201 and two diffs (a test over the writer).
- [x] The schemas are regenerated and `versions.json` has the new
      version.
- [x] `shuyi-perf` has its heap section, `cargo shuyi check-skills
      skills` passes, and `cargo xtask skill-examples shuyi-perf` runs
      its example green.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `diff`; the rows' order; the fake's snapshots on frames 0, 1,
  29 and 30 and their sum; `snapshot(counts, rows)`; the text line.
- Integration: `run --perf` with the fake (snapshots) and with the real
  counter; the MCP client with snapshots and a diff; the trace writer's
  snapshot lines.

## Completion notes

**What was done.**

- `shuyi_ui::perf`: `TagRow`, `HeapSnapshot` (`new` puts the rows in
  order: live bytes, most first, then class number, then name),
  `TagDelta`, `HeapDiff` and `HeapSnapshot::diff`; `PerfLine` gains
  `Snapshot` and `Diff`. All serde, `JsonSchema` under `schema`,
  `#[non_exhaustive]` with constructors.
- `shuyi-platform`: `Platform::heap_snapshot` (defaulted to `None`),
  `App::heap_snapshot` (stamps `frame_number`; `None` without
  `DEBUG_TOOLS`, or while the running game's `debug_tools` is off),
  `Harness::heap_snapshot` and `set_heap_snapshots`,
  `HeadlessPlatform::set_heap_snapshots`, and the fake memory's
  snapshots (the ticket's four rows; `update` is left out before the
  first frame, as a counter leaves out a tag with nothing ever
  allocated). The new `heap` feature has `heap::snapshot`, with an
  optional dependency on `shuyi-alloc-count` without `global`, and its
  edge in `tools/layers.ron`.
- `shuyi-app`: `Macroquad::heap_snapshot` from the global counter. The
  trace file (`SHUYI_PERF`) writes a snapshot on its first frame that
  gives one, then every 600 frames, each after the first followed by
  its diff (`perf_file::Snapshots`).
- `shuyi-cli`: `Session::heap_snapshot` (defaulted to `None`;
  `Headless` over the Harness, with this program's counter handed over
  by `set_perf`). `run --perf` takes a snapshot after `set_perf`, before
  the script, and one after the last frame. `PerfSummary` gains
  `heap_tags` and `heap_growth`, `--perf-out` ends with the two
  snapshots and their diff (`perf::heap_lines`), and the text has the
  `heap grew …` line (`perf::heap_text`). MCP's `perf` takes `snapshot`
  and `diff_from`, and answers `snapshot`, `diff` and `kept`; the host's
  session keeps the last 8 by frame. Contract 24, schemas regenerated.
- Docs: `docs/contract.md` (*What holds the heap*, under *Perf
  traces*, and the `perf` tool's row), `docs/platform-backends.md`
  (`heap_snapshot`), the crates' rustdoc, and `shuyi-perf`'s section
  "What holds the heap, and what grew" (version 1.2) with its worked
  example.
- Checked by hand: `cargo shuyi run --frames 120 --perf` on Hello, Grid
  with the real counter answers `heap_tags` with `load` first and the
  asset kinds (`english`, `font atlas`, `keymap`, …) after it, and
  `heap grew 28.0 KB from frame 0 to 120: paint +13.5 KB, perf +11.0
  KB, other +3.5 KB`.

**Deviations.**

- `heap::snapshot` takes three arguments, `snapshot(counts, &rows,
  overflowed)`: the overflowed count comes from `tags::overflowed()`,
  global state, so the caller reads it and the conversion stays pure
  and testable.
- `PerfSummary::heap_growth` is `Option<HeapDiff>`, the whole diff with
  its first 10 rows, not only the rows: the text line needs its frames
  and its total, and an agent reads them too.
- A tool program's session start now runs under the `load` phase
  (`session::measured_start`, used by `run --perf` and MCP's `start`),
  and sets the decode watch before the game loads. Without it the
  game's load in a headless session ran before any tag, so `heap_tags`
  showed it all as `other`, and the acceptance criterion on Hello, Grid
  could not hold.
- A new limit, `perf::HEAP_UNTAGGED` ("the heap isn't counted by memory
  tag, so no heap snapshot was taken"), when the heap is counted without
  tags, or when `perf` with `snapshot` gets none. Where the heap isn't
  counted at all, 3064's `HEAP_UNCOUNTED` says why, as the ticket asks.
- The worked example of `shuyi-perf` checks `heap_tags`, `heap_growth`
  and the trace's snapshot and diff lines, but not "a `diff_from` that
  isn't kept answers its code": a worked example has no step that calls
  an MCP session tool. Follow-up **2563** adds that step and the check.
  The behaviour is tested in `crates/cli/tests/it/mcp.rs` and
  `crates/cli/src/mcp/host/tests.rs`.
- The text line's example in the docs reads `124.0 KB`: the ticket's
  "120.0 KB" didn't add up to its own three changes.

**Follow-ups.** 2563 (an `Mcp` step for worked examples). 2555's clause
on this ticket is rewritten as what now holds: it adds snapshots to the
live link.

No break: every type that grew is `#[non_exhaustive]` (`PerfLine`,
`PerfSummary`, `PerfInput`, `PerfOutput`), and the trait methods
(`Platform::heap_snapshot`, `Session::heap_snapshot`) have defaults.

Nick input: none.
