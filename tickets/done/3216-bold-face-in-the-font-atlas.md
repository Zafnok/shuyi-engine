---
id: "3216"
title: "A bold face in the font atlas: font-atlas --bold, and key caps drawn from it"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["3215"]
nick_input: none
completed: 2026-11-16
---

# 3216 — A bold face in the font atlas

## Context

**From 3215 (done):** key caps (ADR-0124) draw a cap's name in bold by
drawing each glyph twice, 1 px apart. 3215's scope also had the font
atlas take a bold face of the same cell size, and the renderers draw a
bold cap's glyphs from it; it was split off here to keep 3215's PR under
its size limit (3215's completion notes). The game asked for it in
game#0250 (*Key names in hints*, Terminus Bold, `ter-u16b.bdf`, the
same family and license as the regular face, SIL OFL 1.1), and
game#0254 turns it on.

What 3215 leaves (engine `main` after 3215's PR): `GlyphStyle::bold` on
a cap's glyphs; both renderers (`shuyi-app`'s `render.rs` and the
raster's `Painter`) draw a bold glyph twice; `FontAtlasDef::ink`
(`FontInk`) holds the regular face's ink, which `KeyCaps` places a name
by, and `KeyCaps` counts a bold name's ink a column wider.

The code was written once and taken out: commit `3f6b8875` on 3215's
branch (`t3215-key-caps-in-text`, reachable from 3215's PR) has
`FontAtlasDef::bold`, `bold_ink`, `bold_rect`, `bold_strikes`,
`font_atlas::add_bold`, `run_with_bold`, the `--bold` option and their
tests. Start from it.

## Nick input

None. The look is the game's decision (game#0250).

## Scope

**In:**
- `FontAtlasDef::bold` (`#[serde(default)]`, a narrow glyph → the cell
  of its bold shape) and `bold_ink` (`Option<FontInk>`, the bold face's
  ink), left out of `atlas.ron` when empty; `bold_rect`; the atlas's
  checks (a bold glyph with no narrow one, two glyphs in one cell, ink
  outside the cell).
- `CellWidths` carries the bold face's ink; `KeyCaps` places a bold
  name by it, and counts no extra column when the face draws it.
- `cargo shuyi font-atlas --bold <bold.bdf>`: the bold shape of each
  narrow glyph the atlas keeps that the bold font has, after the wide
  sets, and its ink; the same cell size or an error; the summary says
  `and N bold`; the usage line and the help say it.
- Both renderers draw a bold glyph from its bold shape once, else twice
  (one helper, `FontAtlasDef::bold_strikes`, both call).
- The render card's atlas gets a bold shape for one of its glyphs, and a
  cap drawn from it.
- Public types follow ADR-0039: `FontAtlasDef` gains two fields (a
  `Breaking:` line for a struct literal, as 3215's `ink` had).

**Out (do not do):**
- Shipping a bold font in the engine's examples or template: a game
  supplies its own (Terminus Bold is the game's, game#0254).
- The direction cluster (3217).

**Estimated size:** about 500 lines (code 220, tests 230, docs 50).

## Implementation steps

1. Bring back from `3f6b8875` the `bold` and `bold_ink` fields, their
   checks and `bold_rect` / `bold_strikes` (`crates/assets/src/font.rs`),
   on today's `FontInk`.
2. `font_atlas::add_bold` and `run_with_bold`, the `--bold` option in
   `crates/cli/src/lib.rs`, and the help.
3. `KeyCaps::with_colors`: with `bold` and a bold face, the face's ink
   and no extra column (`crates/ui/src/frame/key_cap.rs`).
4. Both renderers through `bold_strikes`; the render card.
5. ADR-0124 gets a line in its §5 (an amendment note) or a new ADR.

## Acceptance criteria

- [x] With a bold face in the atlas, a cap draws its glyphs from it (a
      test reads a face drawn unlike two strikes); without one,
      double-struck, as before.
- [x] A cap of `W`, `D`, `F`, `E` drawn from a bold face has 2 px
      between its ink and its box on every side.
- [x] `font-atlas --bold` writes the bold cells and their ink, refuses a
      face of another cell size, and an atlas without `--bold` is
      written as before.
- [x] The render card holds a cap drawn from a bold shape.
- [x] `frame-png` of a line with caps from a bold face reviewed.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the atlas's checks; `bold_strikes`; `add_bold`'s layout and RON.
- Pixels: a face that draws unlike two strikes is drawn once.
- Snapshot: the render card.

## Completion notes

Done as scoped, from 3215's commit `3f6b8875`, on today's `FontInk`
and `RunOptions`:

- `FontAtlasDef::bold` and `bold_ink` (`#[serde(default)]`, left out
  of `atlas.ron` when empty), `bold_rect`, `bold_strikes`; the atlas's
  checks name a bold glyph with no narrow one, a bold shape in another
  glyph's cell (the shared-cell check now sees bold cells) and a
  `bold_ink` outside the cell.
- `CellWidths::ink(bold)` carries both faces' ink; `KeyCaps` with
  `bold` and a bold face places the name by the face's ink and counts
  no extra column, in its box and its extra cells.
- `cargo shuyi font-atlas --bold <bold.bdf>` (`font_atlas::add_bold`):
  the bold shapes after the wide sets, their ink, a bold font of
  another cell size refused, `and N bold` in the summary, the usage
  line and the help.
- Both renderers (`shuyi-app`'s `render.rs`, the raster's `Painter`)
  draw through `bold_strikes`.
- The render card's atlas has a bold `o` (2 px outline), its `o` cap
  drawn from it and its `|` cap, which the face lacks, struck twice; a
  test reads the cap's pixels with and without the face.
- ADR-0125 amends ADR-0124 §5 and §6; `docs/formats.md` and the book's
  *Actions* chapter say how to add a bold face.

Deviations:

- `run_with_bold` is not brought back: `font_atlas::run` takes a
  `RunOptions` now (ADR-0039 kind 4, 2086), so the bold face is a new
  field, `RunOptions::bold`, which breaks nothing.
- ADR-0124 is amended by a new ADR (0125) rather than a line in its §5:
  the `write-adr` skill amends an accepted ADR only so.
- The pack format moves to 17: the two fields are in the atlas's
  postcard form (3215 moved it to 16 for `ink`).
- The picture (`cargo test -p shuyi-cli key_caps_pictures -- --ignored`,
  `target/key-caps/caps-bold-face.png`, reviewed) and the pixel tests in
  `frame_png/key_cap_tests.rs` use a bold face made from Terminus by
  thickening each glyph a pixel left and right: Terminus Bold is not in
  the engine (the ticket's Out), so its own pictures are the game's
  (game#0254). The caps of `W`, `D`, `F`, `E` (and lowercase) have 2 px
  between ink and box on every side with it.
- A glyph the bold face lacks is struck twice but placed by the face's
  ink (ADR-0125 §4); `font-atlas` gives every glyph a complete bold
  font has a shape, so only a hand-made atlas meets it.

Book: `site/content/learn/book/actions.md` (key caps) says how to add a
bold face with `font-atlas --bold`.

Game: game#0254 (the game adds Terminus Bold with `--bold`). Its pin
move breaks the struct literal in its `crates/xtask/src/icon.rs`
(found with `gh search code --repo Zafnok/visions-of-shuyi
'FontAtlasDef {'`), which already lacks 3215's `ink`; no game code
calls `CellWidths::ink` (searched `widths.ink` and `.ink()`: no hit).

Breaking: shuyi-assets: FontAtlasDef: two new fields, `bold` (`BTreeMap<char, u32>`) and `bold_ink` (`Option<FontInk>`); a struct literal adds them, and an atlas file without them still reads

```migration
since: 0.1.0
what: The font atlas's description has two new fields, bold and bold_ink, which font-atlas --bold writes; a game that builds a FontAtlasDef by hand adds them.
old: "FontAtlasDef { cell_w, cell_h, columns, glyphs, wide, wide_sets, ink }"
new: "FontAtlasDef { cell_w, cell_h, columns, glyphs, wide, wide_sets, bold, ink, bold_ink }"
before: |
  FontAtlasDef {
      cell_w: 2,
      cell_h: 4,
      columns: 2,
      glyphs: [('b', 0), ('a', 1)].into_iter().collect(),
      wide: BTreeMap::new(),
      wide_sets: BTreeMap::new(),
      ink: None,
  }
after: |
  FontAtlasDef {
      cell_w: 2,
      cell_h: 4,
      columns: 2,
      glyphs: [('b', 0), ('a', 1)].into_iter().collect(),
      wide: BTreeMap::new(),
      wide_sets: BTreeMap::new(),
      bold: BTreeMap::new(),
      ink: None,
      bold_ink: None,
  }
```

Breaking: shuyi-assets: CellWidths::ink: takes `bold: bool` (the bold face's ink with `true`); a caller of `ink()` writes `ink(false)`

```migration
since: 0.1.0
what: CellWidths::ink names the face whose ink it returns.
old: "CellWidths::ink(&self) -> Option<&Rc<FontInk>>"
new: "CellWidths::ink(&self, bold: bool) -> Option<&Rc<FontInk>>"
before: |
  let ink = widths.ink();
after: |
  let ink = widths.ink(false);
find: ["widths.ink()"]
```

Breaking: shuyi-assets: pack::VERSION: 17 (a font atlas's bold face); a pack baked before is refused, so a game bakes again with `shuyi bake`

```migration
since: 0.1.0
what: The pack format is version 17; a pack of version 16 is refused.
old: "pack::VERSION == 16"
new: "pack::VERSION == 17"
before: |
  cargo shuyi bake   # with the engine before 3216
after: |
  cargo shuyi bake   # again, with this engine
```

Breaking: shuyi-assets: the golden value `font atlas: font-atlas` (`crates/assets/tests/golden/formats.ron`): a decoded atlas has `bold` and `bold_ink`, so its state hash changed; a game's golden table of a font atlas is blessed again

```migration
since: 0.1.0
what: A decoded font atlas has bold and bold_ink fields, so its state hash in a golden table changes.
old: "golden font atlas hash before 3216"
new: "golden font atlas hash with an empty bold face"
before: |
  cargo xtask golden
after: |
  cargo xtask golden --bless
```

Behaviour: shuyi-cli: `font-atlas`'s usage line and help name `--bold <bold.bdf>`; an atlas made without it is written byte for byte as before

Behaviour: shuyi-ui, shuyi-app: with a bold face in the atlas, a bold key cap's glyphs are drawn once from it and its box placed by its ink, no column wider; without one (every game today), nothing changes
