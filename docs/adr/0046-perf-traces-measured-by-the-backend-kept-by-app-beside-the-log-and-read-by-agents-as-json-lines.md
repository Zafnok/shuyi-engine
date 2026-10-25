# ADR-0046: Perf traces: measured by the backend, kept by App beside the log, and read by agents as JSON lines

- **Status:** Accepted
- **Date:** 2026-10-25
- **Related tickets:** 3063 (this: frame times and spans), 2553 (the
  trace agents read), 3064 (the heap counter), 3065 (memory tags), 2554
  (heap snapshots and diffs), 2555 (the trace over the live link), 2325
  and 2368 (parked: the debug view draws these numbers), 2382 (parked:
  GPU times), 3010 (the Playdate backend)
- **Extends:** ADR-0002 §1 and §9 (the layers, `unsafe`), ADR-0009 §4
  (the default engine's crates), ADR-0024 (the platform boundary),
  ADR-0030 (the log), ADR-0039 (public types that can grow)

## Context

- Nick, 2026-10-24 (`docs/decisions.md`, "perf traces agents can
  read"): "easily agent-readable / streamable perf traces when building
  in debug -- both timings and heap snapshots", in the MVP. Six tickets
  make them: 3063, 2553, 3064, 3065, 2554 and 2555.
- **The core has no clock** and must never get one (`AGENTS.md` rule 5).
  `App::frame` takes the backend's time in whole milliseconds for the
  game's steps (ADR-0024 §3), too coarse for a phase of a frame; the
  Harness drives `App::frame_after` with no clock at all.
- **What a frame is.** `App` reads the system's events, polls the input,
  calls `Game::frame` (the input, the modules' `before_update`, the
  screens' update, then the drawing, `Game::redraw`), plays the audio and
  calls `Platform::present`. The backend's loop then swaps the buffers
  and waits for vsync or a frame cap (`next_frame().await` in
  `shuyi-app`), outside `App`.
- **The log (ADR-0030)** is how `App` keeps data for readers: a ring
  numbered from 0, `since(seq)` for what is new, kept per `App` and never
  in a global, so two Harnesses on two test threads never mix.
- **Other engines.** Unity's Profiler and Unreal's Insights record
  frames and named scopes and stream them to a tool; Bevy and many Rust
  games use the `tracing` crate's spans with a Tracy or Chrome-trace
  subscriber; Unreal's Low-Level Memory Tracker counts the heap by a tag
  per scope, with no call stacks. Chrome's trace-event JSON is what
  Perfetto reads.
- **Agents read text.** The contract answers in JSON (ADR-0022), the MCP
  server is JSON-RPC (ADR-0031), and a file an agent tails is easiest one
  line per record.
- **A counting allocator is `unsafe`** (`GlobalAlloc` is an unsafe
  trait), and a binary has one global allocator. A game's binary links
  `shuyi-app`; its tool program links `shuyi-app` and `shuyi-cli`.

## Decision

1. **Only a backend measures.** It gives a clock, `Platform::clock(&self)
   -> Option<PerfClock>` (defaulted to `None`, so a backend without one
   builds), and later a heap counter (3064) and memory tags (3065). `App`
   and the game's context read them; `shuyi-core` never does. A
   measurement never reaches an update: no screen, module or rule reads a
   time to decide anything, so a replay's hash and a run's screens and
   frames are the same with measuring on and off.
2. **Debug builds only, with nothing to turn on.** `App` takes the clock
   from the platform once, on its first frame, and only when
   `shuyi_ui::DEBUG_TOOLS` is on; it times the game's load with it. When
   the game runs it moves the clock into the game's context
   (`Ctx::set_perf_clock`), and from then on reads the time through the
   context, so `App`'s phases and the game's spans read one clock. It
   measures a frame only while the game's `Ctx::debug_tools` is on; with
   it off the clock is never read. The heap counter is a global allocator
   in every debug build of a game on `shuyi-app` and of its tool program,
   and in no release build (3064); a game with a global allocator of its
   own opts out with a `--cfg`.
3. **App keeps a record per frame,** the last `PERF_FRAMES` (600, ten
   seconds at 60 a second) in a `PerfRing`, numbered like the log's lines
   (`next_seq`, `since(seq)`, `dropped`), so a reader asks for what is
   new since its last read: the contract, MCP, a file and the live link
   read them the same way. `App::perf` reads the ring and
   `App::take_new_perf` gives a backend's sinks the records since its
   last call, as `take_new_log` does. A loading, suspended, failed or quit
   frame makes no record.
4. **A record is integers:** `FrameRecord { frame, times, spans,
   spans_dropped }`, the frame being `App::frame_number`. `FrameTimes`
   holds whole microseconds as `u32`: `update_us` (`Game::frame` from its
   start to the drawing), `draw_us` (the drawing), `paint_us`
   (`Platform::present`), `present_us` (from the end of the last frame
   to the start of this one: the backend's swap and its wait) and
   `frame_us` (`present_us` plus this frame, never less than the four).
   3064 adds heap counts in bytes. No float leaves the backend: a real
   clock gives `u64` microseconds from a fixed start.
5. **A span is a closure on the context,** `ctx.span("name", |ctx| …)`,
   which keeps a `SpanEvent { name, start_us, us, depth }` in the frame's
   record (the spans in the order they ended, at most `MAX_SPANS`, 64,
   the rest counted). A guard holding `&mut Ctx` would lock the context
   for the span's length. The `tracing` crate and Tracy stay out, as
   ADR-0030 keeps `tracing` out of the log: our spans are a few lines and
   need no global subscriber, which would mix test threads.
6. **A heap snapshot** (3065, 2554) is live bytes and counts by memory
   tag: the frame's phase, the game's span, or the asset kind being
   decoded, the innermost one winning; a realloc moves the bytes to the
   current tag. A diff compares two snapshots. No call stacks and no
   single allocations: native tools do those (heaptrack, Instruments,
   Windows Performance Analyzer).
7. **The form agents read is JSON lines,** one object per line tagged by
   `kind` (2553). The types are `shuyi_ui::perf`'s, with serde's
   `Serialize` and `Deserialize` and `deny_unknown_fields`, as the
   contract's answers have, and their JSON schema is the contract's.
   Chrome trace-event JSON, for Perfetto, is derived from the records
   (2325's "Save profile"), never the source.
8. **Fakes make tests exact.** `PerfClock::fake()` starts at 0 and its
   read number *n* (from 0) adds 100 × (*n* mod 7 + 1) microseconds, and
   3064's fake counter moves by fixed amounts, so a trace on the headless
   backend is the same on every run and can be a snapshot.
9. **The counting allocator lives in `shuyi-alloc-count`**
   (`crates/alloc-count`), one more crate where ADR-0002 §9 allows
   `unsafe`, as ADR-0021 §6 did for the platform crates: one
   `#[allow(unsafe_code)]` module, a `// SAFETY:` comment on every
   `unsafe` block. It joins the default engine's crates (ADR-0009 §4 had
   six), since `shuyi-app` and `shuyi-cli` depend on it and no default
   crate may depend on a module. Its `global` feature holds the one
   `#[global_allocator]` static, so the backends that turn it on share it
   (3064 says why).

## Consequences

- An agent reads a run's frame times and the game's own spans as data,
  with no window and no debug view, and the same records reach the
  contract, MCP, a file and the live link.
- The simulation stays pure: the clock is a function the backend hands
  over, read by `App` and the context, never by `shuyi-core`.
- Timings from the headless backend with the fake clock are exact and
  meaningless; real numbers come from `shuyi-app` (or `shuyi-cli`'s real
  clock, 2553), on that machine, in an unoptimized build.
- A release build pays nothing: no clock is taken, no span reads one, and
  no record is kept.
- A span costs two clock reads and a push in a measured frame, and one
  branch otherwise.
- A backend that wants traces implements one method; one that doesn't
  (a console without a timer) leaves it `None`.
- 2325's on-screen profiler draws these records instead of measuring in
  `shuyi-app` on its own.

## Alternatives considered

- **Measure in `shuyi-app` only** (2325's first plan) — agents would get
  nothing from the headless backend, the contract or another backend,
  and the game's spans would need a second path.
- **A clock in the core** — breaks rule 5; a replay could come to depend
  on it.
- **Times as `f32` seconds or `Duration`** — floats leave the backend and
  differ between builds; `Duration` is wider than needed and serializes
  as two fields. Whole microseconds in a `u32` hold over an hour a frame.
- **A guard type for spans** (`let _s = ctx.span("ai");`) — it borrows
  the context for the span's length, so the code inside can't use it.
- **The `tracing` crate with a subscriber, or Tracy** — a global, which
  mixes test threads (ADR-0030's reason), a larger dependency tree, and
  Tracy is a native tool agents can't read.
- **Chrome trace-event JSON as the stored form** — nested and verbose,
  and an agent asking "what is new since frame N" would parse it all;
  it is derived instead (§7).
- **An unbounded record list** — grows without end on a small device;
  600 frames with a count of the dropped says enough, as the log's 500
  lines do.
- **Call stacks per allocation** — needs unwinding in the allocator and
  is what native tools already do well (§6).
