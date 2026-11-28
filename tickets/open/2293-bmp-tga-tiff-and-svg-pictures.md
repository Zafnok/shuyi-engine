---
id: "2293"
title: "More picture formats as plugins: BMP, TGA, PNM, TIFF, ICO and a GIF's still, and SVG rasterised to a size at bake, each behind its own feature"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2293 — BMP, TGA, TIFF and SVG pictures

## Context

2203 and 2234 (done) read PNG, JPEG, WebP, QOI and JPEG XL as picture
formats ([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)),
and 2248 reads APNG and GIF as one-bit flipbooks. 2338's **Out** leaves
SVG. The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's
game coverage*) found the games this leaves out: mod-friendly games
that load players' pictures, games built from older tools' pipelines
(BMP, TGA, TIFF, PNM, ICO), and vector-art games and UI icons that must
stay sharp at any size (SVG). Bevy loads all of these through
`bevy_image`'s features, and SVG through resvg
(`crates/bevy_image/Cargo.toml`, `crates/bevy_image/src/svg.rs`).

Rule 13 applies: each format is a toggle a game can turn off, with its
size cost stated.

## Nick input

None.

## Scope

**In:**
- **Raster formats:** BMP, TGA, PNM, TIFF and ICO as picture formats in
  `crates/assets/src/image/codecs.rs`, and a GIF's first frame as a
  still picture (separate from 2248's flipbook kind). Each decoder is
  scalar and permissively licensed (the `image` crate's own decoders or
  small dedicated crates), checked with `cargo deny` (ADR-0002 §8).
- **SVG:** a picture format over `resvg` and `tiny-skia`. Rasterising
  uses floats, so it runs only at import and bake: `shuyi bake` stores
  pixels at the size the image's `.meta` import settings give (or the
  target profile's), and the pack ships pixels. A development build may
  rasterise at load; it is tested equal to the bake (ADR-0007).
- **Features:** each format behind its own `shuyi-assets` feature, off
  by default; the ticket's completion notes give each one's added size
  in a release web `.wasm` (rule 13).
- Each format has a golden row in `crates/assets/tests/golden/formats.ron`
  (`image: <format id>`), equal on wasm32 (2116), and a fuzz target.
  **Either order with 2020:** if 2020 is done, add each format to its
  fuzz list; if not, nothing to do (2020 fuzzes every registered
  format).

**Out (do not do):**
- HDR and EXR (they need 3227's colour space and a GPU renderer);
- GPU-compressed textures (KTX2, Basis, DDS): 2901's research names
  them;
- animated SVG, and SVG text that needs system fonts (refused with a
  problem code).

**Estimated size:** about 1,000 lines (code 400, tests 450, docs 150).

## Implementation steps

1. Read 2203's codecs and its golden table, ADR-0007 and the `.meta`
   import settings.
2. Pick each decoder; record its licence and how `cargo deny` checked it.
3. Add the five raster formats and the GIF still, each behind its
   feature.
4. Add SVG at import and bake, with the size from import settings.
5. Add golden rows and fuzz targets; measure each feature's size.
6. Update the formats page of the docs and the skill pack.

## Acceptance criteria

- [ ] A 24-bit and an 8-bit BMP, an RLE TGA, a PPM, a TIFF and an ICO
  fixture decode to their golden hashes on native and wasm32.
- [ ] An SVG baked at 32 and 64 pixels gives the golden pixels, and the
  pack holds no SVG.
- [ ] A game with none of the features builds without the decoders
  (`cargo tree` shows none).
- [ ] `cargo deny check` passes with every feature on.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each format's header errors.
- Property: fuzz targets for each decoder.
- Snapshot / integration: golden rows on wasm32; the bake equal to the
  development-build load for SVG.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
