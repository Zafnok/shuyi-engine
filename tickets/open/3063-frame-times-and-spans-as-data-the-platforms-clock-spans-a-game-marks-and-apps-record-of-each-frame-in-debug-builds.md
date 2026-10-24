---
id: "3063"
title: "Frame times and spans as data: the platform's clock, spans a game marks, and App's record of each frame, in debug builds"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3063 — Frame times and spans as data

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md),
"perf traces agents can read"):

> "we should also have easily agent-readable / streamable perf traces
> when building in debug -- both timings and heap snapshots"

He put the traces in the MVP, and the live link (2521) with them. Six
tickets make them, in this order:

1. **3063 (this one):** frame times and spans, measured as data and kept
   by `App`, and the ADR for all six;
2. **2553:** the trace agents read: `shuyi run --perf`, an MCP `perf`
   tool read from a number, and a running build's trace file, all JSON
   lines;
3. **3064:** the heap counter, a counting allocator in debug builds, and
   heap counts in each frame's record;
4. **3065:** memory tags: live bytes by the frame's phase, the game's
   spans and each asset kind;
5. **2554:** heap snapshots and diffs in the trace;
6. **2555:** the trace over the live link (2521).

**What Shuyi has today** (read on 2026-10-24):

- **No clock in the core,** and there must never be one (`AGENTS.md`
  rule 5). `App::frame(platform, now: Millis)`
  (`crates/platform/src/app.rs:394`) takes the backend's time in whole
  milliseconds for the game's steps. That is too coarse for a phase of a
  frame, and the Harness drives `App::frame_after` with no clock at all.
- **The `Platform` trait** (`crates/platform/src/platform.rs:199`) has
  `display`, `present`, `poll_input`, `system_events`, `storage`,
  `audio` and `text_input`. No clock, no memory.
- **A frame in `App::run`** (`app.rs:482`): `step` (`app.rs:661`) polls
  the input, hands the game the music and the ticks, calls
  `Game::frame`, and plays the audio; then `present_game` (`app.rs:706`)
  calls `Platform::present`. `Game::frame` (`crates/ui/src/game.rs:304`)
  runs `step` (line 340), which updates the screens and ends with
  `self.redraw()` (line 426), the drawing; then `collect_audio`.
- **The log (3040, ADR-0030)** is the pattern for data an `App` keeps:
  `shuyi_ui::log::LogRing` (`crates/ui/src/log.rs:142`) numbers the
  lines it keeps from 0, `since(seq)` gives those numbered `seq` or
  more, and `next_seq()` is the next number, so a reader asks for what
  is new since its last read. `App::log` and `App::take_new_log` read
  it; `Harness::log` too; the contract's `run` and `scene` carry it.
- **`shuyi-app`'s loop** (`crates/app/src/lib.rs:252`): `app.frame`,
  then the new log lines, then `next_frame().await` (the buffer swap and
  the wait for vsync). `crates/app/src/backend.rs`: `Macroquad`
  implements `Platform`.
- **`shuyi_ui::DEBUG_TOOLS`** (`crates/ui/src/lib.rs:92`): a debug
  build, or the `debug-tools` feature. `Ctx::debug_tools`
  (`crates/ui/src/ctx.rs:275`) starts as it.
- **2325 (parked)** planned `FrameTimes`, spans and phase times measured
  in `shuyi-app`, for its graph in the debug view. Those move here, so
  agents get them without the debug view, and 2325 now draws what this
  ticket records (it is blocked by this ticket).

**The rules this keeps:**

- **The core stays pure** (`AGENTS.md` rule 5). Only a backend reads a
  clock. It hands `App` a clock, and `App` and the game's context read
  the time through it.
- **Read-only.** A time never reaches an update. A game run with the
  clock on has the same screens and frames as with it off.
