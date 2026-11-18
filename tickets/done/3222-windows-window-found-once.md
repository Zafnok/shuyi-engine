---
id: "3222"
title: On Windows the backend finds miniquad's window once, not by listing the thread's windows every frame
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-17
---

# 3222 — On Windows the backend finds miniquad's window once, not by listing the thread's windows every frame

## Context

The retro of PRs #379–#484 (2026-11-17) found a per-frame cost that 3044
(#469, the window opens as it was left) brought in on Windows.

- `Backend::after_frame` (`crates/app/src/backend.rs`, about line 232)
  reads `window_size()` every frame, so the window's record (`crate::window`)
  can see the size settle.
- On Windows, `window_size()` (about line 450) calls
  `crate::window::win32::client_size()` (`crates/app/src/window/win32.rs`,
  line 18).
- That calls `crate::drops::miniquad_window()` (`crates/app/src/drops/windows.rs`,
  lines 71–97). It runs `EnumThreadWindows` over every window of the
  thread, and for each one calls `GetClassNameW` and builds a `String`
  with `String::from_utf16_lossy` to compare with miniquad's class name.

All of that happens every frame, to find a window that never changes
while the game runs. `switch_fullscreen` (about line 270) and the drop
hook's `install` (`drops/windows.rs`, line 54) call the same lookup. The
hook already does its work once, behind an `INSTALLED` flag.

3221 (a pinned allocation ceiling for a steady frame) leaves this out:
it lives in the backend, only on Windows, where a test of `Game::frame`
can't see it.

## Nick input

None.

## Scope

**In:**
- `miniquad_window()` finds the window once and keeps its `HWND` for
  the run, for example in an `AtomicPtr` or a `OnceLock` (an `HWND` is
  a pointer, so it needs a `Send`/`Sync` wrapper or an atomic). It
  doesn't cache a miss: before miniquad has made its window, a later
  call looks again.
- The class-name comparison stops allocating: it compares the UTF-16
  units with a `const` UTF-16 copy of `MINIQUAD_CLASS`.
- `drops` and `window::win32` share the one cached lookup.

**Out (do not do):**
- Any other per-frame work in `after_frame`: the cursor, the record.
- macOS and Linux, which don't use this lookup.
- The frame's own allocations (3221).

## Implementation steps

1. In `crates/app/src/drops/windows.rs`, add a static that holds the
   found `HWND`, null until found. `miniquad_window()` returns it when
   it is set. Otherwise it runs `EnumThreadWindows` as today, and stores
   and returns the result when that finds one.
2. In `find`, compare `&class[..len]` with a `const` UTF-16 array of
   `MINIQUAD_CLASS` (or `MINIQUAD_CLASS.encode_utf16().eq(...)`), with no
   `String`.
3. Keep `win32::client_size()` and `install()` calling
   `miniquad_window()`, which is now cheap.
4. Comment the static: miniquad makes one window, on this thread, and it
   lives for the whole run (game ADR-0034's backend `unsafe`).

## Acceptance criteria

- [x] After the first frame that finds miniquad's window, `after_frame`
      calls `EnumThreadWindows` no more on Windows. A counter visible to
      the tests shows it.
- [x] `find` makes no `String`.
- [x] The window's record and the drop of files behave as before.
      ~~3044's manual check (resize, then relaunch at 175 % scale, on
      Windows) is repeated, and the completion notes say what it showed.~~
      (not done: it changes the system's display scale, which the agent
      may not change; see the completion notes)
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (Windows only, `#[cfg(windows)]`): the lookup's counter stays at
  one over many calls once the window is found. It counts again while
  nothing has been found.
- Unit: the UTF-16 class comparison matches `MINIQUAD_CLASS` and rejects
  a different class name and a prefix of it.

## Completion notes

- `drops/windows.rs` keeps miniquad's window in a static `AtomicPtr`
  (`WINDOW`), null until found. `miniquad_window()` returns it once set,
  and otherwise lists the thread's windows as before, keeping what it
  finds; a miss isn't kept. `install()` and `window::win32::client_size()`
  still call `miniquad_window()`, so they share the one lookup.
- `find` compares the class name's UTF-16 units with a `const` UTF-16
  copy of `MINIQUAD_CLASS` (`MINIQUAD_CLASS_UTF16`, built by a `const fn`),
  with no `String`.
- Tests (`drops::windows::tests`, Windows only as the module is): the
  comparison accepts `MINIQUADAPP` and refuses another name, a prefix and
  a longer name; `cached` searches again on each miss and once only after
  a find; and a real window of miniquad's class, made on the test's
  thread, is found with one listing over 100 calls, after two listings
  that found nothing. A `#[cfg(test)]` counter, `ENUMERATIONS`, counts the
  listings.
- Deviations: none. The ticket was written today, so no age check.
- Docs: none. The change is inside the backend, with no public item,
  format or rule.
- No break, and no `Behaviour:` line: a game sees the same window size
  and the same drops.
- Manual check: not repeated by the agent. 3044's check (resize, then
  relaunch at 175 % scale) changes the system's display scale, which the
  agent may not change. The real-window test covers the lookup itself;
  Nick can repeat the check on his machine if he wants it seen.
