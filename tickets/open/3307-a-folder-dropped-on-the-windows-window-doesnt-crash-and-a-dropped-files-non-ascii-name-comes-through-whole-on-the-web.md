---
id: "3307"
title: "A folder dropped on the Windows window doesn't crash, and a dropped file's non-ASCII name comes through whole on the web"
type: bug
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3092"]
nick_input: none
completed:
---

# 3307 — Dropped folders and non-ASCII names

## Context

3092 hands files dropped on the window to the game
(`SystemEvent::FilesDropped`, [ADR-0103](../../docs/adr/0103-dropped-files-a-system-event-with-names-and-bytes-limits-kept-by-app-and-a-fact-the-backend-says.md)),
from what miniquad 0.4.11 reads. Two bugs of miniquad's sit in front of
it, found reading its source while 3092 was worked:

- **Windows:** `WM_DROPFILES` reads each path with
  `std::fs::read(&path).unwrap()` (`src/native/windows.rs`). A folder,
  or a file the player can't read, panics inside miniquad's window
  procedure: the game crashes when a player drops a folder on it. X11,
  Wayland and macOS skip such a path instead.
- **The web:** the loader (`js/gl.js`, vendored as
  `crates/cli/web/mq_js_bundle.js`) sizes a dropped file's name with
  `file.name.length`, its UTF-16 length, then writes it as UTF-8 into
  that many bytes: a name with letters beyond ASCII (`ドゥーム.wad`,
  `niño.txt`) comes through cut short.

## Nick input

None.

## Scope

**In:**
- Windows: a dropped folder or unreadable file is left out, and the
  others of the drop still reach the game. Fixed upstream in miniquad
  (a PR there) and, until a release has it, here: a `[patch.crates-io]`
  of miniquad, or the backend reading drops itself on Windows. Choose,
  and write it in the completion notes (an ADR if it is a patch).
- The web: the name's UTF-8 length (`new TextEncoder().encode(name)`),
  in the vendored `mq_js_bundle.js` and upstream; the vendored file's
  origin note says what changed (`crates/cli/web/README.md`).

**Out (do not do):**
- A file picker (3092's **Out**).
- Anything of 3092's limits or event.

## Implementation steps

1. Read miniquad 0.4.11's `src/native/windows.rs` (`WM_DROPFILES`) and
   `js/gl.js` (`window.ondrop`), and `crates/app/src/drops.rs`.
2. The Windows fix, and its upstream PR.
3. The web fix in the vendored bundle, and its upstream PR.
4. Try both: a folder dropped on Hello, Grid's window on Windows; a
   file with a non-ASCII name dropped on its web build.

## Acceptance criteria

- [ ] A folder dropped on the window on Windows doesn't crash the game,
      and a file dropped with it still reaches the game.
- [ ] A file named with non-ASCII letters, dropped on the web build,
      reaches the game with its whole name.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the name's length in the loader, if it can be tested in
  `tools/web-check`; otherwise by hand, written in the notes.
- Snapshot / integration: by hand on Windows (a drag can't be scripted
  in CI).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