- **Debug builds only.** With debug tools off, nothing is measured and
  the clock is never read.

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number): "Perf
  traces: measured by the backend, kept by App beside the log, and read
  by agents as JSON lines". It decides for all six tickets:
  1. **Only a backend measures.** It gives a clock
     (`Platform::clock`), and later a heap counter (3064) and memory
     tags (3065). `App` and the game's context read them; the core never
     does. A measurement never reaches an update, so a replay's hash is
     the same with measuring on and off.
  2. **Debug builds only, with nothing to turn on.** App measures only
     while the game's `Ctx::debug_tools` is on (and times the load only
     with `DEBUG_TOOLS`). The heap counter is a global allocator in
     every debug build of a game on `shuyi-app` and of its tool program,
     and in no release build (3064); a game with a global allocator of
     its own opts out with a `--cfg`.
  3. **App keeps a record per frame,** the last `PERF_FRAMES` (600, ten
     seconds at 60 a second), numbered like the log's lines, so a reader
     asks for what is new since its last read: the contract, MCP, a
     file and the live link read them the same way.
  4. **A record is integers:** the frame, times in whole microseconds,
     the spans that ended in it, and (3064) heap counts in bytes. No
     float leaves the backend.
  5. **A span is a closure on the context,** `ctx.span("name", |ctx|
     …)`: a guard holding `&mut Ctx` would lock the context for the
     span's length. The `tracing` crate and Tracy stay out, as ADR-0030
     keeps `tracing` out of the log: our spans are a few lines and need
     no global subscriber.
  6. **A heap snapshot** (3065, 2554) is live bytes and counts by memory
     tag: the frame's phase, the game's span, or the asset kind being
     decoded, the innermost one winning; a realloc moves the bytes to
     the current tag. A diff compares two snapshots. No call stacks and
     no single allocations: native tools do those (heaptrack,
     Instruments, Windows Performance Analyzer).
  7. **The form agents read is JSON lines,** one object per line tagged
     by `kind` (2553). The types are `shuyi_ui::perf`'s, with serde, and
     their JSON schema is the contract's. Chrome trace-event JSON, for
     Perfetto, is derived from the records (2325's "Save profile"),
     never the source.
  8. **Fakes make tests exact.** A fake clock and a fake counter give
     the same numbers on every run, so a trace is a snapshot.
  9. **The counting allocator lives in `shuyi-alloc-count`**
     (`crates/alloc-count`), one more crate where ADR-0002 §9 allows
     `unsafe`, as ADR-0021 §6 did for the platform crates: one
     `#[allow(unsafe_code)]` module, a `// SAFETY:` comment on every
     `unsafe` block. It joins the default engine's crates (ADR-0009 §4
     had six), since `shuyi-app` and `shuyi-cli` depend on it and no
     default crate may depend on a module. Its `global` feature holds
     the one `#[global_allocator]` static, so the backends that turn it
     on share it (3064 says why).
- **`shuyi_ui::perf`**, a new module (`crates/ui/src/perf.rs`, its tests
  in `crates/ui/src/perf/tests.rs`), `no_std` + `alloc`, like
  `shuyi_ui::log`:
  - `PerfClock`, a `Copy` enum, `#[non_exhaustive]`:
    `Real(fn() -> u64)`, microseconds from any fixed start, never going
    back; and `Fake(FakeClock)`, made with `PerfClock::fake()`.
    `read(&mut self) -> u64`. The fake starts at 0, and its read number
    *n* (from 0) adds 100 × (*n* mod 7 + 1) microseconds and returns the
    new time (100, 300, 600, 1000, …). `FakeClock::reads()` counts its
    reads, for the tests;
  - `FrameTimes { update_us, draw_us, paint_us, present_us, frame_us }`,
    each a `u32` of microseconds, `#[non_exhaustive]`, with `Default`
    and `FrameTimes::new()` (all 0), its fields public (ADR-0039 §3:
    2382, parked, adds a GPU time). Its doc comments say what each is:
    - `update_us`: `Game::frame` from its start to the drawing (the
      input, the modules' `before_update`, the screens' update);
    - `draw_us`: the drawing (`Game::redraw` inside `step`);
    - `paint_us`: the `Platform::present` call (in `shuyi-app`, painting
      the frame into the window);
    - `present_us`: from the end of the last frame to the start of this
      one, outside `App`: the backend's buffer swap and its wait for
      vsync or the frame cap. 0 on the first measured frame;
    - `frame_us`: `present_us` plus this frame from its start to its
      end. It is never less than the four above; the rest is `App`'s
      own (the system's events, the audio, the record);
  - `SpanEvent { name: Cow<'static, str>, start_us: u32, us: u32,
    depth: u8 }`, `#[non_exhaustive]`: `start_us` from the frame's
    start, `depth` 0 for a span inside no other;
  - `FrameRecord { frame: u32, times: FrameTimes, spans:
    Vec<SpanEvent>, spans_dropped: u32 }`, `#[non_exhaustive]`: the
    spans in the order they ended (so an inner one before its outer
    one), at most `MAX_SPANS` (64), the rest counted in `spans_dropped`.
    3064 adds `heap`;
  - `PERF_FRAMES: usize = 600`;
  - `PerfRing`, `LogRing`'s twin: `new(capacity)`, `push(record)` (the
    oldest dropped and counted when full), `records()` (oldest first),
    `next_seq() -> u64`, `since(seq)` (the kept records numbered `seq`
    or more), `dropped() -> u64`;
  - `Spread { p50, p99, worst, mean }` (`u32` microseconds) and
    `spread(values: impl IntoIterator<Item = u32>) -> Option<Spread>`:
    nearest-rank percentiles (the value at rank ⌈p × n / 100⌉ of the
    sorted values), the mean rounded down, all in integers; `None` for
    no values;
  - serde's `Serialize` and `Deserialize` on `FrameTimes`, `SpanEvent`,
    `FrameRecord` and `Spread`, with `deny_unknown_fields` as the
    contract's answers have (`LogEntry`). 2553 adds their JSON schema.
