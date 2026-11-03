---
id: "2245"
title: "The one-bit bake takes convert's styles and scale, a brightness lift and a mask rim, from a convert import-settings section"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2245 — The one-bit bake takes styles, scale, a lift and a rim

## Context

A request from the game.

1. **The game ticket:** game#1221, the game's one-bit pictures baked by
   this engine's bake
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1221-the-playdate-pictures-baked-by-the-engines-bake.md`).
   It is not a blocker of this one. The game ticket game#1170 (the Playdate front
   end on the engine's backend) wires its pictures through an adapter
   that 1221 then moves onto this bake.
2. **The job in one sentence:** make a colour picture one-bit at bake
   time enlarged to the size it is shown at, in several named styles,
   optionally lightened first and given a white rim round its figure,
   with every choice written in a `.meta` file beside it or its folder.
3. **The friction numbers:** no friction review: the game/engine
   boundary audit of 2026-11-03 found it. The game converts its pictures
   itself with `bitmap::convert`: a table of kinds, a build-script
   step and a preview tool, about 460 production lines
   (`crates/content/src/playdate_images.rs` 265, `build.rs` about 60,
   `crates/xtask/src/playdate_images.rs` 138),
   because `OneBitSettings` (`bitmap/dither.rs`) has only `dither`,
   `alpha_cut` and `invert`: no scale, no `Method` knobs, no list of
   styles, no lift, no rim. The nearest row, game 1041's (estimate 200,
   measured 317, 1.6×), made the rim and the lift game code. The game's
   pin ledger (row #190) said 3007's bake was "not for the game" for the
   same gap.
4. **The generic shape:** a second import-settings section, `convert`,
   that the one-bit bake reads before `onebit`: `bitmap::convert`'s
   scale and methods as integers (ADR-0057 §9), a style list with names,
   and two picture steps that name no game concept, a brightness lift
   before converting and a mask rim after.
5. **Examples from games that aren't tactics:** a farming game on the
   Playdate whose villager portraits come in two styles the player picks
   (plain and lined), shown at 2×; a platformer whose sprites get a
   one-pixel white rim so they read against a dithered sky, its
   backdrops lightened so they stay behind the player.

## Nick input

None.

## Scope

**In:**
- `crates/assets/src/bitmap/convert.rs`: `pub fn lift(rgba: &[u8],
  scale: u16, floor: u8) -> Vec<u8>`: each colour channel `v` becomes
  `round(scale / 1000 × v + floor)`, clamped to a byte, alpha kept; and
  `pub fn with_rim(file: &[u8], width: u16) -> Result<Vec<u8>, String>`:
  each see-through pixel within a round brush of `width` of a shown one
  (`dx² + dy² ≤ r² + r`) becomes shown and white in every style,
  stopping at the picture's edge. Both as the game's `Lift::apply` and
  `with_rim` compute them today (read them for the exact arithmetic).
- `ConvertSettings`, section `convert` (`#[non_exhaustive]`, every field
  defaulted, integers and strings only): `scale: (u32, u32)` (1, 1);
  `styles: Vec<ConvertStyle>` (empty: the step is off); `lift:
  Option<ConvertLift>` (`scale` in thousandths, `floor` 0–255); `rim:
  u16`. `ConvertStyle`: `name: String`, `curve` (thousandths, 1000),
  `stretch` (thousandths, 30), `levels: u8` (0), `lines:
  Option<ConvertLines>` (`contrast`, `dark` in thousandths). Methods
  `is_on`, `methods() -> Vec<Method>` (thousandths `as f32 / 1000.0`)
  and `make(width, height, rgba) -> Result<Vec<u8>, String>`: lift,
  `convert`, rim.
- `register_standard` registers the section beside `onebit`; `Dithered`
  makes a picture whose `convert` is on with it, else as now, so `shuyi
  bake --target playdate` and `--sheet` take it.
- `import_settings::nearest::<S>(file, lookup) -> Result<S,
  Vec<ContentError>>`: the chain lookup `DecodeCx::import_settings` does,
  public, for a build script or tool that walks files itself;
  `DecodeCx::import_settings` calls it.
- Additive: no `Breaking:` line. A game that lists the registry's
  sections in a test sees `"convert"`. The game's PR that moves the pin
  (game#1221) follows this one.

**Out (do not do):**
- Changing `OneBitSettings`, `Dither` or `convert`'s output for any
  existing `Method`.
- A game's choice of which pictures, sizes and styles (its `.meta`
  data), or a scale read from a game's own sidecar files.
- Faster or integer-only conversion (the game's look is `convert`'s).

## Implementation steps

1. **Read** `bitmap/convert.rs`, `bitmap/dither.rs` (`Dithered`),
   `import_settings.rs`, `format.rs` (`DecodeCx::import_settings`,
   `register_standard`), `pack/one_bit_tests.rs`, `cli` `bake/sheet.rs`,
   ADR-0057 and the game's `crates/content/src/playdate_images.rs`.
2. `lift` and `with_rim`, with tests.
3. `ConvertSettings` and its types, `methods`, `make`; registration.
4. `Dithered` choosing `convert`; `nearest`, with `DecodeCx` on it.
5. The book's assets chapter and `docs/` on import settings: the
   `convert` section, an example `_folder.meta`.

## Acceptance criteria

- [ ] `methods()` of `curve: 800, stretch: 30, lines: Some((contrast:
      300, dark: 150))` equals the `Method` built from the literals 0.8,
      0.03, 0.3 and 0.15 (a test: the f32s are equal, not near).
- [ ] `lift(_, 600, 120)` equals `0.6 × v + (120.0 / 255.0) × 255.0`
      rounded, for every byte `v` (a test).
- [ ] A rim of 2 round a one-pixel figure gives the brush's 21 pixels,
      white in every style; a picture with no mask comes back unchanged.
- [ ] A PNG under an `_folder.meta` with `(convert: (scale: (2, 1),
      styles: [(name: "a"), (name: "b", curve: 800)]))` bakes to a
      two-style file at twice its size, equal to `convert` with those
      methods; `(convert: ())` beside one file turns it off (tests).
- [ ] A golden row for the `convert` fixture (`shuyi_assets::golden`).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `lift`, `with_rim`, `methods`, `nearest`, the section's defaults
  and a typo refused.
- Integration: a bake of a fixture folder with `convert` settings,
  determinism golden.

Expected size: about 250 production lines (about 65 of them the game's
`Lift` and `with_rim`, moved), about 250 of tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
