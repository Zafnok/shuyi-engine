---
id: "2779"
title: "A flipbook player example, the Bad Apple test: an original clip in time with its music on the desktop, the web, the Playdate and the text console, and the player's own clip"
type: feature
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2399", "2248"]
nick_input: sign-off
completed:
---

# 2779 — A flipbook player example: the Bad Apple test

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*): "as is tradition we simply must have a Bad
Apple and DOOM demo for our engine... anything that can display
graphics must run Bad Apple". Bad Apple!! is a shadow-art music video
(a fan work set to a Touhou Project song) that people play on
everything with a screen: oscilloscopes, calculators, terminals in
ASCII, the Playdate, Minecraft. Its frames are black and white, so it
is a fair test of a one-bit target.

**What we can't ship.** The video and its song belong to their makers;
nothing about either is under a license ADR-0002 §8 allows (`AGENTS.md`
rule 7). So the repository carries none of its frames, its audio or
its name in a file the example loads. What it carries is:

- an **original clip**: a short shadow animation of our own, made by a
  program in the repository, with an original tune, both under the
  engine's licenses;
- a **player** that plays any flipbook (2248) in time with its music;
- **the recipe**, in the example's README: the player makes a flipbook
  from their own copy of the video with `ffmpeg` and `shuyi bake`, and
  drops it on the window or puts it in a folder beside the program.

It builds on:

- 2399: a picture a skin paints at run time. Each frame is decoded
  into one one-bit picture;
- 2248: the flipbook kind, read from APNG and GIF and baked to `.flip`;
- `ctx.music_clock` (`crates/ui/src/ctx.rs`): where the music is in its
  track, so the frame shown is `seconds × rate`, and the picture keeps
  time with the sound even when frames are dropped. In silence it
  counts `input.ticks()` (60 a second, `crates/platform/src/tick.rs`);
- `shuyi_app::read_folder` for the player's clip on the desktop;
- 3010: the Playdate backend and `shuyi export playdate`; 3007, a
  target's own copies of files (a `files@playdate` flipbook at 400 ×
  240).

## Nick input

**Sign-off:** play the example's web build on Pages and the Playdate
simulator (or a device), and, if you like, your own Bad Apple
flipbook made with the README's recipe. Comment on anything that looks
or sounds out of time.

## Scope

**In:**
- **`examples/flipbook`**, a workspace member on the short path
  (`shuyi::prelude`, `run_game`), on the `shuyi` crate alone for the
  engine, as `examples/hello-grid` is:
  - **the view** (`src/view.rs`, pure): the flipbook's id and the
    frame due, worked out from the music clock or the steps;
  - **the pixel skin**: decodes the due frame with `FlipPlayer`
    (sequential `next()` while playing, `seek` after a skip) and paints
    it into one one-bit `Painted` picture, centered and scaled by the
    largest whole number that fits;
  - **the text skin**: the same view drawn on the glyph console in
    half blocks (`▀`, `▄`, `█` and space: two pixels to a cell, top
    and bottom), each cell the majority of the pixels it covers. The
    player switches skins with an action. This is the "terminal Bad
    Apple", and a test of rule 9: one view, two skins;
  - **the screens**: a title ("Play", "Text mode", "Quit" where the
    platform can quit), the player (`Confirm` pauses, `Cancel` goes
    back, `Left` and `Right` skip 5 seconds), and an end card;
  - **the actions**: `Up`, `Down`, `Left`, `Right`, `Confirm`,
    `Cancel`, `Skin`. `files/keymap.ron` has tables for the Playdate
    and the Steam Deck. Nothing names a key (the `input-actions`
    skill);
  - `shuyi.ron` lists `desktop`, `web`, `steam-deck` and `playdate`.
