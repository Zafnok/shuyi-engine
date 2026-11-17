# ADR-0125: A bold face in the font atlas, drawn once and placed by its own ink

- **Status:** Accepted
- **Date:** 2026-11-16
- **Amends:** ADR-0124 §5 (a bold glyph is drawn twice) and §6 (a bold name's ink is a column wider); its other sections stand
- **Related tickets:** 3216 (this); 3215 (key caps); game#0250, game#0254

## Context

ADR-0124 draws a key cap's name in bold by drawing each glyph twice,
1 px apart, and counts a bold name's ink a column wider. That fakes
bold: a stroke only grows to the right, and a letter's counters (`D`,
`E`) fill in. A bitmap font often comes with a real bold face of the
same cell size: Terminus Bold (`ter-u16b.bdf`) beside Terminus, the same
family and license (SIL OFL 1.1). The game asked for it (game#0250) and
turns it on in game#0254. 3215 had written the code and split it off to
keep its PR under its size limit (3215's completion notes).

## Decision

1. **The atlas holds the bold face beside the regular one.**
   `FontAtlasDef::bold` (`BTreeMap<char, u32>`, `#[serde(default)]`)
   gives a narrow glyph the cell of its bold shape, and
   `FontAtlasDef::bold_ink` (`Option<FontInk>`) that face's ink, measured
   as `ink` is. Both are left out of `atlas.ron` when empty, so an atlas
   without a bold face is written as before. The atlas's checks refuse a
   bold glyph with no narrow one, a bold shape sharing a cell, and a
   `bold_ink` outside the cell. Only narrow glyphs have bold shapes; a
   wide glyph is never bold.
2. **`font-atlas --bold <bold.bdf>`** (`font_atlas::add_bold`,
   `RunOptions::bold`) keeps the bold shape of each narrow glyph the
   atlas keeps that the bold font has, in codepoint order after every
   other cell (the wide sets'), and measures its ink. A bold font of
   another cell size is refused. The summary says `and N bold`.
3. **One helper says how to draw bold**: `FontAtlasDef::bold_strikes(c,
   rect, bold)` returns the bold shape once, or without one the glyph's
   rect twice, 1 px apart; plain glyphs once. Both renderers call it
   (`shuyi-app`'s `render.rs`, the raster's `Painter`, so `frame-png` and
   the web too). The one-bit renderer draws no caps (ADR-0124's
   `one_bit`) and is unchanged.
4. **A cap is placed by the ink of the face it is drawn in.**
   `CellWidths` carries both inks (`ink(bold)`); `KeyCaps` with `bold`
   and a bold face takes the face's ink and counts no extra column, in
   its box and in its extra cells. Without a face it is as ADR-0124
   says. A glyph the face lacks is struck twice but placed by the face's
   ink; `font-atlas` gives every glyph of a complete bold font a shape,
   so only a hand-made atlas meets this, and its `bold_ink` columns can
   say so.
5. **The pack's binary form changes** (`pack::VERSION` 17): the two
   fields are part of the atlas's postcard form.

## Consequences

- A game with a bold font gets real bold caps by adding `--bold` to its
  `font-atlas` line; one without changes nothing but baking its pack
  again.
- The atlas grows by one cell per narrow glyph with a bold shape
  (Terminus's ASCII: about three more rows of 32 cells).
- The renderers' bold rule lives in one function, so a new backend
  (`shuyi-playdate` if it ever draws caps) calls it rather than
  repeating it.
- A struct literal of `FontAtlasDef` names two more fields (a
  `Breaking:` line, as `ink` was in 3215).

## Alternatives considered

- **A second atlas for the bold face** (its own `atlas.png` and
  `atlas.ron`): every loader, pack and renderer would carry two atlases
  and keep them in step; one atlas with a map beside `glyphs` costs a
  field.
- **Bold shapes for every glyph, wide ones too**: no bold CJK font of
  the wide fonts' size is in sight, and a cap's name is a key's, ASCII
  in practice.
- **Keep striking twice and only shift the box**: the look the game asked
  to replace (game#0250).
