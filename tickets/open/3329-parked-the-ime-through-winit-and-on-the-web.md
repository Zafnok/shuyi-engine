---
id: "3329"
title: "Parked: The IME on the winit backend: composing text with its caret as input events, drawn underlined in the text box, and the IME on and placed from Frame::typing on Windows, macOS, Linux and the web"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3327", "3330", "2346"]
nick_input: none
completed:
---

# 3329 — The IME on the winit backend

> **Parked** (Nick, 2026-11-22, `docs/decisions.md`): written by research 3034, whose research alone he unparked. The build waits for the MVP with the rest of the in-house platform layer (2026-10-19), and for 3079 through 3327 and 3330.

## Context

2346 turns the IME on only in the name box, on Windows, with miniquad
0.4.11's calls. It adds `Frame::typing` and the buffer's text caret
(`GlyphBuffer::text_caret`). miniquad has no IME on macOS, Linux or the
web, and gives no composing text, so a Japanese or Chinese player can't
see what they are composing.

winit gives the IME's events on Windows, macOS, X11 and Wayland:

- `Ime::Preedit(text, cursor)` and `Ime::Commit(text)`;
- `set_ime_allowed` and `set_ime_cursor_area` (`request_ime_update` in
  0.31).

It has none on the web, in 0.30.13 or 0.31's betas (its issue 4424).

[ADR-0136](../../docs/adr/0136-replacing-macroquad-winit-wgpu-and-wasm-bindgen.md)
§12 does the web with a hidden text area of our own.

## Nick input

None.

## Scope

**In:**
- **`RawInputEvent::Composing { text, caret }`** in `shuyi-ui`:
  - the IME's composing text and its caret (a `char` index, or `None`
    for no caret);
  - an empty `text` when composing ends;
  - additive, since the enum is `#[non_exhaustive]` (ADR-0039).
  - The committed text is `Text` events, one per `char`, as typing is.
- **The name box's typing** (2346's `NameBox`, and every text box built
  on it):
  - The box keeps the composing text apart from the typed text.
  - The box draws it after the typed text, underlined, its caret where
    the IME says.
  - The text is never stored until it is committed.
  - **Either order with 1149:** if 1149 is done, its chat box draws the
    composing text too, with a test; if not, nothing to do (1149's box,
    built on the name box's typing, gets it from there).
- **The desktop** (Windows, macOS, X11, Wayland):
  - `Ime::Preedit` and `Commit` into those events;
  - `set_ime_allowed(true)` while `Frame::typing` is set, `false`
    otherwise;
  - `set_ime_cursor_area` at the caret's cell, in window pixels, from
    the layout, while typing, when the caret moves.
  - 2346's Windows behaviour (the IME off during play, on in the name
    box) carries over.
- **The web**, written from MDN and the UI Events specification
  (ADR-0021 §2), with `web-sys`:
  - a hidden `<textarea>` placed over the caret's cell (CSS position in
    the canvas's coordinates), focused while `Frame::typing` is set;
  - its `compositionstart`, `compositionupdate` and `compositionend` give
    the composing text, and `input` with no composition gives `Text`;
  - its value cleared after each commit;
  - the canvas takes the focus back when typing ends, so the keys reach
    the game as before (3505).
- **The docs:** the `input-actions` skill and the book's chapter on
  text input say that composing text arrives as its own event.

**Out (do not do):**
- The macroquad backend: it keeps 2346's Windows calls until 3333.
- The soft keyboard on Android and iOS (3015).
- `set_ime_purpose` (Wayland only).

**Estimated size:** about 1,200 lines (code 650, tests 400, docs 150).

**Split:** past 1,000 lines, split off the web's text area into a new
ticket.

## Implementation steps

1. Read ADR-0136 §12, 2346's ticket and its code (`NameBox`,
   `Frame::typing`, `text_caret`), winit's `Ime` docs, and MDN's
   `CompositionEvent` and `InputEvent` pages and the UI Events
   specification's composition events.
2. The event in `shuyi-ui`, and the name box's composing text, with
   Harness tests (`Harness::frame` with `Composing` events).
3. The desktop.
4. The web's text area.
5. The docs.

## Acceptance criteria

- [ ] A Harness test types `に` by composing `n`, `i` and committing,
      and sees the composing text underlined in the box before the commit
      and in the name after it.
- [ ] With Microsoft IME (Japanese) on Windows, and with the IME on
      macOS and on Linux (IBus on X11, and Wayland), the composing text
      shows in the box and the candidates sit at the caret (manual, in
      the notes).
- [ ] With the Japanese IME in Chrome and Firefox, the same on the web
      (manual, in the notes).
- [ ] During play the IME is off on every platform: letters reach the
      game (a manual check on Windows with the Japanese IME on).
- [ ] The `render` and `web-render` jobs stay green on both backends:
      their plans type nothing, and the hidden text area draws nothing on
      the canvas.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Composing` into the box's state (start, update, caret, cancel,
  commit).
- Snapshot: the box with composing text underlined (`frame-png`).
- Browser (`tools/web-check`): composition events dispatched on the text
  area reach the game.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
