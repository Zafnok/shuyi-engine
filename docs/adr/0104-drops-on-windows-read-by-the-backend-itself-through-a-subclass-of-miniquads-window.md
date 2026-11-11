# ADR-0104: Drops on Windows, read by the backend itself through a subclass of miniquad's window

- **Status:** Accepted; amends ADR-0103 §4
- **Date:** 2026-11-10
- **Related tickets:** 3307, 3092

## Context

ADR-0103 §4 has `shuyi-app` read drops from macroquad's
`get_dropped_files`. On Windows, miniquad 0.4.11 answers `WM_DROPFILES`
in its own window procedure (`src/native/windows.rs`) with two faults:

- it reads each path with `std::fs::read(&path).unwrap()`, so a dropped
  folder, or a file the player can't read, panics inside the window
  procedure, and the process aborts (a panic can't unwind out of an
  `extern "system"` function);
- it stores the files but never calls `files_dropped_event`, so macroquad
  never hands a drop on Windows to anyone.

A fix in miniquad has to reach every game on the engine. A
`[patch.crates-io]` in this workspace does not: Cargo applies patches
only from the root workspace, and a game builds this repository as a git
dependency, so each game would have to copy the patch.

## Decision

1. **The backend answers `WM_DROPFILES` on Windows.** In its first
   `system_events`, `shuyi-app` finds miniquad's main window among this
   thread's (class `MINIQUADAPP`) and replaces its procedure with its own
   (`SetWindowLongPtrW(GWLP_WNDPROC)`); every other message goes on to
   miniquad's with `CallWindowProcW`. Its procedure reads each path of
   the drop, at any length, leaves out what isn't a file it can read,
   frees the drop with `DragFinish` and keeps the files until the next
   frame (`crates/app/src/drops/windows.rs`).
2. **One way out.** The files join what `get_dropped_files` gives, in
   `drops::poll`, so the rest of ADR-0103 holds unchanged: the same
   event, limits, logs and fact.
3. **`unsafe` stays in the backend.** The module holds the Win32 calls
   under `#[allow(unsafe_code)]`, as the web plugins' declarations do
   (game ADR-0034). Its calls come from `winapi` 0.3, which miniquad
   already builds; `shuyi-app` adds the `processthreadsapi` feature.
4. **Upstream too.** The same fix is offered to miniquad. When a miniquad
   release reads drops without panicking and fires `files_dropped_event`
   on Windows, the subclass goes, and `get_dropped_files` is the one way
   in again.

## Consequences

- A folder dropped on the window on Windows is left out and the files
  dropped with it still reach the game; before, drops never reached a
  game on Windows at all.
- A path longer than `MAX_PATH` comes through whole, where miniquad's
  buffer would cut it.
- The backend depends on miniquad's window class name and on being on
  the thread that made the window. A miniquad that renames the class
  leaves drops unheard on Windows, not crashing; the hand check in
  ticket 3307's notes finds it.
- Reading the files still blocks the window's message loop while they
  load, as miniquad's reading did.

## Alternatives considered

- **A `[patch.crates-io]` of miniquad**: fixes this repository's builds
  only; every game would need the same patch, and a forked miniquad to
  point it at.
- **Polling miniquad's `dropped_file_count` on Windows**: it keeps the
  last drop until the next, so two drops of the same files look like
  one, and the panic stays.
- **`SetWindowSubclass` (comctl32)**: the same result with one more
  system library to link, on every toolchain.
