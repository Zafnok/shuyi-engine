---
id: "3231"
title: "A game draws its UI and text at the window's own resolution, in a window layer over the scaled game frame"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2333", "2334"]
nick_input: none
completed:
---

# 3231 — UI and text at the window's own resolution, over the scaled game frame

## Context

Found by the engine's blind-spot audit of 2026-11-23 (its finding "No UI
at the window's own resolution", severity high, nothing planned).

Every frame a screen draws is the game's logical frame, and the backend
scales the whole of it:

- `Screen::draw(&self, ctx, buf: &mut GlyphBuffer)`
  (`crates/ui/src/screen.rs:55`) is the only place a screen paints.
  `GlyphBuffer` (`crates/ui/src/frame/glyph_buffer.rs:520`) is cells of
  the console plus items in console pixels.
- `Renderer::draw` (`crates/app/src/render.rs:163`) places that one
  buffer in the window's physical pixels (the window is `high_dpi`,
  `crates/app/src/window.rs:117`) and draws every texture with
  `FilterMode::Nearest`.
- 2333 (todo) adds `place(mode, frame, window) -> Placement`, integer or
  fit, still for the one frame.
- 2224 (todo) bakes TTF and OTF to atlases at one pixel size each
  (`--px`). Its ADR step 7 and its Out list put rasterising at run time,
  SDF and MSDF out.
- 2334 (todo) lays text runs out as sprites. Its "pixel-placed UI" means
  pixels of the logical frame.
- 2323 (parked) scales glyphs by whole numbers 1 to 3.
- Screens don't know the window's size: `PlatformFacts`
  (`crates/ui/src/platform_facts.rs`) holds facts constant for a run,
  and no `SystemEvent` (`crates/platform/src/platform.rs:249`) says the
  window changed.

So a game with a 320 × 180 pixel-art frame shown at 6× on a 1080p
screen draws its menus and its text at 6× too: blocky, or at a whole
console size chosen for the smallest screen. The common shape elsewhere
is a low-resolution game under a sharp, full-resolution UI: GameMaker's
GUI layer at display size, Godot's canvas layers with stretch modes,
Unity's Canvas Scaler, Bevy's `UiScale`. A pixel game wants it for its
menus and dialogue text; a high-resolution 2D game needs it for
everything. The editor (2506, 2507, both parked) will want it too.

What this ticket makes true: a screen may also paint a **window layer**,
a second plain-data frame sized to the window's physical pixels, drawn
over the scaled game frame. It holds rectangles, sprites and 2334's text
runs, in whole pixels. Fonts stay baked (2224): a game bakes its UI font
at a few sizes, and the layer picks the one nearest the window's scale.

The name is "window layer", not "overlay": `Overlay` is already the
type of a frame's solid rectangle (`Item::Rect(Overlay)`,
`glyph_buffer.rs:381`).