- **The context** (`crates/ui/src/ctx.rs`) keeps a private perf state:
  the clock, the frame's start, the depth, this frame's spans, and the
  update and draw times:
  - `Ctx::span<R>(&mut self, name: &'static str, f: impl FnOnce(&mut
    Self) -> R) -> R`. With no clock, or outside a measured frame, it
    calls `f` and reads nothing. Otherwise it reads the clock before
    and after `f` and keeps a `SpanEvent` at its depth; past `MAX_SPANS`
    in a frame it counts one more dropped;
  - for App and tests (their doc comments say so):
    `Ctx::set_perf_clock(Option<PerfClock>)`, `Ctx::perf_now(&mut self)
    -> Option<u64>`, and a pair that starts and ends a frame's measuring
    (`Ctx::begin_perf_frame` reads the frame's start and clears the
    frame's spans; `Ctx::end_perf_frame` hands back the update and draw
    times and the spans). Name them as fits the code.
- **`Game::frame`** (`crates/ui/src/game.rs`) reads the context's clock,
  when it has one in a measured frame, at its start, just before `step`
  calls `self.redraw()`, and just after it, and keeps the update and
  draw times in the perf state. `Game::act` and a `redraw` from outside
  the frame loop measure nothing.
- **`Platform::clock(&self) -> Option<PerfClock>`**, defaulted to
  `None`, so no backend breaks.
- **`App`:**
  - it takes the clock from `Platform::clock` once, on its first frame,
    when `DEBUG_TOOLS` is on. While the game loads it keeps the clock and
    times the load (`App::load_us() -> Option<u32>`). When the game runs
    (after its load, or on the first frame of `App::running`) and its
    `Ctx::debug_tools` is on, it moves the clock into the context
    (`Ctx::set_perf_clock`). From then on it reads the time through the
    context, so `App`'s phases and the game's spans read one clock;
  - each running frame that isn't suspended and has a clock in the
    context: it reads the time at the frame's start (in `begin`, before
    the system's events), just before and after `present_game`, and at
    the end. It builds a `FrameRecord` with `App::frame_number()`, the
    game's update and draw times and spans, and pushes it into a
    `PerfRing` of `PERF_FRAMES`. A loading, suspended, failed or quit
    frame makes none;
  - `App::perf(&self) -> &PerfRing`, and `App::take_new_perf(&mut self)
    -> Vec<FrameRecord>`, the records since the last call, for a
    backend's sinks (2553's trace file), as `take_new_log` is.
- **The headless backend and the Harness:**
  `HeadlessPlatform::with_clock(self, PerfClock) -> Self` and
  `set_clock(&mut self, Option<PerfClock>)`; `Harness::perf() ->
  &PerfRing`; `Harness::set_clock(&mut self, Option<PerfClock>)`, which
  sets the platform's clock and, once the game runs, the context's.
