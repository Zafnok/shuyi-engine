---
name: shuyi-perf
description: Find what makes a game on Shuyi Engine slow or heavy, from the times of its frames and of the spans it marks around its own code, and from the heap, its counts and its snapshots by memory tag (what holds it and what grew), read as JSON lines from a headless run, an MCP session or a running debug build. Use when a frame takes too long, a stage stutters, a bot's turn takes too long, the heap grows as the game plays, or a stage or a target runs out of memory. Not for a wrong result (shuyi-debug-replay), for writing a test (shuyi-test), or for how often bots win (shuyi-balance-bots).
license: MIT OR Apache-2.0
metadata:
  version: "1.3"
---

# Find what makes a game slow

A debug build of a game measures each of its frames: how long the
update, the drawing, painting into the window and the wait for the next
frame took, in whole microseconds, how long each span the game marked
around its own code took, and the heap at the frame's end: the bytes
live, their peak, and the allocations in the frame. It keeps the last
600 frames. You read them as data: a summary in `run`'s answer, and
every frame as a JSON line.

Times are this machine's, and from an unoptimized build: compare two
runs on the same machine, never numbers across machines. A time never
changes what the game does, so a run plays the same with them on.

## When to use it

- A frame takes too long: the game drops below its frame rate.
- A stage stutters: most frames are fast, a few are slow.
- A bot's turn, a path search or a stage's load takes too long.
- The heap grows from frame to frame, or a frame allocates too much.

## When not to

- A wrong result, or two runs that differ: `shuyi-debug-replay`.
- A test that proves a change: `shuyi-test`.
- How often bots win a stage: `shuyi-balance-bots`.

## Steps

