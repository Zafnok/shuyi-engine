---
id: "3446"
title: "frame-png --gif for a store or README picture: changed-rectangle frames, a crop, several time spans in one run, a held last frame, past 64 frames"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3446 — `frame-png --gif` for a store or README picture

## Context

- **The game ticket:** game#1403
  (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1403-the-readme-gif-from-frame-png-alone.md`),
  from the game's third engine-friction sweep, game#1397 (row C7). It is
  not a blocker.
- **The job:** make an animated picture of a game's own moments for a
  README or a store page in one command, small enough to put on the
  page.
- **The friction numbers:** sweep game#1397.
  - What the game would keep: about 40 lines, its table of clips.
  - What it has today: about 350 production lines of
    `crates/tools/src/readme_pictures.rs` (game ticket 1264).
  - Ratio: about 9×. Outcome 1, an engine feature.
- **What the engine has:** `frame-png --gif` (`crates/cli/src/frame_png.rs`,
  `codec::encode_gif`) stores every frame whole, so the file is about
  twice the size it needs to be. It also has these limits:
  - it can't leave out rows (a help bar, say);
  - it caps a run at `MAX_CAPTURES` = 64 frames;
  - it can't skip from one stretch of time to another;
  - it doesn't hold the last frame before the loop.

  So the game draws four clips, stitches them together, crops them and
  computes the changed rectangles itself.
- **The generic shape:**
  - `--gif` keeps only each frame's changed rectangle, with
    transparent unchanged pixels.
  - A frame that repeats the one before it becomes one longer frame.
  - `--crop x,y,w,h` (cells or pixels, as the other options read).
  - `--span <from>..<to>`, repeatable: the frames of several stretches
    in one file.
  - `--hold <seconds>` on the last frame.
  - A frame limit for `--gif` well above 64, with the time and memory
    bound stated.
- **An example from a game that isn't tactics:** a platformer's store
  GIF showing the first jump and the boss intro, without the debug
  overlay's row.

## Nick input

None.

## Scope

**In:**
- The flags above, on `frame-png` and on the contract's `frame_png`
  operation.
- Delta encoding in `codec.rs` (with a per-frame palette, as today).
- Help text.
- **The API is additive.** It adds new flags and changes no default
  output. `--gif` without the new flags writes the same bytes as before.

**Out (do not do):**
- Lossy palettes or dithering.
- WebP or video output.

**Estimated size:** about 700 lines (code 300, tests 300, docs 100).

## Implementation steps

1. Read the game's `crates/tools/src/readme_pictures.rs` (public
   repository): `Joiner`, `join` and `paint` are the reference.
2. Add the delta encoder to `crates/cli/src/codec.rs`, behind an option
   so today's output is unchanged.
3. Add the span, crop and hold flags to `frame_png.rs`, with their
   parsing and errors, and raise the GIF frame limit.
4. Update the book's frame-png page and the `shuyi-ship` skill (store
   pictures).

## Acceptance criteria

- [ ] Decoding a delta GIF gives frames pixel-identical to the whole-
      frame GIF of the same run (a decoder round trip in the tests).
- [ ] Two `--span` stretches give one file whose frame count is their
      sum. A third, overlapping span is refused with a message.
- [ ] `--crop` leaves the given rows out of every frame.
- [ ] `--hold 3` makes the last frame's delay 3 s.
- [ ] `--gif` with none of the new flags writes the same bytes as
      before (a golden file).
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated, or the completion notes
      have a `Skills:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the changed-rectangle encoder, frame merging, and flag
  parsing.
- Integration: `frame-png` on the toy game with spans, crop and hold.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
