---
id: "2325"
title: "Parked: A frame profiler in the debug view: time per phase from the platform, spans a game marks, a graph, and p50, p99 and the worst frame"
type: feature
milestone: "Engine: debug view"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2315", "2316"]
nick_input: none
completed:
---

# 2325 — A frame profiler in the debug view

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't ask for it, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the debug view (2315, 2316). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6. Godot
has a Profiler and Monitors in its debugger; Bevy has frame-time
diagnostics and Tracy through `trace_tracy`. Shuyi has:

- 2315: gizmos drawn over the frame, and channels turned on from the
  debug menu, under a read-only rule;
- 2316: pause, step, slow motion and a readout;
- 3008: budgets in CI, as exact proxies (instruction counts, memory);
- 3011 and 3013: frame times measured on devices, in logs.

Nothing shows where a frame's time goes while a developer plays. This
ticket adds that to the debug view.

The core has no clock and must not get one (`AGENTS.md` rule 5). Only
the platform backend reads the time. So the backend measures the
phases and hands the numbers in, and a game's own spans read a clock
the backend hands in. With debug tools off, nothing is measured.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::debug::profile`:**
  - `FrameTimes { update_us, draw_us, paint_us, present_us, frame_us }`,
    which the platform hands to `Game::frame` as an `Option` (the
    headless backend hands `None`, or a fake for tests);
  - a ring of the last 240 frames in `DebugView` (2315), with p50, p99
    and the worst frame over it;
  - **spans:** `Ctx::span(name: &'static str) -> Span`, a guard that
    reads the clock on creation and on drop. The clock is a
    `fn() -> u64` (microseconds) the platform puts in `Ctx` when debug
    tools are on; with no clock a span does nothing. Spans nest; the
    ring keeps the total per name per frame.
- **The read-only rule holds:** times never reach an update. A test
  plays a record with the profiler on and off and gets the same hash.
- **A `profile` channel** (2315's channels):
  - one bar per frame for the last 120 frames (`Rect` gizmos), its
    height the frame's time, split by phase;
  - a line at the frame budget: the cap's frame time from 2321
    (16.7 ms at 60);
  - labels: p50, p99, the worst frame, and the five spans with the
    largest mean;
  - **either order with 2368:** if 2368 is done, use its ring
    (`debug::memory::Ring<T>`) for the 240 frames and the bars, and draw
    this graph below the memory graph when both channels are on. If not,
    write the ring here, as a `Ring<T>` whose length is set when it is
    made (240 here), and 2368 uses it and the bar drawing.
- **The readout** (2316): one line, "frame 4.1 ms (p99 7.9)". It comes
  before 2368's heap line.
- **Measuring in `shuyi-app`:** around update, draw, paint and present,
  with the backend's clock, through 3003's `Platform`.
- **`frame-png --debug profile`** works with the fake clock, which
  moves by fixed amounts per phase and span, so its snapshot is stable.
- **Saving a profile:** a "Save profile" entry on the debug menu writes
  the ring as Chrome trace-event JSON (`profile.json`; Perfetto and
  `chrome://tracing` open it) through the platform's storage. Debug
  tools only.

**Out (do not do):**
- GPU times (they need timestamp queries on the wgpu renderer; a
  follow-up after 3034);
- memory: 2368 (parked), which shares this ticket's ring and bars
  (either order);
- Tracy or the `tracing` crate (a follow-up if a developer asks);
- log messages with levels and categories (3040), and the shipped
  build's log file (3041): this ticket measures time only;
- budgets in CI (3008).

## Implementation steps

1. Read 2315's ADR and `debug` module, 2316's readout, 2321's cap, and
   2368's ring if it is done.
2. Write `FrameTimes`, the ring, the percentiles and spans, with a fake
   clock.
3. Add the channel and the readout line.
4. Measure the phases in `shuyi-app`.
5. Add the trace JSON writer and the debug menu entry.
6. Run the gates.

## Acceptance criteria

- [ ] With the fake clock, the channel's snapshot is committed and
      reviewed with `frame-png --debug profile`.
- [ ] The replay hash is the same with the profiler on and off.
- [ ] p50, p99 and the worst frame are right on a known list of times.
- [ ] A span with no clock costs nothing (no clock calls, by a counting
      fake).
- [ ] The saved JSON parses as a trace-event file, and a snapshot of it
      from the fake clock is committed.
- [ ] A shipped build (no debug tools) never measures (a test with the
      flag off).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: percentiles; the ring wrapping; nested spans; the trace JSON.
- Property: totals per frame never exceed the frame's time with the fake
  clock.
- Snapshot / integration: the `profile` channel; the replay hash on and
  off.

## Completion notes

*(Filled in by the session that completes the ticket.)*
