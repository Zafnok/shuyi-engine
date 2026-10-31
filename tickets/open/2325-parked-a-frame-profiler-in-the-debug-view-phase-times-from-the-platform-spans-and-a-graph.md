---
id: "2325"
title: "Parked: A frame profiler in the debug view: time per phase from the platform, spans a game marks, a graph, and p50, p99 and the worst frame"
type: feature
milestone: "Engine: debug view"
model: implementation
effort: high
status: blocked
blocked_by: ["2315", "2316", "2321", "3063", "2553"]
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
- 3011 and 3013: frame times measured on devices, in logs;
- 3063 (in the MVP, since Nick asked on 2026-10-24 for "easily
  agent-readable / streamable perf traces when building in debug"):
  each frame's times (update, draw, paint, present, the whole frame)
  and the spans a game marks (`ctx.span("ai", |ctx| …)`), measured with
  a clock the platform hands in and kept by `App` as a `FrameRecord` per
  frame. 2553 and 2555 hand them to agents as JSON lines.

Nothing shows where a frame's time goes while a developer plays. This
ticket draws 3063's records in the debug view.

The core has no clock and must not get one (`AGENTS.md` rule 5). 3063
keeps it so: only the backend reads the time, and with debug tools off
nothing is measured.

## Nick input

None.

## Scope

**In:**
- **From 3063** (a blocker): `shuyi_ui::perf`'s `FrameTimes`,
  `SpanEvent`, `FrameRecord`, `PerfClock` and its fake; `spread` (p50,
  p99, the worst and the mean), which 2553 writes (it moved there from
  3063); spans (`Ctx::span`); the
  platform's clock; and the records `App` makes each frame
  (`App::perf`). This ticket draws them, in
  `shuyi_ui::debug::profile`:
  - `Game::set_perf(&FrameRecord)`, beside `set_ticks` in
    `crates/ui/src/game.rs`, which `App` calls with each record it
    makes;
  - a ring of the last 240 records in `DebugView` (2315), never saved
    with it, with `spread`'s p50, p99 and the worst frame over it, and
    each span's total per frame.
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
    (`debug::memory::Ring<T>`) for the 240 frames and the bars, and its
    `Game::set_perf` if it added it, and draw this graph below the
    memory graph when both channels are on. If not, write the ring here,
    as a `Ring<T>` whose length is set when it is made (240 here), and
    `Game::set_perf`, and 2368 uses them and the bar drawing.
- **The readout** (2316): one line, "frame 4.1 ms (p99 7.9)". It comes
  before 2368's heap line.
- **`frame-png --debug profile`** works with 3063's fake clock, which
  moves by fixed amounts per read, so its snapshot is stable. Where
  2315's `--debug` reaches the game's Harness, `profile` among the
  channels starts it on `HeadlessPlatform::new().with_clock(
  PerfClock::fake())`.
- **Saving a profile:** a "Save profile" entry on the debug menu writes
  the ring as Chrome trace-event JSON (`profile.json`; Perfetto and
  `chrome://tracing` open it) through the platform's storage: each
  phase and each span a complete event (`"ph": "X"`) at its place in
  its frame. Debug tools only.

**Out (do not do):**
- measuring: the clock, the phases, spans and `App`'s records are
  3063's, for every backend;
- the trace agents read: 2553 (JSON lines from `shuyi run`, MCP and a
  running build's file) and 2555 (over the live link), both in the MVP;
- GPU times (they need timestamp queries on the wgpu renderer): 2382
  (parked) adds them once 3034's renderer exists;
- memory: 2368 (parked), which shares this ticket's ring and bars
  (either order);
- Tracy or the `tracing` crate (a follow-up if a developer asks);
- log messages with levels and categories (3040), and the shipped
  build's log file (3041): this ticket measures time only;
- budgets in CI (3008).

## Implementation steps

1. Read 2315's ADR and `debug` module, 2316's readout, 2321's cap,
   3063's ADR and `shuyi_ui::perf`, and 2368's ring if it is done.
2. Add `Game::set_perf` and the ring of records.
3. Add the channel and the readout line.
4. Add the trace JSON writer and the debug menu entry.
5. Run the gates.

## Acceptance criteria

- [ ] With the fake clock, the channel's snapshot is committed and
      reviewed with `frame-png --debug profile`.
- [ ] The replay hash is the same with the profiler on and off.
- [ ] The saved JSON parses as a trace-event file, and a snapshot of it
      from the fake clock is committed.
- [ ] Without debug tools the channel is never offered.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the ring wrapping; the five spans with the largest mean; the
  trace JSON.
- Property: with any records, every bar lies inside the graph's
  rectangle.
- Snapshot / integration: the `profile` channel; the replay hash on and
  off.

## Completion notes

*(Filled in by the session that completes the ticket.)*
