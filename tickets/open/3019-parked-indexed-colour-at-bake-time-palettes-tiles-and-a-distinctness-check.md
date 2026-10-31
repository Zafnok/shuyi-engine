---
id: "3019"
title: "Parked: Indexed colour at bake time: images quantized to 8×8 tiles and palette banks, palette roles at fixed indices with a distinctness check, and frame-png --target gba"
type: feature
milestone: "Parked: export targets"
model: reasoning
effort: high
status: blocked
blocked_by: ["2116", "2204", "2229", "3007", "3017"]
nick_input: none
completed:
---

# 3019 — Indexed colour at bake time

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§4. The GBA and the DS's 2D engines draw from palettes, not from
RGBA. From GBATEK (ADR-0012, *The hardware*; 3018 re-checks):

- colours are 15-bit (5 bits each of red, green and blue);
- tiles are 8 × 8 pixels, at 4 bits a pixel (one of 16 banks of 16
  colours, picked per tile) or 8 bits a pixel (one bank of 256);
- colour 0 of each bank is transparent, so a 4-bit tile has 15 colours
  of its own (re-check).

The 1-bit work has the pattern, and this follows it:

- 3007 bakes sprites to 1-bit once, tests the result equal to its
  source (ADR-0007 §4), and lets `assets@playdate/` override any image;
- 3006 maps palette roles to patterns and checks that the roles in a
  `distinct` group stay apart.

Here the bake quantizes to palettes, roles get fixed indices, and the
same `distinct` groups are checked. The target's palette shape comes
from its profile's `capabilities.colour` (3017). The pack (2204) holds
the result.

## Nick input

None.

## Scope

**In:**
- **The indexed image kind**, in `shuyi_assets::indexed`:
  - `Bgr555`, a 15-bit colour, with `from_rgba8` (round to nearest) and
    `to_rgba8` (`c << 3 | c >> 2`);
  - `IndexedImage { width, height, bpp, tiles, bank_per_tile }`, with
    `bpp` `Four` or `Eight` and the size padded to whole tiles;
  - `PaletteSet`: up to 16 banks of 16 `Bgr555` colours;
  - a `Format` for each inside the pack (2204), with ADR-0007's
    fuzz-style test.
- **Palette groups.** Images that share palettes on the device are
  quantized together: one group per world layer, one for sprites, one
  for the glyph cells. An image's import settings (2229) name its group
  in its `indexed` section; the default is `sprites`.
- **The quantizer**, integers only and deterministic:
  1. **median cut** over a group's opaque pixels, to the target's
     colour count, splitting the box with the widest channel and taking
     the lowest channel on ties;
  2. **nearest colour** in integer Rec. 601 distance,
     `77·dr² + 150·dg² + 29·db²`, the same weights as 3006, lowest index
     on ties;
  3. **4-bit banks**: tiles are grouped by the colours they use, and
     assigned to banks greedily in a fixed order (by tile index). A tile
     that fits no bank is re-mapped to its nearest bank, and the bake
     report counts it;
  4. **ordered dither**, optional per image: Bayer 8 × 8 before the
     nearest-colour step.
- **Per-image settings** are an `indexed` section of the image's import
  settings (2229: `<image>.meta`, or a folder's `_folder.meta`):
  `(group: "sprites", bpp: 4, dither: none | bayer, alpha_cut: 128)`, a
  type implementing `ImportSettings`, registered in
  `Formats::register_standard`. With none, the defaults apply.
- **Tiles are de-duplicated** within a group, flipped matches
  included, because the hardware flips tiles for free. The bake reports
  the tile count of each group against the target's video RAM
  (`capabilities.memory.video_ram_bytes`, 3017), and too many is an
  error naming the group.
- **Palette roles at fixed indices.** A file beside the palette,
  `palette@gba.ron` (and `palette@nds.ron`), maps each role (ADR-0008
  §4) to a bank and an index, and to its colour. Those indices are
  reserved before the quantizer fills the rest. The glyph cells use only
  roles.
- **The distinctness check**, in `shuyi check --target <id>` for
  profiles with `colour: Indexed`:
  - every role the skin uses has an index;
  - roles in one `distinct` group get different `Bgr555` colours
    (error), and a warning when two are closer than a distance set in
    the file;
  - the same groups 3006 reads, with no second list.
- **Override layers.** `assets@gba/` and `assets@nds/` replace any asset
  for that target, through 3007's per-target layers. A hand-made indexed
  PNG there is used as is, and must fit the target's banks (an error
  otherwise).
- **`shuyi bake --target gba`** (and `nds`) writes a pack in which every
  image is an `IndexedImage` with its group's `PaletteSet`. It prints
  each image it quantized, each tile re-mapped to another bank, and each
  group's tile count.
- **Equality** (ADR-0007 §4): decoding the baked images and palettes of
  a group equals running the quantizer over the decoded sources of that
  group, on every fixture.
- **`frame-png --target gba`** (and `nds`) renders what the device would
  show: cells through their roles' colours, sprites and layers from the
  baked indexed images, every colour through `Bgr555`. On `nds` it draws
  each surface (one image per surface until 3023 lands).
- **A contact sheet**, `shuyi bake --target gba --sheet out.png`, as
  3007 has for 1-bit.

**Out (do not do):**
- run-time quantizing;
- the settings file itself, its folder defaults and its errors (2229);
- baking audio (3027 for the GBA; the DS's audio is 3024's);
- turning a map layer into the hardware's screen entries (3021);
- DS textures for 3D (3025);
- choosing the game's palettes, which is the game's work.

## Implementation steps

1. Add `Bgr555`, `IndexedImage`, `PaletteSet` and their formats, with
   fixtures.
2. Write median cut and the nearest-colour step, with hand-checked
   fixtures (a 4 × 4 ramp, a two-colour image, an image with more
   colours than one bank).
3. Write the bank assignment, the de-duplication with flips, and the
   tile counts.
4. Register the `indexed` section (2229) and add the dither.
5. Add the role file, its `Format`, and the check.
6. Wire `--target gba` and `--target nds` into `bake`, with the override
   layers, the report and the contact sheet.
7. Write the equality test, and add `frame-png --target gba` with one
   snapshot per example.

## Acceptance criteria

- [ ] The quantizer's output on each fixture matches its row in a
      golden table (2116's `shuyi_assets::golden`), checked on x64,
      ARM64 and wasm32 by CI's `test` and `determinism` jobs.
- [ ] A fixture skin with `team.player` and `team.enemy` on the same
      `Bgr555` colour in a `distinct` group fails `shuyi check --target
      gba`, naming both roles.
- [ ] A file in `assets@gba/` replaces the quantized image, and the bake
      report says so. One with too many colours fails, naming the file.
- [ ] A group with more tiles than the video RAM holds fails, naming the
      group and both numbers.
- [ ] The equality test passes on every fixture.
- [ ] `frame-png --target gba` of an example screen matches its
      checked-in PNG.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Bgr555` round trips; median cut and nearest colour on small
  hand-checked images; bank assignment; flip de-duplication.
- Property: random images never panic, every output index is in range,
  and the output has the source's size rounded up to whole tiles.
- Golden: the quantizer's output on each fixture, natively and on
  wasm32 (2116).
- Snapshot: one `frame-png --target gba` per example.
- Integration: bake an example for `gba` and load it through the
  headless backend.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
