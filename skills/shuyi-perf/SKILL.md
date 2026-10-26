---
name: shuyi-perf
description: Find what makes a game on Shuyi Engine slow, from the times of its frames and of the spans it marks around its own code, read as JSON lines from a headless run, an MCP session or a running debug build. Use when a frame takes too long, a stage stutters, or a bot's turn takes too long. Not for a wrong result (shuyi-debug-replay), for writing a test (shuyi-test), or for how often bots win (shuyi-balance-bots).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Find what makes a game slow

A debug build of a game measures each of its frames: how long the
update, the drawing, painting into the window and the wait for the next
frame took, in whole microseconds, and how long each span the game
marked around its own code took. It keeps the last 600 frames. You read
them as data: a summary in `run`'s answer, and every frame as a JSON
line.

Times are this machine's, and from an unoptimized build: compare two
runs on the same machine, never numbers across machines. A time never
changes what the game does, so a run plays the same with them on.

## When to use it

- A frame takes too long: the game drops below its frame rate.
- A stage stutters: most frames are fast, a few are slow.
- A bot's turn, a path search or a stage's load takes too long.

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
   from a desktop debug build:

   ```bash
   SHUYI_PERF=perf.jsonl cargo run
   ```

   It logs the file's name and writes each frame's line as the frame
   ends, so `tail -f perf.jsonl` follows it, or read the file again from
   the last line you read. Play the slow part, then read the lines of
   those frames.

A line reads:

```json
{"kind":"frame","frame":12,"times":{"update_us":310,"draw_us":420,"paint_us":610,"present_us":15800,"frame_us":17200},"spans":[{"name":"ai","start_us":40,"us":250,"depth":0}],"spans_dropped":0}
```

`depth` is how many spans it was inside. A frame keeps 64 spans and
counts the rest in `spans_dropped`.

## When it fails

- **`perf` is `null`, with the limit "nothing was measured":** the
  game's tool program was built without debug tools (a release build).
  Run it as `cargo shuyi run` builds it, in debug.
- **"the trace keeps the last 600 frames"**: the run was longer, and its
  first frames are gone. Run fewer frames, or read a session as a
  stream.
- **`SH3004`:** the trace file can't be written: its folder is a file,
  or not yours. A relative path is from the project's folder.
- **`SH3001` from `perf`** on a session attached to a running build:
  the link doesn't carry the frames' records. Run the build with
  `SHUYI_PERF` and read its file.
- **`SHUYI_PERF` writes nothing:** the build has no debug tools; its
  log says so. A file it can't create or write is one warning in the
  log, and the game goes on.
