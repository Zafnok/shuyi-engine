# ADR-0089: One-bit roles draw everything the inks do: a `Dots` pattern, and a role's `edge` and `invert`

- **Status:** Accepted
- **Date:** 2026-11-06
- **Related tickets:** 2388 (this), 2389 (removing the inks)

## Context

[ADR-0026](0026-one-bit-by-role-a-pattern-map-beside-the-palette-colours-find-their-roles-and-the-renderer-takes-its-shades.md)
gave a one-bit screen two ways to say what a colour is drawn in, both
behind the renderer's `Shades` trait: the game's eight inks (`InkShades`,
found by reserved RGB codes, game ADR-0074) and the engine's roles (a
pattern map beside the palette, `PatternMap`). It kept the inks so the
game's Playdate build wouldn't change, and said the game can move to
roles one screen at a time. It couldn't: roles lacked three things the
inks do.

| Ink | As a role before | As a role now |
| --- | ---------------- | ------------- |
| `White`, `Black` | `White`, `Black` | the same |
| `Grey` (black where `x + y` is even) | `Dither50` | the same |
| `Diagonals` (black where `x − y` is a multiple of 4) | `DiagonalDown` | the same |
| `Dashes` (2 × 2 squares) | `Checker2` | the same |
| `Dots` (one pixel in eight, rows staggered) | none | `Dots` |
| `BlackEdged` (a glyph gets a white edge one pixel wide) | none | `(fg: Black, edge: true)` |
| `Invert` (a rectangle flips what is under it) | none | `(fg: Black, invert: true)` |

The renderer could already draw all three: a `Shade` has `edged` and
`flips`, and only `InkShades` set them.

## Decision

1. **`Base::Dots`** joins the patterns of ADR-0026 §1: black where `y` is
   even and `x % 4 == y % 4`, the `Dots` ink's pixels exactly (8 black
   in 64). A map spells it `Dots` or `Inverted(Dots)`. It looks like no
   other pattern, so the `distinct` rule needs nothing new.
2. **A role says what its glyphs and rectangles do besides**
   (`RolePatterns`, ADR-0026 §2): `edge: bool` (a glyph in the role gets
   a white edge one pixel wide, `Shade::edged`) and `invert: bool` (a
   rectangle in the role flips the pixels under it, after everything
   else, `Shade::flips`). Both are `false` when the map doesn't say, so
   every map written before reads the same. A map spells them
   `"cursor": (fg: Black, invert: true)`. `RolePatterns` becomes
   `#[non_exhaustive]` (ADR-0039), built by `RolePatterns::new(fg, bg)`
   or `RolePatterns::of(fg)` (no `bg`), and changed by `with_edge` and
   `with_invert`.
3. **`PatternMap` sets them where the renderer reads them** (ADR-0026
   §3–4): `glyph` is the role's `fg` with `edged` from `edge`, `shape`
   its `fg` with `flips` from `invert`, `ground` neither. The renderer
   reads `edged` only off a glyph's shade and `flips` only off a
   rectangle's, so nothing else needs them.
4. **The check** (ADR-0026 §6) gains one error: a text role with
   `invert` (`Rule::Text`, "its glyphs would flip"). Two roles of one
   colour must also agree on `edge` and `invert`, as on their patterns
   (`Rule::SharedColour`): the frame can't tell them apart.
5. **The proof.** A test draws random buffers in the inks' colours with
   `render` (the inks) and with `render_with` a pattern map of the table
   above over a palette of the inks' codes, and finds them equal, byte
   for byte. Colours of no ink or role differ, as ADR-0026 §3 says
   (roles dither them by luminance, the inks draw them solid), so the
   proof holds for the ink colours only.

## Consequences

- A game can draw with roles everything it drew with inks, and the inks
  (`InkShades`, `ink_of`, `Ink`) can go once the game has moved (2389).
- A skin of the engine's own screens can use `edge` and `invert` for
  text over a pattern and a cursor.
- The public API breaks: `Base` gains a variant and `RolePatterns` gains
  fields and `#[non_exhaustive]`. The game matches no `Base`, but its
  `check --target` tool builds `RolePatterns { fg, bg: None }`, which
  becomes `RolePatterns::of(fg)`.
- A pattern map's binary form changes, so the pack's format is
  version 10 (`pack::VERSION`), and its golden state hash moves.

## Alternatives considered

- **`edge` and `invert` as patterns** (`Edged(Black)`, `Invert`) — they
  aren't pixels but what a glyph or a rectangle does, and a pattern is
  also a role's background, where neither means anything.
- **Setting them on the ground too, as `InkShades` does** — the renderer
  never reads them there, and the proof passes without.
- **A rule of its own for an inverting text role** — `Rule::Text` is
  already "what a text role must be"; a new variant would only break
  the API again.
