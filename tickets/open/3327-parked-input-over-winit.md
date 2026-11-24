---
id: "3327"
title: "Parked: Input over winit: keys and text, the mouse and touch, focus and visibility as ADR-0076 maps them, the idle cursor, dropped files and the cursor grab"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3326"]
nick_input: none
completed:
---

# 3327 — Input over winit

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 through 3326.

## Context

3326 runs the game in a winit window, with no input.
[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
§12 says how winit's events become the engine's `RawInputEvent`s and
`SystemEvent`s, which `crates/app/src/keys.rs`, `focus.rs`, `drops.rs`
and `backend.rs` make from macroquad's today. This ticket does that on
the desktop, in `src/winit/`. The web gets the same mapping from 3330,
since winit's events are the same there.

## Nick input

None.

## Scope

**In:**
- **Keys:**
  - `KeyboardInput`'s `physical_key` maps to `shuyi_ui::input::Key`, by
    position, with a table as total as `keys::to_key`'s and its tests
    (every engine key has a code; other codes are ignored).
  - The Shift chord comes from `ModifiersChanged`.
  - Synthetic presses (`is_synthetic`) are dropped.
  - The fullscreen shortcut is `keys::fullscreen_shortcut`'s chord.
- **Text:** `KeyEvent::text` becomes one `RawInputEvent::Text` per
  `char`, the control characters left out as today.
- **The mouse and touch,** as 2413 (done) maps macroquad's in
  `shuyi-app`, with its tests passing on both backends:
  - `CursorMoved` maps to the console's pixels through the layout;
    `CursorLeft` becomes `PointerLeft`.
  - The left button gives `PointerDown` and `PointerUp`.
  - `MouseWheel`'s lines and pixels become whole `Wheel` notches, as
    2413 counts them per system.
  - The first touch is the pointer, and a tap has no hover after it
    (ADR-0134).
- **Focus and visibility (ADR-0076):**
  - `Focused` gives `FocusLost` and `FocusGained`.
  - `Occluded` gives `Suspend` and `Resume` on macOS and X11.
  - On Windows a minimized window (`is_minimized()`, read on `Resized`)
    gives `Suspend`, and its restore gives `Resume`. Check it on the
    machine.
  - Wayland gets focus alone.
  - `focus.rs`'s quirks for miniquad (the double minimize, the move on
    macOS) aren't needed here. Keep them for the macroquad path.
- **The idle cursor** (3314): hidden after two seconds at rest and at
  once in fullscreen, shown on a move, a press or a focus loss, with
  `set_cursor_visible`.
- **Dropped files** (3307): `WindowEvent::DroppedFile` gives the game a
  `DroppedFile`, as `drops.rs` does, on every desktop system.
- **The cursor grab** (ADR-0136 §12):
  - a game's request, carried on `Frame`, held with
    `CursorGrabMode::Locked`, else `Confined`;
  - a locked cursor never shown;
  - let go on focus loss and taken again on focus.
  - **Either order with 3106:** if 3106 is done, move its pointer lock
    in `shuyi-app` to winit's grab, with its tests green on both
    backends; if not, nothing to do (3106 adds the request and maps it
    on each backend that exists then).
- **The check of one non-US layout.** Press the letters and the keys
  around them on a French AZERTY layout, in the macroquad build and in
  this one, as black boxes. Write any difference as a `Behaviour:` line.

**Out (do not do):**
- The IME: turning it on and off, placing it, its composing text, and
  the web's (3329, after 2346).
- Controllers: the native pad source stays as it is on the macroquad path,
  whichever `shuyi-app` has (ADR-0146 names SDL3); winit has no gamepads.
- The window facts miniquad lacks (3328).

**Estimated size:** about 1,300 lines (code 750, tests 450, docs 100).

**Split:** past 1,000 lines, split off the cursor grab and dropped files
into a new ticket.

## Implementation steps

1. Read ADR-0136 §12, ADR-0076, ADR-0025, the `input-actions` skill,
   `keys.rs`, `focus.rs`, `drops.rs`, `backend.rs`'s `poll_input` and
   `system_events`, and winit 0.30's `event` docs.
2. `src/winit/keys.rs`: the table, the chords, text, synthetic presses.
3. The pointer, the wheel and touch.
4. Focus, visibility and minimize; the idle cursor; drops; the grab.
5. The AZERTY check, and the docs: `docs/platform-backends.md`'s
   `poll_input` and `system_events` say what the winit backend maps.

## Acceptance criteria

- [ ] Every engine `Key` has a winit `KeyCode`, and a winit code the
      engine doesn't know is ignored (tests).
- [ ] A frame's key events come in the order they happened, as
      `keys::ordered` has them (a test over the handler).
- [ ] Hello, Grid plays with the keyboard and the mouse under `winit` on
      Windows, and on Linux (X11 and Wayland), recorded in the notes.
- [ ] Alt-tabbing away gives `FocusLost`, and minimizing on Windows
      gives `Suspend`, then `Resume` (a manual run's log).
- [ ] The idle cursor behaves as 3314 says, under `winit`.
- [ ] The `render` job stays green on both backends.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the key table, the chords, text without control characters, the
  wheel's steps, the pointer's console pixels at scale 1, 2 and 3 with a
  letterbox, focus and minimize into events.
- Manual: the AZERTY check, Windows and Linux.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
