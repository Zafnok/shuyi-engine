---
id: "2553"
title: "Perf traces agents read: shuyi run --perf, an MCP perf tool read from a number, and a running debug build's trace file, as JSON lines"
type: feature
milestone: MVP
model: implementation
effort: high
status: done
blocked_by: ["3063"]
nick_input: none
completed: 2026-10-25
---

# 2553 — Perf traces agents read

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md),
"perf traces agents can read"):

> "we should also have easily agent-readable / streamable perf traces
> when building in debug -- both timings and heap snapshots"

He put them in the MVP. 3063 measures each frame as data: `App` keeps a
`FrameRecord` per frame (its times in microseconds and the spans a game
marked) in a `PerfRing` of the last 600, numbered like the log's lines,
read with `App::perf`, `App::take_new_perf` and `Harness::perf`. Its ADR
decides the form agents read: JSON lines, one object per line tagged by
`kind`, with `shuyi_ui::perf`'s types and the contract's schema.

This ticket hands the records to agents three ways:

- **`shuyi run --perf`**: a summary in the answer, and with `--perf-out`
  every kept frame as a JSON line in a file;
- **MCP's `perf` tool**: the frames since a number, so an agent that
  steps a session reads what is new each time: a stream it pulls, as
  2521 reads the log;
- **`SHUYI_PERF=<file>`**: a running desktop debug build writes each
  frame as a JSON line as it runs, so an agent reads the file while it
  grows.

**What Shuyi has today** (read on 2026-10-24):

- `crates/cli/src/contract/run.rs`: `RunInput { start, script, frames
  }` (line 27), `RunOutput` (line 41) with the log (3040), `played`
  (line 98) starts a session and plays it, `limits_of` (line 117).
  `run::text` prints the answer for people.
- `crates/cli/src/contract/session.rs`: the `Session` trait (line 172)
  with default bodies for what a game's own session may lack (`log`,
  `log_dropped`, `modules`, `picture`), and `Headless<G>` over the
  Harness. `LogEntry` (line 113) is the pattern of an answer's type with
  `deny_unknown_fields` and `#[non_exhaustive]`.
- `crates/cli/src/contract.rs`: `VERSION` (line 73, 17 today), `Op`
  (line 78) and `Op::SESSION`, the session's tools (`start`, `step`,
  `scene`, `frame_png`, `events`, `reset`, `stop`). The MCP server
  (`crates/cli/src/mcp.rs`, `mcp/host.rs`) serves each as a tool, with
  `session.<tool>.input.json` and `.output.json` under
  `crates/cli/schemas/`. `docs/contract.md`, *The schemas*, says how to
  regenerate them and raise the version.
- `shuyi-core` has an optional `schemars` dependency behind its
  `schema` feature, for the playtest report's schema; `shuyi-ui` has
  serde but not `schemars` (`tools/layers.ron`).
- `frame-png <out.png>`: "a relative path is from the project's folder".
- `crates/app/src/lib.rs:252`: `shuyi-app`'s loop prints
  `app.take_new_log()` after each frame. It reads environment variables
  for its storage folders, `MUSIC_DIR` and `VOICE_CLIP`.

## Nick input

None.

## Scope

**In:**
- **From 3063 (done):** `Spread` and `spread` move here, by 3063's size
  rule (its diff passed 600 lines). In `shuyi_ui::perf`: `Spread { p50,
  p99, worst, mean }` (`u32` microseconds), a plain value left open
  (ADR-0039 §1), with serde's `Serialize` and `Deserialize` and
  `deny_unknown_fields`; and `spread(values: impl IntoIterator<Item =
  u32>) -> Option<Spread>`: nearest-rank percentiles (the value at rank
  ⌈p × n / 100⌉ of the sorted values), the mean rounded down, all in
  integers; `None` for no values. Unit tests on known lists: one value,
  two, a hundred, all equal.
- **`PerfLine`**, in `shuyi_ui::perf`: one line of a trace, a serde
  enum tagged by `kind` (`#[serde(tag = "kind", rename_all =
  "kebab-case")]`), `#[non_exhaustive]`, with one variant here,
  `Frame(FrameRecord)`. 2554 adds `Snapshot` and `Diff`. A line is
  `serde_json::to_string(&line)` and a `\n`.
