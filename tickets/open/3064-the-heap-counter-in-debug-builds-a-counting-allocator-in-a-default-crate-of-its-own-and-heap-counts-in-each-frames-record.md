---
id: "3064"
title: "The heap counter in debug builds: a counting allocator in a default crate of its own, on in every debug build of a game and its tool program, and heap counts in each frame's record"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3063", "2553"]
nick_input: none
completed:
---

# 3064 — The heap counter in debug builds

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md),
"perf traces agents can read"):

> "we should also have easily agent-readable / streamable perf traces
> when building in debug -- both timings and heap snapshots"

He put them in the MVP. 3063 keeps a `FrameRecord` per frame in `App`
and writes the ADR for the traces; 2553 hands the records to agents
(`shuyi run --perf`, MCP's `perf` tool, a running build's trace file).
This ticket counts the heap and puts its counts in each record. 3065
then tags the bytes, and 2554 puts heap snapshots in the trace.

**What Shuyi has today** (read on 2026-10-24):

- Nothing counts allocations. There is no `GlobalAlloc` anywhere in
  `crates/`, `examples/` or `templates/`.
- The workspace's lints forbid `unsafe_code` (the root `Cargo.toml`).
  `shuyi-app` alone denies it instead, with `#[allow(unsafe_code)]` only
  in `src/pads/web.rs`, and a test,
  `the_lints_are_the_workspace_lints_except_unsafe_code`
  (`crates/app/src/lib.rs:516`), keeps its other lints equal to the
  workspace's. `GlobalAlloc` is an unsafe trait, so a counting
  allocator can't live in `shuyi-core`, `shuyi-ui` or `shuyi-platform`.
  A `#[global_allocator]` static of a type that implements it needs no
  `unsafe` of its own.
