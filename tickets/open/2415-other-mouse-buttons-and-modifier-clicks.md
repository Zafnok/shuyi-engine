---
id: "2415"
title: "Right, middle and side mouse buttons as bindable inputs, and the modifiers held on a click, for turn games: split out of parked 1121"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2409"]
nick_input: none
completed:
---

# 2415 — Other mouse buttons and modifier clicks

## Context

`shuyi-app` reads only the primary mouse button
(`crates/app/src/pointer.rs`, `read`: `MouseButton::Left`). The only
plan for the others is 1121, parked with the real-time work, and 2409
and 3233 both put "mouse buttons other than the primary" out. The Bevy
coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*)
found the turn games that need them now: tactics and strategy games
that right-click to cancel or inspect, card and inventory games that
shift-click to move a stack, and point-and-click adventures with look
and use on two buttons. Bevy reads every mouse button and modifiers
(`crates/bevy_input/src/mouse.rs`; `examples/input/mouse_input.rs`,
`keyboard_modifiers.rs`). As 2405 split the menus' mouse out of 1121,
this splits out the other buttons for turn games; 1121 keeps their use
in `TickInput`.

Rule 8 holds: a game binds a button to an action, and text names it
through the keymap.

## Nick input

None.

## Scope

**In:**
- `shuyi-app` turns the right, middle, back and forward buttons into
  `RawInputEvent`s, beside the primary's (ADR-0134).
- `shuyi_ui::input` gains `Button::MouseRight`, `MouseMiddle`,
  `MouseBack` and `MouseForward` as buttons a keymap binds (ADR-0039:
  the button enum is `#[non_exhaustive]`), named in help text and on
  the key-bindings screen.
- `PointerInput` carries the modifiers held on each press as a plain
  bitset of actions held (a game's `Shift`-like action, never key
  codes), so a screen reads shift-click as data.
- The web backend reads the same buttons and suppresses the browser's
  context menu over the canvas.
- The Harness gains a right click and a click with a modifier held.

**Out (do not do):**
- the buttons in `TickInput` and the pointer as a game action (1121,
  parked, which reuses these events);
- mouse chords (two buttons at once as one binding);
- a context menu widget.

## Implementation steps

1. Read `crates/app/src/pointer.rs`, ADR-0134, the `input-actions`
   skill, `crates/ui/src/input/` and 2409's code.
2. Read the other buttons in `shuyi-app` (native and web) into events.
3. Add the bindable buttons and their names.
4. Add the modifiers to `PointerInput`.
5. Extend the Harness; write the docs page: right-click to cancel on a
   grid map.

## Acceptance criteria

- [ ] A keymap binding `Cancel` to `MouseRight` cancels on a right
  click in the Harness.
- [ ] A shift-click reports the modifier on the press, and a plain click
  doesn't.
- [ ] The key-bindings screen shows `MouseRight` with its name from the
  keymap's language.
- [ ] A right click over the web canvas opens no browser menu (checked
  in the web build's test page).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: event reading, the bitset, the names.
- Snapshot / integration: Harness runs of each button and a modifier.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
