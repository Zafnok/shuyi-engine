# ADR-0163: A panel's border may be a picture in nine slices; its text stays glyph cells

- **Status:** Accepted
- **Date:** 2026-11-27
- **Related tickets:** 3249 (this: picture frames and title plates on
  panels), 2338 (the rest: tiled pictures and every engine panel wearing a
  game's frame), 2235 (frames in a data file), game#1380 (the first game
  to use it)
- **Extends:** [ADR-0003](0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
  §5 ("menus, text boxes and help bars stay glyph cells in every style")

## Context

A panel (`GlyphBuffer::panel`, `PanelLook`) is cells: its border is
box-drawing glyphs, its title and text are glyphs. Every 2D engine also
draws a window's frame from one small picture cut in nine slices
(Godot's `NinePatchRect` and `StyleBoxTexture`, Unity's 9-sliced sprites,
GameMaker's nine-slice, Defold's slice-9), and most 2D games' windows,
menus and name plates look that way. The engine's first game asked for it
(game#1380): its windows wear a bought UI kit's frames.

Two facts of the frame shape it:

- Sprite items in `Layer::Under` are drawn after the cells' backgrounds
  and before their glyphs. A highlighted row is cell backgrounds. A
  picture over an inside cell would hide such a highlight.
- A kit's corners are often wider than a cell (a 22-pixel corner over
  8-pixel cells). Laid over the border cells alone, they would reach over
  the text.

## Decision

1. **Only the border becomes a picture.** A `PanelFrame` (a `NineSlice`
   and an outset) in `PanelLook::frame` replaces the lines. Its corners
   and edges are plain sprites in `Layer::Under`; its middle is never
   drawn, and no part of it lies over the inside cells (`rect` less one
   cell each side): their clips are split around them. The inside stays
   the look's background cells, so text, highlights and everything a
   skin paints there work as before. The border cells become blanks.
2. **A wide frame moves out, the text stays.** `PanelFrame::outset`
   widens the frame's rectangle past the panel by that many console
   pixels on each side. The panel's cells and size do not change. The
   frame never reaches past the buffer: at its edge, the outset is cut
   there, so the corners stay whole. (A title plate stays where its title
   sits on its body, cut at the edge if it must: moved in, its rim would
   cross the text.)
3. **A title keeps its glyphs.** A `TitlePlate` (a `NineSlice` whose left
   and right borders are end caps) is drawn behind the title's text, over
   the frame; the title is printed as before.
4. **No frame, no change.** `PanelLook::new` sets neither, and a panel
   without them draws exactly as before.
5. **The slices are sprites.** `nine_slice_sprites` makes plain `Sprite`s
   (a cut corner or tile is a whole-size `dest` with a smaller `clip`),
   so every backend, `frame-png` and the one-bit renderer draw them
   unchanged.

## Consequences

- ADR-0003 §5 now reads: menus, text boxes and help bars stay glyph cells
  in every style; only a panel's border may be a picture.
- A frame keeps its own colours: `PanelLook::border` is not applied.
- A frame's outset lies over cells outside the panel; a later fill of
  those cells cuts it there, as it cuts any item.

## Alternatives considered

- **A frame over the inside too, its middle drawn.** Hides highlight
  bars, and every skin would have to draw its highlights as items.
- **A new kind of item for a nine-slice.** Every backend would draw it;
  plain sprites need nothing new.
- **Shrinking the inside to fit wide corners.** Moves every panel's text
  and changes its size, which a game lays out by cells.