- **The original clip** (`files-src/clip/`, made by
  `cargo run -p flipbook --example make-clip`):
  - 30 seconds at 30 frames a second, 480 × 360 (and 400 × 240 for the
    Playdate's layer): white shapes on black that turn into one
    another, a figure that walks and spins, a wipe to black, drawn in
    integers by the program (no hand-drawn frames, no outside art);
  - written as an APNG in `files-src`, baked to `files/clip.flip` and
    `files@playdate/clip.flip` by `shuyi bake`;
  - an original tune for it in `files/clip.wav`, rendered by the same
    program from a few notes in integers (square and triangle waves
    with an envelope), so the music clock has a track to follow.
    Credited in the example's credits as our own.
- **The player's own clip**:
  - on the desktop, `clip.flip` (and its music, `clip.ogg`) in a
    folder named `mine` beside the program is read with `read_folder`
    and laid over the example's files;
  - **From 3092 (done):** a flipbook and its music dropped on the
    window are played too, on the desktop and the web: the game's system
    hook hears `SystemEvent::FilesDropped`, and
    `DroppedFile::layer(files)` makes a layer named `dropped` to lay over
    the example's files. The title says "Drop a flipbook here" where
    `ctx.platform.can_drop_files()`; a test drops a clip with
    `Harness::drop_file` or the `drop` step;
  - the README's recipe: `ffmpeg -i bad_apple.mp4 -vf scale=480:360
    -f apng clip.png`, then `ffmpeg -i bad_apple.mp4 -vn clip.ogg`,
    then `shuyi bake`. It says plainly that the video and song aren't
    the engine's and the player needs their own copy.
- **Pages**: the web build plays at `/examples/flipbook/`, with a card
  in `site/content/examples/` beside `hello-grid.md`, and the Pages
  workflow builds it as it builds Hello, Grid.
- **Tools** (`src/tools.rs`): a `GameTools` through 2511's `BasicTools`,
  with `session` giving a scene of the frame due, the skin and the
  clip's length, so `scene` and `frame_png` show where playback is.
- **The docs** (the `document` skill): the example's `README.md`, the
  examples page, and `docs/README.md`'s row if it lists examples.

**Out (do not do):**
- Shipping Bad Apple's frames, audio, a file derived from them, or a
  link to a download of them.
- Colour video and video codecs (2909, parked).
- Measuring the Playdate device's frame times (3011).
- A falling-note or rhythm game over the clip.

## Implementation steps

1. Read `examples/hello-grid` end to end (its `shuyi.ron`, `build.rs`,
   `files@playdate`, `src/tools.rs`, `src/playdate.rs`), 2399's ADR,
   2248's kind, and `ctx.music_clock`.
2. Write `make-clip`: the shapes and the walk cycle in integers, the
   APNG through the `png` crate, the tune.
3. The view, the pixel skin, the screens and the actions.
4. The text skin.
5. The player's own clip on the desktop, and the drop (3092).
6. The web build on Pages, the Playdate build in the simulator.
7. The README with the recipe; the docs; the gates.
8. Play it start to end on the desktop and the web, and in the
   Playdate simulator; make a flipbook of a long clip of your own
   (any five-minute one-bit APNG `make-clip` can write with
   `--seconds 300`) and check it holds time with its music at the end.

## Acceptance criteria

- [ ] `examples/flipbook` plays the original clip in time with its
      music on the desktop, the web (on Pages) and the Playdate
      simulator.
- [ ] At the 30-second mark, the frame shown is within one frame of
      `seconds × rate` of the music clock (a harness test reads the
      scene).
- [ ] The text skin draws the same view in half blocks, and switching
      skins changes neither the view nor the frame due.
- [ ] A five-minute 480 × 360 flipbook plays to its end without falling
      behind on the desktop and the web, and on the Playdate simulator
      at 400 × 240.
- [ ] The repository holds nothing of Bad Apple!!: no frames, no audio,
      no derived file. The README's recipe uses the player's own copy.
- [ ] Nick signed off.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the frame due from a music clock and from steps; the half-block
  cell from its pixels; `make-clip`'s frames are the same on every run.
- Snapshot / integration: the scene and snapshot of the pixel and text
  skins at frames 0, 300 and 899; `frame-png` and `frame-png --target
  playdate` of frame 300 as golden PNG files; a harness run that plays the
  whole clip and checks the end card; a skip and a pause.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