- **A crate under `crates/` is a module** unless it is one of the
  default engine's six crates, the front door or `xtask`
  (`crates/xtask/src/modules.rs`, `DEFAULT_CRATES`, line 43), and no
  default crate may depend on a module (ADR-0009 §4, checked by `cargo
  xtask check-modules`). Every feature of the `shuyi` front door must be
  a module's (rule 5 there). So the counter's crate joins the default
  crates (3063's ADR says so), and a game can't turn it on through a
  feature of `shuyi`.
- **One global allocator per binary.** A game's binary links
  `shuyi-app` through `shuyi`; its tool program (`examples/tools.rs`)
  links `shuyi-app` and `shuyi-cli`. Two `#[global_allocator]` statics
  in one binary don't link. So the static lives once, in the counter's
  crate, and both backends turn it on.
- **Planned elsewhere:** 2906 (research on asset memory) and 3008
  (`shuyi bench`) each planned this counting allocator, "a reusable
  `CountingAlloc<A: GlobalAlloc>`" in a crate of its own, suggested as
  `crates/alloc-count` (`shuyi-alloc-count`). 3020 (parked) counts
  allocations per frame with it. 3010 (the Playdate backend) has a
  global allocator over the SDK's `realloc`. 2368 (parked) draws the
  heap in the debug view and is now blocked by this ticket.

## Nick input

None.

## Scope

**In:**
- **`shuyi-alloc-count`** (`crates/alloc-count`), a new crate.
  **Either order with 3008 and 2906:** if either has written it, add
  what is missing below to that crate; if neither has, write it here,
  and they use it (their clauses say so).
  - `CountingAlloc<A: GlobalAlloc>` around an inner allocator, with
    `const fn new(inner: A)`. It counts the bytes asked for (the
    `Layout`'s size), never the inner allocator's own overhead: live
    bytes, the peak of live bytes, and calls to `alloc`, `alloc_zeroed`
    and `realloc`;
  - safe reads: `live_bytes()`, `peak_bytes()`, `reset_peak()`, and
    `take_allocs() -> u32`, the calls since the last take, read and
    reset in one step. `counts() -> Counts { live_bytes, peak_bytes,
    allocs }` reads all three, taking the calls;
  - atomic read-modify-write under `cfg(target_has_atomic = "ptr")`
    (`AtomicUsize` for bytes, enough on a 32-bit target), and a
    single-threaded fallback without it, so 3020's targets build it;
  - `no_std`; a `std` feature for `std::alloc::System` and the `global`
    module;
  - **`global`**, a feature (with `std`): the one `#[global_allocator]`
    static, `CountingAlloc<System>`, under `#[cfg(all(debug_assertions,
    not(shuyi_no_heap_count)))]`, and `global::reading() ->
    Option<Counts>`, which is always there: `Some` where the static is
    compiled, `None` in a release build or with the `--cfg`. Declare the
    cfg for `check-cfg` in a build script. Its docs say why the static
    is here: `shuyi-app` and `shuyi-cli` both turn it on, and a binary
    that links both gets one static (cargo unifies the feature), where
    two would not link;
  - its lints are the workspace's except `unsafe_code = "deny"`, with
    one `#[allow(unsafe_code)]` module, a `// SAFETY:` comment on every
    `unsafe` block, and a test like `shuyi-app`'s
    `the_lints_are_the_workspace_lints_except_unsafe_code`;
  - **a default crate:** it joins `DEFAULT_CRATES` in
    `crates/xtask/src/modules.rs` (seven, and the doc comments that say
    six), and gets its entry in `tools/layers.ron` (`engine: []`,
    `outside: Only([])`, `devices: false`, `floats: false`).
    `shuyi-app` and `shuyi-cli` gain it in their `engine` lists. If the
    front door must name every default crate, add it there too.
- **Plain data, in `shuyi_ui::perf`:**
  - `HeapCounts { live_bytes: u64, peak_bytes: u64, allocs: u32 }`,
    `#[non_exhaustive]`, with `new`: `allocs` is the calls since the
    last reading, so one a frame;
  - `MemoryReading { heap: HeapCounts }`, `#[non_exhaustive]`, with
    `MemoryReading::new(heap)`. 2368 (parked) adds a budget and loads by
    kind to it;
  - `FrameRecord::heap: Option<HeapCounts>`, additive (3063 made
    `FrameRecord` non-exhaustive);
  - `bytes_text(u64) -> String`: under 1,024 bytes as `1023 B`, then
    `KB`, `MB` and `GB` with one decimal, rounded down, in integers
    (`1.0 KB` is 1,024 bytes, `1.0 MB` 1,048,576);
  - serde, and `JsonSchema` under 2553's `schema` feature.
- **`Platform::memory(&mut self) -> Option<MemoryReading>`**, defaulted
  to `None`. `App` asks for it once a frame, only in a frame it makes a
  record of (3063's rule), at the frame's end, and puts its `heap` in
  the record.
- **The headless backend and the Harness:**
  - `HeadlessPlatform::with_fake_memory(self) -> Self`: a reading that
    moves by fixed amounts. With *n* the frames it has presented: live
    bytes 1,048,576 + 16,384 × (*n* mod 30), the peak the most of those
    so far, allocations *n* mod 7;
  - `HeadlessPlatform::set_memory(&mut self, Option<fn() -> HeapCounts>)`,
    for a real counter, and `memory_calls()`, the calls to `memory`, for
    the tests; `Harness::set_memory` sets it.
- **`shuyi-app`** depends on `shuyi-alloc-count` with `global`.
  `Macroquad::memory` gives `global::reading()` as a reading. Its crate
  docs say that every debug build counts the heap, and how a game with
  its own `#[global_allocator]` opts out: `--cfg shuyi_no_heap_count`
  in its `.cargo/config.toml`'s `rustflags`.
- **`shuyi-cli`** depends on it with `global`. `Headless::set_perf(true)`
  (2553) also sets the Harness's memory to `global::reading()` when it
  is `Some` and the platform has no memory of its own (a test's fake is
  kept, as 2553 keeps a fake clock), so `run --perf` and MCP sessions
  read the tool program's heap. The limit when it is `None`: "the heap isn't counted: the
  game's tool program is a release build, or opted out".
- **The summary and the text** (2553's `PerfSummary` and `run::text`):
  `heap: Option<HeapSummary>`, with `peak_bytes` (the most over the
  kept records), `live_bytes` (the last record's) and `allocs` (a
  `Spread` of the calls per frame); the text line ends `heap peak 4.1
  MB`, with `bytes_text`. The schemas regenerated, and the contract's
  version raised by one with its line in `versions.json`.
- **The heap counter counts the whole process,** so other threads'
  allocations count too. The docs say so, and the limit says it when
  more than one thread ran (the tool program runs one game at a time,
  so this is rare).
- **Either order with 3010:** if 3010 is done, `shuyi-playdate`'s debug
  build wraps its SDK allocator in `CountingAlloc` (without `std`) and
  implements `Platform::memory` from it; if not, 3010 does it (its own
  clause says so).
- **Either order with 3020:** if 3020 is done, it already counts calls
  in this crate: use its counter for `take_allocs`; if not, write
  `take_allocs` here, and 3020 uses it (its own clause says so).
- **Docs** (the `document` skill): `docs/platform-backends.md`
  (`Platform::memory`); `docs/contract.md`'s *Perf traces* (each
  record's `heap`, and the summary's); the crate's README-like module
  docs; and `docs/adr/0009-…` or 3063's ADR, whichever names the
  default crates, gets the seventh.
- **The skill pack:** 2553's `shuyi-perf` skill learns the heap. Its
  summary step reads the heap peak and the allocations per frame, it
  says how a game with its own `#[global_allocator]` opts out, and its
  `metadata.version` is raised. Its worked example checks that the
  summary's `heap` is there (never its values).
- The new public types follow ADR-0039.

**Out (do not do):**
- memory tags, live bytes by phase, span and asset kind (3065), and
  heap snapshots and diffs in the trace (2554);
- the debug view's `memory` channel, its readout, a budget line and
  loads by asset kind (2368, parked, which draws these counts);
- the peak-heap gate in CI (3008) and allocations per frame on the GBA
  and the DS (3020, parked);
- call stacks per allocation and leak checks: native tools do those
  (3063's ADR);
- GPU and texture memory.

## Implementation steps

1. Read 3063's ADR and `shuyi_ui::perf`; 2553's `PerfSummary`;
   `crates/xtask/src/modules.rs` and `layers.rs`; `tools/layers.ron`;
   `crates/app/Cargo.toml` and its lints test; and 3008's, 2906's and
   3020's tickets (and their code, if any is done).
2. Write `shuyi-alloc-count`, its lints test and its unit tests, through
   an instance called directly (not the global one, which other test
   threads use).
3. Make it a default crate: `DEFAULT_CRATES`, `tools/layers.ron`, the
   ADR's list.
4. Add `HeapCounts`, `MemoryReading`, `FrameRecord::heap`, `bytes_text`,
   `Platform::memory`, App's read, and the headless backend's fake and
   real memory.
5. Turn it on in `shuyi-app` and `shuyi-cli`, and add the summary's
   heap and the text.
6. Regenerate the schemas, raise the version, write the docs, run the
   gates.

## Acceptance criteria

- [ ] An instance called directly counts a 1 MB block: `live_bytes` and
      `peak_bytes` at least 1 MB while it lives, `live_bytes` back after
      it is freed, `peak_bytes` kept until `reset_peak`, and
      `take_allocs` at least 1, then 0.
- [ ] In a debug test binary that links the crate with `global`,
      `global::reading()` is `Some`; the crate's docs and a test with
      the `--cfg` (or a `cfg` test of the condition) show the opt-out.
- [ ] With the fake memory, the toy's records carry the fake's heap
      counts (a committed snapshot), and `memory_calls()` stays 0 with
      `Ctx::debug_tools` off.
- [ ] `cargo shuyi run --perf` on Hello, Grid answers a summary whose
      heap peak is above 0, and its text line ends with the peak.
- [ ] `cargo xtask check-modules` and `check-layers` pass with the
      seventh default crate.
- [ ] `shuyi-app` and a game made from the template build in debug and
      release; the release build has no `#[global_allocator]`.
- [ ] The schemas are regenerated and `versions.json` has the new
      version.
- [ ] `shuyi-perf` teaches the heap, and `cargo xtask skill-examples
      shuyi-perf` runs its example green.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: live, peak, `reset_peak`, `take_allocs` and `realloc` growing
  and shrinking, on an instance; `bytes_text` (`0 B`, `1023 B`, `1.0
  KB`, `3.2 MB`, `1.0 GB`); the fake's readings on frames 0, 1, 29 and
  30; the summary's heap.
- Property: for random sequences of allocations, reallocations and
  frees on an instance, `live_bytes` equals a model's sum and
  `peak_bytes` its most.
- Integration: `run --perf` on Hello, Grid with the real counter; the
  lints test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
