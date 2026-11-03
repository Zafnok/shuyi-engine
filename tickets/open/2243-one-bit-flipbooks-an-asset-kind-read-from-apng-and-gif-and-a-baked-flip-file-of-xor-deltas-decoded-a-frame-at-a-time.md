---
id: "2243"
title: "One-bit flipbooks: an asset kind read from APNG and GIF, and a baked .flip file of XOR deltas decoded a frame at a time"
type: feature
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2243 — One-bit flipbooks

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*): "anything that can display graphics must run
Bad Apple". Bad Apple!! is a shadow-art music video, black and white,
about 3 minutes 39 seconds at 30 frames a second: about 6,500 frames.
Played on every target, the Playdate among them, it is a long one-bit
animation, far too big to hold as pictures: 6,500 one-bit frames of
400 × 240 are about 78 MB, against the Playdate's 16 MB of memory.

This ticket gives the engine the asset kind such a clip needs, a
**flipbook**: a long sequence of one-bit frames at a fixed rate,
decoded one frame at a time from a small file. Beside Bad Apple
(2779): a title screen's looping animation on the Playdate, a
one-bit cutscene, a flip-book intro, a demo-scene effect, an
animated loading screen.

Formats are plugins (ADR-0007, `AGENTS.md` rule 10): the kind isn't tied
to one file. Standard formats always load, and our optimized one is
derived from them and tested equal:

- **APNG**, read with the `png` crate already in the tree
  (`Cargo.lock`, 0.17 and 0.18), which decodes APNG frames. `ffmpeg -i
  clip.mp4 -f apng clip.png` makes one;
- **GIF**, through the `gif` crate. It is new: check its license is
  `MIT OR Apache-2.0` on crates.io and in its `Cargo.toml` at the
  version taken, and that `cargo deny check` passes (ADR-0002 §8);
- **`.flip`**, our own: made by `shuyi bake` from either.

A colour frame is made one-bit with the picture's import settings'
`onebit` section (`crates/assets/src/bitmap/dither.rs`, ADR-0057), so a
flipbook takes the same `(onebit: (dither: threshold))` or `bayer` a
picture does. Bad Apple wants a threshold.

## Nick input

None.

## Scope

**In:**
- **The kind**, `shuyi_assets::flipbook`:
  - `Flipbook`: its size (at most 1024 × 1024), its rate (frames a
    second, a whole number 1–60), its frame count, and
    `Flipbook::player()`;
  - `FlipPlayer`: holds the file, one current frame and the state to
    decode the next. `frame()` is the current frame as an
    `onebit::Image` (the Playdate's layout, `crates/ui/src/frame/onebit.rs`;
    if the type lives in `shuyi-ui`, use the bytes and stride here and
    let the caller wrap them). `next()` decodes the next frame;
    `seek(n)` jumps to the keyframe at or before `n` and decodes
    forward. Memory: the file plus two frames, never all of them;
  - no float, no clock: the decoder runs on the Playdate and its
    output is the same everywhere.
- **`.flip`, version 1**, written down in the format's module docs:
  - a header: a magic `SHFLIP`, the version, width, height, rate,
    frame count, and the keyframe interval (default 30, one a second);
  - an index of each keyframe's byte offset, so `seek` is fast;
  - each frame: a keyframe holds its pixels run-length coded; any
    other frame holds the XOR of it with the frame before, run-length
    coded (most of a shadow video doesn't change from frame to frame,
    so the XOR is mostly zeros);
  - the run-length code: a byte `n`, then for `n < 128` that many
    literal bytes `n + 1`, else one byte repeated `n − 125` times (as
    PackBits). Write it down exactly, with an example.
- **The formats**, registered as ADR-0007 says
  (`crates/assets/src/format.rs`): `flipbook: apng`, `flipbook: gif`,
  `flipbook: flip`. APNG and GIF frames are composed with their
  disposal and blend rules into whole frames before they are made
  one-bit. A GIF's frame delays and an APNG's `fcTL` delays must all be
  equal and give a whole rate; otherwise the problem says so, with the
  frame.
- **The bake**: `shuyi bake` writes `.flip` beside the standard file in
  the baked pack, and the bake's own test that a baked file equals
  its source compares every frame.
- **Limits**: `DEFAULT_MAX_BYTES` for the file, and a frame count of at
  most 30 frames a second for an hour. A file that breaks one is a
  problem, never a panic.
- **The golden table** (`shuyi_assets::golden`, ticket 2116): a row
  `flipbook: flip` and a row `flipbook: apng`, each the `state_hash` of
  every frame of a small fixture (32 × 24, 40 frames, made by a test
  helper that draws moving shapes, committed under the crate's
  fixtures). CI's `determinism` job checks them on wasm32 too.
- **`shuyi import`** (the CLI's import, `basictools`): `shuyi import
  clip.png --kind flipbook` reads an APNG and prints its size, rate and
  frame count.
- **The docs** (the `document` skill): the kinds reference and the
  formats list name the kind and its three formats.

**Out (do not do):**
- Colour, grey or sound in a flipbook: video playback is 2909's
  research (parked).
- MP4, WebM or any video codec: the player makes an APNG with their
  own tools.
- Playing a flipbook in a game: the player and the Bad Apple test are
  2779, painting a frame is 2399.
- A frame-by-frame editor.

## Implementation steps

1. Read ADR-0007, ADR-0057, `crates/assets/src/format.rs`,
   `crates/assets/src/bitmap/`, `crates/assets/src/onebit.rs`,
   `crates/assets/src/golden.rs` and one existing format with a bake
   step (the one-bit pictures of 3007).
2. Write `.flip`'s encoder and decoder with the run-length code, and
   its docs.
3. Add APNG reading (compose, then dither), then GIF behind a `gif`
   feature on `shuyi-assets`, on by default where `png` is.
4. Register the three formats; add the bake step and its equality
   check.
5. The fixture helper, the golden rows, the fuzz-style tests, then
   `shuyi import`.
6. The docs, then the gates.

## Acceptance criteria

- [ ] An APNG, a GIF and a `.flip` of the same frames decode to equal
      frames.
- [ ] `shuyi bake` writes a `.flip` that decodes equal to its source,
      and the equality is checked.
- [ ] `seek(n)` gives the same frame as playing from the start to `n`,
      for every `n` of the fixture.
- [ ] Playing a 400 × 240 flipbook holds no more than the file and two
      frames (a test with the heap counter, 3064, reads it).
- [ ] The golden rows pass on native and wasm32.
- [ ] No byte string panics the decoder; limits give problems.
- [ ] The `gif` crate's license is recorded in the completion notes
      with where it was checked, and `cargo deny check` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the run-length code on hand-worked bytes, both run kinds and
  the 128 boundary; the header; the index; XOR of equal frames is all
  zeros.
- Property: encode then decode is the identity for any frames up to
  64 × 64 × 50; any bytes after the magic never panic (ADR-0007's
  fuzz-style test).
- Snapshot / integration: the golden rows; the bake's equality check
  over the fixture; `shuyi import` on the fixture's APNG.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