ADRs: ADR-0002 (graphics are a skin; the core pure), ADR-0060 (screens
read where they run, the rules never), ADR-0078 (painted pictures),
ADR-0134 (pointing), ADR-0039 (types that grow). Tickets it builds on:
2333 (the frame's placement), 2334 (fonts by id and text runs), 2224
(baked TTF), 2405 and 2413 (done: the pointer and its hits), 3053 (done:
platform facts).

## Nick input

None. A game opts in by drawing a layer. A game that draws none sees no
change, and the game's look stays its own choice.

## Scope

**In:**
- **An ADR** (the `write-adr` skill): a frame may have a second layer,
  the window layer, in the window's physical pixels, painted over the
  game frame and never under it. It stays plain data with a snapshot
  (ADR-0002 §4). Its fonts are baked at chosen sizes, not rasterised
  while the game runs. Where a backend shows its display 1:1, the layer
  is drawn into the frame instead, so every backend and test sees the
  same items. It names 3236 (parked) for run-time fonts and MSDF. It
  cites ADR-0002 §4 without amending it: the layer is a second frame a
  skin paints into, so the rule holds.
- **`shuyi_ui::frame::window`** (`crates/ui/src/frame/window.rs`),
  `no_std`, integers only:
  - `WindowFacts { size: (u32, u32), frame: PxRect, scale: Factor,
    safe: PxRect, separate: bool }`, `#[non_exhaustive]` with
    constructors (ADR-0039): the window in physical pixels, where the
    game frame lies in it (2333's `Placement`), the UI scale (the
    window's DPI factor as a `Factor` in thousandths, at least 1/1), the
    safe rectangle (the whole window here; 3015, parked, gives a phone's
    notch), and whether the backend draws the layer apart;
  - `WindowFacts::frame_only(spec: ConsoleSpec)`: the frame's own pixel
    size, the frame at (0, 0), scale 1, safe the whole frame, `separate`
    false. It is the default until a backend says otherwise;
  - `WindowLayer`: a size, `items: Vec<Item>` (only `Item::Rect`,
    `Item::Sprite` and `Item::Painted`; a `Gizmo` is refused with a
    debug assertion and dropped), 2334's `TextRun` records, and its own
    `Hits` (2405). It has `add_rect`, `add_sprite`, `add_text` (2334's
    layout, the same rules as the frame's), `clear`, `bounds`, and an
    item cut to its bounds as `GlyphBuffer` cuts them;
  - `Anchor { TopLeft, Top, TopRight, Left, Center, Right, BottomLeft,
    Bottom, BottomRight }` and `anchor(rect_size, anchor, offset: (i32,
    i32), within: PxRect) -> PxRect`: a rectangle placed against an edge
    of the safe rectangle or of the game frame. The offset is in layer
    pixels and moves the rectangle inward from each edge it is anchored
    to; on a centred axis it is not used. A leftover odd pixel rounds
    down;
  - `FontSizes { sizes: Vec<(u16, FontId)> }` with `FontSizes::pick(px:
    u16) -> Option<FontId>`: the largest baked size not over `px`, else
    the smallest. `WindowFacts::font_px(design_px: u16) -> u16` is the
    design size times the UI scale, rounded down, at least 1.
- **The screen paints it:** `Screen::draw_window(&self, ctx, layer: &mut
  WindowLayer)`, a default method that draws nothing. `Game::redraw`
  (`crates/ui/src/game.rs:778`) clears the layer to the window's size
  and calls it for each screen drawn, in the stack's order, after
  `draw`. Modules don't get a hook here (a follow-up if one asks).
- **The facts reach the screens:**
  - `SystemEvent::Window { width: u32, height: u32, frame: (i32, i32,
    u32, u32), dpi_milli: u32 }` (`SystemEvent` is `#[non_exhaustive]`).
    A backend sends it before the first game frame and whenever one of
    them changes;
  - `App` keeps the last one in the context, and `Ctx::window() ->
    &WindowFacts` reads it, with `separate` true. Before one comes, or
    on a backend that never sends one, it is `frame_only`;
  - it is a screen's fact only: the rules never read it (ADR-0060), and
    a test can set it (`Harness::set_window(w, h, dpi_milli)`).
- **Drawn apart, or into the frame:**
  - with `separate` true, `present_game` (`crates/platform/src/app.rs`)
    passes the layer as `Frame::window: Option<&WindowLayer>` (`Frame`
    is `#[non_exhaustive]`; `Frame::new` gives `None`);
  - with `separate` false, `Game::redraw` copies the layer's items into
    the `GlyphBuffer` on `Layer::Over`, after the screens and modules
    and before the gizmos, so the Playdate, the one-bit look, the
    headless backend and `frame-png` see them with no change of their
    own.
- **`shuyi-app` draws it:**
  - `Macroquad::present` (`crates/app/src/backend.rs:545`) draws
    `frame.window` after the game frame, with the default camera, one
    layer pixel per physical pixel (divided by the DPI factor, as
    `Renderer::draw` does), textures `FilterMode::Nearest`: a glyph baked
    at its size lands on whole pixels, so no filter is needed;
  - it sends `SystemEvent::Window` from the window's size, 2333's
    placement and `screen_dpi_scale()` (`dpi_milli` = the factor ×
    1000, rounded), when any changes;
  - the web build runs the same code.
- **The pointer in the layer:**
  - `RawInputEvent::PointerInWindow { x: i32, y: i32 }`
    (`RawInputEvent` is `#[non_exhaustive]`), in layer pixels, sent by a
    backend that draws the layer apart, before each `PointerMoved` or
    `PointerLeft`;
  - `shuyi_ui::input::pointer::Pointer` keeps it, and the layer's hits
    are tested first: a press on a layer hit goes to it, and the frame's
    hits are tested only when the layer has none there;
  - with `separate` false there is no such event, and the layer's hits
    are copied into the frame's with its items.
- **The snapshot:** the layer prints after the frame's lines, as
  `window <w>x<h>` and one line per item and text run, as 2334's runs
  print. A frame without a layer prints as today.
- **`frame-png --window <w>x<h>[@<dpi_milli>]`**: paints the game frame
  at 2333's placement and the layer over it, through the same raster
  (`crates/cli/src/frame_png.rs`), so a look test and an agent can see
  the composed window at a fixed size. Without the flag it is as today,
  with the layer drawn into the frame.
- **An example:** hello-grid's title screen draws its version line in
  the layer, anchored bottom right of the safe rectangle, in a font
  baked at two sizes with 2224's `text-font` (a font whose license the
  example already ships, or the engine's fixture TTF of 2224). Its frame
  snapshots don't change: the layer is a new snapshot.
- **Docs:** `docs/platform-backends.md` gets the layer and the event
  (a backend that shows its display 1:1 sends no `SystemEvent::Window`
  and needs nothing more); the book's UI chapter (2802) gets a section;
  the shipped skill on screens gets a line, if it lists `Screen`'s
  methods.

**For the game:** additive. `Screen::draw_window` has a default;
`Frame`, `SystemEvent` and `RawInputEvent` are `#[non_exhaustive]`;
`WindowFacts` and `WindowLayer` are new `#[non_exhaustive]` types with
constructors. A game whose `match` on `SystemEvent` or `RawInputEvent`
already has a `_` arm compiles unchanged. A session record without the
new events replays as before. The game's PR that moves the pin follows
this one; CI's `api` job shows the new items.

**Out (do not do):**
- run-time rasterising of TTF and OTF, SDF and MSDF text, filtered or
  rotated layer items, fractional zoom: 3236 (parked research, the
  smooth 2D tier). The layer's fonts are baked by 2224;
- smooth (filtered, fractionally zoomed or rotated) drawing of the
  game's own images, and fonts at any size for the game frame: 3236
  researches that tier too (parked);
- a phone's safe area, notches and rounded corners: 3015 (parked) fills
  `WindowFacts::safe` from the platform. Here it is the whole window;
- the widget contract and widgets measuring in either layer: 3230. The
  layout of 2329 already works in `i32` pixels, so it places widgets in
  the layer unchanged;
- a theme file restyling engine screens into the layer: 3235;
- moving any engine screen (options, key bindings, dialogue) into the
  layer: each is a follow-up when a game asks;
- a module hook to draw in the layer;
- the wgpu renderer (3325, parked): it draws the layer as macroquad
  does, by the ADR, when it comes;
- screen effects over the layer (2326, parked);
- the IME's caret position in the layer (2346 keeps it in frame pixels).

**Estimated size:** about 1,700 lines (code 800, tests 600, docs 300),
doubled from a count of about 850 by the rule.

**Split:** past 900 lines, split off steps 6 and 7 (the pointer in the
layer and `frame-png --window`) into a new ticket blocked by this one;
the layer's hits are then copied into the frame's as with `separate`
false, and the PR says so.

## Implementation steps

1. Read 2333's `place` and `Placement`, 2334's `TextRun`, `add_text`
   and `FontTable` as they landed, `glyph_buffer.rs` (`Item`, `Hits`,
   cutting), `screen.rs`, `game.rs` (`redraw`), `platform.rs` (`Frame`,
   `SystemEvent`), `app.rs` (`present_game`), `harness.rs`,
   `crates/app/src/backend.rs` (`present`, `draw_game`) and `render.rs`,
   `crates/ui/src/input/pointer.rs`, and `frame_png.rs`. Check which of
   3230, 3235, 3015 and 3236 are done.
2. Write the ADR.
3. Write `frame::window`: `WindowFacts`, `WindowLayer`, `Anchor`,
   `anchor`, `FontSizes`, with their unit and property tests.
4. Add `Screen::draw_window`, the layer in `Game`, the copy into the
   frame when not separate, and `Ctx::window`.
5. Add `SystemEvent::Window`, `Frame::window`, `present_game`'s part,
   and `Harness::set_window`.
6. Draw the layer in `shuyi-app` and send the event; add
   `RawInputEvent::PointerInWindow` and the hit order in `Pointer`.
7. Add the snapshot lines and `frame-png --window`.
8. Add hello-grid's layer and its font sizes. Run it at 1280 × 800 and
   1920 × 1080, at DPI factors 1 and 1.5 (Windows display scale), on the
   desktop and in the web build, and put screenshots in the PR.
9. Write the docs. Count the changed lines; past 900, take the split
   above. Run the gates.

## Acceptance criteria

- [ ] `anchor` gives these in a 1920 × 1080 safe rectangle at (0, 0), a
      200 × 40 rectangle, offset (16, 16) inward: `TopLeft` (16, 16),
      `BottomRight` (1704, 1024), `Center` (860, 520), `Bottom` (860,
      1024). Against a game frame at (160, 28, 1600, 1024), `TopRight`
      gives (1544, 44). (A table test.)
- [ ] `FontSizes` with 12, 18 and 24: `pick(17)` is 12, `pick(18)` is
      18, `pick(40)` is 24, `pick(8)` is 12. `font_px(12)` at
      `dpi_milli` 1500 is 18, at 1250 is 15, at 1000 is 12.
- [ ] A `Harness` test: a screen drawing a text run in its layer, with
      `set_window(1920, 1080, 1000)`, presents a frame whose `window`
      holds the run at the anchored place, and whose `GlyphBuffer` is
      unchanged from a screen that draws no layer.
- [ ] Without `set_window`, the same screen's run is in the
      `GlyphBuffer` on `Layer::Over`, and `Frame::window` is `None`.
- [ ] A pointer test: with a layer hit over a frame hit, a
      `PointerInWindow` on it then `PointerDown` reaches the layer's
      hit; off it, the frame's hit gets the press as today. (Unless the
      split moved it out.)
- [ ] `frame-png --window 1920x1080` of hello-grid's title writes a
      1920 × 1080 picture with the frame at 2333's placement and the
      version line at the bottom right; its reference PNG is checked
      in. (Unless the split moved it out.)
- [ ] Every existing frame snapshot is unchanged.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] The PR has screenshots of hello-grid at 1280 × 800 and
      1920 × 1080, at DPI factors 1 and 1.5, on the desktop and in the
      web build: the version line's glyphs are whole pixels, not scaled.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `anchor` for every `Anchor` and an odd leftover; `FontSizes::pick`
  and `font_px`; `WindowFacts::frame_only`; `WindowLayer` cutting an item
  at its bounds and refusing a gizmo; the snapshot lines of a layer.
- Property (proptest, windows 1 to 8000 a side, rectangles 0 to 4000):
  `anchor`'s rectangle lies inside `within` when it fits; `pick` never
  returns a size over `px` unless `px` is below the smallest; the same
  inputs give the same answer twice.
- Integration: the `Harness` and pointer tests above; an `App` test in
  `crates/platform/src/app/tests.rs` that a `SystemEvent::Window` changes
  `Ctx::window()` and that a session record without it replays as
  before; hello-grid's layer snapshot; the `frame-png --window`
  reference.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
