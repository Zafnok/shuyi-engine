---
id: "2389"
title: "The inks removed: one-bit screens draw only by roles' patterns"
type: infra
milestone: After MVP
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["2388"]
nick_input: none
completed:
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

2388 gives roles what only the inks could do (a `Dots` pattern, glyphs
edged in white, rectangles that invert) and proves a pattern map draws
every ink-coloured frame exactly as the inks do. Its PR writes the
game's ticket that moves the Playdate build onto a pattern map, and adds
it to this ticket's `blocked_by`. Once both are done, nothing uses the
inks.

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
- **Tests:** every test that drew with `InkShades` draws with a
  `PatternMap` of 2388's proof (move that map into a test helper both
  use), and the fast-equals-reference proptests run with pattern maps
  only.
- **Docs** through the `document` skill: `docs/carve.md` (one-bit inks
  no longer in the engine), `docs/platform-backends.md` if it names an
  ink, and ADR-0026 amended (the `write-adr` skill): the inks are gone.
- **For the game:** an API break, which CI's `api` job reports, with
  `Breaking:` lines (ADR-0038 §6): the `ink` module and the ink-only
  renderer functions are gone, and `render`, `render_marked` and
  `render_changed` take a `Shades`. The game's ticket from 2388 has
  already moved its Playdate build onto a pattern map; its PR that moves
  the pin past this one deletes its own `content::ink` re-export.

**Out (do not do):**
- Anything the game's ticket does (its palette, its pattern map, its
  Playdate crate).
- One-bit frames of another size (ADR-0026's `OneBitFrame` note).
- The renderer's speed: the `_with` forms are the ones the speed tests
  already time.

## Implementation steps

1. Check `blocked_by`: 2388 done, and the game's ticket it added done
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

- [ ] `grep -rn "Ink\b\|InkShades\|ink_of\|of_ink" crates` finds
      nothing but unrelated words (the toy palette's colour `ink` is a
      name, not an ink).
- [ ] One `render`, `render_marked` and `render_changed`, each taking a
      `Shades`; the fast-equals-reference proptests pass.
- [ ] A grid map's flash draws as an inverted rectangle on a one-bit
      frame through a pattern map (a test over the toy's grid).
- [ ] The completion notes carry the `Breaking:` lines.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the flash rectangle in `MapColors`' flash colour; the toy's
  pattern map has the flash role with `invert`.
- Property: fast equals reference with random pattern maps, as today
  with inks and maps.
- Snapshot: the grid's glyph snapshots stay as they are (the flash's
  colour changes only on a one-bit look).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
