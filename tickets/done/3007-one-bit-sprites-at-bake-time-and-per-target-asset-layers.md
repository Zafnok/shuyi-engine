---
id: "3007"
title: "1-bit sprites at bake time: Atkinson or Bayer dithering per image, tested equal to the source, and per-target asset layers for hand-drawn overrides"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: done
blocked_by: ["2204", "2229", "3006"]
nick_input: none
completed: 2026-10-30
---

# 3007 — 1-bit sprites at bake time, and per-target asset layers

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§4. Sprites (portraits, map art, effects) are colour PNG files. Dithering
them every frame on a 168 MHz Playdate wastes the frame budget and gives
artists no control. So `shuyi bake --target playdate` dithers them once.
Following ADR-0007 §4, the baked image is derived from its source and
tested equal to it. And where dithering spoils a picture, a hand-drawn
1-bit file in a per-target layer replaces it.

**From 3006 (done):** `to_one_bit` already draws sprites from one-bit
pictures, through `shuyi_ui::frame::onebit::Images`. Its `Image` has
whole-byte rows (`Image::stride`: the width over 8, rounded up), an
optional mask and a source size, and `Image::from_bitmap` reads one
style of a `shuyi_assets::bitmap` file (game ADR-0085: `1BIT`, one or
more styles, a mask, the source size; the format `OneBitInfo`).
`bitmap::convert` already makes that file from a PNG, enlarged to the
size it is shown at, with a 4 × 4 Bayer dither in floats.

## Nick input

None. Which pictures the game redraws by hand is the game's choice
(game 1025).

## Scope

**In:**
- **The image kind gets a 1-bit form: the `bitmap` file.** A baked
  picture is a `shuyi_assets::bitmap` file (its styles, mask and source
  size), so a baked pack feeds `to_one_bit` through `Image::from_bitmap`
  with no copying, and the pack (2204) holds it as a `Format` like any
  other. If the bake needs more than that file holds, extend `bitmap`
  (its `VERSION`) rather than add a second one-bit picture format, and
  the PR says why.
- **Dithering** in integers, as a choice `bitmap::convert` takes beside
  its 4 × 4 Bayer (it already enlarges and cuts the mask), or in
  `shuyi_assets::onebit` with the PR saying why:
  - `atkinson` error diffusion, the default for pictures;
  - `bayer` ordered 8 × 8, for flat art and anything that moves;
  - `threshold(N)` for line art;
  - `none`, which means the image must already be black and white, and
    errors otherwise.
- **Per-image settings** are an `onebit` section of the image's import
  settings (2229): `OneBitSettings { dither, alpha_cut, invert }`
  implements `ImportSettings` and is registered in
  `Formats::register_standard`, so `Formats::check_import_settings`
  checks it with no Playdate bake running. `images/hero.png.meta` holding `(onebit:
  (dither: bayer))` sets one image, and `images/_folder.meta` sets a
  folder. With neither, the defaults apply (`atkinson`, 128, `false`).
  The dithering format reads it with
  `cx.import_settings::<OneBitSettings>(file)`.
- **Per-target asset layers.** A folder `assets@<target>/` is a bundle
  layer above `assets/`, used only when baking or running for that
  target. Any asset may be replaced there: a hand-drawn `hero.png` in
  `assets@playdate/` wins over dithering `assets/hero.png`. A settings
  file is a file like any other:
  `assets@playdate/images/hero.png.meta` replaces the base one (2229).
  At run time the target's id is `PlatformFacts::target`.
  **From 3053 (done):** a running game's load also picks the layer by
  `setup.platform.target`, so a run that isn't from a target's baked
  pack still uses its layer.
- **A language's copies** (2349: `lang/<tag>/files/<path>`) are plain
  paths in the bundle, so a target layer may hold its own
  (`assets@playdate/lang/ja/files/title/end.png`). `Ctx::localized`
  looks for the language's copy first, then the plain path, each
  through the layers in order: a language's copy in `assets/` beats
  the target's copy of the plain path, and `bake --target playdate`
  dithers a language's copy like any other image.
- **`shuyi bake --target playdate`** writes a pack in which every image
  is a one-bit picture (a `bitmap` file), taken from the target layer
  when one exists and dithered otherwise. It prints the images it
  dithered, so that an artist sees what to check.
