---
id: "2382"
title: "Parked: GPU frame times in the frame profiler: wgpu timestamp queries per backend, a GPU time in FrameTimes, and its ticks in the graph, the readout and the trace file"
type: feature
milestone: "Engine: debug view"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2325", "3034", "2018"]
nick_input: none
completed:
---

# 2382 — GPU frame times in the frame profiler

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later", for the frame profiler 2325; and Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`, for the wgpu renderer that 3034's research plans): a follow-up of both, and a follow-up of parked work is parked. It needs 2325's profiler and the renderer's GPU, so it starts after 2325 and the renderer's ticket are unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2325 (parked) adds a frame profiler to the debug view: the time of
each phase of a frame (update, draw, paint, present), read with the
backend's clock; spans a game marks; a `profile` channel with a bar
per frame; a readout line; and a trace file. Those are CPU times. Its
**Out** leaves GPU times to this ticket, because they need timestamp
queries on the wgpu renderer.

A CPU time can't say what the GPU did. The CPU may record a frame's
draws in 0.3 ms while the GPU takes 12 ms to run them. That wait shows
up only as a long `present`, mixed with the wait for vsync and the
frame cap. A developer whose game misses its frame budget needs to
know which side is slow: the CPU (the update, the painting) or the
GPU (too many pixels, a heavy effect pass). Godot's Visual Profiler
shows CPU and GPU time per frame, and Bevy's `RenderDiagnosticsPlugin`
reads GPU times through wgpu's timestamp queries.

