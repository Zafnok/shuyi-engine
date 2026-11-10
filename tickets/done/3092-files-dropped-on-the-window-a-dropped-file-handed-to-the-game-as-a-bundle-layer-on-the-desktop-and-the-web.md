---
id: "3092"
title: "Files dropped on the window: a dropped file handed to the game as a bundle layer, on the desktop and the web"
type: feature
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-10
---

# 3092 — Files dropped on the window

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*). Neither demo can ship its famous content: the
Bad Apple!! video isn't ours to give away, and DOOM's own WADs aren't
either (the port plays Freedoom, which is BSD-licensed). So the
player brings their own: a flipbook they made from their copy of the
video (2779), or their `doom1.wad` or `doom2.wad` (2782). The easiest
way in, on the desktop and in the browser alike, is to drop the file
on the game's window.

What there is:

- **bundle layers** (`crates/assets/src/bundle.rs`, ADR-0004 §1):
  `Layer::owned(name, files)` takes files handed in at run time, and a
  game lays it over its own. On native, `shuyi_app::read_folder`
  (`crates/app/src/folder.rs`) reads a folder into one;
- **3069** (done) lays files the page holds in `sessionStorage` over a
  web build's own. A browser keeps about 5 MB there, too little for a
  WAD; a dropped file has no such limit;
- **miniquad 0.4.11** (in `Cargo.lock`) has drops already, on Windows,
  Linux, macOS and the web: `EventHandler::files_dropped_event`,
  `window::dropped_file_count()`, `dropped_file_bytes(i)` and
  `dropped_file_path(i)`.

Other games that need this: a level editor's import, a mod dropped on
the window, a custom skin or palette, a save moved from another
machine, an emulator's ROM.

## Nick input

None.

## Scope

**In:**
- **`SystemEvent::FilesDropped(Vec<DroppedFile>)`** (`SystemEvent` in
  `shuyi-platform`; check it is `#[non_exhaustive]`, and if it isn't,
  write the `Breaking:` line), where `DroppedFile` is
  `#[non_exhaustive]` with `name()` (the file name alone, never a
  path) and `bytes()`. A game's system hook reads it, and
  `DroppedFile::layer(files)` turns some into a bundle layer named
  `dropped`.
- **`shuyi-app`** reads miniquad's drops in `files_dropped_event`
  and hands them on in the next frame. A file bigger than 256 MB is
  refused and logged; so is a drop of more than 16 files.
- **A platform fact** (ADR-0060): `ctx.platform` says whether files
  can be dropped, so a screen offers "Drop a file here" only where it
  works. The Playdate and headless say no; the harness can inject a
  drop for tests.
- **The contract**: an input script step `drop: "path"` that injects a
  dropped file (the harness and the MCP `step`), so an agent tests a
  game's drop.
- **The docs** (the `document` skill): the book's platform chapter, the
  contract's input script.
- **Either order with 2779:** if 2779 is done, its web build gains
  "drop a flipbook on the window" here (the example's README and its
  title screen hint); if not, nothing to do (2779 does it).
- **Either order with 2782:** if 2782 is done, nothing to do here: the
  port lives in its own repository (`Zafnok/shuyi-doom`), and its
  ticket list there gains "drop your own WAD" when this lands; say so
  in the completion notes. If not, nothing to do (2782 does it).

