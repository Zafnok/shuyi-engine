# ADR-0124: Key caps: a key's name drawn as a cap, in the room its box needs, its glyphs moved off the grid

- **Status:** Accepted
- **Date:** 2026-11-16
- **Supersedes:** ADR-0122 §4 (a role never changes how many cells a span takes); its other sections stand
- **Related tickets:** 3215 (this); 3216 (a bold face in the font atlas), 3217 (the direction cluster), 2347 (styled text); game#0250, game#0254

## Context

ADR-0122 gave a line of text roles: a key's name is a `TextRole::Key`
span, and the skin paints it in the palette's `key` and `key_bg`. Its §4
said a role never changes how many cells a span takes, and left "a cap's
box" to this ticket.

The game decided how a key's name looks in a hint (game#0250, after five
rounds of mockups): the name in a small rounded box like a keyboard key,
in bold, a single letter as its capital, the same margin between the
letters and the box on all four sides, and nothing beside a cap hidden
under it. On its Terminus 8 × 16 mockups a capital's ink is rows 2–11
and columns 1–6 of its cell, so a box 3 px outside it reaches 2 px into
each neighbouring cell, and covered a `.` or `,` after it.

What the frame had: cells with a glyph and two colours, drawn on the
grid; `Overlay` rectangles under or over the glyphs; no bold, no glyph
off the grid. `Cell` is a plain value (ADR-0039 kind 1): another field
would make it another type. And a game measures a hint before it draws
it (`StyledText::width(buf.widths())`), so a cap's room has to be in
what it measures with.

## Decision

1. **A key cap may take more cells than its name.** A role still never
   changes the text: `plain()`, `Display` and the cells' glyphs are the
   name (a one-letter name drawn as its capital is `F` in its cell, `f`
   in `plain()`). Only a `Key` span drawn as a cap takes more: its name's
   cells and a fixed number more, the same for every name
   (`KeyCaps::extra_cells`), so measuring needs no font: the room a glyph
   inked across its whole cell, a column more if bold, needs for its
   margin and edge on both sides. For Terminus and a 2 px margin, one
   cell.
2. **The room is measured where widths are.** `CellWidths` carries it
   (`with_key_cap_cells`, `key_cap_cells`); `StyledText::width` adds it
   for each `Key` span, and `cut` keeps a cap whole or not at all, its
   `…` plain. A frame that draws caps has them in its
   `GlyphBuffer::widths`, so a skin that fits a hint by measuring it
   with `buf.widths()` fits it with caps, unchanged. `text_width` of a
   plain string is unchanged. Word wrap wraps no `StyledText` today
   (ADR-0122 §8); when it does, it measures with the same widths.
3. **The style is the game's, in code; its colours the theme's.**
   `GameData::key_caps` returns a `KeyCapStyle` (bold, capitals,
   `margin_px`, `one_bit`), `None` by default, so no game's look
   changes until it says. The box's fill is the palette's `key_bg`, its
   1 px edge `key_edge` (new, optional; else the fill), the name `key`:
   a theme's palette changes them. A palette file holds only colours, so
   the switches and the margin are code, not the theme's file.
   `Ctx::key_caps` makes the `KeyCaps` the frame draws with (the style,
   the palette's colours, the font's ink, the cell's size), and `Game`
   hands it to its `GlyphBuffer` before each drawing
   (`set_key_caps`). On a one-bit screen (`PlatformFacts::screen`, from
   the target's display colour) it is `None` unless `KeyCapStyle::one_bit`:
   ADR-0122's patterns hold there until a game decides (game#0256).
4. **A cap is cells, rectangles and a glyph style.** `print_styled`
   prints a cap's name in the key's colour on the background plain text
   gets, then its extra cells as spaces; puts the box under the glyphs as
   `Overlay` rectangles (`Layer::Under`) inside the cap's cells, its
   fill, and its edge without the four corner pixels (a rounded corner);
   and gives the name's cells a **`GlyphStyle`**: moved `dx`, `dy` console
   pixels, and bold. A cap the buffer's edge cuts gets no box and a plain
   style. The rectangles are what every renderer, snapshot and the
   one-bit renderer already draw.
5. **A glyph's style is kept beside the cells, not in them.** The buffer
   holds a sparse map from cell to `GlyphStyle` (`glyph_style`,
   `set_glyph_style`, `glyph_styles`); whatever replaces a cell (`set`,
   a print, `fill_rect`, `blit`) makes it plain, and `blit` brings the
   source's along. `Cell` keeps its four fields. A snapshot lists the
   styles in a `--- glyph styles ---` section when there are any.
   Renderers draw a styled glyph moved by its pixels, and a bold one
   twice, 1 px apart (`shuyi-app`, the raster's `Painter`, so `frame-png`
   and the web too); the render card holds caps. A bold face in the
   atlas, drawn once, is 3216.
6. **Even margins come from the font's ink, measured once.**
   `font-atlas` writes `ink` into `atlas.ron` (`FontInk`): the box most
   capitals `A`–`Z` share the edges of (Terminus: columns 1–6, rows
   2–11), and the columns of each printable ASCII glyph whose ink differs
   (Terminus's `W`: 0–6). A cap's box is `margin + 1` px outside its
   name's ink across, from its first glyph's leftmost ink to its last
   one's rightmost (a column more, bold), centred in its cells; down, as
   far outside the capitals' rows if the line has room (the box is never
   taller than the line), centred in the line. With Terminus and a 2 px
   margin the box is the line's height and 2 px from the ink on all four
   sides. An atlas without `ink` (made before) counts the whole cell as
   ink: a bigger box, still in its cells.

## Consequences

- A game turns caps on with one function, and every hint the engine
  draws with `print_styled` (its own screens' help, the text box's
  corner, the question widget) gets them with no other change.
- A glyph beside a cap is drawn whole: the cap's box and its moved
  glyphs stay inside its own cells.
- A line with caps is wider: a hint that fit a panel by a cell or two may
  be cut. A game measures it as before and sees the new width.
- `FontAtlasDef` has a new field (`ink`), so a struct literal of it
  breaks; an atlas file without it still reads. The template's and the
  examples' atlases carry it.
- A renderer that ignores glyph styles (the one-bit renderer, the
  Playdate's) draws a cap's name on the grid; the style is off there
  unless a game says.
- The direction cluster, one picture for the four arrow keys, is 3217.

## Alternatives considered

- **A field in `Cell`** for the offset and bold: every renderer reads it
  for free, but `Cell` is a plain value (ADR-0039 kind 1), built by
  literal in games; a print that keeps a cell's other fields
  (`print_fg`) would keep a stale offset; and every snapshot and hash of
  cells would change shape.
- **The name drawn as a picture item** (a painted sprite of the name):
  the cells would hold spaces, so a test or a screen reader reading the
  frame's text would lose the key's name, and the ui crate has no glyph
  pixels to paint with.
- **Room per name, measured from its ink**: a tighter cap for a narrow
  name, but `StyledText::width` would need the font's ink and the margin,
  not just widths; one fixed number keeps measuring as simple as wide
  glyphs.
- **The ink of the capitals together** (their union): Terminus's `Q`
  tail and wide `M` and `W` push it to columns 0–6 and row 12, and most
  letters would get 3 px. The most common edges, and per-glyph columns
  where they differ, give every name its margin.
- **Brackets around a key** (`[F]`): text, not a cap; a game can already
  write them in `GameData::key_text` (ADR-0122 alternatives).
