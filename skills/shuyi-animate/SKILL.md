---
name: shuyi-animate
description: Motion in a game on Shuyi Engine, drawn from the time alone by the skin and never by the rules. It covers what exists today (sprites cut from a sheet, a grid walk shown at the player's speed, text typing out, the hurry clock, idle motion, cinematic pans and eases). Animation clips as data and particles aren't built yet. Use when something on screen should move, blink, walk or type out, or when a test needs any moment of a motion. Not for a stage that moves in real time (shuyi-stage-field) or timed shots set to music (shuyi-cinematic).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
  waits_for: "2231 2103"
---

# Animate

Motion is a skin's: how long the player watches a move, never what the
move does. The rules change only through their commands or steps, and a
skin shows the change over time. Every frame of a motion is worked out
from the time it has run, never stepped along from the frame before,
so a test can draw any moment and a faster or slower machine shows the
same thing.

**Not built yet:** animation clips as data (a sheet's frames, each with
its time, played forward, backward or back and forth, looped or once)
and particles (emitters as data, each particle worked out from a seed
and the time). When the engine has them, this skill gets their steps and
a worked example. Until then, use what is below.

## When to use it

- Something on screen should move: a unit walking, a cursor blinking,
  a sprite's frames, text typing out, a picture panning.
- A test needs a motion at a given moment.

## When not to

- A world that moves in real time, with collisions: `shuyi-stage-field`.
- Timed shots over a length or a track of music: `shuyi-cinematic`.
- Pictures and sheets coming in from other tools: `shuyi-import-art`.

## What exists today

- **Sprites from a sheet.** A screen draws a picture as a `Sprite` on
  its `GlyphBuffer` (`add_sprite`): an image of the game's image table,
  the part of it shown (`src`, in the image's pixels) and where
  (`dest`). A sheet's frames are rectangles of one image, so a frame of
  a motion is a choice of `src`.
- **A walk on a grid map.** The grid kit's skins show a unit's step
  over time at `WalkSpeeds` (tiles a second: `normal`, with the
  player's fast animations, and while a key hurries another side's
  turn). The move itself happened at once, in the rules.
- **Text typing out.** The dialogue text box types a page at the
  player's text speed and shows it whole on a press (`shuyi-dialogue`).
- **The hurry clock.** `HurryClock`: the time of something the player
  can hurry by holding a key and finish with a press (a tally counting
  up, a fight played back). `tick(dt, held)` gives the time passing
  this frame.
- **Idle motion.** `ctx.idle_motion` (`IdleMotion::On` or `Off`) says
  whether a screen left alone may move its decoration. A screen that
  sways or twinkles on its own checks it first.
- **Eases and pans.** The cinematic player's `ease_in_out` and `Pan`
  (a picture moved across the frame's backdrop) work outside a
  cinematic too (`shuyi-cinematic`).

## Steps

1. **Keep the time in the screen, not the rules.** A screen adds up
   `input.dt()` (seconds since the last frame) into a field of its
   own, or reads `ctx.clock_s` (seconds the game has run). That time is
   view state: never sent to the rules, never saved, never part of a
   state hash.
2. **Work the frame out from the time.** For a sheet of `n` frames of
   `ms` milliseconds each, the frame shown is `(elapsed_ms / ms) % n`,
   and its `src` is that frame's rectangle. Never add one frame per
   update: a frame rate that changes would change the motion.
3. **Respect the player.** Decoration moves only while
   `ctx.idle_motion` is `On`; a walk's speed is `WalkSpeeds`'; text
   types at the player's text speed. A setting that turns motion off
   leaves the screen readable, standing still.
4. **Test a moment, not a run.** Give the screen's draw a time and read
   what it built: the view, the cells, or the buffer's `sprites()` for
   a look. `cargo shuyi frame-png` draws a frame after a script with
   waits (`--wait 0.5`), to look at.

## Particles

Particles are to come: emitters as data, painted by the skin, each
particle worked out from the emitter's seed and the time so it never
depends on the frames drawn before. Until then a game draws a few
sparks itself, by the steps above, with their places worked out from a
seed and the time.

## When it fails

Branch on the answer's `code`, never its message, and read its page
with `cargo shuyi explain <code>`:

- `SH2103`: the rules read a float or a clock: a motion's time belongs
  to the screen, and the rules count turns or steps.
- `SH2003`: a picture the game names isn't in its files.

## Worked example

None yet: it comes with the engine's animation clips.
