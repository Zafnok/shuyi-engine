---
id: "3249"
title: "Picture frames and title plates on panels: nine-slice sprites, a frame that reaches past the panel, and a plate behind the title"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 3249 — Picture frames and title plates on panels

## Context

1. **The game ticket:** game#1380
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1380-the-battles-windows-framed-by-the-ui-kits-pictures.md`):
   its battle windows are drawn with a bought UI kit's frames. It is not a
   blocker of this ticket.
2. **The job in one sentence:** frame a window with a picture whose edges
   stretch to any size, and put a name plate behind its title.
3. **No friction review:** a missing feature (game#1380's Context: the
   engine has no picture frame).
4. **The generic shape:** a picture cut in nine slices (four corners kept,
   edges and middle stretched or repeated) laid over a rectangle as plain
   sprites, and a panel (`GlyphBuffer::panel`, `PanelLook`) that wears one
   in place of its box lines, reaching a few pixels past the panel when
   the picture's corners are wider than a cell, with an optional plate
   (end caps and a stretched body) behind its title.
5. **An example that isn't tactics:** a farming game's shop window and its
   "General Store" plate; a visual novel's text box.

2338 plans all of this and more (tiled pictures, `GameData::nine_slice`,
every engine panel and text box asking for the game's frame). This ticket
takes its first part, the primitive and the panel, so the game can use it
now, and adds two things 2338 leaves out that the game's kit needs:

- **The outset.** 2338 lays a frame over the border cells only. A kit
  whose corners are 22 pixels wide over 8-pixel cells would reach over the
  text; drawn 5 pixels outside the window, it doesn't (the game's mockups
  of 1379 drew it 10 to 16 screen pixels out at scale 2).
- **The title plate.** 2338 leaves "a gap in the frame behind a title"
  out; the game puts its title on a plate (two end caps and a body
  stretched to the title's width).

## Nick input

None. (Nick agreed to this engine PR on 2026-11-27, game#1380's session.)

## Scope

**In:**
- `shuyi_assets::nine_slice` (`no_std`): `EdgeFill { Stretch, Tile }`,
  `CentreFill { Stretch, Tile, Empty }` (both `#[non_exhaustive]`),
  `Borders { left, top, right, bottom }` (a plain value, kind 1 of
  ADR-0039, with `uniform` and `fitted`), and `NineSlice { picture,
  borders, edges, centre, zoom }` (kind 2: `new(picture, borders)`,
  `with_edges`, `with_centre`, `with_zoom`, `scale`).
- `shuyi_ui::frame::slice`: `nine_slice_sprites(slice, dest, within,
  layer)` by 2338's nine-slice rule, and `GlyphBuffer::nine_slice`.
- `shuyi_ui::frame::draw`: `PanelFrame { slice, outset }` and
  `TitlePlate { slice, pad, text_top }` (kind 2), `PanelLook::frame` and
  `PanelLook::plate` (`None` from `new`, `with_frame`, `with_plate`),
  `GlyphBuffer::panel_border`, and `panel` drawing both;
  `GlyphBuffer::title_plate` public, for a title a skin prints itself.
- The menu widget in a look of the caller's: `widgets::menu::glyph::look`
  (the look `paint` uses) and `paint_with(ctx, view, buf, x, y, &look)`,
  its rows on `look.bg` and no `├` `┤` joins under a frame. `paint` is
  `paint_with` in `look`. (2338 makes every engine panel ask the game for
  its frame; a game that frames only its own menus needs no more.)
- ADR-0163: only a panel's border becomes a picture (extends ADR-0003 §5).
- The book's chapter *The frame and skins*: a paragraph on panels and
  their frames.
- Additive: no break. With no frame or plate, every panel draws as today.

**Out (do not do):**
- `tiled_sprites`, `src_rect`'s move, `GameData::nine_slice`,
  `Ctx::panel_frame`, the toy's frames and every engine panel and text
  box wearing the game's frame: 2338.
- A file format for frames: 2235.
- A frame over the inside (ADR-0163 §1).

**Estimated size:** about 900 lines (code 400, tests 350, docs 150).

## Implementation steps

1. `crates/assets/src/nine_slice.rs` and its `pub mod`.
2. `crates/ui/src/frame/slice.rs`, `pub mod slice` in `frame.rs`.
3. `draw.rs`: the two types, the look's fields, `panel_border`, the plate
   in `panel`. `subtract` and `cells_to_px` in `glyph_buffer.rs` become
   `pub(crate)`.
4. Tests, the ADR, the book; 2338 and 2235 told what this did.

## Acceptance criteria

- [x] `nine_slice_sprites` follows the rule: corners, edges, the middle,
      cut corners on a narrow rectangle, tiles, `within`.
- [x] A framed panel draws no lines, keeps its cells, and puts no sprite
      over its inside cells; with an outset the frame reaches past it.
- [x] A plate hugs the title with its pad and caps, over the frame.
- [x] Without a frame or a plate, every snapshot is unchanged.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the rule's parts and numbers, a 1-cell panel, the fallback.
- Property: the parts cover a rectangle of any size exactly once.

## Completion notes

- Built as the Scope says: `shuyi_assets::nine_slice` (`EdgeFill`,
  `CentreFill`, `Borders`, `NineSlice`), `frame::slice`
  (`nine_slice_sprites`, `GlyphBuffer::nine_slice`), and in `frame::draw`
  `PanelFrame`, `TitlePlate`, `PanelLook::frame` and `plate` with their
  `with_` methods, `GlyphBuffer::panel_border` and `title_plate`, and
  `panel` drawing both; the menu widget's `look` and `paint_with`.
  `subtract` and `cells_to_px` in `glyph_buffer.rs` are `pub(crate)` now.
- Deviation from 2338's text: the borders are a `Borders` value, not four
  flat fields, so `NineSlice::new` takes no four neighbouring numbers
  (ADR-0151); `PanelLook::frame` is an `Option<PanelFrame>`, not an
  `Option<NineSlice>`, to carry the outset. 2338 and 2235 say so in a
  "From 3249 (done)" note, with what is left to each.
- ADR-0163 records that only a panel's border becomes a picture; it is the
  ADR 2338 planned.
- Found by the game's pictures: a frame's outset never reaches past the
  buffer (a panel at the screen's edge had its corners cut), as the
  game's mockups of 1379 drew it: the outset is cut at the edge. A title
  plate on the top row stays where its title sits on its body, its top
  cut (moved in, its rim crossed the text).
- No break: every addition is a new item or a new field of a
  `#[non_exhaustive]` type built by its constructor; with no frame or
  plate `panel` calls `draw_box` as before, and no snapshot changed.
  Searched the game for a `PanelLook` literal (`gh search code --repo
  Zafnok/visions-of-shuyi 'PanelLook {'`): none, so its new fields break
  nothing there.
- Book: the chapter *The frame and skins* has a paragraph on panels, their
  frames and plates.
- Skills: none, no schema, template, format or problem code changed.
- Game: game#1380, whose PR moves the pin to this change.

