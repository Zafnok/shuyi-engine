---
id: "3217"
title: "The cursor's four direction keys as one picture: a direction cluster span, drawn as the theme's arrow cluster with key caps on"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3215"]
nick_input: none
completed:
---

# 3217 — The cursor's four direction keys as one picture

## Context

**From 3215 (done):** key caps (ADR-0124): with a game's
`KeyCapStyle` on, every `TextRole::Key` span of a `StyledText` is drawn
as a key cap, in the room its box needs. 3215's ticket also asked for
the cursor's four direction keys as one *direction cluster*, and split
it off here by its own size valve ("past 900 lines, split off the
direction cluster").

The game decided the look in game#0250 (*Key names in hints* in the
game's `docs/design/look-and-feel.md`): the four arrow keys, when they
move the cursor, as one small picture of the arrow cluster (24 × 16 px,
four 8 × 8 caps in an upside-down T, which the game draws); any other
four keys (`W A S D`) as four caps. game#0254 turns it on.

What the engine has (after 3215): `Keymap::cursor_keys_name`
(`crates/ui/src/input/keymap.rs`) gives the language's arrows word when
the four cursor actions are on the arrow keys, else the four chords run
together (`wasd`); `help_keys::cursor_keys_name`
(`crates/ui/src/input/help_keys.rs`) returns it as one `String`, and
`text::key_text` fills `{Cursor}` with it, which `fill_styled` puts in
one `Key` span. `KeyCaps` (`crates/ui/src/frame/key_cap.rs`) and
`GlyphBuffer::print_styled` draw a cap; `Sprite` items draw a part of
an image.

## Nick input

None. The look is the game's decision (game#0250).

## Scope

**In:**
- A way to say "the four direction keys, as the arrow keys": a new
  `TextRole` value or a flag on a `Key` span (`TextRole` is
  `#[non_exhaustive]`), the implementer's choice, recorded in ADR-0124's
  successor or a note on it.
- `help_keys::cursor_keys_name`'s styled form: the cluster span when the
  four cursor actions are on the arrow keys, else four `Key` spans in
  up, left, down, right order with one space between them; `{Cursor}`
  in `fill_styled` gives the same. The `String` form stays its
  `plain()`.
- `KeyCapStyle` names the cluster's picture (an image and its part, one
  line tall); with caps on, a cluster is drawn as that picture (a
  `Sprite` item) and takes its cells (`CellWidths`, `StyledText::width`
  and `cut` count them); with caps off, or no picture, as today's
  arrows word.
- Public types follow ADR-0039.

**Out (do not do):**
- Any game's picture: the game draws its own (game#0254).
- A controller's direction pad (game#0255).
- The bold face (3216).

**Estimated size:** about 550 lines (code 250, tests 250, docs 50).

## Implementation steps

1. The cluster role or flag in `crates/ui/src/text.rs`; the styled
   `cursor_keys_name` in `help_keys.rs`; `key_text` and `fill_styled`.
2. The picture in `KeyCapStyle` (`crates/ui/src/frame/key_cap.rs`) and
   its cells in `CellWidths` (beside `key_cap_cells`).
3. `print_styled`: a cluster as a `Sprite` in its cells, or the word.
4. Tests; ADR note; docs.

## Acceptance criteria

- [ ] The cursor on the arrow keys gives one cluster span, drawn as the
      theme's picture with caps on, and as the arrows word with them
      off; on W A S D, four `Key` spans in up, left, down, right order
      with spaces between.
- [ ] `plain()` of the styled form is the `String` form's text.
- [ ] `StyledText::width` counts a cluster's cells, and `print_styled`
      advances by the same.
- [ ] With the style off, every engine snapshot is unchanged.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the spans for arrow keys and for other keys; widths.
- Snapshot: a help line with a cluster, caps on and off.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
