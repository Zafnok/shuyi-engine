---
id: "2589"
title: "frame-png --gif: frames over time as a GIF with an exact palette, never quantized"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2514"]
nick_input: none
completed: 2026-11-11
---

# 2589 — frame-png --gif: frames over time as a GIF with an exact palette

## Context

2514 (done) gave `frame-png` frames over time: `--every <seconds>
--count <n>` paints `n` frames (`Capture` in
`crates/cli/src/frame_png.rs`) and writes them as one sheet (the
default, `--sheet`) or as an APNG (`--apng`, `encode_apng` in
`crates/cli/src/codec.rs`). Its scope planned a GIF too, last, and left
it out if the PR passed about 600 changed lines; it passed them, so the
GIF is this ticket. An APNG plays in every current browser, but some
places a person puts a moving picture of a game (a forum, a chat, older
store pages) take only a GIF.

The frames come from the raster painter
(`shuyi_ui::frame::raster`), which paints opaque pixels, and a game's
palette is small: a frame of a 2D game fits a GIF's 256 colours almost
always. So the GIF is exact or it is refused, never quantized or
dithered behind the user's back.

**Either order with 2248** (one-bit flipbooks, which read GIFs in
`shuyi-assets`): if 2248 is done, the `gif` crate is already in
`Cargo.lock` with its licenses checked: add it to `shuyi-cli` with the
writer's features only. If not, this ticket adds it first, and 2248
adds it to `shuyi-assets` with the reader's.

## Nick input

None.

## Scope

**In:**
- **`--gif`**, a third output flag of `frame-png` beside `--sheet` and
  `--apng` (`CaptureFlags` in `crates/cli/src/frame_png.rs`), and
  `Output::Gif` (the enum is `#[non_exhaustive]`, ADR-0039: an additive
  variant). One output flag only, as today; `--columns` stays for the
  sheet only; the usage text lists it.
- **`encode_gif(frames: &[&Image], every: u32) -> Result<Vec<u8>,
  String>`** in `crates/cli/src/codec.rs`:
  - the `gif` crate (0.14 on 2026-11-11), `MIT OR Apache-2.0`, as is
    its LZW crate `weezl`; `deny.toml` allows both. Its default
    features pull in `color_quant`, a quantizer we don't call: take
    `default-features = false` with only `std` (check its `Cargo.toml`
    on the day; `cargo deny check` passes);
  - an **exact palette** of RGB colours, built in scan order: one
    global palette when all frames together have at most 256 colours,
    else one per frame. A frame with more than 256 colours is an error
    naming the frame (from 1) and its count, suggesting `--apng`. Never
    quantize or dither;
  - each frame's delay is `every` frames in hundredths of a second,
    rounded to the nearest, at least 2 (browsers slow a GIF whose delay
    is under 2): 1 frame gives 2, 3 give 5, 6 give 10;
  - it loops forever (`Repeat::Infinite`);
  - all frames one size, as `encode_apng` checks.
- **The summary** `run` prints says the speed the GIF plays at when the
  delay isn't exact (`every` × 100 / 60 not a whole number, or under 2):
  a line such as `the GIF shows each frame 0.02 s (every is 0.017 s)`.
- **The skill pack:** `skills/shuyi-ship/SKILL.md`'s APNG paragraph
  (step 7) adds `--gif` for where only a GIF is taken. `check-skills`
  passes.
- **The book:** `site/content/learn/book/testing.md`'s paragraph on
  frames over time names `--gif` beside `--apng`.

**Out (do not do):**
- GIF from the MCP session tool `frame_png`: an agent reads the sheet,
  and a person wants the file (2514's decision);
- quantizing, dithering, or a `--colors` option;
- reading GIFs as an asset (2248).

## Implementation steps

1. Read 2514's code: `frame_png.rs` (`Capture`, `Output`,
   `CaptureFlags`, `run`, `capture_summary`), `codec.rs`
   (`encode_apng` and its test), `frame_png/tests/capture.rs`.
2. Add the dependency, with its comment in `crates/cli/Cargo.toml`, and
   run `cargo deny check`.
