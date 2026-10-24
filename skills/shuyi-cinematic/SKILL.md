---
name: shuyi-cinematic
description: Make a cinematic on Shuyi Engine, a file of timed shots of the game's own kinds over a length in seconds or a music cue's. Covers reading and checking it with the game's files, and a screen that draws each frame from the time alone, keeping its own time or the music's. Use when adding an opening, an ending or a cut scene of shots, a title that keeps time with its music, or a test of what plays at a moment. Not for a talk the player reads on (shuyi-dialogue), or for a screen that reacts to the player beyond skipping (shuyi-screen).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Cinematics

A cinematic is a list of shots, each starting at a time, over a length
of time. The file says when; what a shot shows is the game's own, a
type of its shot kinds. The engine checks the times, the game checks
its shots, and a player draws the shot at any moment. Every frame is
drawn from the time alone, so whoever owns the clock can follow the
music, jump or loop, and a test can draw any moment.

## When to use it

- An opening, an ending, a cut scene: shots one after another, cut at
  set times.
- A title or a screen that keeps time with its music.
- A test of what shows at a given second.

## When not to

- A scene the player reads on, line by line, with replies:
  `shuyi-dialogue`.
- A screen that reacts to the player beyond skipping it: `shuyi-screen`.

## The file

```ron
(
    id: "intro",
    length: Seconds(6.0),       // or Music("title")
    shots: [
        (at: 0.0, shot: Line("intro.dawn")),
        (at: 3.0, shot: Typed("intro.gate")),
    ],
)
```

- `id` is the file's name without `.ron`.
- `length` is `Seconds(n)` (above 0, at most an hour), or `Music(cue)`:
  the cue's `length_ms` in the game's audio manifest, so the file never
  repeats a number the manifest owns.
- `shots`: each `at` seconds from the start, in order, the first at
  `0.0`, each before the end. A shot lasts until the next one starts,
  and the last until the end. Shots cut: there is no fade.
- `actions` (optional): lists by name that shots may share, such as
  what the game's actors act out. The game says what a list is.
- `shot` is the game's own kind, read with serde: `Line("intro.dawn")`
  is a variant of the game's `enum`. A shot that shows text names a text
  key, so the words stay in `files/en.ron`.
- Unknown fields are refused. Times are kept in whole milliseconds.

## Steps

1. **Give the game its shot kinds**: an `enum` that derives
   `serde::Deserialize` (the game crate needs `serde` with `derive`).

2. **Read the file and check it** in the game's own part of its data
   (a stage's `own` on the short path):

   ```rust
   let mut formats = Formats::new();
   let _ = formats.register(CinematicRon::<Shot>::new());
   let raw = formats.load::<CinematicFile<Shot>>(bundle, INTRO)?;
   let file = bundle.display_path(INTRO);
   let intro = cinematic::check(&file, ID, &raw, &music, Keys(&english))?;
   ```

   `CinematicRon` is the format, `CinematicFile` the file as written,
   and `cinematic::check` gives a `CinematicDef`: each shot's
   `at_ms`, the `length_ms` and the `music` cue it follows. It reports
   every problem: an id that isn't the file's name, a cue the manifest
   lacks, a length out of range, no shots, a time that isn't one, a
   first shot not at 0, shots out of order or past the end, then the
   game's own. For a game whose shots share no `actions`,
   `cinematic::from_source` reads and checks a file's text in one call.

3. **Check the game's shots** with a type that implements `ShotCheck`:
   `check(n, shot, problems)` adds each problem as `shot {n}: …` and
   returns the shot as the game keeps it; `finish` adds what only shows
   once every shot is seen (a shared list no shot uses). A key the
   English lacks, a picture the game doesn't have.

4. **Paint the shots**: the kept shot implements `ShotPainter`. Its
   `frame(ctx, shared, moment, screen)` says what a frame shows, as plain
   data, and `paint` draws that frame. A `Moment` is the shot's `index`,
   its `progress` (0 to 1), `into_ms`, `shot_ms` and `t_ms`. A shot keeps
   no state that an update advances: whatever moves is worked out from
   the moment. The engine gives `ease_in_out(progress)`, and a `Pan`
   over a picture shown as the frame's backdrop (`PanFrame`,
   `paint_pan`).

5. **Play it in a screen** ([`example/intro.rs`](example/intro.rs) is a
   whole one):

   - `CinematicPlayer::of(&def, Shot::clone)` builds each shot once.
   - The screen owns the clock. Each update adds `input.dt()` to its
     seconds. While the music cue the cinematic follows is sounding,
     `ctx.music_clock` (a `MusicClock`: its `cue`, its `position` in
     seconds, its `length`) is the time instead. The player has no clock
     of its own, and it loops at its length (`Timeline::wrapped_ms`).
   - `draw(ctx, &shared, t, brightness, buf)` draws the moment `t`;
     `brightness` 1 is as it is, and 0 is black. `frame(ctx, &shared, t,
     screen)` is what shows at `t`, for a test.
   - It closes itself at `timeline().length_ms()`, and on a skip.

6. **Look at a moment** headless: `--wait` lets seconds pass.

   ```bash
   cargo shuyi run --script "Enter" --wait 5 --frames 1
   cargo shuyi frame-png gate.png --keys "Enter" --wait 4
   ```

   `run` answers with the frame as text (`/result/frame`); `frame-png`
   draws it (its steps are `--keys`, `--pad` and `--wait`). In a test,
   the Harness's `wait(seconds)` does the same, and `music_clock()` says
   where the music is.

## Music

`Music(cue)` needs the cue in the game's audio manifest, and a screen
that keeps time with it reads `ctx.music_clock`. The engine's short
path (`BasicData`) has no music: a game on it gives its cinematics a
`Seconds` length, and its screen counts its own time. A game with music
of its own writes its `GameData` with its manifest, as the game's
`AGENTS.md` says under the long form.

## When it fails

- `check` answers `ok: false` with each problem in `result.errors`, its
  `file` and its `message`. A problem of the times or the shots names
  the shot (`shot 1: the first shot must be at 0.0, not 0.5`).
- `SH2001`: the file's syntax is wrong; the problem has its `line`
  (`cargo shuyi explain SH2001`).
- `SH2003`: no file where `own` loads it: the path is from the files
  folder (`intro.ron`).
- `SH3005`: the game's tool program didn't build: the answer ends with
  the compiler's output.

## Worked example

[`example.ron`](example.ron) runs on a new `blank` game. It gives the
game `serde`, writes a cinematic of two shots of its own kinds
([`example/intro.ron`](example/intro.ron)), the stage that reads, checks
and plays it ([`example/stages.rs`](example/stages.rs),
[`example/intro.rs`](example/intro.rs)), and the shots' text. `check`
passes. `run` shows the first shot, two thirds of the second's line
typed five seconds in, and the title back after six seconds or a skip;
`frame-png` draws the moment four seconds in. A first shot at 0.5, a
text key the English lacks and a missing comma each fail `check` with
what is wrong, the last with `SH2001` and its line. Fixed, every gate
passes.