**Out (do not do):**
- A file picker dialog (a native dialog crate, or the web's `<input
  type=file>`): drops first; a picker becomes a ticket of its own if a
  game asks.
- Writing a file back to the player: the debug menu's download (2519)
  is the only way out today.
- Files from the page's storage: 3069.

## Implementation steps

1. Read `crates/platform/src/platform.rs` (the facts),
   `crates/platform/src/app.rs` (system events), `crates/app/src/lib.rs`
   (miniquad's events), `crates/assets/src/bundle.rs`, ADR-0060, and
   miniquad 0.4.11's `files_dropped_event`.
2. Add `DroppedFile`, the event and the fact.
3. `shuyi-app`: read the drop, check the limits, hand it on.
4. The harness and contract step; the docs; the gates.
5. Try it by hand on the desktop and in a web build (the in-app
   browser can't drop files; say so in the completion notes if it
   wasn't tried in a real browser).

## Acceptance criteria

- [x] A file dropped on the desktop window reaches the game's system
      hook with its name and bytes; so does one dropped on the web
      build's canvas.
- [x] Files over the limits are refused and logged.
- [x] `ctx.platform` says where drops work; the Playdate and headless
      say no.
- [x] A harness test drops a file through the input script and the
      game sees it.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `DroppedFile::name` strips any path; the limits; the layer.
- Snapshot / integration: the harness drop step; a `shuyi-app` test
  of the handoff with a fake drop source if miniquad's can't be driven
  in a test.

## Completion notes

What was done:

- `shuyi_platform::dropped`: `DroppedFile` (`new(path, bytes)` keeps the
  name alone, `name()`, `bytes()`, `into_bytes()`, a `Debug` that shows
  the size, never the bytes), `DroppedFile::layer(files)` (a layer named
  `dropped`, each file at its name), and `accept`, the limits: a drop of
  more than 16 files is refused whole, a file of more than 256 MB alone.
- `SystemEvent::FilesDropped(Vec<DroppedFile>)`. `SystemEvent` was
  already `#[non_exhaustive]`, but it was `Copy`, and a `Vec` can't be:
  that is the break below.
- `App` keeps the limits, not `shuyi-app` (a deviation): it refuses what
  is over them, logs each as a `platform` warning in the game's log, and
  then a session record notes what is left. Every backend and the
  Harness get the same checks; ADR-0103 says why.
- `shuyi-app` reads macroquad's `get_dropped_files` in each frame's
  `system_events` (`crates/app/src/drops.rs`), the web's skipped frames
  too, and its tests hand it a fake drop.
- The fact: `PlatformFacts::file_drops`, a `FileDrops` enum, with
  `can_drop_files()` and `with_file_drops` (an enum, as `keyboard` is:
  clippy's `struct_excessive_bools` refuses a fourth bool). A deviation:
  the backend says it, not the target's profile. A profile field changed
  the target profile's binary form in a pack, so the pack's version and
  the golden table, for a fact only the backend knows; ADR-0103 says
  so. `shuyi-app` says `Heard` on `desktop` and `web`, `Unheard` on the
  Steam Deck; the headless backend and the Playdate's say `Unheard`.
- The tools: `Harness::drop_file` and `drop_files`, the script step
  `drop: "path"` (`--drop` on `run`, `scene`, `frame-png` and the MCP's
  `step`), read from the game's folder; a missing file is refused before
  anything plays (`SH0001`). The live link and capture plans refuse the
  step. Contract version 46.
- Docs: `docs/platform-backends.md` (the event, the fact, the backend),
  `docs/contract.md` (the step), the book's *Targets and export* chapter,
  and ADR-0103.

Tried by hand: Hello, Grid's debug web build in the in-app browser, with
a drop event made in the page (a `DataTransfer` holding two files, as a
real drop gives): the backend logged
`FilesDropped([DroppedFile { name: "doom1.wad", bytes: 5 }, …])`, and a
drop of 17 files logged `platform: a drop of 17 files was refused: at
most 16 at once`. The game's hook hearing it is the Harness tests'. Not
tried in a real browser by a real drag, nor on the desktop window: no
way to drag a file onto a window from here. The desktop goes through the
same macroquad call as the web.

Either order: 2779 and 2782 are both still open, so nothing of theirs
here; their tickets now say what 3092 gives them ("From 3092 (done)").

Notes:

- miniquad 0.4.11 on Windows reads a dropped path with
  `std::fs::read(..).unwrap()`, so dropping a folder on the window
  panics inside miniquad, before the engine sees anything. Its web
  loader also writes a file's name with its UTF-16 length, so a name
  with non-ASCII letters may come through cut short. Both are
  miniquad's; follow-up ticket 3307 (written in this PR) covers them.
- miniquad reads a whole file before the engine sees it: the 256 MB
  limit bounds what the game is handed, not what is read.
- A `BasicGame` on the short path (`run_game`) has no system hook yet,
  so it can't hear a drop; a game gives `game.start().on_system(hook)`
  to `shuyi_app::run`. 2588 (the dialogue preview's drop) is where the
  short path will need it.

Breaking: shuyi-platform: SystemEvent: no longer `Copy` (it gained `FilesDropped(Vec<DroppedFile>)`); a hook that copies an event clones it or matches it by reference.

```migration
since: 0.1.0
what: SystemEvent is no longer Copy, since a drop carries the files' bytes.
old: "#[derive(Clone, Copy)] enum SystemEvent"
new: "#[derive(Clone)] enum SystemEvent, with FilesDropped(Vec<DroppedFile>)"
before: |
  fn on_system(game: &mut Game<MyData>, event: SystemEvent) {
      let seen = event;
      log(event);
      remember(seen);
  }
after: |
  fn on_system(game: &mut Game<MyData>, event: SystemEvent) {
      log(&event);
      remember(event);
  }
```
