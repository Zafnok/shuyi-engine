# ADR-0026: One bit by role: a pattern map beside the palette, colours find their roles, and the renderer takes its shades

- **Status:** Accepted
- **Date:** 2026-10-20
- **Related tickets:** 3006 (this), 3007 (one-bit sprites), 3010 (the Playdate backend), 2324 (colour vision), game 1025

## Context

[ADR-0008](0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§4 decides that a one-bit target maps each palette role to an 8 × 8
pattern, that groups of roles that tell things apart must not collapse
into one pattern, and that a colour of no role is Bayer-dithered by its
luminance. Ticket 3006 builds it. What the engine already had:

- **Palettes are named colours** (`shuyi_assets::palette::PaletteDef`,
  `name → #rrggbb`). The names are the roles; nothing needed adding.
- **The frame holds colours, not roles.** A `GlyphBuffer` cell has an
  `fg` and a `bg` `Rgb`, an item a `color`. Skins look a role up in the
  palette and write its colour.
- **A fast one-bit renderer** came with the carve
  (`shuyi_ui::frame::onebit`, game ADR-0074 and game ticket 1055): it draws
  a `GlyphBuffer` into the Playdate's 400 × 240 frame buffer (rows of 52
  bytes, so 416 bits, a whole number of 32-bit words), with a plain
  `reference` renderer the tests prove it equal to. It maps colours to
  the game's eight `Ink`s by reserved RGB codes, and two inks do more
  than paint: `BlackEdged` puts a white edge round a glyph, `Invert`
  flips what a rectangle covers. The game draws its Playdate build with
  it, so it must keep working as it is.

## Decision

1. **Patterns** live in `shuyi_assets::onebit` (the format reads them, and
   `shuyi-assets` can't see `shuyi-ui`), re-exported from
   `shuyi_ui::frame::onebit`. A `Pattern` is a `Base` (`Black`, `White`,
   `Dither25`/`50`/`75`, `DiagonalUp`, `DiagonalDown`, `HLines`,
   `VLines`, `Checker2`) and whether it is inverted; a map spells it
   `Dither50` or `Inverted(Dither50)`. Its 8 rows are a byte each, the
   highest bit the left pixel, a set bit white, as in the frame. A dither
   of `N`% has `N`% black pixels, black where the Bayer 8 × 8 order is
   below `N × 64 / 100`; the lines are one pixel in four, `Checker2` half.
   `onebit::dither(darkness)` is the same order for any grey, so 64, 128
   and 192 are exactly the three named dithers.
2. **The pattern map** (`PatternMapDef`, kind `pattern map`, format
   `PatternMapRon`, id `pattern-map`, `.ron`, standard) sits beside the
   palette as `palette@onebit.ron` (`onebit::map_path`). It holds
   `roles: { name: (fg, bg) }`, `distinct: [[role, …], …]` and
   `text: [role, …]`. `fg` is what is drawn in the role's colour: a
   glyph, a rectangle, a sprite painted solid. `bg` is a cell's
   background in it, and is `fg` when not given. RON's `implicit_some` is
   on, so `bg: White` needs no `Some(…)`.
3. **Colours find their roles.** `shuyi_ui::frame::onebit::PatternMap::new`
   takes the palette's `(name, Rgb)` pairs and the map, and keys each
   role's patterns by its colour. A colour of no role is dithered by its
   luminance `(77 r + 150 g + 29 b) >> 8`; a glyph in such a colour is
   solid white or black (luminance ≥ 128), as text is never dithered.
   When roles share a colour, the first the palette gives with patterns
   wins, and the check (below) makes that an error unless their patterns
   are the same.
4. **The renderer takes its shades.** The fast renderer and the reference
   now draw `Shade`s (8 rows, plus `edged` and `flips`) from a `Shades`
   trait with three questions: a cell's `ground`, a `glyph`, a `shape`.
   `InkShades` is the game's inks, exactly as before, and `render`,
   `render_marked` and `render_changed` keep their signatures and
   results. `render_with`, `render_marked_with` and `render_changed_with`
   take any `Shades`; `PatternMap` is one. Patterns stay fixed to the
   screen's pixels, so they don't swim when things move. The
   fast-equals-reference proptests run with both.
5. **`to_one_bit(frame, map, font, images) -> OneBitFrame`** draws a frame
   with a `PatternMap` into a boxed 400 × 240 frame buffer, always the
   display's size. It takes the one-bit font and the sprites' one-bit
   pictures, which the ticket's sketch left out: glyphs and sprites
   can't be drawn without them. `OneBitFrame::changed_rows(&previous)`
   and `onebit::changed_rows(old, new)` give a `RowMask` of the rows that
   differ, as the Playdate sends only those to its display.
6. **The check** is `shuyi_assets::onebit::check(palette, map)`. Errors: a
   palette role with no patterns; a role, group member or text role that
   the palette lacks; two roles of one colour whose patterns differ; two
   roles of a `distinct` group whose foregrounds **look alike**. Two
   patterns look alike when one is the other moved, so `Dither50` and
   `Inverted(Dither50)` collide, as do `Dither25` and
   `Inverted(Dither75)` (Bayer's order is the same upside down, moved).
   Warning: a text role that isn't `Black` on `White` or the inverse.
   2324 reuses the groups.
7. **The tools.** `GameTools` gains two optional methods:
   `one_bit(target) -> Option<Result<OneBitSkin, String>>` (palette and
   map, with their files) and `frame_one_bit(target, options, script,
   scale)`. `check --target` runs the check on every target whose
   profile's `display.color` is `OneBit`, as `TargetCheck::OneBit`
   problems with their `roles`, and says in its limits when the game's
   tools don't offer a skin. `frame-png --target <id>` looks the profile
   up (the engine's and the game's) and, on a one-bit one, asks the game
   for `frame_one_bit`; other targets get the game's usual frame. The
   contract's version is 5.

## Consequences

- A skin gets a one-bit look by writing one file beside its palette, and
  `check --target playdate` says when the look loses meaning, naming the
  roles.
- The game's Playdate build is unchanged: inks still draw as they did,
  through the same functions. It can move to roles (game 1025) one
  screen at a time, since both are `Shades`.
- Roles sharing a colour must share patterns. A skin that wants two roles
  apart in one bit needs them apart in colour too, which is what
  colour-blind players need anyway (2324).
- Glyphs in a role's foreground are drawn in its pattern, so a dithered
  role's glyph is dotted; only text roles are held to solid black and
  white.
- `OneBitFrame` is the Playdate's 400 × 240 alone. A one-bit target of
  another size would make it generic, and the fast renderer's constants
  with it.
- `GameTools` has two more methods, both with defaults, so a game that
  doesn't offer them still builds.

## Alternatives considered

- **Roles in the frame** (each cell and item carrying a role id) — exact,
  but it changes every skin and widget and the frame's shape for every
  target, to serve one. Colours are already unique per role in practice,
  and the shared-colour check catches the rest.
- **A second one-bit renderer** for roles — it would duplicate the fast
  renderer and its reference, and the two would drift; a `Shades` trait
  over the one renderer keeps one set of proofs.
- **`Pattern` as an enum with `Inverted(Box<Pattern>)`** — the RON
  spelling the ticket asks for, but not `Copy` and nests without end. A
  base and a flag read and write the same text.
- **Distinct means different bits** — `Dither50` and its inverse differ
  in every pixel yet look the same on a screen; comparing up to a move
  is what a player sees.
- **Comparing backgrounds in `distinct` groups too** — the ticket and
  ADR-0008 compare foregrounds, what is drawn in the role's colour; a
  role used as a background sets only `fg`, which is then its
  background as well.