3. Write the palette builder (a private function returning the global
   palette or one per frame, or the error) and `encode_gif`.
4. Add `Output::Gif` and `--gif`, and write it in `run`, with the
   summary's speed line.
5. Update the skill and the book.
6. Run the gates.

## Acceptance criteria

- [x] `frame-png out.gif --every 0.05 --count 8 --gif` on the toy
      writes a GIF that decodes (with `gif`) to exactly the captured
      pixels, looping forever, each delay 5.
- [x] A frame with 257 colours is an error naming the frame and 257,
      suggesting `--apng`; nothing is written.
- [x] Frames of 200 colours each, 300 together, get a palette each;
      frames of 256 together get one global palette.
- [x] `--gif` with `--apng` or `--sheet`, and `--gif` with `--columns`,
      are errors with the usage.
- [x] `cargo deny check` passes with the `gif` crate, and its
      `color_quant` isn't in `Cargo.lock` for `shuyi-cli` (`cargo tree
      -p shuyi-cli -e features -i color_quant` finds none).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the palette builder (global, per frame, too many); GIF delays
  (1 frame gives 2, 3 give 5, 6 give 10, 2 000 give 3 333); the new
  flag and its errors in `parse_args`; the summary's speed line.
- Round trip: a GIF decoded back equals the captured images, its
  delays and its repeat.
- Integration: the toy through `run` in `crates/cli/tests/it/toy.rs`.

## Completion notes

- **Done.** `frame-png --gif` (`Output::Gif`, an additive variant of
  the `#[non_exhaustive]` enum) writes the frames over time with
  `codec::encode_gif`: an exact RGB palette in scan order, global when
  all frames have at most 256 colours together, else one per frame; a
  frame of more than 256 is an error naming it and its count and
  suggesting `--apng`, and nothing is written. Delays are
  `codec::gif_delay` (hundredths to the nearest, at least 2), the GIF
  loops forever, and frames must be one size (`one_size`, now shared
  with `encode_apng`). The summary adds `the GIF shows each frame 0.02
  seconds (every is 0.017 seconds)` when the delay isn't exact (units
  written in full: the engine's lint reads a lone `s` as a key).
- **Dependency.** `gif` 0.14.2 with `default-features = false` and
  `std` only; `gif` and its LZW crate `weezl` 0.1.12 are both
  `MIT OR Apache-2.0` in their `Cargo.toml`s; `cargo deny check`
  passes. `cargo tree -p shuyi-cli -e features -i color_quant` finds no
  `color_quant` for `shuyi-cli`. `color_quant` is still in `Cargo.lock`,
  through `image` 0.24 elsewhere in the workspace, as before this PR.
- **Compile budget.** A new `blank` game depends on `shuyi-cli`, so
  `gif` and `weezl` are 2 more crates in its tree: 146, measured with
  `cargo metadata` on a game fresh from `shuyi new --template blank`
  and the engine's `Cargo.lock`. `tools/compile-budget.ron`'s
  `dependencies` goes from 144 to 146 with the reason beside it.
- **Deviation.** `encode_gif` also refuses a frame with a pixel that
  isn't opaque, naming the frame and suggesting `--apng`: the raster
  paints opaque pixels, and a GIF of partial alpha wouldn't be exact,
  which the ticket's "exact or refused" rules out.
- **Handoffs.** 2248's clause on the `gif` crate now says what holds
  (the crate is in, its licenses checked: add it with the reader's
  features); 2357's capture paragraph names `--gif` and `encode_gif`
  beside the APNG.
- **Docs.** `skills/shuyi-ship/SKILL.md` step 7 and the book's testing
  chapter (`site/content/learn/book/testing.md`) name `--gif`.

No break: `Output` is `#[non_exhaustive]` (ADR-0039), and
`codec::encode_gif` and `codec::gif_delay` are new. Searched the game
for `apng` and `frame_png::Output` (`gh search code --repo
Zafnok/visions-of-shuyi`): no hit.

Behaviour: shuyi-cli: `frame-png`'s usage text lists `--gif` under frames over time.

Game: nothing to adopt, the game names no game ticket here and uses `frame-png` as a tool only.