- **From 2204 (done):** `shuyi bake --target <id>` and
  `shuyi check --baked` exist (contract version 3), and `bake --target`
  refuses a target with no profile. The target reaches the game as
  `GameTools::bake(Some(id))` (`crates/cli/src/game.rs`), and the game
  picks its bundle and its `Bake` plan by it. `Bake::bake` only records
  the target in the pack's info, and no encoder sees it. So the
  `assets@<target>/` layer is an engine helper on `Bundle` that a
  game's `GameTools::bake` uses for its target, and "every image a
  one-bit picture" is that game's plan baking its images
  (`Bake::under`) as `bitmap` files through the dithering format.
  Hello, Grid's `GameTools::bake` (`examples/hello-grid/src/tools.rs`)
  ignores the target today; this ticket's integration test makes it
  use one.
- **Equality** (ADR-0007 §4): decoding a baked one-bit picture equals
  dithering the decoded source with the same settings, on every fixture.
- **A contact sheet.** `shuyi bake --target playdate --sheet out.png`
  draws every dithered image beside its source, for review.

**Out (do not do):**
- run-time dithering of sprites;
- other bit depths (2-bit, 4-bit greys); write a ticket when a target
  needs one;
- audio conversion for the Playdate (3029: `shuyi export playdate`
  bakes sounds and music to ADPCM);
- the settings file itself, its folder defaults and its errors (2229);
- `shuyi check` running `Formats::check_import_settings` on a game's
  bundle (2571);
- a language's copies of files and `Ctx::localized` (2349): nothing
  here builds them, and the order above is stated in both tickets;
