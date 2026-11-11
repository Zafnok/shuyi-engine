# ADR-0107: The inks removed: one-bit screens draw only by roles' patterns

- **Status:** Accepted
- **Date:** 2026-11-11
- **Related tickets:** 2389 (this), 2388, game 1253

## Context

[ADR-0026](0026-one-bit-by-role-a-pattern-map-beside-the-palette-colours-find-their-roles-and-the-renderer-takes-its-shades.md)
§4 kept the game's eight inks (`shuyi_assets::ink::Ink`, `InkShades`,
game ADR-0074) beside roles' patterns so the game's Playdate build
wouldn't change: `render`, `render_marked` and `render_changed` drew in
the inks, and `render_with`, `render_marked_with` and
`render_changed_with` took any `Shades`.
[ADR-0089](0089-one-bit-roles-draw-everything-the-inks-do-a-dots-pattern-and-a-roles-edge-and-invert.md)
gave roles everything the inks did, and game ticket 1253 moved the
game's Playdate build onto a pattern map. Nothing used the inks after
that, and keeping them cost:

- two forms of every one-bit renderer function, in `reference` too;
- two rules for "is this colour light?" that disagreed (`ink_of`'s
  `299/587/114` against half of 1000, roles' `luminance`'s `77/150/29`
  against 128);
- a game's palette in the engine: the inks were named for the game's
  uses, which `AGENTS.md` rule 4 keeps out.

## Decision

1. **The inks are gone.** `shuyi_assets::ink` (`Ink`,
   `PaletteDef::from_ink_source`) and, in `shuyi_ui::frame::onebit`,
   `InkShades`, `ink_of`, `ink_is_white` and `Shade::of_ink` are
   removed. A colour is drawn as a `Shades` says; the engine's is
   `PatternMap` (ADR-0026 §3).
2. **One form of each renderer function**, taking a `Shades`:
   `render(buf, font, images, shades, out)`,
   `render_marked(buf, font, images, shades, out, mark)` and
   `render_changed(old, new, font, images, shades, out)`; the same in
   `onebit::reference`. This replaces ADR-0026 §4's two forms.
3. **The renderer's own grey** (a missing picture's rectangle, and the
   pixels a dimmed picture turns white) is `Base::Dither50`'s pixels,
   which were the grey ink's.
4. **A grid's inverted flash** (`FlashLook::Invert`) covers its tile with
   a rectangle in the colour `MapColors` names for a flash
   (`MapColors::with_flash`, the cursor's when none is named). A one-bit
   skin gives that colour a role with `invert: true`, so the rectangle
   flips the pixels under it. `GlyphStyle::draw_tiles` takes the
   `MapColors` to find it.
5. **Tests** draw in a test palette of eight swatches, each a role of a
   pattern map made for them (`onebit::test_map`, the map ticket 2388
   proved draws as the inks did). The fast-equals-reference proptests run
   with pattern maps only.

## Consequences

- One rule for a colour of no role: dithered by its `luminance`, a glyph
  in it solid by the same.
- A one-bit screen with no pattern map of its own passes
  `PatternMap::default()`: every colour is of no role. The toy's
  `frame-png --target playdate` does, so its backgrounds are dithered
  where the inks' rule made them white or black.
- An API break for the game: it already draws with a pattern map; its PR
  that moves the pin past this one renames the `_with` calls and deletes
  its own `content::ink` re-export.

## Alternatives considered

- **Keep `render` drawing in a fixed default** (`PatternMap::default()`)
  beside the `Shades` forms: two forms again, and the default is rarely
  what a screen wants. Taking the `Shades` makes each caller say.
- **Keep `Ink` as a test helper in `shuyi-assets`**: still a game's
  palette in the engine's public API; the tests' swatches live with the
  tests instead.
- **Read the flash's colour from the glyph style** (`FlashLook::Invert`
  carrying a name): every other colour a grid skin paints in is a
  `MapColors` name, so the flash's is one too.