- **`shuyi-app`:** `Macroquad::clock` gives `PerfClock::Real(now_us)`,
  where `now_us` is `macroquad::time::get_time()` (seconds since the
  start, an `f64`, on native and the web) in whole microseconds. `App`
  decides whether to use it.
- **Either order with 3010:** if 3010 is done, `shuyi-playdate`
  implements `Platform::clock` with the SDK's high-resolution timer
  (check its name and its unit in the SDK's docs; never reset it in the
  frame loop); if not, 3010 does it (its own clause says so).
- **Docs** (the `document` skill): `docs/platform-backends.md` gains
  `Platform::clock` (what it gives, that only debug builds read it, and
  that a backend without a clock leaves it `None`); `shuyi_ui::perf`'s
  module docs say how a game marks a span (`ctx.span("ai", |ctx| …)`)
  and that a span's time never reaches an update.
- The new public types follow ADR-0039: `PerfClock`, `FrameTimes`,
  `SpanEvent` and `FrameRecord` are `#[non_exhaustive]` with
  constructors; `Spread` is a plain value, left open.
- **Size:** if the diff passes about 600 changed lines, `Spread` and
  `spread` move to 2553, which is their first reader, and the PR says
  so.

**Out (do not do):**
- the trace in the contract's answers, MCP and a file (2553);
- the heap: the counter (3064), memory tags (3065), snapshots in the
  trace (2554);
- the trace over the live link (2555);
- the debug view's `profile` channel, its readout line and "Save
  profile" as Chrome trace-event JSON (2325, parked), which draw these
  records;
- GPU times (2382, parked);
- spans the engine makes by itself, one per screen or per module hook:
  a follow-up if agents ask;
- Tracy, the `tracing` crate and flame graphs;
- budgets in CI (3008), and frame times on devices (3011, 3013).

## Implementation steps

1. Read ADR-0030 (the log) and ADR-0039; `crates/ui/src/log.rs`,
   `ctx.rs` and `game.rs`; `crates/platform/src/platform.rs`, `app.rs`,
   `headless.rs` and `harness.rs`; `crates/app/src/lib.rs` and
   `backend.rs`; and 2325's and 3064's tickets, so the ADR fits them.
2. Write the ADR.
3. Write `shuyi_ui::perf` (`pub mod perf;` in `crates/ui/src/lib.rs`)
   and its unit tests.
4. Add the context's perf state, `Ctx::span`, and the reads in
   `Game::frame`.
5. Add `Platform::clock`, App's clock, records, ring, `perf`,
   `take_new_perf` and `load_us`, and the headless backend's and the
   Harness's clock.
6. Add `Macroquad::clock`, write the docs, and run the gates.

## Acceptance criteria

- [ ] With the fake clock, the toy game (`shuyi_ui::toy`) on the Harness
      gives the same records on every run: the first three, with a span
      a test screen marks inside another, are a committed snapshot.
- [ ] The toy plays the same random script with the fake clock on and
      off and ends with the same screens and the same frame text.
- [ ] With `Ctx::debug_tools` off, `App` keeps no record and never reads
      the clock (`FakeClock::reads()` stays 0).
- [ ] `Ctx::span` with no clock calls its closure once and records
      nothing; nested spans have depths 0 and 1, the inner one first; a
      65th span in a frame is counted in `spans_dropped`.
- [ ] `spread` gives the right p50, p99, worst and mean on known lists
      (one value, two, a hundred, all equal).
- [ ] `PerfRing`'s `since`, `next_seq` and `dropped` behave as
      `LogRing`'s do (the same cases as `LogRing`'s tests).
- [ ] `Macroquad::clock` builds on native and in the web build (the
      examples' web build gate).
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the fake clock's reads; `FrameTimes`' sum never above
  `frame_us`; spans, their depth, their order and the cap; `spread`;
  `PerfRing` wrapping, `since` and `dropped`; the load's time with the
  fake clock; no record on a suspended frame.
- Property: for random scripts on the toy with the fake clock, every
  record's four phase times add up to no more than its `frame_us`, and
  the screens and frame text equal a run with no clock.
- Snapshot: the toy's first three records with the fake clock.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
