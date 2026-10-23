---
id: "2338"
title: "Nine-slice and tiled pictures as plain sprites, and a picture frame for engine panels and text boxes"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2338 — Nine-slice and tiled pictures, and picture frames for panels

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Every one of them draws a picture in nine slices (the corners kept, the
edges and the centre stretched or repeated) and repeats a picture over
an area: Bevy's sliced and tiled image modes, Godot's `NinePatchRect`
(<https://docs.godotengine.org/en/stable/classes/class_ninepatchrect.html>)
and `StyleBoxTexture`, Unity's 9-sliced sprites, Unreal's box brushes,
GameMaker's nine-slice per sprite, Defold's slice-9, and Construct's
9-patch and Tiled Background. Most 2D game UI is dialogue boxes, menus
and frames drawn from one small picture.

What Shuyi has today:

- `Sprite` (`crates/ui/src/frame/glyph_buffer.rs`) shows one `src`
  stretched into `dest`, with a `clip` that only ever shrinks what is
  drawn (`Sprite::clipped_src`). Nothing tiles or slices a picture.
- `BoxStyle` (same file) is `Single` or `Double` box-drawing glyphs.
- `PanelLook` (`crates/ui/src/frame/draw.rs`) holds a `BoxStyle` and
  four colours, and no picture. `GlyphBuffer::panel` draws the options,
  key bindings and layout picker screens and the menu widget. The
  dialogue text box (`dialogue/text_box/glyph.rs`, `paint_frame`), the
  question widget (`widgets/ask/glyph.rs`, `paint`), the name boxes
  (`widgets/name_entry/glyph.rs`, `paint_box`) and the profiles screen
  (`screens/profiles/glyph.rs`, `paint` and `paint_actions`) call
  `draw_box` directly.
- `grid::sprite::src_rect` turns an `ImageRect` into a sprite's `src`,
  and `grid::sprite::tile_sprite` makes the sprite of one map tile. A
  game can build a frame by hand from many `Sprite`s in its own
  painter. Nothing in the engine helps.

Plans that touch it: 1127 (parked) wants "a backdrop that repeats
sideways"; 2722 (parked) keeps the visual novel's text box in glyph
cells; 2329 and 2330 lay out widgets but give them no look; 1123
(parked) reads Aseprite's JSON, whose slices can carry a 9-patch; 2229
plans import settings beside a file.

Why it matters:

- The game's panels stay glyph boxes (game ADR-0038 §3), so it doesn't
  need this, and it costs the game nothing: with no frame set, every
  screen draws exactly as today.
- For other 2D games, a framed menu or text box is the first thing a
  pixel-art RPG, a visual novel or a platformer's menu asks for. Today
  an engine panel can't have one. Nick said glyphs alone undersell the
  engine's 2D side (`docs/decisions.md`, the name).
- The parked styles that would use it (2722, 1127) get one shared
  primitive instead of each inventing its own.
- The end goal (2026-10-19) asks for games in few lines: a game frames
  every engine panel by naming one picture, with no drawing code.

Two choices, checked against the code:

- **Plain sprites, not a new kind of item.** The helpers push ordinary
  `Sprite`s. A partial tile is a whole-size `dest` with a cut `clip`,
  so the existing `clip` already gives "cut, never squeezed".
  `shuyi-app` (`crates/app/src/render.rs`, `draw_sprite`), the
  `frame-png` painter (`crates/cli/src/frame_png.rs`, `draw_sprite`)
  and the one-bit renderer (`crates/ui/src/frame/onebit.rs`,
  `draw_sprite`) each draw a sprite from its `src`, `dest` and `clip`,
  so none of them changes. The Playdate backend (3010) and 2222's
  atlases take these sprites like any other. `Item` and `overhang`
  don't change.
- **An engine panel's frame shows over its border cells only.**
  Highlight bars are cell backgrounds: the options and profiles screens
  print the focused row with `print(x, y, label, bg, bar)`, and the
  question widget its focused answer. `Layer::Under` items are drawn
  after cell backgrounds, so a frame part over an inside cell would
  hide a bar there. So an engine panel draws no centre, and the parts
  of its edges that reach past the border cells are cut off there:
  their `clip`s are split as `cut_items` splits an item's, and other
  items are left alone. The inside stays `look.bg` cells. A game's own
  panel can still draw a centre with `GlyphBuffer::nine_slice`.

[ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
§5 says menus, text boxes and help bars stay glyph cells in every
style. Their text still does: only the border becomes a picture, under
the glyphs. A short ADR records that as an extension of §5.

## Nick input

None.

## Scope

**In:**
- **`shuyi_assets::nine_slice`** (`crates/assets/src/nine_slice.rs`,
  `pub mod` in `lib.rs`), `no_std`, plain types beside
  `image::Picture`, so the format of step 8 can read into them:
  - `EdgeFill { Stretch, Tile }` and `CentreFill { Stretch, Tile,
    Empty }`;
  - `NineSlice { picture: Picture, left: u32, top: u32, right: u32,
    bottom: u32, edges: EdgeFill, centre: CentreFill, zoom: u8 }`: the
    picture (an `ImageId` and an `ImageRect`), its borders in image
    pixels, and how many console pixels one image pixel is;
  - all three derive `Debug, Clone, Copy, PartialEq, Eq, Hash`, so
    `PanelLook` stays `Copy` and `Eq`.
- **`shuyi_ui::frame::slice`** (`crates/ui/src/frame/slice.rs`),
  `no_std`, integers only; `frame` re-exports `NineSlice`, `EdgeFill`
  and `CentreFill`:
  - `tiled_sprites(picture: Picture, area: PxRect, origin: (i32, i32),
    zoom: u8, layer: Layer) -> Vec<Sprite>` (named apart from the grid
    kit's `tile_sprite`): one sprite per tile that touches `area`. A
    tile is `picture.rect` times `zoom`, and the tiles are laid from
    `origin`, which may lie outside `area` (a scrolling backdrop moves
    its origin). Each sprite's `dest` is a whole tile and its `clip` is
    the tile's part inside `area`. An empty picture or area gives none;
  - `nine_slice_sprites(slice: &NineSlice, dest: PxRect, within:
    PxRect, layer: Layer) -> Vec<Sprite>`: up to nine parts, more where
    a part tiles (the rule below). Every clip is also cut to `within`,
    and a part or tile wholly outside it is not made, so a tiled centre
    over a huge `dest` makes only the tiles `within` shows. An empty
    `dest` or `within` gives none;
  - `GlyphBuffer::tiled(picture, area, origin, zoom, layer)` passes
    `area` cut to `pixel_bounds()`, and `GlyphBuffer::nine_slice(slice,
    dest, layer)` passes `pixel_bounds()` as `within`. Both push the
    sprites with `add_sprite`;
  - `src_rect` moves from `grid::sprite` to `frame::slice`, and
    `grid::sprite` re-exports it (`pub use`), so `frame` doesn't reach
    into the grid kit and the old path still works. A `zoom` of 0
    counts as 1. `left` and `right` are cut so that together they are
    at most the picture's width (`right` first, then `left`), and `top`
    and `bottom` likewise to its height, so neither function panics on
    a slice built by hand. Positions are worked out in `i64`, and a
    tile whose `dest` doesn't fit in `i32` is left out;
  - the sprites are plain: a caller may set `paint`, `opacity` or
    `brightness` on them before pushing them.
- **The nine-slice rule**, in console pixels, with `z = slice.zoom`:
  - each corner is its border sizes times `z` (the top-left is
    `left * z` by `top * z`), anchored at its corner of `dest`;
  - when `dest` is narrower than the left and right corners together,
    they are cut on their inner side: the left keeps
    `lw = min(left * z, max(dest.w - right * z, (dest.w + 1) / 2))`
    columns and the right keeps `dest.w - lw`. The top and bottom are
    cut the same way, the top taking the odd row. A corner is never
    squeezed: its `dest` keeps its full size and only its `clip`
    shrinks;
  - the edges fill what lies between the corners. `Stretch` is one
    sprite stretched along the edge, as thick as the border times `z`.
    `Tile` repeats the edge's picture from the corner's inner edge, and
    the last tile is cut;
  - the centre fills what is left: `Stretch`, `Tile` from its top-left
    corner, or nothing (`Empty`);
  - an edge or a centre whose picture is 0 wide or tall (borders that
    meet) is not drawn.
- **Panels wear the frame:**
  - `PanelLook` gains `frame: Option<NineSlice>` (`None` from
    `PanelLook::new`) and `PanelLook::framed(self, frame:
    Option<NineSlice>) -> Self`;
  - `GlyphBuffer::panel_border(rect: Rect, look: &PanelLook)`. With no
    frame it is `draw_box(rect, look.style, look.border, look.bg)`, as
    today. With one, it sets the border cells to blanks in `look.text`
    on `look.bg` (a `fill_rect` of each side), makes the frame's
    corners and edges (its centre taken as `Empty`) on `Layer::Under`
    over the panel's pixels with `nine_slice_sprites`, splits each
    sprite's `clip` with `subtract` so no part lies over the inside
    cells (`rect` less one cell on each side), and pushes the parts with
    `add_sprite`. Like `draw_box`, it leaves the inside cells, and the
    items already over them, as they are. `panel` calls it in place of
    `draw_box`;
  - a title stays glyphs over the frame's top edge;
  - the frame keeps its own colours: `look.border` is not applied to
    it.
- **The game names its frames:** `GameData::nine_slice(&self, name:
  &str) -> Option<NineSlice>`, `None` by default. The engine asks for
  two names, constants in `frame::draw`: `PANEL_FRAME` (`"panel"`) for
  `BoxStyle::Single` panels, and `FOCUS_FRAME` (`"focus"`) for
  `BoxStyle::Double` ones (questions, name boxes, the key bindings
  screen's choices, the focused layout in the layout picker), which use
  `"panel"` when the game has no `"focus"`. `Ctx::panel_frame(&self,
  style: BoxStyle) -> Option<NineSlice>` gives the one for a style. The
  toy (`crates/ui/src/toy.rs`) gets `pub frames: BTreeMap<String,
  NineSlice>`, empty by default, which its `nine_slice` reads, so tests
  frame its `CARD`.
- **Every engine panel and text box asks for it:**
  - through `panel`: the options screen, the key bindings screen (its
    panel and its choices), the layout picker and the menu widget, each
    building its look with `.framed(ctx.panel_frame(style))`;
  - through `draw_box` today: each call becomes `panel_border` with the
    same rect, style and colours, and the rest of the painter stays as
    it is. That is the text box (`paint_frame`), the question widget,
    the name boxes (`paint_box`), and the profiles screen (its panel and
    its actions menu);
  - `ask::glyph::paint`, `name_entry::glyph::paint_typing` and
    `paint_grid`, and the text box's private `paint_frame` get a
    `&Palette`, not the `Ctx`, so they take the frame as a new argument
    (`frame: Option<NineSlice>`). The engine's callers pass
    `ctx.panel_frame(BoxStyle::Double)` (`BoxStyle::Single` for the
    text box). **Either order with 2346:** it sets the IME caret in
    `paint_typing`; the frame argument is added the same way whichever
    lands first;
  - the menu's note rule (`widgets/menu/glyph.rs`) leaves out its `├`
    and `┤` joins when its look has a frame; the rule's `─` stays.
- With no frame, every screen draws as today, and every existing
  snapshot is unchanged.
- **Public types (ADR-0037):** `NineSlice`, `EdgeFill` and `CentreFill`
  follow it. `PanelLook` isn't under it yet, and this PR adds to it, so
  it moves under it here, as the ADR's *When* says.
- **For the game:** `PanelLook` gains a field and moves under ADR-0037
  (a struct literal moves to its constructor), and `ask::glyph::paint`
  and the name boxes' painters gain an argument. The PR says so, with a
  `Breaking:` line if 2010 is done, and the game's PR that moves its
  pin follows it. The game sets no frame, so it draws as before.
- **Lines in other tickets**, so the plans that touch this stay true:
  - 1127 (parked), its step 2: the backdrop that repeats sideways is
    `tiled_sprites` with an origin that scrolls;
  - 2722 (parked): its text box, name box and choices wear the game's
    `"panel"` and `"focus"` frames through these painters;
  - 1123 (parked): its Aseprite reader also reads slice centres into
    the kind of the step 8 follow-up, as a second format (rule 10);
  - 2330, if it is not done: a node may name its frame (a
    `GameData::nine_slice` name), and with none its boxes wear
    `"panel"` and `"focus"`.
- **An ADR** (the `write-adr` skill picks the number) that extends
  ADR-0003 §5: a panel's border may be a picture on `Layer::Under`
  over its border cells only, while its text and title stay glyph
  cells; nine-slice and tiled pictures are plain sprites made by
  helpers, not a new kind of item. ADR-0003's status line and its row
  in `docs/adr/README.md` say "extended by ADR-NNNN". **Either order
  with 2224:** if 2224's ADR (text and UI off the cell grid) is
  accepted first, this ADR cites it and records only the helpers and
  the panel's frame; if not, this one comes first and 2224's builds on
  it.
- **Docs:** module docs for `frame::slice` with a doc test of each
  helper; `GameData::nine_slice`'s doc names the two names; a short
  section in the book's chapter on the frame, if 2802 has written it.

**Out (do not do):**
- a fill field on `Sprite`, a new kind of `Item`, or any change to
  `shuyi-app`, the `frame-png` painter or the one-bit renderer;
  drawing tiles with a GPU wrap mode (a backend may do that later as an
  optimisation, tested equal to the sprites);
- a file format for nine-slices, and the pack's form of it: the
  follow-up ticket this PR writes (step 8). Until then a game builds
  its `NineSlice`s in `GameData::nine_slice`;
- slice borders read from Aseprite (1123) or from 2229's import
  settings; image paths checked by `shuyi check` (2516);
- a frame name per node in 2330's `.ui.ron`: 2330's boxes draw through
  these painters, so they wear the game's `"panel"` and `"focus"`
  frames with no field. A name per node is 2330's if it is not done
  when this lands (its line above), and the step 8 follow-up's if it
  is;
- a picture inside engine panels (it needs highlight bars that are
  items, not cell backgrounds);
- mirrored or hidden slices, random tiles, rotation (2335), SVG;
- tinting the frame with `look.border` (2337's tints apply to the
  sprites these helpers return);
- borders scaled by anything but a whole zoom;
- see-through frame pixels that show what is behind the panel (border
  cells are solid `look.bg`), and a gap in the frame behind a title;
- a font's own frame glyphs as a third `BoxStyle`;
- the debug view's boxes (`crates/ui/src/debug/glyph.rs`), which stay
  plain glyphs;
- panels placed in pixels (2334), which call `nine_slice` with their own
  rect;
- the parallax backdrop on world layers (1127, with `tiled_sprites`) and
  the visual novel's boxes (2722);
- frames in the templates and the examples.

## Implementation steps

1. Read ADR-0003 §5; `Sprite`, `add_sprite`, `cut_items`, `subtract`
   and `draw_box` in `crates/ui/src/frame/glyph_buffer.rs`;
   `crates/ui/src/frame/draw.rs`; and `grid::sprite::src_rect`.
2. Write `shuyi_assets::nine_slice`.
3. Write `frame::slice` (with `src_rect` moved in) and its property
   tests.
4. Add `PanelLook::frame` and `panel_border`, and draw `panel` through
   it. `cells_to_px` and `subtract` in `glyph_buffer.rs` are private:
   make them `pub(crate)` for `panel_border`.
5. Add `GameData::nine_slice`, `Ctx::panel_frame` and the toy's
   `frames`, and move the painters listed above.
6. Write the look tests and the `frame-png` test.
7. Write the ADR, the docs and the lines in other tickets.
8. Write the follow-up ticket (the `write-ticket` skill; After MVP, not
   parked): nine-slices in a data file. A `NineSliceDefs` kind of named
   entries (an image's bundle path, an optional rect, borders, `edges`,
   `centre` and `zoom`); its `NineSliceRon` format (id `nine-slices`,
   extension `ron`, `TEXT_MAX_BYTES`) in `Formats::register_standard`,
   refusing a `zoom` of 0 and borders past the rect; a `resolve`
   against the `ImageTable` after loading (ADR-0019 §5) that gives
   `NineSlice`s; errors naming the file and the entry; the kind in
   `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
   (`crates/assets/src/pack/tests.rs`), with `VERSION`
   (`crates/assets/src/pack.rs`) the next number; its row in
   `docs/formats.md`; Aseprite's slice centres as a second format of
   the kind once 1123 is done (`AGENTS.md` rule 10); and the `.ui.ron`
   frame name if 2330 is done.
9. Run the gates. If the PR still grows well past about 600 changed
   lines (fixtures and snapshots aside), leave the moves of the
   `draw_box` callers (text box, question, name boxes, profiles) out,
   and write a ticket for them too (After MVP, not parked). The
   acceptance criteria on those painters, and the either-order clause
   with 2346, then move to it.

## Acceptance criteria

- [ ] With no frame, every existing snapshot and look test is
      unchanged.
- [ ] No file under `crates/app/` changes, nor
      `crates/ui/src/frame/onebit.rs`, nor the drawing code of the
      `frame-png` painter.
- [ ] `tiled_sprites`: the clips cover `area` exactly with no overlap,
      every `dest` is a whole tile on the `origin` grid, and there is
      one sprite per tile that touches `area` (property tests).
- [ ] `GlyphBuffer::tiled` over an area a thousand times the buffer's,
      and `GlyphBuffer::nine_slice` with a `Tile` centre over a `dest`
      that big, push only the sprites that touch the buffer (a test of
      the count).
- [ ] `nine_slice_sprites`, for slices whose edges and centre have
      pictures, with `within` equal to `dest`: the clips lie inside
      `dest`, don't overlap, and cover all of it but the centre when the
      centre is `Empty`; no corner's `dest` is smaller than its picture
      times the zoom. With a smaller `within`, the sprites are those
      clips cut to `within` (property tests).
- [ ] The cut rule gives hand-checked numbers for a `dest` narrower
      than its corners, one with no room for edges, and one a pixel
      wide.
- [ ] With a `"panel"` frame (4-pixel borders at zoom 2, on 8 × 16
      cells), the options screen's border cells are blanks, the frame's
      sprites are on `Layer::Under` and show over the border cells
      only, and the focused row's bar still shows (a look test reading
      cells and sprite items, and a text snapshot).
- [ ] A frame thicker than a cell (6-pixel borders at zoom 3) shows no
      part over the inside cells of the menu widget or the question
      widget (a look test).
- [ ] `Ctx::panel_frame(BoxStyle::Double)` gives `"focus"`, or
      `"panel"` when the game has no `"focus"`, or `None` (a unit
      test).
- [ ] `panel_border` with a frame keeps a sprite that was already over
      the panel's inside cells (a unit test).
- [ ] A panel framed with the test card
      (`shuyi_ui::frame::raster::test_card` if 2332 is done; else
      `crates/cli/src/test_card.rs`) shows the card's corner pixels at
      the panel's corners in a `frame-png` picture (hand-checked
      pixels). **Either order with
      2332:** if 2332 is done, the test uses `shuyi_ui::frame::raster`;
      if not, it goes in `crates/cli/src/frame_png/tests.rs` and 2332
      moves it.
- [ ] The follow-up ticket of step 8 is in `tickets/open/`, and 1127,
      2722 and 1123 have their lines (2330 too, if it is not done).
- [ ] `shuyi-assets` and `shuyi-ui` still build for
      `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cut rule's hand-checked cases; `panel_border` with and
  without a frame, and keeping a sprite inside; `Ctx::panel_frame`
  falling back from `"focus"` to `"panel"`; the item count of a huge
  tiled area and of a huge nine-slice with a tiled centre.
- Property: the tile and nine-slice coverage above, at random sizes,
  zooms and origins; with `origin` and `dest` at `i32` extremes and
  `area` or `within` no bigger than a console, never a panic.
- Snapshot / look: the framed options screen (cells and sprite items,
  and a text snapshot); the thick frame on the menu and the question
  widget; the `frame-png` picture.

## Completion notes

*(Filled in by the session that completes the ticket.)*