1. **Measure the stage.** Play into it and run some frames, measuring
   each:

   ```bash
   cargo shuyi run --script "Enter" --frames 300 --perf --perf-out perf.jsonl --json
   ```

   `result.perf` is the summary: `frames` kept, and a spread each for
   `frame`, `update`, `draw`, `paint` and `present`: `p50` (half the
   frames are this or less), `p99`, `worst` and `mean`, in
   microseconds. Read the frame's `p99` and `worst` first: a `worst`
   far above the `p99` is a stutter, a high `p50` is a slow frame
   everywhere. Then the part that takes it: `update` (the screens' and
   the modules' updates, your rules), `draw` (building the frame),
   `paint` (the window), `present` (waiting for the next frame: a
   high one is a frame that was early). `spans` names the five spans
   with the most time. `perf.jsonl` has every frame as a line, oldest
   first; `file` is its path.

   `heap` is the heap's summary, in bytes the program asked for:
   `peak_bytes`, the most live at once, and `live_bytes` at the last
   frame's end; `allocs` spreads the allocations per frame. A
   `live_bytes` that climbs from line to line in `perf.jsonl` is memory
   kept and never freed; a high `allocs` `p50` is a frame that
   allocates every time (a list built each frame: keep it and clear it
   instead). The text line ends `heap peak 4.1 MB`. The heap counter
   counts the whole tool program, every thread's allocations.

2. **Mark the suspect code** with a span, around a part of an update:

   ```rust
   let plan = ctx.span("ai", |ctx| plan_turn(ctx));
   ```

   The closure runs as before and gives back what it returns; in a
   measured frame the span's time goes into the frame's record. Spans
   nest. Run step 1 again: the span's `count`, `total_us`, `mean_us`
   and `worst_us` are in `spans`, and each frame's line lists its spans.
   Narrow the span until it holds the slow part, change the code, and
   run again to compare on the same machine.

3. **A session, as a stream.** With the MCP server, `start` measures
   the session. After each `step`, call `perf` with the last answer's
   `next` as `since`: it answers the frames since the last read
   (`records`), their `summary`, and `next` for the read after. After
   `reset` the numbers start again: the limit says to read from 0.

4. **A running build, in its window.** The real renderer's times come
   from a desktop debug build: attach to it and follow its frames
   (*A running build*, below). A build no agent attaches to writes its
   own trace when started with a file named:

   ```bash
   SHUYI_PERF=perf.jsonl cargo run
   ```

   It logs the file's name and writes each frame's line as the frame
   ends, so `tail -f perf.jsonl` follows it, or read the file again from
   the last line you read. Play the slow part, then read the lines of
   those frames.

A line reads:

```json
{"kind":"frame","frame":12,"times":{"update_us":310,"draw_us":420,"paint_us":610,"present_us":15800,"frame_us":17200},"spans":[{"name":"ai","start_us":40,"us":250,"depth":0}],"spans_dropped":0,"heap":{"live_bytes":4194304,"peak_bytes":4521984,"allocs":12}}
```

`depth` is how many spans it was inside. A frame keeps 64 spans and
counts the rest in `spans_dropped`. `heap` is left out where the heap
isn't counted.

## A running build

Read the build running in its window, not a headless `run`, when the
window is the point: a stutter only the real renderer shows (`paint`
and `present` are the window's), or a person playing while you read.

1. **Start it with the live link**, in its own terminal:

   ```bash
   SHUYI_LINK=0 cargo run --features link
   ```

   A debug build listens on a free port, and its log's line ending
   `listening on 127.0.0.1:<port>, token <token>` gives the address and
   the token every `attach` below takes.

2. **Follow its frames into a file**, in the background:

   ```bash
   cargo shuyi attach 127.0.0.1:<port> --token <token> --perf-follow perf.jsonl --for 30 --json
   ```

   Every 100 ms it reads the frames run since its last read and appends
   each as a `frame` line, so the file grows as the build runs. It
   prints nothing until it ends, after `--for` seconds (10 if not said,
   at most 3,600) or when the build quits. Meanwhile read `perf.jsonl`
   again from the last line you read, while the slow part is played. Its
   answer is `perf_summary` over every frame it wrote (`file` the file's
   path) and `follow`: `lines`, `dropped`, and `closed` if the build
   quit.

3. **Or read once:** `--perf-since 0` answers the frames the build keeps
   (`perf`), their `perf_summary`, and `perf_next`, the `--perf-since`
   of the next read. With MCP, `start` with `link` attaches the session,
   and `perf` answers as it does on a headless session.

4. **What holds its heap:** add `--perf-snapshot`. With `--perf-follow`
   it takes a snapshot as the following starts and one as it ends; the
   file ends with the two and their `diff`, and the summary has
   `heap_tags` and `heap_growth` (*What holds the heap*, below).

A build faster than the reads, more than 600 frames between two, loses
the frames between: the limit says "N frame(s) were dropped between
reads", and `follow.dropped` counts them. Compare the frames you have,
or follow a build that runs at its own frame rate.

[`example-follow.ron`](example-follow.ron) reads and follows a new
`grid` game this way. It needs the game running in its window, so CI
can't run it (it is marked manual): start the game with the link as its
first comment says, then run it.

## What holds the heap, and what grew

Use it when the heap grows as the game plays (a `live_bytes` that climbs
from frame to frame), or a stage or a target runs out of memory. The
heap is counted by **memory tag**: each block under the tag current
when it was allocated, or last grown. A tag's `class` says what it is:

- `phase`: a part of the frame, `load` (the game's load), `update`,
  `draw`, `paint`, `perf`, or `app` (the rest of the engine's frame);
- `span`: a span you marked, by its name;
- `asset`: an asset kind being decoded, `image`, `palette`, …;
- `other`: `other`, allocated outside any tag, and `overflow`, the
  names past the first 254.

A tag says where the bytes were allocated, not who holds them now: a
list built in `update` and kept counts in `update`.

1. **From a run.** `run --perf` takes a heap snapshot once the game has
   started, before the steps, and one after the last frame:

   ```bash
   cargo shuyi run --script "Enter" --frames 600 --perf --perf-out perf.jsonl --json
   ```

   `result.perf.heap_tags` is what holds the heap at the end: the first
   10 tags, the most `live_bytes` first, each with its `class`, `name`,
   `live_bytes`, `live_blocks`, `allocated_bytes` (ever) and
   `peak_bytes`. `result.perf.heap_growth` is what changed between the
   two: `from` and `to` (frames), `live_bytes` in all (less than 0 for
   fewer), and the 10 largest changes in `tags`. The text line reads
   `heap grew 124.0 KB from frame 0 to 600: update +96.0 KB, image
   +24.0 KB, ai +4.0 KB`. `perf.jsonl` ends with the two `snapshot`
   lines and their `diff`.

2. **In an MCP session.** Call `perf` with `snapshot: true` to take one
   now; the session keeps the last 8, and every answer lists their
   frames in `kept`. Play on with `step`, then call `perf` with
   `snapshot: true` and `diff_from` set to a kept frame: `diff` is what
   changed from it to now (without `snapshot`, to the newest kept). A
   `diff_from` that isn't kept is wrong arguments, `SH0001`, and the
   error lists the kept frames.

3. **Give the suspect code a tag of its own.** A tag that grows (most
   often `update`) holds everything its part of the frame allocates.
   Mark the suspect code with a span (*Steps*, 2): what the closure
   allocates counts under the span's name, so the next diff says how
   much of the growth is that code's. Narrow it as you would a slow
   span.

A running build's trace file (`SHUYI_PERF`) has a snapshot on its first
frame and every 600 frames after, each followed by its diff from the
one before: play, then read the `diff` lines.

## A game with its own allocator

Every debug build counts the heap with a global allocator the engine
turns on, and a program has one global allocator. A game with a
`#[global_allocator]` of its own fails to link until it opts out, in
its `.cargo/config.toml`:

```toml
[build]
rustflags = ["--cfg", "shuyi_no_heap_count"]
```

Its frames then have no `heap`, and the times are still measured.

## When it fails

- **`perf` is `null`, with the limit "nothing was measured":** the
  game's tool program was built without debug tools (a release build).
  Run it as `cargo shuyi run` builds it, in debug.
- **`heap` is `null`, with the limit "the heap isn't counted":** the
  tool program is a release build, or the game opted out
  (`shuyi_no_heap_count`, above).
- **`heap_tags` empty and `heap_growth` `null`, with the limit "the
  heap isn't counted by memory tag":** the heap was counted without
  tags, so no snapshot was taken. A `perf` call with `snapshot` answers
  `snapshot` `null` and the same limit.
- **The limit "the heap counts the whole tool program":** another
  thread ran beside the game (the MCP server reads its messages on
  one), and its allocations count too. Compare the heap over many
  frames, not one frame's `allocs`.
- **"the trace keeps the last 600 frames"**: the run was longer, and its
  first frames are gone. Run fewer frames, or read a session as a
  stream.
- **`SH3004`:** the trace file can't be written: its folder is a file,
  or not yours. A relative path is from the project's folder.
- **`SH1006` from `attach`:** the build speaks another link version: it
  was built with another engine. Attach with the `shuyi` of the build's
  own engine.
- **`SH3006` from `attach`** while following: another client is
  attached (one at a time), or the link was lost. A build that quits
  ends the following early instead, with `follow.closed` and the limit
  "it quit", and the file keeps what was written.
- **`SHUYI_PERF` writes nothing:** the build has no debug tools; its
  log says so. A file it can't create or write is one warning in the
  log, and the game goes on.
