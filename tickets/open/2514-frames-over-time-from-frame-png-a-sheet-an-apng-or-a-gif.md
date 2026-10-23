---
id: "2514"
title: "Frames over time from frame-png: --every and --count make a sheet labelled with times, an APNG or a GIF, and the MCP frame_png tool returns the sheet"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["1133"]
nick_input: none
completed:
---

# 2514 — Frames over time from frame-png: a sheet, an APNG or a GIF

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Nick put the features most 2D games expect after the
MVP, not parked ([`docs/decisions.md`](../../docs/decisions.md),
2026-10-20, "Split by group"); this is one of them.

The benchmarks record motion from a running window:

- Godot's Movie Maker mode runs the game at a fixed frame rate and
  writes PNG sequences or AVI
  (docs.godotengine.org/en/stable/tutorials/animation/creating_movies.html);
- Bevy, since 0.18, has `EasyScreenshotPlugin` and
  `EasyScreenRecordPlugin` in `bevy_dev_tools` (bevy.org/news/bevy-0-18);
- Unity's Recorder package writes image sequences and movies;
- the agent servers 1133 names (Godot's community MCP servers, Unity's
  and Unreal's MCP plugins, `bevy_brp_mcp`) take one screenshot per
  call.

Shuyi has, today:

- `frame-png` (`crates/cli/src/frame_png.rs`): it plays a `Script` of
  keys, pad buttons and waits in the game's Harness and writes **one**
  PNG. `GameTools::frame` (`crates/cli/src/game.rs`) returns one
  `Shot`. One wait is at most `MAX_WAIT` (`frame_png.rs`), which is
  `MAX_FRAMES` (2 000) frames of `FRAME_DT` (1/60 s)
  (`crates/platform/src/harness.rs`).
- `encode_png` in `crates/cli/src/codec.rs`, over `png` 0.18
  (`crates/cli/Cargo.toml`). `png` 0.18 also writes APNG
  (`Encoder::set_animated`, `Writer::set_frame_delay`). No GIF crate is
  in `Cargo.lock`.
- `contract::session::Session` (`crates/cli/src/contract/session.rs`)
  plays a script and idles frames (`Session::idle`). 1133 plans the MCP
  session tool `frame_png` over it, which returns one image per call.
- Picture snapshots: `examples/hello-grid/tests/it/snapshot.rs` checks
  painted frames as PNG bytes (`insta::assert_binary_snapshot!`). CI's
  `test` job runs them on Windows, Linux and macOS, so the painter's
  `f32` blending already gives the same pixels on all three.

Nothing plans more than one frame per call. 3007 and 3019 draw contact
sheets of baked images, not of frames over time. 2803 leaves out video.

Why it matters:

- **The game first.** An agent can't watch an animation. A session
  working on the game checks a text box typing on, a walk on the grid, a
  cinematic shot or a transition by stepping and calling `frame-png`
  again and again. One picture of 8 frames at known times shows the
  same in one call.
- **Other 2D games.** Every ticket that adds motion can prove it with
  a sheet: tweens (2109), screen transitions (2339), shake and flash
  (2340), rotated sprites (2335), and clips and animated tiles (2231,
  2359) when unparked.
- **The end goal** (2026-10-19): an agent makes a game in under an
  hour. Seeing motion without a window is part of that.
- The Harness's frames are exact, so a capture is the same on every
  run and every OS, offline, and can be a snapshot. A recording of a
  window can promise none of that.
- APNG and GIF files go in PRs, READMEs, the website (2801) and store
  pages.

## Nick input

None.

## Scope

**In:**
- **`Script::then_frames(n) -> Script`** (`crates/cli/src/frame_png.rs`):
  a copy with `n` more `Step::Wait(FRAME_DT)` steps. Each such wait
  runs exactly one frame (`Harness::advance` subtracts `FRAME_DT` from
  `FRAME_DT` and stops at zero). `Step`, `Script` and the contract's
  schemas don't change, so the contract's `VERSION` doesn't move.
- **`Capture { every: u32, count: u32 }`** in `frame_png.rs`: `count`
  frames, `every` frames apart, the first right after the script.
- **`GameTools::frames(options, script, scale, capture) -> Result<Vec<Shot>, String>`**,
  a provided method in `crates/cli/src/game.rs`. Its default calls
  `frame` once per capture with `script.then_frames(i * every)`. Every
  game that implements `GameTools` today, the game among them, gets it
  with no change. A game whose `frame` is slow overrides `frames` to
  play once. The toy (`crates/cli/src/toy.rs`) overrides it: build the
  painter once, play the script, paint, run `every` frames with
  `Harness::frame(&[], FRAME_DT)`, paint again.
- **`frame_png::Options`** gains one field, `capture: Option<(Capture,
  Output)>`, where `Output` is `Sheet { columns: Option<u32> }`, `Apng`
  or `Gif`. A field added to a public struct is an API change: say so
  in the PR (only this crate's tests build `Options` today).
- **New `frame-png` flags** (`parse_args`, `usage`), matched before the
  game's options, as `--scale` is:
  - `--every <seconds>`, read like `--wait` and rounded to whole frames
    of 1/60 s. A value that rounds to 0 frames is an error naming the
    shortest, 1/60 s;
  - `--count <n>`, from 1 to `MAX_CAPTURES` (64, a new constant), with
    `every × (count − 1)` at most `MAX_FRAMES`;
  - `--columns <n>`, for the sheet only;
  - the output: `--sheet` (the default when `--count` is given),
    `--apng` or `--gif`, one of them.
  - `--every` without `--count`, `--count` without `--every`, an
    output flag or `--columns` without `--count`, or two output flags
    are errors with the usage, as other bad arguments are today.
  - The output path is written as given: no extension is added or
    checked.
  - Without `--count`, `frame-png` writes one PNG exactly as now.
- **The sheet** (`pub mod sheet`, `crates/cli/src/frame_png/sheet.rs`):
  `sheet::build(shots: &[Shot], every: u32, columns: Option<u32>) ->
  Result<Image, String>`, one picture:
  - cells left to right, then top to bottom;
  - `columns` from 1 to `count`; the default is the smallest `n`
    with `n × n ≥ count`;
  - a 2 px gutter, and a fixed dark grey background (`#202020`);
  - above each cell, a strip 14 px tall with the cell's time since the
    script ended, as `+<ms>ms` (`frames × 1000 / 60`, rounded down). It
    is drawn in light grey (`#e0e0e0`) with a 3 × 5 pixel font in the
    code, for the digits, `+`, `m` and `s`, at twice its size, and cut
    at the cell's right edge. The labels are the tool's, not a game's
    text.
  - It writes `Image`'s public fields (`width`, `height`, `rgba`)
    directly and calls none of its private methods, so it builds the
    same before and after 2332.
  - A sheet wider or taller than 16 384 px is an error that names its
    size and suggests a smaller `--scale` or other `--columns`. Nothing
    is written.
- **APNG** (`encode_apng` in `crates/cli/src/codec.rs`): every frame
  at full size, `set_animated(count, 0)` (0 plays: it loops forever),
  each frame's delay `every / 60` s (`set_frame_delay(every, 60)`;
  `every` is at most `MAX_FRAMES`, so it fits the `u16`).
- **GIF** (`encode_gif` in `crates/cli/src/codec.rs`):
  - the `gif` crate, `MIT OR Apache-2.0`, as is its LZW crate `weezl`;
    `deny.toml` allows both licenses. Its default features pull in
    `color_quant`, a quantizer we don't call: take
    `default-features = false` with only the features writing needs
    (check its `Cargo.toml` on the day; `cargo deny check` passes);
  - an **exact palette** of RGB colours (the painter's pixels are
    opaque), built in scan order: one global palette when all frames
    together have at most 256 colours, else one per frame. A frame with
    more than 256 colours is an error naming the frame and its count,
    suggesting `--apng`. Never quantize or dither silently;
  - each frame's delay is `every` frames in hundredths of a second,
    rounded, at least 2 (browsers slow a GIF whose delay is under 2).
    When that isn't exact, the summary says the speed the GIF plays at;
  - it loops forever (`Repeat::Infinite`).
- **The summary** `run` prints: the count, `every` in seconds and
  frames, the output path and its width × height, then one line per
  frame with its time and its top screen (`Shot::shows`). A warning
  (`Shot::warning`, bought art) prints once if any frame has one.
- **`lib.rs`'s command list** says `frame-png` also writes a sheet, an
  APNG or a GIF.
- **The MCP session tool `frame_png`** (1133, which this ticket waits
  for) takes optional `every` (seconds), `count` and `columns`. With
  `count` above 1 it paints the session's frame as 1133 does, then
  `count − 1` times runs `every` frames with `Session::idle` and paints
  again. It returns the sheet as one image and the cells' times and
  screens as text. The session stays where the last capture left it,
  and the tool's description says so. APNG and GIF stay on the CLI: an
  agent reads the sheet, and a person wants the file.
  **Either order with 2521** (the live link): if 2521 is done, a
  session attached over a link refuses `every` and `count` ("one
  picture at a time over the link"), with a test. If not, 2521 refuses
  them (its own clause says so).
- **The order of the work:** the flags and `frames`, the sheet, the
  APNG, the MCP tool, then the GIF. If the PR would pass about 600
  changed lines (not counting snapshots), leave the GIF out: `--gif` is
  not added, and a ticket for it is written (`write-ticket`).
- **The skill pack. Either order with 1134:** if 1134 is done,
  `shuyi-test`, `shuyi-animate` and `shuyi-cinematic` say to check
  motion with a sheet, and `shuyi-ship` says how to make an APNG or a
  GIF for a README or a store page. If 1134 isn't done, 1134 writes
  those lines.
- **Either order with 2315:** if 2315 is done, `--debug` draws its
  channels on every captured frame (a test with a two-frame sheet). If
  not, 2315 does that.
- **Either order with 2332:** if 2332 has moved `Painter` and `Image`
  into `shuyi_ui::frame::raster`, the sheet takes that `Image` through
  `frame_png`'s re-export. The sheet and the encoders stay in
  `shuyi-cli` either way: they need `png` and `gif`, and the labels are
  the tool's.
- **Public types (ADR-0038):** `Capture` and `Output` follow it.

**Out (do not do):**
- capturing a range of a recorded session: 2518 adds session records
  and 2519 records real builds; captures from them are a follow-up
  after both, as 2519's Out says;
- a screenshot of a running build: 2521's `shuyi attach --screenshot`
  serves a developer, and a screenshot the player takes has no ticket
  yet (it would build on 2332's raster);
- sound in a capture, MP4 or WebM, and recording a windowed build in
  real time;
- a new `Step`, a new contract operation, or any change to the
  contract's schemas;
- APNG or GIF as asset formats the engine loads. They are files a tool
  writes, so ADR-0007's registry doesn't apply;
- scales below 1 (shrinking frames);
- previews on the website's Examples cards (2801): a follow-up;
- 3007's and 3019's contact sheets of baked images.

## Implementation steps

1. Read `crates/cli/src/frame_png.rs` and its `tests.rs`, `game.rs`,
   `codec.rs`, `toy.rs`, `lib.rs`, `crates/cli/tests/it/toy.rs`,
   `examples/hello-grid/src/tools.rs` and
   `examples/hello-grid/tests/it/snapshot.rs`. Read how 1133's session
   paints its `frame_png`, and 2315's `--debug` and 2332's `raster` if
   they are done, and ADR-0038.
2. Add `Script::then_frames`, with a test that it runs exactly `n` more
   frames (`Harness::frame_count`).
3. Add `Capture`, `Output`, `Options::capture` and `GameTools::frames`
   with its default, and the toy's override. Test that the override
   equals the default through a test-only wrapper that forwards
   `frame` to `ToyTools` and keeps the default `frames`.
4. Add the flags to `parse_args` and `usage`.
5. Write `frame_png/sheet.rs`: the layout, the label font, the size cap.
   Add `proptest.workspace = true` to `shuyi-cli`'s dev-dependencies
   for the layout's property test.
6. Write `encode_apng`.
7. Make `run` write the chosen output and print the summary. Update
   `lib.rs`'s command list and `frame_png.rs`'s module docs.
8. Add hello-grid's walk sheet snapshot.
9. Add `every`, `count` and `columns` to the MCP session tool
   `frame_png`, and a step to 1133's scripted client test.
10. Add the `gif` dependency and write `encode_gif` with its exact
    palette, unless the PR is already about 600 lines (then write its
    ticket). Run `cargo deny check`.
11. Update the skills if 1134 is done.
12. Run the gates.

## Acceptance criteria

- [ ] On the toy, `frame-png toy.png --scale 1 --lobby --keys "Down Down
      f" --every 0.05 --count 8` (the lobby's Talk opened) writes a
      sheet of 8 labelled cells, and each of the first five cells
      differs from the next as the line types on. The summary lists
      each cell's time and screen (`talk`).
- [ ] Without `--count`, `frame-png` writes the same PNG as before (the
      existing tests pass unchanged).
- [ ] `examples/hello-grid/tests/it/snapshot.rs` has a binary snapshot
      `walk_sheet.png`: `--stage`, the keys `Enter Up Right Right
      Enter` (your piece picked up, steered and sent), `every` 3 frames,
      `count` 8, 4 columns, through `HelloTools.frames` (the default
      method) and `sheet::build`. It shows the piece on its way. It is
      reviewed by eye in the PR and passes on all three OSes in CI's
      `test` job. If it differs between OSes, the painter's float step
      that differs is fixed, or a bug ticket is written and this PR
      waits; the test is never loosened.
- [ ] The toy's `frames` override gives the same images and screens as
      the default.
- [ ] An APNG decodes (with `png`) to the same frames, each with delay
      `every/60`, looping forever.
- [ ] If the GIF is in this PR: a GIF decodes (with `gif`) to exactly
      the same pixels; a frame with 257 colours is an error naming the
      frame and the count; `cargo deny check` passes with the `gif`
      crate. If it isn't, its ticket is in `tickets/open/`.
- [ ] Each cap is an error that names it: `--count` over 64,
      `every × (count − 1)` over `MAX_FRAMES`, a sheet over 16 384 px.
- [ ] The MCP `frame_png` with `every` and `count` returns a sheet and
      the cells' times, in 1133's scripted client test.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each new flag and each error in `parse_args`; `then_frames` runs
  exactly `n` frames; each glyph of the label font; the sheet's cell
  positions for a full and a partial last row; a label cut at a narrow
  cell; the palette builder (global, per frame, too many); GIF delays
  (1 frame gives 2, 3 give 5, 6 give 10).
- Property (`proptest`): for any `count` and `columns`, the cells lie
  inside the sheet and don't overlap.
- Round trip: APNG and GIF decoded back equal the captured images and
  delays.
- Snapshot / integration: hello-grid's `walk_sheet.png`; the toy through
  `run` in `crates/cli/tests/it/toy.rs`; the MCP client step.

## Completion notes

*(Filled in by the session that completes the ticket.)*
