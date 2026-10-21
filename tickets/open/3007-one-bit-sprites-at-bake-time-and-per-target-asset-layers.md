---
id: "3007"
title: "1-bit sprites at bake time: Atkinson or Bayer dithering per image, tested equal to the source, and per-target asset layers for hand-drawn overrides"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2204", "2229", "3006"]
nick_input: none
completed:
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

## Nick input

None. Which pictures the game redraws by hand is the game's choice
(game 1025).

## Scope

**In:**
- **The image kind gets a 1-bit form.** `OneBitImage` holds the pixels
  and a mask, with rows padded to 32 bits (the same layout as 3006's
  `OneBitFrame`), and a `Format` for it inside the pack (2204).
- **Dithering**, in `shuyi_assets::onebit`, in integers:
  - `atkinson` error diffusion, the default for pictures;
  - `bayer` ordered 8 × 8, for flat art and anything that moves;
  - `threshold(N)` for line art;
  - `none`, which means the image must already be black and white, and
    errors otherwise.
- **Per-image settings** are an `onebit` section of the image's import
  settings (2229): `OneBitSettings { dither, alpha_cut, invert }`
  implements `ImportSettings` and is registered in
  `Formats::register_standard`, so `shuyi check` checks it with no
  Playdate bake running. `images/hero.png.meta` holding `(onebit:
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
  At run time the target's id is `PlatformFacts::target` (3053).
  Either order: if 3053 is done, a running game's load picks the layer
  by `setup.platform.target`. If not, the layer is picked when baking
  only, and the pack a target runs from already holds it.
- **A language's copies** (2349: `lang/<tag>/files/<path>`) are plain
  paths in the bundle, so a target layer may hold its own
  (`assets@playdate/lang/ja/files/title/end.png`). `Ctx::localized`
  looks for the language's copy first, then the plain path, each
  through the layers in order: a language's copy in `assets/` beats
  the target's copy of the plain path, and `bake --target playdate`
  dithers a language's copy like any other image.
- **`shuyi bake --target playdate`** writes a pack in which every image
  is a `OneBitImage`, taken from the target layer when one exists and
  dithered otherwise. It prints the images it dithered, so that an
  artist sees what to check.
- **Equality** (ADR-0007 §4): decoding a baked `OneBitImage` equals
  dithering the decoded source with the same settings, on every fixture.
- **A contact sheet.** `shuyi bake --target playdate --sheet out.png`
  draws every dithered image beside its source, for review.

**Out (do not do):**
- run-time dithering of sprites;
- other bit depths (2-bit, 4-bit greys); write a ticket when a target
  needs one;
- audio conversion for the Playdate (3010);
- the settings file itself, its folder defaults and its errors (2229);
- a language's copies of files and `Ctx::localized` (2349): nothing
  here builds them, and the order above is stated in both tickets;
- knowing the target at run time (3053's `PlatformFacts`).

## Implementation steps

1. Write the three ditherers with fixture images and their expected
   outputs (rows of 2116's golden table, or checked-in bits).
2. Add `OneBitSettings` as 2229's `onebit` section, registered in
   `Formats::register_standard`, and the `OneBitImage` format that
   reads it.
3. Add the target layers to the bundle, and the `--target` flag to
   `bake`. If 3053 is done, pick the layer at load by
   `setup.platform.target`.
4. Write the equality test, and add the contact sheet.

## Acceptance criteria

- [ ] Each ditherer's output on the fixtures matches its row in a
      golden table (2116's `shuyi_assets::golden`) in `shuyi-assets`,
      checked on x64, ARM64 and wasm32 by CI's `test` and
      `determinism` jobs (if 2116 isn't done, plain checked-in bits,
      which 2116's job then runs).
- [ ] A file in `assets@playdate/` replaces the dithered image, and the
      bake report says so.
- [ ] `dither: none` on a colour image is an error naming the file.
- [ ] The equality test passes on every fixture.
- [ ] An unknown field in an `onebit` section is an error with the
      settings file and line from `shuyi check`, with no Playdate bake
      running.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each ditherer on small hand-checked images (a 4 × 4 grey ramp).
- Property: random images never panic, and the output has the source's
  size.
- Integration: bake an example for `playdate`, and load it through the
  headless backend.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
