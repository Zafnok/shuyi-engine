---
id: "3216"
title: "A bold face in the font atlas: font-atlas --bold, and key caps drawn from it"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3215"]
nick_input: none
completed:
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

- [ ] With a bold face in the atlas, a cap draws its glyphs from it (a
      test reads a face drawn unlike two strikes); without one,
      double-struck, as before.
- [ ] A cap of `W`, `D`, `F`, `E` drawn from a bold face has 2 px
      between its ink and its box on every side.
- [ ] `font-atlas --bold` writes the bold cells and their ink, refuses a
      face of another cell size, and an atlas without `--bold` is
      written as before.
- [ ] The render card holds a cap drawn from a bold shape.
- [ ] `frame-png` of a line with caps from a bold face reviewed.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the atlas's checks; `bold_strikes`; `add_bold`'s layout and RON.
- Pixels: a face that draws unlike two strikes is drawn once.
- Snapshot: the render card.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
