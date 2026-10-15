---
id: "3007"
title: "1-bit sprites at bake time: Atkinson or Bayer dithering per image, tested equal to the source, and per-target asset layers for hand-drawn overrides"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2204", "3006"]
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
- **Per-image settings** in the image's sidecar: `onebit: (dither:
  atkinson, alpha_cut: 128, invert: false)`. With no sidecar, the
  defaults apply.
- **Per-target asset layers.** A folder `assets@<target>/` is a bundle
  layer above `assets/`, used only when baking or running for that
  target. Any asset may be replaced there: a hand-drawn `hero.png` in
  `assets@playdate/` wins over dithering `assets/hero.png`.
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
- audio conversion for the Playdate (3010).

## Implementation steps

1. Write the three ditherers with fixture images and checked-in
   expected outputs.
2. Add the sidecar settings and the `OneBitImage` format.
3. Add the target layers to the bundle, and the `--target` flag to
   `bake`.
4. Write the equality test, and add the contact sheet.

## Acceptance criteria

- [ ] Each ditherer's output on the fixtures matches its checked-in
      bits, on native and web.
- [ ] A file in `assets@playdate/` replaces the dithered image, and the
      bake report says so.
- [ ] `dither: none` on a colour image is an error naming the file.
- [ ] The equality test passes on every fixture.
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
