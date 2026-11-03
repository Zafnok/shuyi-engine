---
id: "3092"
title: "Files dropped on the window: a dropped file handed to the game as a bundle layer, on the desktop and the web"
type: feature
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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
- **3069** (open) lays files the page holds in `sessionStorage` over a
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

- [ ] A file dropped on the desktop window reaches the game's system
      hook with its name and bytes; so does one dropped on the web
      build's canvas.
- [ ] Files over the limits are refused and logged.
- [ ] `ctx.platform` says where drops work; the Playdate and headless
      say no.
- [ ] A harness test drops a file through the input script and the
      game sees it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `DroppedFile::name` strips any path; the limits; the layer.
- Snapshot / integration: the harness drop step; a `shuyi-app` test
  of the handoff with a fake drop source if miniquad's can't be driven
  in a test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
