---
id: "3269"
title: "A colour picker widget: a swatch grid of the palette's colours, and a stepped hue, saturation and value picker with a hex field, in integers"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3230", "3232"]
nick_input: none
completed:
---

# 3269 — A colour picker widget

## Context

The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found that creative and customisation games need to pick a
colour: character creators, paint and pixel-art games, team colours,
and level editors inside a game. Bevy's feathers have swatches, a
swatch grid, colour sliders, a colour plane and a hex input
(`crates/bevy_feathers/src/controls/color_swatch.rs`,
`color_swatch_grid.rs`, `color_slider.rs`, `color_plane.rs`,
`color_input.rs`). No Shuyi ticket plans one. 3230 gives every widget
one contract, 3232 the text-editing core a hex field needs, and 2287
an integer HSV conversion.

## Nick input

None.

## Scope

**In:**
- `shuyi_ui::widgets::colour`:
  - `SwatchGrid`: the game's palette roles or a list of `Rgb`s, the
    value an index; works on the glyph and one-bit looks (2371), where
    each swatch shows its role;
  - `HsvPicker`: hue, saturation and value as `u8` steps moved by
    actions and dragged by the pointer, converted to `Rgb` in integers,
    with a preview swatch;
  - a hex field (`#rrggbb`) on 3232's core, which refuses other text.
- The value is plain data in the game's state, changed through the
  screen's commands.
- Each implements 3230's contract.
- The `.ui.ron` nodes. **Either order with 2330:** if 2330 is done, add
  `swatches` and `colour` nodes and their schema; if not, nothing to do
  (2330 lists the widgets that exist when it lands).
- **Either order with 2287:** if 2287 is done, use its HSV conversion;
  if not, write the conversion here, and 2287 reuses it.

**Out (do not do):**
- a colour wheel and a 2D colour plane (sliders do the job in integers);
- alpha;
- colour spaces other than sRGB (3227 decides the frame's).

## Implementation steps

1. Read 3230's contract, 3232's core, `frame/color.rs` and 2287.
2. Write `SwatchGrid` with its cursor and look.
3. Write `HsvPicker` with the integer conversion.
4. Write the hex field.
5. Add the `.ui.ron` nodes as the clause says.
6. Write the docs page: a team-colour picker.

## Acceptance criteria

- [ ] HSV to RGB and back round-trips every `Rgb` within one step.
- [ ] Typing `#ff8800` sets the value; `#ff88` is refused and says why.
- [ ] Every swatch is reachable by actions alone.
- [ ] The one-bit look shows a role name per swatch (golden).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the conversion, the hex parser, cursor moves.
- Property: the round trip over random colours.
- Snapshot / integration: glyph and one-bit goldens; a Harness run.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
