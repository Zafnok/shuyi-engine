---
id: "2389"
title: "The inks removed: one-bit screens draw only by roles' patterns"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2388", "game#1253"]
nick_input: none
completed: 2026-11-11
---

# 2389 — The inks removed: one-bit screens draw only by roles' patterns

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-27). A one-bit screen has two ways to say what a colour is
drawn in (2388's Context has the details):

- **the game's eight inks** (`shuyi_assets::ink::Ink`, `InkShades`,
  game ADR-0074), found by reserved RGB codes in the palette, which came
  with the carve;
- **roles' patterns** (ADR-0026, `shuyi_assets::onebit`, `PatternMap`),
  the engine's generic way.

ADR-0026 kept the inks only so Visions of Shuyi's Playdate build
wouldn't change. Keeping both costs:

- two answers to one question, through every one-bit renderer
  (`render` against `render_with`, and the same in `reference`);
- two rules for "is this colour light?" that disagree: `ink_of` weighs
  red, green and blue `299/587/114` against half of 1000, roles'
  `luminance` weighs them `77/150/29` against 128. For 16,893 of the
  16.7 million colours they give opposite answers (`(0, 169, 253)` is
  white by the inks, black by roles);
- a game's palette in the engine: the inks are named for the game's
  uses (the move range's dots, the attack range's diagonals, the
  danger zone's dashes), which `AGENTS.md` rule 4 keeps out.

**From 2388 (done):** roles have what only the inks could do (a `Dots`
pattern, a role's `edge` and `invert`, ADR-0089), and a pattern map
draws every ink-coloured frame exactly as the inks do (the proof,
`INKS_AS_ROLES` in `crates/ui/src/frame/onebit/tests.rs`; the `dashes`
ink is `Checker2`, not `Inverted(Checker2)`). The game moves its
Playdate build onto a pattern map in game#1253. Once both are done,
nothing uses the inks.

## Nick input

None.

## Scope

**In:**
- **Removed from `shuyi-assets`:** the `ink` module (`Ink`,
  `PaletteDef::from_ink_source`), and its line in
  `crates/assets/src/lib.rs`'s docs.
- **Removed from `shuyi_ui::frame::onebit`:** `InkShades`, `ink_of`,
  `ink_is_white`, `Shade::of_ink`, and the ink-only `render`,
  `render_marked` and `render_changed`. The `_with` forms take their
  names: `render(buf, font, images, shades, out)`,
  `render_marked(…, shades, …)`, `render_changed(…, shades, …)`. The
  same in `onebit::reference`, whose debug mark of the shown region
  (`Shade::of_ink(Ink::Grey)`) becomes `Pattern::new(Base::Dither50)`'s
  shade.
- **`Shade`'s docs** say what `edged` and `flips` do without naming an
  ink (a role's `edge` and `invert`, 2388).
- **The grid's glyph skin** (`crates/ui/src/grid/glyph/style.rs`):
  `FlashLook::Invert` covers a flashing tile with a rectangle in the
  colour `MapColors` names for a flash, a role whose pattern map says
  `invert: true`, instead of `Ink::Invert.code()`. Add
  `const fn MapColors::with_flash(self, name)` (`MapColors` is
  `#[non_exhaustive]` and built in `const`s); without it the flash uses
  the cursor's colour. Only the toy's grid (`crates/ui/src/toy/grid.rs`)
  and the style's tests use `FlashLook::Invert` today; the toy names a
  flash colour.
- **Other one-bit code it moves**, from four tickets that draw on the
  one-bit renderer. Each draws through `Shades::shape` and reads
  `flips` off it, never `ink_of` or an `Ink`, so only their tests can
  name an ink:
  - **Either order with 2336** (drawn shapes): if 2336 is done, move
    its one-bit shape tests onto the pattern map helper, and check its
    runs fill through `Shades::shape`. If not, nothing to do (2336
    draws through `Shades` and tests with a pattern map, its clause
    says so).
  - **Either order with 2337** (tint and blend modes): if 2337 is
    done, move its one-bit tests onto the pattern map helper; its
    flipping rectangle is a role with `invert: true`. If not, nothing
    to do (2337's clause says so).
  - **Either order with 2339** (the veil): if 2339 is done, move its
    one-bit veil tests and their text snapshot onto the pattern map
    helper (the snapshot changes only if a veil colour of no role now
    dithers by luminance; review it). If not, nothing to do (2339's
    clause says so).
  - **Either order with 2340** (shake and flash): if 2340 is done, its
    one-bit flash tests that flash in `Ink::Invert`'s code flash in the
    helper map's `invert` role instead, and its veil still flips by
    `Shades::shape`'s `flips`. If not, nothing to do (2340 flashes in
    a role with `invert`, its clause says so).
- **Tests:** every test that drew with `InkShades` draws with a
  `PatternMap` of 2388's proof (`INKS_AS_ROLES` and `inks_as_roles` in
  `onebit/tests.rs`; move them into a test helper both use), and the fast-equals-reference proptests run with pattern maps
  only.
- **Docs** through the `document` skill: `docs/carve.md` (one-bit inks
  no longer in the engine), `docs/platform-backends.md` if it names an
  ink, and ADR-0026 amended (the `write-adr` skill): the inks are gone.
- **For the game:** an API break, which CI's `api` job reports, with
  `Breaking:` lines (ADR-0038 §6): the `ink` module and the ink-only
  renderer functions are gone, and `render`, `render_marked` and
  `render_changed` take a `Shades`. The game's ticket game#1253 has
  already moved its Playdate build onto a pattern map; its PR that moves
  the pin past this one deletes its own `content::ink` re-export.

**Out (do not do):**
- Anything the game's ticket does (its palette, its pattern map, its
  Playdate crate).
- One-bit frames of another size (ADR-0026's `OneBitFrame` note).
- The renderer's speed: the `_with` forms are the ones the speed tests
  already time.

## Implementation steps

1. Check `blocked_by`: 2388 done, and game#1253 done
   (the game draws its Playdate build with a pattern map).
2. Delete `crates/assets/src/ink.rs` and its module line.
3. In `crates/ui/src/frame/onebit.rs` and `onebit/reference.rs`: delete
   the ink items, rename the `_with` forms, fix the docs.
4. Give `MapColors` a flash colour and move `FlashLook::Invert` onto it;
   name one in the toy. The toy has no pattern map today: its test
   builds one in code (`PatternMapDef`) with an `invert: true` role for
   that colour.
5. Move the tests onto pattern maps.
6. Docs and the ADR-0026 amendment.
7. Run the gates.

## Acceptance criteria

- [x] `grep -rn "Ink\b\|InkShades\|ink_of\|of_ink" crates` finds
      nothing but unrelated words (the toy palette's colour `ink` is a
      name, not an ink).
- [x] One `render`, `render_marked` and `render_changed`, each taking a
      `Shades`; the fast-equals-reference proptests pass.
- [x] A grid map's flash draws as an inverted rectangle on a one-bit
      frame through a pattern map (a test over the toy's grid).
- [x] The completion notes carry the `Breaking:` lines.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the flash rectangle in `MapColors`' flash colour; the toy's
  pattern map has the flash role with `invert`.
- Property: fast equals reference with random pattern maps, as today
  with inks and maps.
- Snapshot: the grid's glyph snapshots stay as they are (the flash's
  colour changes only on a one-bit look).

## Completion notes

Done as scoped, decided in
[ADR-0107](../../docs/adr/0107-the-inks-removed-one-bit-screens-draw-only-by-roles.md),
which amends ADR-0026 §4.

- `shuyi_assets::ink` is gone, and so are `InkShades`, `ink_of`,
  `ink_is_white` and `Shade::of_ink`. `render`, `render_marked` and
  `render_changed` take a `Shades` (the `_with` forms' bodies), in
  `onebit::reference` too. The renderer's own grey (a missing picture,
  a dimmed one) is a private `GREY` shade, `Base::Dither50`'s pixels; the
  reference uses `Shade::from(Pattern::new(Base::Dither50))`, and a test
  holds the two equal.
- `MapColors` has a `flash: Option<&str>` and `const fn with_flash`;
  `Looks::flash` gives its colour, the cursor's when none is named.
  `FlashLook::Invert` covers the tile in it, so `GlyphStyle::draw_tiles`
  takes the `MapColors` (a break, below). The toy names `flash`
  (`[255, 0, 255]`, a new entry of `MAP_PALETTE`).
- Tests: the inks became the tests' eight swatches, each a role of the
  map 2388 proved (`crates/ui/src/frame/onebit/test_map.rs`: `Swatch`,
  `c`, `TEST_ROLES`, `test_map()`), which `tests.rs`, `speed_tests.rs`
  (and its timing) and `painted_tests.rs` all use. The proof that the
  map draws as the inks did had nothing left to compare and is replaced
  by `each_swatch_is_its_role`. The fast-equals-reference proptests run
  with `test_map()` and with the varied `roles()` map.
- New grid tests: the toy's flash colour is a role that flips in a
  pattern map made in code, and a flashing tile of the one-bit skin
  draws through it as exactly its rectangle's pixels flipped.

Deviations:

- **Snapshots.** The grid's one-bit example snapshot names the flash's
  colour (`flash`) where it showed `#ff00ff`: the colour is now in the
  toy's palette. The toy's `frame-png --target playdate` picture
  (`it__toy__plasma_playdate.snap.png`) changed: the toy has no pattern
  map, so it draws with `PatternMap::default()`, and its colours of no
  role dither by luminance where the inks' rule made each white or
  black (its dark background is now a dither). Reviewed; the plain
  glyph snapshots are unchanged.
- **The free `glyph::draw_tiles`** (the plain style, which blends its
  flashes) keeps its signature and passes empty `MapColors`, which it
  never reads.

No follow-up tickets. The clauses of 2336, 2337, 2339 and 2340 that
waited on this one now say what holds (the test helper's names), and
2565's **Out** line about `ink_of` is gone.

The game's ticket game#1253 already draws its Playdate build with a
pattern map; its PR that moves the pin past this change renames the
`_with` calls and deletes its own `content::ink` re-export (no such PR
yet).

Breaking: shuyi-assets: ink: the module, `Ink` and `PaletteDef::from_ink_source` are gone; give a one-bit target a pattern map beside its palette (ADR-0026) instead.

```migration
since: 0.2.0
what: The one-bit inks are gone; a one-bit palette is an ordinary palette with a pattern map beside it.
old: shuyi_assets::ink::Ink, PaletteDef::from_ink_source(file, source, base)
new: PaletteDef::from_source and a PatternMapDef in palette@onebit.ron
before: |
  use shuyi_assets::ink::Ink;
  let pal = PaletteDef::from_ink_source("p.ron", src, &base)?;
after: |
  let pal = PaletteDef::from_source("p.ron", src, &required)?;
  let map = PatternMapDef::from_source("p@onebit.ron", map_src)?;
find: ["from_ink_source", "shuyi_assets::ink"]
```

Breaking: shuyi-ui: frame::onebit::render, render_marked, render_changed (and reference's): each takes a `Shades` before `out`; the `_with` forms, `InkShades`, `ink_of`, `ink_is_white` and `Shade::of_ink` are gone.

```migration
since: 0.2.0
what: The one-bit renderer's functions take the Shades to draw with; the _with forms took their names.
old: onebit::render_with(buf, font, images, shades, out)
new: onebit::render(buf, font, images, shades, out)
before: |
  onebit::render_with(buf, font, &images, &map, &mut frame);
  onebit::render_changed_with(old, buf, font, &images, &map, &mut frame);
after: |
  onebit::render(buf, font, &images, &map, &mut frame);
  onebit::render_changed(old, buf, font, &images, &map, &mut frame);
find: ["render_with(", "render_changed_with("]
```

Breaking: shuyi-ui: grid::GlyphStyle::draw_tiles: takes the skin's `MapColors` after the `Looks`, for an inverted flash's colour.

```migration
since: 0.2.0
what: GlyphStyle::draw_tiles takes the MapColors that name the flash's colour.
old: GlyphStyle::draw_tiles(&self, looks, buf, scene, layout)
new: GlyphStyle::draw_tiles(&self, looks, colors, buf, scene, layout)
before: |
  style.draw_tiles(&looks, &mut buf, &scene, &layout);
after: |
  style.draw_tiles(&looks, &COLORS, &mut buf, &scene, &layout);
```

Book: none, the book teaches neither the one-bit renderer's functions
nor the glyph skin's styles; it reaches one-bit screens only through
`frame-png --target` and the pattern map's file, which keep their shape.
