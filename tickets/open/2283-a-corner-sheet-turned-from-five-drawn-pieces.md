---
id: "2283"
title: "A corner sheet turned from 5 drawn pieces: corners::rotated and a Rotated recipe, the other 10 pictures quarter turns"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2283 — A corner sheet turned from 5 drawn pieces

## Context

**From 2259 (done):** this is that ticket's split, at its `**Split:**`
line: 2259 made a terrain's corner sheet from its one tile
(`shuyi_assets::corners::generate`, behind `shuyi-assets`' `corners`
feature; `shuyi-cli`'s `corners` command, `make` and `check`;
ADR-0160). This ticket adds its second recipe, for hand-drawn art.

A dual grid needs only 5 pieces drawn: one corner in (mix 8, `#...`),
two side by side (12, `##..`), two across (9, `#..#`), three in (14,
`###.`) and all four (15, `####`). The other 10 are their quarter turns.
The sheet is the one `generate` makes: 8 × 2 pictures, mix `m` at column
`m % 8`, row `m / 8` (`corners::sheet_rect`), mix 0's place clear.

A quarter turn clockwise moves top-left to top-right, top-right to
bottom-right, bottom-right to bottom-left and bottom-left to top-left,
both for the pixels (`(x, y)` to `(n − 1 − y, x)` on an `n × n` piece)
and for the mix's bits (8 to 4, 4 to 1, 1 to 2, 2 to 8). The orbits:
8 → 4 → 1 → 2; 12 → 5 → 3 → 10; 9 → 6; 14 → 13 → 7 → 11; and 15 alone.

## Nick input

None.

## Scope

**In:**
- `pub fn rotated(pieces: [&Image; 5]) -> Result<Image, CornersError>`
  in `shuyi_assets::corners`: the pieces of the mixes in
  `pub const PIECES: [u8; 5] = [8, 12, 9, 14, 15]`, each other mix a
  quarter turn of one of them; mix 15 the piece as drawn, never turned.
  Square pieces only.
- `CornersError` (`#[non_exhaustive]`) gains: pieces of different sizes
  (which piece, its size, the first's), and pieces that aren't square.
  An empty first piece, or a side over `MAX_SIDE`, is `Empty` or
  `TooLarge`, as a tile is.
- The recipe's `Make` (`#[non_exhaustive]`) gains `Rotated { image:
  String, size: (u32, u32), pieces: [(u32, u32); 5] }`, written
  `make: Rotated(image: "tiles/shore.png", size: (16, 16), pieces: [(0,
  0), (16, 0), (32, 0), (48, 0), (64, 0)])`. RON reads a fixed-size
  array as a tuple, so `pieces` reads a list and refuses one not of 5,
  with its line. A piece outside its picture is an error with the line
  of `pieces` (as a tile's gives its `at`'s, `Recipe::sheet`).
- `Recipe::sheet` makes it; `shuyi_cli::corners`' `make`, `check` and
  command then work for it unchanged.
- A row `rotated` in `crates/assets/tests/golden/corners.ron`, the hash
  of a fixed recipe's sheet, as `generated`'s is.
- Docs: the module docs of `corners` and `corners::recipe`, the
  `corners` feature's comment in `crates/assets/Cargo.toml`, and the
  book's paragraph on corner sheets
  (`site/content/learn/book/frame-and-skins.md`).

**Out (do not do):**
- Rotating sprites as they are drawn (2335): the turns are baked.
- Variants of a mix.

**Estimated size:** about 300 lines (code 110, tests 150, docs 40).

## Implementation steps

1. `turn` (a picture) and `turn_mix` (a mix's bits), private.
2. `rotated`: check the pieces, then for each piece and its mix, its
   four turns, filling each mix not yet filled.
3. The recipe's `Rotated`, its `pieces` list of 5, and `Recipe::sheet`.
4. The golden row, the docs.

## Acceptance criteria

- [ ] `rotated`: mix 4's picture is mix 8's piece turned a quarter
  clockwise, and mix 6's is mix 9's turned once; the 5 pieces' mixes are
  the pieces as drawn; all 15 mixes are filled and mix 0 is clear.
- [ ] Pieces of two sizes, pieces not square, and an empty first piece
  are each their error.
- [ ] A `Rotated` recipe makes the sheet; 4 pieces, and a piece outside
  its picture, are errors with the recipe's file and line.
- [ ] The golden table has a `rotated` row, run on wasm32 in the
  `determinism` job.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each acceptance case; each error.
- Property: a quarter turn done four times gives the piece back.
- Snapshot / integration: the golden row; `shuyi-cli`'s command on a
  `Rotated` recipe.

## Completion notes
