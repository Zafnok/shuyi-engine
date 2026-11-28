---
id: "3260"
title: "The screens' glyph painters share the panel, a focus bar, Rect::centred_in and one scroll window"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3260 — The screens' glyph painters share the panel, a focus bar, `Rect::centred_in` and one scroll window

## Context

The redundancy sweep of 2026-11-28 found that the engine's screen and
widget painters hand-draw what `GlyphBuffer::panel` and
`ListCursor::window_by_heights` already do:

- **Hand-drawn boxes.** Seven production `draw_box`, fill and title
  sites outside `frame/`: `screens/profiles/glyph.rs` ×2,
  `widgets/ask/glyph.rs`, `widgets/name_entry/glyph.rs`,
  `dialogue/text_box/glyph.rs` and `debug/glyph.rs` ×2.
- **The focus bar.** The focused `' label '` bar is written at about 7
  sites: `options/glyph.rs` 187, `profiles` 122 and 151,
  `key_bindings/glyph.rs` 319, `ask` 76, `name_entry` 74 and the
  preview.
- **Centring.** Centring is hand-written in `profiles`' `middle()` and
  in `ask`. `name_entry/glyph.rs:17` has a private `centred_in`. The
  game's sweep (game#1397) counted 20 copies of the same `centred_in`
  in the game.
- **Scroll windows.** A "keep the focus in view" window is computed
  three more times:
  - options' `first_shown` (about 96–105);
  - the dialogue preview's `first_shown` (about 199–213);
  - key_bindings' `Scroll`, `scroll` and `shown` (about 224–279);

  beside `ListCursor::window_by_heights` (`widgets/list.rs:164`).

About 115 lines go. More valuable: 2338 (picture frames), 2371 (one-bit
looks) and 2409 (pointer input) each edit these painters. After this
ticket each of them changes one place, not seven.

## Nick input

None.

## Scope

**In:**
- **Painters call `buf.panel(rect, &PanelLook, title)`** in place of
  hand-drawn boxes, where the cells come out the same.
- **`bar_label`**, a helper for the focused label bar.
- **`Rect::centred_in(outer, w, h)`**, public and additive. The game's
  copies can use it.
- **A stateless `window_for(heights, focus, room, scroll)`** pulled out
  of `ListCursor::window_by_heights`, which keeps its behaviour. Options
  and key_bindings use it (key_bindings with its group gaps as heights).
  So does one scroll-marks painter.
- **Behaviour:** none. Every glyph snapshot stays byte-equal. A site
  whose cells would change keeps its own code.
- **Either order with 2338, 2371 and 2409:** if they are done, move
  whatever they added at these sites onto the shared helpers. If not,
  nothing to do (they call the helpers, as their own clauses say).

**Out (do not do):**
- Moving profiles' `paint_actions` onto `menu::glyph::paint_with`: its
  padding differs, so its snapshot would change.
- The dialogue preview's upward fill. Its rule differs, so it keeps
  `first_shown`.

**Estimated size:** about 450 lines changed (about 280 removed, 170
added).

## Implementation steps

1. Add `Rect::centred_in` and `bar_label` with unit tests.
2. Replace the box, bar and centring sites, one screen at a time, with
   the snapshots green.
3. Extract `window_for`, then move options and key_bindings onto it.

## Acceptance criteria

- [ ] Every glyph snapshot in `crates/ui` is unchanged (`git diff
      --exit-code` on the snapshot files after the tests).
- [ ] `grep -rn "draw_box" crates/ui/src --include=*.rs` finds no call
      outside `frame/`, apart from sites the completion notes list with
      their reason.
- [ ] `window_by_heights`' unit tests pass unchanged.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `centred_in` (odd and even sizes, larger than the outer),
  `bar_label`, and `window_for` against `window_by_heights` on the same
  cases.
- The snapshots are the regression suite.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
