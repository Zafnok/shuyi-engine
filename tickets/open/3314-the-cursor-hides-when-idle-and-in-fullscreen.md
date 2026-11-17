---
id: "3314"
title: "The cursor hides when the mouse is still and in fullscreen, and shows again when it moves"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["3044"]
nick_input: none
completed:
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

- [ ] `IdleCursor`, with made-up times: shown at a windowed start,
      still shown at 1999 ms, hidden at 2000 ms; shown again on a move
      or a button; hidden at once on entering fullscreen and at a
      fullscreen start; a position change within 199 ms of a mode
      change shows nothing, and one at 200 ms shows it; shown while the
      window lacks focus; never hidden with `keep_shown`; a change is
      reported only when the answer changes.
- [ ] `show_mouse` is called from `after_frame` only on a reported
      change.
- [ ] The PR says what was seen on Windows for the checks in step 4.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/window.rs`): every case of `IdleCursor` above.
- Manual: the checks on Windows in step 4.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