[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§1: graphics go through wgpu, and our own 2D renderer sits on it. 3034
is the research that plans that renderer and writes its tickets. Its
scope asks it to leave room for GPU times in the renderer's ADR, and
to record whether wgpu's timestamp queries give them on each backend.
When it writes the renderer's ticket, it adds that id to this
ticket's `blocked_by`, and the ticket whose backend first runs the
renderer in a window, if that is another one.

**What wgpu gives** (its documentation; check each name against the
version 3034's ADR pins, since wgpu renames things between releases):

- `Features::TIMESTAMP_QUERY`, an optional feature. An adapter that
  has it lists it in `Adapter::features()`. A device asks for it in
  `DeviceDescriptor::required_features`, and asking for a feature the
  adapter lacks fails.
- With it: a `QuerySet` of `QueryType::Timestamp`, and a render pass's
  `timestamp_writes` (`RenderPassTimestampWrites`), which write a
  timestamp at the pass's beginning and at its end.
- `CommandEncoder::resolve_query_set` copies the timestamps (`u64`
  ticks) into a buffer. That is copied to a mappable buffer and read
  with `map_async`, a frame or more later.
- `Queue::get_timestamp_period()`: nanoseconds per tick, an `f32`.

Which backends give timestamps (Vulkan, Metal, Direct3D 12, GL and
GLES, WebGL 2, and WebGPU in a browser) is what 3034's report
records. A browser may also round the times it gives.

The rules this keeps (`AGENTS.md`):

- **The core stays pure** (rule 5). The renderer, a backend crate,
  measures. The time reaches the debug view as plain data in
  `FrameTimes` (3063's, in `shuyi_ui::perf`, which 2325 draws) and
  never reaches an update. A replay has the same hash with GPU times on
  and off.
- **Graphics are a skin** (rule 9). The ticks are the debug view's
  gizmos on 2325's channel; tests of what the channel shows read the
  gizmo list.
- **Debug tools only.** A build without debug tools never asks the
  device for timestamp queries and writes none.

## Nick input

None.

## Scope

**In:**
- **`GpuTime`**, in `shuyi_ui::perf` beside `FrameTimes` (3063). A
  `#[non_exhaustive]` enum (ADR-0039 §6), since a backend may later
  give more:
  - `NoTimer`: the backend has no GPU timer (a backend without the
    wgpu renderer, the headless one without its fake, a build without
    debug tools);
  - `Unsupported { api: &'static str }`: the renderer runs, but its
    adapter has no timestamp queries. `api` names the wgpu backend as
    a person says it (`"Vulkan"`, `"Metal"`, `"Direct3D 12"`, `"GL"`,
    `"WebGL 2"`, `"WebGPU"`), so the label can say why;
  - `Pending`: the timer runs, and no time came back this frame;
  - `Measured { frames_ago: u8, us: u32 }`: the GPU time of the frame
    `frames_ago` frames before this one, in whole microseconds. GPU
    results come back late, so `frames_ago` is usually 1 to 3.
- **`FrameTimes::gpu: GpuTime`**: `NoTimer` from `FrameTimes::new`,
  assigned by the backend (ADR-0039 §3). 3063 makes `FrameTimes`
  `#[non_exhaustive]`, so the field is additive. If it isn't, the
  field is a break: write a `Breaking:` line in the completion notes.
  `FrameTimes` is in the trace agents read (2553), so the field joins
  its JSON lines and its schema: regenerate the schemas and raise the
  contract's version.
- **The ring** (2325's, in `DebugView`):
  - each frame's slot gains `gpu_us: Option<u32>`;
  - a `Measured` writes its `us` into the slot `frames_ago` frames back
    from this frame's, if that slot is still in the ring, and is
    dropped if not;
  - the ring keeps the last `NoTimer` or `Unsupported` it saw, for the
    label;
  - the GPU's p50, p99 and worst frame, over the slots that have a GPU
    time, with 2325's percentile function.
- **The `profile` channel** (2325's):
  - a tick per frame: a `Line` across the frame's column at the height
    of its GPU time, on the same scale as the bars, so a tick above its
    bar is a frame the GPU held up. Its role is one 2325's phases don't
    use (2325's channel lists the roles it uses). A GPU time above the
    graph's top is drawn at the top. A frame with no GPU time has no
    tick;
  - a label after 2325's labels, once a GPU time has come:
    `gpu p50 2.3 ms  p99 3.1 ms  worst 4.0 ms`, numbers in 2325's form;
  - with `Unsupported`, the label `gpu n/a: no timestamp queries on
    WebGL 2` (its `api`) and no ticks;
  - with `NoTimer`, nothing: the channel draws exactly what it drew
    before, so a backend without a GPU timer changes nothing.
- **The readout** (2325's line): `frame 4.1 ms (p99 7.9)` gains
  `  gpu 2.3 ms (p99 3.1)` once a GPU time has come, with the
  statistic 2325's line uses for the frame. With `NoTimer` or
  `Unsupported`, the line is 2325's.
- **The trace file** (2325's "Save profile"): one counter event per
  frame that has a GPU time, `{"name": "gpu", "ph": "C", "ts": <the
  frame's start on the CPU clock>, "pid": <2325's>, "args": {"gpu_us":
  875}}`. Perfetto and `chrome://tracing` draw counters as a graph.
  The GPU's clock is not the CPU's: the length is measured, and the
  place is its frame's start. The writer's doc comment says so.
- **The fake** (3063's fake clock, used by 2325's `frame-png --debug
  profile`): it gains GPU times. On frames 0 and 1 it gives `Pending`;
  on frame *n* after that, `Measured { frames_ago: 2, us: 1_500 + 250
  × (n mod 8) }`. A second form, for tests, gives `Unsupported { api:
  "WebGL 2" }`. 2325's committed `profile` snapshot is redrawn with
  the ticks and the label, reviewed and committed.
- **The timer, in the wgpu renderer** (the crate 3034's tickets build;
  "the renderer" here), in a module of its own:
  - an option the backend sets when it makes the renderer,
    `gpu_times: bool`, true only when `shuyi_ui::DEBUG_TOOLS` is on.
    With it off, the renderer never asks for `TIMESTAMP_QUERY`, writes
    no timestamps and hands `NoTimer`;
  - with it on, the renderer asks for `TIMESTAMP_QUERY` when the
    adapter lists it. If the adapter doesn't, it hands
    `Unsupported { api }`, named from `AdapterInfo::backend` (GL on
    wasm is `"WebGL 2"`);
  - four slots, each a `QuerySet` of two timestamps, a resolve buffer
    (`QUERY_RESOLVE | COPY_SRC`, 16 bytes) and a read buffer
    (`MAP_READ | COPY_DST`). Each frame takes a free slot. The frame's
    first render pass writes timestamp 0 at its beginning, and its last
    pass writes timestamp 1 at its end, whatever passes the renderer
    records; a frame of one pass writes both on it. After the passes,
    the renderer resolves the set, copies it to the read buffer,
    submits, and maps the buffer. With no free slot, the frame is not
    timed;
  - each frame, it polls the device without waiting (in the browser,
    the browser polls) and reads each slot whose map has finished: the
    end minus the beginning, in ticks, times
    `get_timestamp_period()`, rounded to whole microseconds.
    `frames_ago` is the frames since that slot was submitted. A pair
    whose end is before its beginning (some drivers do this) is
    dropped. It hands the newest finished time as `Measured`, drops an
    older one that finished in the same frame, and hands `Pending`
    when none finished;
  - `gpu_time(&mut self) -> GpuTime` on the renderer (named to fit its
    API), called once a frame after present;
  - the period is an `f32`, so the conversion stays in the renderer, a
    backend crate where floats are allowed (`tools/layers.ron`); only
    the integer leaves it.
- **The backend** that runs the renderer sets `FrameTimes::gpu` from
  `gpu_time` where it fills the rest of `FrameTimes`. If that backend
  doesn't fill `FrameTimes` yet (2325 measured only in `shuyi-app`),
  first add 2325's phase measuring to it: the same few lines around
  update, draw, paint and present. Every other backend leaves
  `NoTimer`.
- **Each backend, checked:**
  - a test in the renderer's crate, `#[ignore]` in `cargo test
    --workspace` (which has no GPU), that 2018's `render` job runs on
    lavapipe (a new step in that job of `.github/workflows/ci.yml`,
    `-- --ignored gpu_times`). It makes the renderer with `gpu_times`
    on, draws 8 frames of a test frame to an offscreen target, and
    polls for up to 30 frames. If the adapter lists `TIMESTAMP_QUERY`,
    at least one frame gives `Measured`, with `frames_ago` at most 4
    and `us` no more than the test's own wall time. If it doesn't,
    every frame gives `Unsupported { api: "Vulkan" }`. The test prints
    which;
  - the same test, run by hand on each backend the working machine
    has (on Windows: Direct3D 12, Vulkan and GL, the backend chosen as
    the pinned wgpu lets a test choose it), and the web build in a
    browser with the `profile` channel on (WebGPU, and WebGL 2 in a
    browser without WebGPU);
  - `docs/platform-backends.md` gains a section, *GPU times in the
    profiler*, with one row per wgpu backend: whether timestamp
    queries give times, any rounding, what the profiler shows, and
    where the answer came from (the `render` job, a run by hand with
    its date and machine, or 3034's report for a backend nobody here
    could run, such as Metal).

**Out (do not do):**
- GPU time per render pass, as named spans in the ring and the trace:
  a ticket if a developer asks;
- lining the GPU's clock up with the CPU's in the trace: the counter
  sits at its frame's start;
- timestamps inside a pass or an encoder
  (`TIMESTAMP_QUERY_INSIDE_PASSES`, `TIMESTAMP_QUERY_INSIDE_ENCODERS`),
  and pipeline statistics queries;
- what the compositor does after the last pass: the time runs from the
  first pass's beginning to the last pass's end;
- GPU and texture memory;
- GPU times on a backend without the wgpu renderer (macroquad while it
  lasts, the headless backend, the Playdate): `NoTimer`;
- GPU times in CI budgets or in device logs: a time on a CI runner's
  software adapter says nothing about a player's GPU;
- Tracy's GPU zones.

## Implementation steps

1. Read 3034's report (its rows on timestamp queries) and its ADR (the
   renderer's passes and the pinned wgpu version); the renderer's
   crate and the backend that runs it; 2325's
   `shuyi_ui::debug::profile` (`FrameTimes`, the ring, the
   percentiles, the channel and its roles, the readout, the trace
   writer and the fake); and 2018's `render` job.
2. Add `GpuTime` and `FrameTimes::gpu`, with doc comments.
3. Add the ring's GPU slot, the late write and the GPU's p50, p99 and
   worst frame, with unit tests.
4. Add the ticks, the label, the readout's GPU part and the trace's
   counter.
5. Give the fake its GPU times. Run `cargo shuyi frame-png --debug
   profile` where 2325 ran it, review the PNG, and commit the redrawn
   snapshot.
6. Write the renderer's timer and `gpu_time`, with the `gpu_times`
   option, and set `FrameTimes::gpu` in the backend.
7. Write the ignored test and add its step to the `render` job.
8. Run it by hand on each backend the machine has, and the web build
   in a browser. Write the table in `docs/platform-backends.md`.
9. Run the gates.

Size: expect about 500 changed lines, without the snapshots.

## Acceptance criteria

- [ ] With the fake GPU times, `frame-png --debug profile` draws a
      tick on each frame's column after frame 1 and the `gpu` label.
      The PNG is reviewed, and its snapshot is committed.
- [ ] With `NoTimer`, the `profile` channel's gizmo list and the
      readout are the same as without this ticket (a test against a
      list built with 2325's code path).
- [ ] With `Unsupported { api: "WebGL 2" }`, the label reads
      `gpu n/a: no timestamp queries on WebGL 2`, and there are no
      ticks.
- [ ] A time two frames late lands in its own frame's slot, and one
      older than the ring is dropped.
- [ ] A replay has the same state hash with the fake GPU times on and
      off.
- [ ] With `gpu_times` off, the renderer asks for no
      `TIMESTAMP_QUERY` (the device's features say so) and writes no
      timestamps.
- [ ] In the `render` job on lavapipe, the ignored test gives
      `Measured` or `Unsupported` as the adapter's features say, and
      its log says which.
- [ ] The saved trace parses as a trace-event file with a `gpu`
      counter on each frame that has a GPU time, and its snapshot from
      the fake is committed.
- [ ] `docs/platform-backends.md` has the table, one row per wgpu
      backend, each saying where its answer came from.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a late time written into its frame's slot, and one too old
  dropped; the GPU's p50, p99 and worst over slots with and without
  times; the label for each `GpuTime`; the readout's GPU part; a time
  above the graph's top drawn at the top; the ticks-to-microseconds
  conversion (a period of 1.0 and a fractional one, rounding, and an
  end before its beginning giving no time); the four slots with none
  free leaving a frame untimed.
- Property: with any GPU times, every tick lies inside the graph's
  rectangle, and p50 ≤ p99 ≤ worst.
- Snapshot / integration: the `profile` channel with the fake GPU
  times; the trace file with its `gpu` counters; the replay hash with
  GPU times on and off; the ignored test in the `render` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