- **The schema of the perf types:** a `schema` feature on `shuyi-ui`
  with an optional `schemars` (as `shuyi-core`'s: `default-features =
  false`, `derive`), and `JsonSchema` derived under it
  (`cfg_attr(feature = "schema", derive(JsonSchema))`) on `PerfLine`,
  `FrameRecord`, `FrameTimes`, `SpanEvent` and `Spread`. `schemars`
  (MIT, already in `Cargo.lock` through `shuyi-core` and `shuyi-cli`;
  check its license with `cargo deny check licenses`) joins
  `shuyi-ui`'s `outside` list in `tools/layers.ron`, with a comment;
  `shuyi-cli` turns on `shuyi-ui/schema`.
- **The session** (`crates/cli/src/contract/session.rs`):
  - `Session::set_perf(&mut self, on: bool)`, by default doing nothing,
    and `Session::perf(&self, since: u64, max: u32) -> PerfRead`, by
    default an empty read with `measured: false`;
  - `PerfRead { records: Vec<FrameRecord>, next: u64, dropped: u64,
    measured: bool }` (in a new `crates/cli/src/contract/perf.rs`,
    with `JsonSchema`): the kept records numbered `since` or more, the
    first `max` of them; `next`, the number to pass next time (the
    number after the last record answered); `dropped`, how many records
    numbered `since` or more the ring no longer keeps
    (`(next_seq - kept).saturating_sub(since)`, as 2521 counts the log);
    `measured`, whether anything was measured;
  - `Headless`: `set_perf(true)` sets a real clock on its Harness
    (`Harness::set_clock(Some(PerfClock::Real(now_us)))`, where `now_us`
    is the microseconds since a `std::sync::OnceLock<Instant>` first
    read) unless the Harness's platform already has a clock (a test's
    fake, kept so its numbers stay the same), and `false` takes away
    the clock it set. `perf` reads `Harness::perf()`.
    `FakeSession` gets a field of records for the tests.
- **`run --perf` and `--perf-out <file.jsonl>`:**
  - `RunInput` gains `perf: bool` and `perf_out: Option<String>` (a
    relative path is from the project's folder, as `frame-png`'s;
    `--perf-out` implies `--perf`). `played` calls `set_perf(true)`
    before the script when either is set;
  - `RunOutput` gains `perf: Option<PerfSummary>` (`None` without
    `--perf`, so a plain `run` answers as before). `PerfSummary`, in
    `perf.rs`: `frames` (the records kept), `dropped`, a `Spread` each
    for `frame`, `update`, `draw`, `paint` and `present` (3063's
    `spread`), `spans` (the five names with the most time over the kept
    records: `{ name, count, total_us, mean_us, worst_us }`), and `file`
    (the absolute path written, with `--perf-out`). `summary(records)`
    makes it, and MCP and 2555 use it too;
  - with `--perf-out`, the kept records are written as `PerfLine::Frame`
    lines, oldest first; a file that can't be written is an error with
    the setup code for a path (2509's table);
  - `limits`: with `--perf`, always "times are this machine's, from an
    unoptimized build unless it says otherwise: compare runs on the same
    machine"; when records were dropped, "the trace keeps the last 600
    frames: N earlier frame(s) were dropped"; when nothing was measured,
    "nothing was measured: the game's tool program was built without
    debug tools" and `perf` is `None`;
  - `run::text` prints one line, `perf: 600 frames, frame p50 1.2 ms p99
    3.4 ms worst 8.0 ms; span ai 0.8 ms mean`, then `trace: <file>` with
    `--perf-out`. Milliseconds with one decimal, worked out from the
    microseconds in integers.
- **MCP's `perf` tool:** a session tool (`Op::Perf`, joining
  `Op::SESSION`), input `since` (default 0) and `max` (default 120, 1 to
  600); it answers `PerfOutput { records, next, dropped, summary }`,
  `summary` over the records answered (`None` when there are none).
  `start` turns perf on for every session (`set_perf(true)`), so the
  tool always has records. After `reset` the numbers start again: a
  `since` past `next` answers no records and the limit "the session was
  reset or started again: read from 0". The answer's limits say the
  same as `run`'s about times. `session.perf.input.json` and
  `session.perf.output.json` are checked in.
- **The trace file of a running build,** in `shuyi-app`
  (`crates/app/src/perf_file.rs`), on native only:
  - `SHUYI_PERF=<path>` turns it on when `DEBUG_TOOLS` is on. Without
    debug tools it logs one line saying why and writes nothing;
  - it creates the file when the game starts, logs one line naming it,
    and after each frame writes `app.take_new_perf()` as `PerfLine::Frame`
    lines and flushes, so a reader sees each frame as it ends. A file
    that can't be created or written logs one warning and stops
    writing; the game goes on;
  - it logs with the backend's own macros, as its other lines;
  - its writer is a function over records and a `Write`, so a test
    writes records into a `Vec<u8>` without a window;
  - `serde_json` joins `shuyi-app`'s native dependencies (MIT OR
    Apache-2.0, already in `Cargo.lock` through `shuyi-cli`).
- **The contract's version:** raised by one for `run`'s and the
  session's new fields and the new tool, with its line in
  `crates/cli/schemas/versions.json` and the schemas regenerated, as
  `docs/contract.md`, *The schemas*, says.
- **Docs** (the `document` skill):
  - `docs/contract.md`: the `run` row names `--perf` and `--perf-out`;
    the MCP table gains `perf`; a new section, *Perf traces*: what a
    line holds (each field and its unit), that times are this machine's
    and only for comparing runs on it, that spans are the game's
    `ctx.span` calls, how to read a session as a stream (`step`, then
    `perf` with the last `next`), and `SHUYI_PERF` for a running build
    (`tail -f` or reading the file as it grows);
  - `crates/app/src/lib.rs`'s module docs: `SHUYI_PERF`.
- **The skill pack** (ADR-0043): a new skill, `skills/shuyi-perf/`,
  "Find what makes a game slow", in the open format the other skills
  use (`metadata.version: "1.0"`). Its description says when to use it
  (a frame too slow, a stage that stutters, a bot's turn that takes too
  long) and when not (a wrong result, which is `shuyi-debug-replay`; a
  test, which is `shuyi-test`). Its steps:
  - `cargo shuyi run --perf --perf-out perf.jsonl` on the stage, and
    reading the summary: the frame's p99 and worst, the phase that
    takes it (update, draw, paint, present), and the slowest spans;
  - marking the suspect code with `ctx.span("name", |ctx| …)` and
    running again to compare. Times are this machine's: compare two
    runs on it, never numbers across machines;
  - an MCP session read as a stream: `step`, then `perf` with the last
    `next`;
  - a running desktop debug build: `SHUYI_PERF=perf.jsonl`, and reading
    the file as it grows.

  Its worked example (`example.ron`) runs on a `grid` game: `run
  --perf --perf-out` checks that the summary's fields are there (never
  their values, which are the machine's) and that the file was written;
  an `Edit` step wraps the stage's update in a span; a second run finds
  that span among the summary's spans. `skills/README.md` lists it under
  *Testing and tuning*, and `shuyi-test`'s *When not to* points to it
  for speed. Update the snapshots that list the pack's skills (`new`'s
  answer and the plans' tests).

**Out (do not do):**
- frame times, spans and the records themselves (3063);
- the heap counter and the heap fields of the summary (3064, which
  comes after this ticket), memory tags (3065), and heap snapshots and
  diffs in the trace (2554). 3064 and 2554 add the heap to
  `shuyi-perf`;
- the trace over the live link, and `shuyi attach` following it (2555,
  which adds its section to `shuyi-perf`);
- the web build's trace (no file there: 2522's link, after the MVP);
- Chrome trace-event JSON for Perfetto (2325's "Save profile", parked);
  a converter of a JSON-lines file to it is a follow-up if a developer
  asks;
- a filter of what a trace keeps, or more than 600 frames in a run;
- `scene`'s answer: it gets no perf.

## Implementation steps

1. Read 3063's ADR and `shuyi_ui::perf`; `crates/cli/src/contract.rs`,
   `contract/run.rs`, `session.rs` and `fake.rs`; the MCP server's
   session tools (`crates/cli/src/mcp.rs`, `mcp/host.rs`, `mcp/server.rs`);
   `docs/contract.md`; `crates/app/src/lib.rs`.
2. Add `PerfLine` and the `schema` feature with its layers line.
3. Add `PerfRead`, `PerfSummary` and `summary`, the session's `set_perf`
   and `perf`, and their tests over a fake clock.
4. Add `run --perf` and `--perf-out`, the limits and the text.
5. Add the MCP `perf` tool and its client test.
6. Regenerate the schemas, raise the version, write the docs and the
   `shuyi-perf` skill, and run its worked example (`cargo xtask
   skill-examples shuyi-perf`).
7. Add the trace file to `shuyi-app`; run Hello, Grid with
   `SHUYI_PERF=perf.jsonl` on your desktop and put two of its lines in
   the PR.
8. Run the gates.

## Acceptance criteria

- [x] `cargo shuyi run --perf --perf-out perf.jsonl --frames 120` on
      Hello, Grid answers a summary and writes one parseable line per
      kept frame (a test reads each as a `PerfLine`).
- [x] With a fake clock (the CLI's toy, `crates/cli/src/toy.rs`, its
      Harness started on `HeadlessPlatform::new().with_clock(
      PerfClock::fake())`), `run`'s summary and the file's lines are
      committed snapshots, the same on every run.
- [x] A plain `run` (no `--perf`) answers as before: `perf` is `null`
      and no limit is added.
- [x] A run of 700 frames keeps 600 and says in a limit that 100 were
      dropped.
- [x] MCP's scripted client starts a session, steps 10 frames, reads
      `perf` from 0 (10 records, `next` 10), steps 5 more and reads from
      10 (5 records); a `since` past `next` after `reset` answers the
      limit.
- [x] `session.perf.*.json` and the regenerated `run` schemas are
      checked in, and `versions.json` has the new version.
- [x] `shuyi-app`'s trace writer turns records into lines a test parses
      back to the same records; with `SHUYI_PERF` set, Hello, Grid run
      by hand writes a line per frame (two lines in the PR).
- [x] `docs/contract.md` has *Perf traces*, and its MCP table lists
      `perf`.
- [x] `cargo shuyi check-skills skills --tickets tickets/open` passes
      with `shuyi-perf`, `cargo xtask skill-examples shuyi-perf` runs
      its worked example green, and a game made with `shuyi new` gets
      the skill in `.claude/skills/`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `spread` (one value, two, a hundred, all equal); `summary` (spreads, the five spans, ties by name); `PerfRead`'s
  `next` and `dropped` past a full ring; `perf_out`'s path from the
  project's folder; the text line's milliseconds; the trace writer.
- Integration: `run --perf --perf-out` on Hello, Grid (lines parse; the
  limit on times); `run` with a fake clock (snapshots); the MCP client
  steps and reads `perf` as a stream.

## Completion notes

Done.

- **`shuyi_ui::perf`:** `Spread` and `spread` (nearest rank, integers),
  `PerfLine` (`kind`-tagged, one variant `Frame`), and a `schema`
  feature (optional `schemars`, its line in `tools/layers.ron`) deriving
  `JsonSchema` on the perf types; `shuyi-cli` turns it on.
- **`contract::perf`** (new): `PerfRead` (`PerfRead::of` over a ring,
  `none`), `PerfSummary` and `SpanSummary`, `summary`, `lines`, the
  limits (`TIMES`, `RESET`, `dropped_limit`, `unmeasured_limit`),
  `millis` and `text`.
- **The session:** `Session::set_perf` and `Session::perf` with default
  bodies; `Headless` gives its Harness a real clock (a `OnceLock<Instant>`)
  unless its platform has one, and `false` takes back only the clock it
  gave. `FakeSession` keeps records. The CLI's toy now offers a session
  (`ToyTools::session`), on `PerfClock::fake()`, so its summary and
  trace are committed snapshots (`crates/cli/tests/it/snapshots/`).
- **`run --perf` / `--perf-out`:** `RunInput::perf` and `perf_out`,
  `RunOutput::perf`, the limits and the text line; a file that can't be
  written fails with `SH3004` (`codes::FILE`, the code `attach
  --screenshot` uses for a path). A run with no frame at all says
  "nothing was measured: no frame ran" rather than blaming the build.
- **MCP `perf`:** `Op::Perf` in `Op::SESSION`, `PerfInput` and
  `PerfOutput` in `mcp.rs`, the host's `perf_read`; `start` (and `reset`)
  turn perf on. After a reset, a `since` past the ring's next number
  answers `next` 0, so an agent that passes `next` each time reads from
  0 again. A session attached over the link refuses it with `SH3001`
  (2555 reads it over the link; its ticket now says where).
- **`shuyi-app`:** `perf_file.rs`, native only: `SHUYI_PERF` with debug
  tools writes and flushes each frame's line after the frame; without
  them it logs why; a file that can't be created or written is one
  warning and the trace stops.
- **Contract 22**, the schemas regenerated (every answer's `op` enum
  gained `perf`), `docs/contract.md`'s *Perf traces*, the `run` row and
  both MCP tables; `SH0002`'s page, the MCP server's instructions; the
  `shuyi-perf` skill with its worked example (green), listed in
  `skills/README.md`, and `shuyi-test` 1.3 points to it.

Deviations:

- `run`'s signature stays `run(game, input)` (a relative `--perf-out`
  from the working folder), and a new `run_at(game, root, input)` writes
  from the project's folder; the CLI and the MCP host call `run_at`.
  Changing `run` broke the `grid` template's own tests, which call it.
- `scene`'s input schema also shows `perf` and `perf_out`: `scene`
  shares `RunInput`, and ignores them.
- The MCP client test reads the stream after a `reset` of its session
  rather than a second `start`, which would build the tool program again
  while another test may hold it (seen once in a full workspace run).

Hello, Grid run by hand with `SHUYI_PERF` (6 seconds, 1275 lines; the
first two frames load and make no record):

```text
{"kind":"frame","frame":3,"times":{"update_us":58,"draw_us":202,"paint_us":294,"present_us":0,"frame_us":663},"spans":[],"spans_dropped":0}
{"kind":"frame","frame":4,"times":{"update_us":25,"draw_us":184,"paint_us":357,"present_us":3565,"frame_us":4157},"spans":[],"spans_dropped":0}
```

No follow-up tickets: the handoffs are written into 2554 and 2555.

Breaking: shuyi-cli: contract::Op: a new variant `Perf` (the variants after it renumbered), and `Op::SESSION` has 8 tools: a match on every `Op` adds an arm; the contract's version is 22. The game matches none.