- knowing the target at run time (3053's `PlatformFacts`).

## Implementation steps

1. Write the three ditherers with fixture images and their expected
   outputs (rows of a golden table, 2116's `shuyi_assets::golden`).
2. Add `OneBitSettings` as 2229's `onebit` section, registered in
   `Formats::register_standard`, and the dithering format that reads
   it and writes a `bitmap` file.
3. Add the target layers to the bundle, and the helper a game's
   `GameTools::bake` calls to use them (the `--target` flag is 2204's).
   Pick the layer at load by `setup.platform.target` (3053, done).
4. Write the equality test, and add the contact sheet.

## Acceptance criteria

- [x] Each ditherer's output on the fixtures matches its row in a
      golden table (2116's `shuyi_assets::golden`) in `shuyi-assets`,
      checked on x64, ARM64 and wasm32 by CI's `test` and
      `determinism` jobs (`shuyi-assets` is already in the latter).
- [x] A file in `assets@playdate/` replaces the dithered image, and the
      bake report says so.
- [x] `dither: none` on a colour image is an error naming the file.
- [x] The equality test passes on every fixture.
- [x] ~~An unknown field in an `onebit` section is an error with the
      settings file and line from `shuyi check`, with no Playdate bake
      running.~~ (not done: plain `shuyi check` doesn't run
      `Formats::check_import_settings` yet, which is 2571 per **Out**;
      the standard formats' check and `shuyi bake` give the error, file
      and line with no Playdate bake)
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each ditherer on small hand-checked images (a 4 × 4 grey ramp).
- Property: random images never panic, and the output has the source's
  size.
- Integration: bake an example for `playdate`, and load it through the
  headless backend.

## Completion notes

What was done:

- **The ditherers**, in integers, in a new module
  `shuyi_assets::bitmap::dither` (no `std`): `Dither::{Atkinson, Bayer,
  Threshold(u8), None}` and `dither(&Image, OneBitSettings)`, which
  writes a `bitmap` file of the picture's own size (one style, a mask
  where a pixel is under `alpha_cut`). Bayer uses `onebit::BAYER` with
  the pattern map's own rule, so a flat grey is `onebit::dither`'s
  pattern (tested). Atkinson divides rounding toward 0, so the inverted
  picture dithers to the exact inverse (a property test). Hand-checked
  on the 4 × 4 ramp; golden table `crates/assets/tests/golden/dither.ron`
  (4 fixtures × 6 settings); random pictures never panic and keep their
  size.
- **Why a module of `bitmap` and not `bitmap::convert` or `onebit`:**
  `convert` is `std`-only and in floats (its enlarging resample and
  `powf` curve), which a golden table can't hold equal across x64, ARM64
  and wasm32; `onebit` is the pattern map. The `bitmap` file needed
  nothing more, so its `VERSION` stays 1.
- **`OneBitSettings { dither, alpha_cut, invert }`** is the `onebit`
  section, registered in `Formats::register_standard`.
- **The kind and its formats:** `bitmap::OneBitPicture` (a checked
  one-bit file, whole), read by `bitmap::OneBitFile` (`one-bit`, by its
  magic bytes, and `Encode`) and made from a PNG by
  `bitmap::dither::Dithered` (`dithered`, `.png`, with the `png`
  feature), both standard; rows in `formats.ron`.
- **Baking in a format of its own:** `Bake::under_as(dir, format)` keeps
  each file of `format`'s kind in `format`'s bytes at its path, so a
  one-bit pack holds a `bitmap` file at `hero.png` that the image table
  (`OneBitInfo`) and the renderer read as they are. `Bake::check` proves
  it equal to its source read with the game's formats, i.e. to dithering
  the PNG again (`pack/one_bit_tests.rs`). A file baked so is baked as no
  other kind. `BakedFile::shown` says which layer each file came from.
- **Target layers:** `Bundle::for_target(layers, per_target, target)`
  lays `<name>@<target>` right above `<name>`, `Layer::target`,
  `Bundle::layer_of`, `bundle::target_layer_name`.
- **`impl Images for Bundle`** in `shuyi_ui::frame::onebit`: a one-bit
  pack's bundle feeds `to_one_bit` with no copying.
- **`shuyi bake`:** the answer's `dithered` and `replaced`, and `--sheet
  <file>` (`contract::bake::sheet`) draws each dithered picture beside
  its source at twice its size. Contract version 31. `shuyi-cli` turns on
  `shuyi-assets/png` (no new crate: `png` was already its dependency).
- **Hello, Grid:** `files@playdate/tiles.png`, its tileset drawn in black
  and white (with `dither: none`), `hello_grid::bundle(target)`,
  `one_bit_bake()`, `Hello::load_on` and `ctx_on`; its binary loads by
  `setup.platform.target`; its tools bake one-bit pictures for a target
  whose engine profile is one-bit. The integration test bakes for
  `playdate` through the contract with a sheet, checks the pack, and
  plays the stage from it on the headless backend, its sprites drawn
  from the pack's pictures.

Deviations:

- A picture from the target's layer goes through the same format and
  its settings (the layer's own `.meta` included), so a hand-drawn
  black-and-white PNG passes unchanged and `dither: none` asserts it; the
  report calls it replaced, not dithered.
- `PackMade` and `BakeInput` became `#[non_exhaustive]` as they grew
  (ADR-0039), and so did `BakedFile`.
- **Not met as written:** "an unknown field in an `onebit` section is an
  error with the settings file and line from `shuyi check`". The section
  is registered in the standard formats, so
  `Formats::standard().check_import_settings` and `shuyi bake` give that
  error with its file and line with no Playdate bake (tested); plain
  `shuyi check` running that check is 2571, as **Out** says, and 2571
  now says what 3007 left it.

Follow-up tickets: 3073 (`frame-png --target playdate` paints sprites
from the target's one-bit pictures, not grey rectangles) and 3074
(`check --baked --target <id>`).

The game: nothing of the game changed here. Its next pin bump follows
the lines below; no game PR exists yet.

Breaking: shuyi-assets: pack::BakedFile: it is `#[non_exhaustive]` and has a new field, `shown`; read its fields, never build one with a struct literal.

```migration
since: 0.1.0
what: A baked file says where its source was read from, and can grow.
old: BakedFile { path, kinds, source }
new: BakedFile { path, shown, kinds, source } (non-exhaustive)
before: |
  let BakedFile { path, kinds, source } = file;
after: |
  let BakedFile { path, kinds, source, .. } = file;
find: ["kinds, source }"]
```

Breaking: shuyi-assets: Formats::register_standard: it registers the `onebit` import settings section (`bitmap::dither::OneBitSettings`) and the one-bit picture formats; a game that registered its own `onebit` section on a standard registry is refused (`SectionTwice`) and reads the engine's.

```migration
since: 0.1.0
what: The engine's standard formats have the onebit section.
old: formats.register_import_settings::<MyOneBit>()
new: cx.import_settings::<shuyi_assets::bitmap::dither::OneBitSettings>(file)
before: |
  let mut formats = Formats::standard();
  formats.register_import_settings::<MyOneBit>()?;
after: |
  let formats = Formats::standard();
find: ["register_import_settings::<MyOneBit>"]
```

Breaking: shuyi-cli: contract::bake::BakeInput and PackMade: both are `#[non_exhaustive]`, `BakeInput` has `sheet` and `PackMade` has `dithered`, `replaced` and `sheet`; build an input with `BakeInput::default()` and assign its fields. The contract's version is 31.

```migration
since: 0.1.0
what: bake's input and answer grew, and are built from their default.
old: BakeInput { target, out }
new: BakeInput::default(), then its fields
before: |
  let input = BakeInput { target: Some("playdate".into()), out: None };
after: |
  let mut input = BakeInput::default();
  input.target = Some("playdate".into());
find: ["BakeInput { target"]
```
