---
id: "2901"
title: "Research: better game images with machine learning (learned compression, upscaling), as a separate repository with a loader plugin, measured against JPEG XL"
type: research
milestone: After MVP
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["2201"]
nick_input: decision
completed:
---

# 2901 — Research: better images with machine learning, as a separate repository

## Context

Nick, 2026-10-14: "If you can find a way to make png/jpegs better using AI
black magic then we can ship a separate repo for that, and support their
loading too" (`docs/decisions.md`). ADR-0007 §6 sets the frame:

- whatever wins lives in its own repository;
- it ships as a `Format` plugin plus a `shuyi bake` step;
- it must stay deterministic and permissively licensed.

"Better" can mean three different things. The research must say which
each idea delivers:

- **Smaller.** The same picture in fewer bytes: a smaller download and
  web build.
- **Faster.** Less time to decode, or less GPU memory.
- **Better looking.** For example, crisp at a large screen size.

Starting points to check on the day, from memory and not verified:

- **Learned image codecs.** One family overfits a tiny neural decoder to
  each image, and its decoders are small enough to run in a game.
  Standards work on learned image coding also exists.
- **JPEG XL.** It is often far smaller than PNG for the same pixels,
  lossless included. A pure-Rust decoder exists. Decoding it is
  2203's, in the engine behind a feature, so here it is the baseline,
  and the separate repository holds only what 2203 doesn't ship
  (encoding JPEG XL at bake, say). **Either order with 2203:** if 2203
  is done, measure its JPEG XL `Format`; if not, measure the decoder
  2203 names, and 2203 still adds it to the engine.
- **GPU texture compression and "supercompressed" textures.** These
  reduce memory, not only file size.
- **Upscaling.** Neural upscalers for painted art. For pixel art, the
  known rule-based scalers: several popular ones are GPL or LGPL and
  denied, though some algorithms can be written fresh.

**Hard rule.** The research uses only our own and CC0 images. **Never
bought art.** At least one bought pack's terms forbid use with
generative-AI tools (the game's `THIRD_PARTY_ASSETS.md`, Shironejiya),
and bought files never leave the private repository (game ADR-0040).

## Nick input

**Decision**, at the end: whether to start the separate repository, and
its name (the `ask-nick` skill, in plain words with the numbers).

## Scope

**In:**
1. **A test set.** About 30 images in four groups:
   - pixel-art sprites and tiles;
   - painted backdrops;
   - portraits;
   - UI.

   All are ours or CC0, credited in `docs/research/images/README.md`.
2. **Candidates.** For each one, check its license from its source, and
   check that it can decode in our web build and on Windows with the GNU
   toolchain. Then measure:
   - size against PNG (and against JPEG for photos);
   - decode time, native and web;
   - quality (PSNR/SSIM, plus "identical" for lossless);
   - determinism across machines;
   - code and model size.
3. **Upscaling.** Pictures before and after (our art only), with decode
   cost and license.
4. **A report.** `docs/research/better-images.md` holds the tables, a
   recommendation, the risks, and, if the answer is go, a plan for the
   separate repository: the `Format` plugin, the bake step, and its
   tickets.
5. **Ask Nick.** Go or no-go, and the repository's name.

**Out (do not do):**
- building the plugin;
- creating the repository before Nick says go;
- any bought art.

## Implementation steps

1. Build the test set and its credits.
2. Write measurement scripts in `xtask` (not shipped). Run them.
3. Write the report. Ask Nick. Record his answer in `docs/decisions.md`.
4. If go, write the tickets.

## Acceptance criteria

- [ ] The report has a source for each license and the measured tables.
- [ ] No bought art was used. List the images used.
- [ ] Nick's answer is recorded. If go, the tickets exist.

## Tests required

None (research). Measurement scripts are kept in `xtask` with a smoke
test.

## Completion notes
