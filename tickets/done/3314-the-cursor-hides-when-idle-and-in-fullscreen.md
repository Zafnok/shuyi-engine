---
id: "3314"
title: "The cursor hides when the mouse is still and in fullscreen, and shows again when it moves"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["3044"]
nick_input: none
completed: 2026-11-17
---

# 3314 — The cursor hides when the mouse is still and in fullscreen

## Context

Split from 3044 by its *Size* clause: 3044 (the window opens as it was
left: its record per computer, `Window::console`, no windowed flash
before fullscreen) passed 600 changed lines without the cursor, so the
cursor is this ticket.

The 2026-10-20 comparison (3044's *Context*) found that no benchmark
engine hides an idle cursor by default: in Bevy (`CursorOptions::visible`),
Godot (`DisplayServer` mouse modes) and GameMaker
(`window_set_cursor(cr_none)`) it is game code, which every
keyboard-and-pad desktop game writes. In Shuyi no code calls
`show_mouse`: the system's arrow stays over the picture, in fullscreen
too. A game made with Shuyi should get the usual rule for free.

What 3044 left in place (`crates/app/src/backend.rs`,
`crates/app/src/window.rs`):

- `Macroquad::after_frame(now: Millis)`, called by `run`
  (`crates/app/src/lib.rs`) after each `App::frame` with the frame's
  clock: it writes the window's record today, and is where the cursor
  is updated;
- `Macroquad::fullscreen`, the mode the window is in, which starts at
  the mode it opened in (`window::Opened::fullscreen`) and changes only
  in `switch_fullscreen`;
- `Macroquad::focus` (`crate::focus::WindowFocus`, 3042), whose
  `is_focused()` says whether the window has the player's focus;
- a capture run (`capture::run`, 2018) never reaches `after_frame`.

What miniquad 0.4.11 gives (the version in `Cargo.lock`):
`window::show_mouse(bool)` on every desktop (a request handled by the
event loop; on Windows it is answered in `WM_SETCURSOR`, so only over the
window), and on the web the canvas's CSS `cursor`. macroquad's
`mouse_position()` is updated from mouse-move events only, and Windows
may send one when the window moves under a still mouse (as on entering
or leaving fullscreen), so a mode change must not read as a move.

## Nick input

None.

## Scope

**In:**
- **A pure `IdleCursor`** in `crates/app/src/window.rs`, on native and
  the web, fed each frame with the time (`Millis`), the mouse's position
  in whole pixels, whether a mouse button was pressed, whether the
  window is fullscreen, whether it has focus, and `keep_shown`:
  - moving the mouse (its position changed since the last frame) or
    pressing a mouse button (`is_mouse_button_pressed` for any button)
    shows the cursor;
  - it hides after 2 s with neither (`CURSOR_IDLE`, *tunable*);
  - it hides at once when the window enters fullscreen, and on the
    first frame of a fullscreen start;
  - for `MODE_SETTLE` (200 ms, *tunable*) after the mode changes, a
    change of position is the window moving, not the mouse: it is taken
    as the new resting place and shows nothing;
  - it is shown while the window lacks focus (3042's `FocusLost`), so it
    is never hidden over another program;
  - `keep_shown: bool` keeps it shown;
  - it reports a change (`Option<bool>`) only when the answer changes,
    and the backend calls `show_mouse` only then. It starts as the
    system's: shown.
- **Wiring:** `Macroquad` keeps an `IdleCursor` and updates it in
  `after_frame` from macroquad's `mouse_position()` and
  `is_mouse_button_pressed`, its own `fullscreen` and `focus`. Reading
  the mouse here is not input: nothing reaches `poll_input` or the
  game, and no action is read (`AGENTS.md` rule 8).
- `keep_shown` is `false` everywhere.
  **Either order with 1121:** if 1121 is done, `shuyi-app` sets it
  while the top screen declares an action bound to a mouse button, with
  a test; if not, nothing to do here (1121 sets it, its clause says so).
- **Either order with 3106:** a locked pointer is never shown. If 3106
  is done, `IdleCursor` takes a `locked: bool`, set while 3106's pointer
  lock holds, that hides the cursor and wins over mouse motion and
  `keep_shown`, with a unit test; if not, nothing to do here (3106 adds
  it, its clause says so).
- **Either order with 3062:** if 3062 is done, `IdleCursor` reads the
  mode from its `FullscreenWatch::mode()`, so a mode changed with
  macOS's own control hides or shows the cursor like any other switch;
  if not, nothing to do here (3062 does it, its clause says so).
- **Docs:** `docs/platform-backends.md`'s section *The window (desktop)*
  (3044) becomes *The window and the cursor (desktop)*, with the cursor
  rule, so the winit backend (3034) does the same; `shuyi-app`'s
  `run_app` docs say the cursor hides.

**Out (do not do):**
- custom cursor images and confining the cursor (the winit backend,
  3034); pointer lock (3106), apart from its clause above;
- pointer input for menus and grid maps (2405) and for games (1121).
  2405 needs nothing from here: moving the mouse or clicking shows the
  cursor;
- an option to turn the rule off: a follow-up if a game asks.

**Estimated size:** about 550 lines (code 150, tests 300, docs 100).

## Implementation steps

1. Read `crates/app/src/window.rs` and `backend.rs` (`after_frame`,
   `switch_fullscreen`, `focus`), and macroquad 0.4.16's
   `src/input.rs` (`mouse_position`, `is_mouse_button_pressed`,
   `show_mouse`).
2. Write `IdleCursor`, `CURSOR_IDLE` and `MODE_SETTLE` in `window.rs`,
   pure, with their tests.
3. Wire it in `Macroquad::after_frame`.
4. Write the docs. Run hello-grid on Windows and check: the mouse still
   for 2 s in a window hides the cursor, a move shows it, F11's
   fullscreen hides it at once and leaving fullscreen doesn't show it by
   itself, and Alt+Tab to another program shows it there. Say in the PR
   what was seen, and that macOS, Linux and the web were not checked by
   hand if they weren't.

## Acceptance criteria

- [x] `IdleCursor`, with made-up times: shown at a windowed start,
      still shown at 1999 ms, hidden at 2000 ms; shown again on a move
      or a button; hidden at once on entering fullscreen and at a
      fullscreen start; a position change within 199 ms of a mode
      change shows nothing, and one at 200 ms shows it; shown while the
      window lacks focus; never hidden with `keep_shown`; a change is
      reported only when the answer changes.
- [x] `show_mouse` is called from `after_frame` only on a reported
      change.
- [x] The PR says what was seen on Windows for the checks in step 4.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/window.rs`): every case of `IdleCursor` above.
- Manual: the checks on Windows in step 4.

## Completion notes

- **`IdleCursor`** (`crates/app/src/window.rs`), pure, with
  `CURSOR_IDLE` (2 s) and `MODE_SETTLE` (200 ms). What it sees each
  frame is one struct, `CursorSeen` (the clock, the `Mouse`: its
  position in whole pixels and a press, the mode, the focus,
  `keep_shown`), rather than six arguments; the mouse is a struct of its
  own because clippy's `struct_excessive_bools` allows three bools in
  one. `see` returns `Some(shown)` only when the answer changes.
  Eleven unit tests cover every case of the acceptance list, and a press
  during the settle (it shows), and focus lost in fullscreen.
- **Wiring:** `Macroquad` keeps an `IdleCursor` and feeds it in
  `after_frame` from `mouse_position()` (rounded), any of the left,
  right and middle buttons' `is_mouse_button_pressed`, its `fullscreen`
  and `focus`, with `keep_shown: false`, and calls `show_mouse` only on
  a reported change. On the web a frame the pacer skips also keeps
  whether a button was pressed (`Held::keep_press` in `hold_input`),
  since macroquad forgets it at the frame's end. Nothing reaches `poll_input` or the game.
- **Either-order clauses:** 1121, 3106 and 3062 are open, so none was
  done here. Each one's clause now says what holds, "From 3314 (done)",
  and names `CursorSeen` as where its field goes (`keep_shown` for 1121,
  a new `locked` for 3106, the mode for 3062).
- **Docs:** `docs/platform-backends.md`'s *The window (desktop)* is now
  *The window and the cursor (desktop)*, with the rule, and its two
  links and 3034's are renamed; `run_app`'s docs (which already linked
  the new name) say the cursor hides; the book's *Games and screens*
  chapter says so in one sentence.
- **Seen on Windows 11** (hello-grid, debug build, on `24c31b3b`, the
  tip of `main` merged), driven by a PowerShell script that moves the
  mouse (`SetCursorPos`), presses F11 (`keybd_event`) and gives the
  focus to the terminal, reading the system's cursor with
  `GetCursorInfo` (`CURSOR_SHOWING`) and checking the mouse is over the
  window: shown after a move, still shown at 1.5 s, hidden at 2.5 s;
  shown after a 5 px move; hidden 120 ms after F11 into fullscreen and
  still at 1.1 s; shown after a move in fullscreen, hidden 2.5 s after it;
  still hidden 400 ms and 1.2 s after F11 out of fullscreen, with the
  mouse over the window; shown over the game once the terminal had the
  focus, and still shown 2.5 s after. A first run that left fullscreen with
  the mouse at the screen's centre read "shown": the window had shrunk
  back, the mouse was no longer over it, and `IdleCursor` had reported
  nothing (a log of each frame said so), so the run was redone with the
  mouse kept over the window's place. macOS, Linux and the web were not
  checked by hand.

Behaviour: shuyi-app: the system's cursor hides over a game's window
once the mouse has neither moved nor clicked for 2 seconds, and at once
when the window enters fullscreen or starts in it; a move, a mouse
button or losing the focus shows it again. Searched the game
(`gh search code --repo Zafnok/visions-of-shuyi` for `show_mouse`,
`set_cursor_grab`, `mouse_position` and `cursor`): it never shows,
hides or reads the system's cursor (its cursor is the grid's, ADR-0006
and ADR-0024 of the game), so it changes nothing.

Book: `site/content/learn/book/games-and-screens.md` says the cursor
hides. No public item changed.

Nick input: none.
