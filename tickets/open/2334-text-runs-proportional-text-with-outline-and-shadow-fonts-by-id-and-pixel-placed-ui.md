---
id: "2334"
title: "Text runs: proportional text placed in pixels with an outline and a shadow, fonts by id, and pixel-placed UI"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2224"]
nick_input: none
completed:
---

# 2334 — Text runs: proportional text with outline and shadow, fonts by id, and pixel-placed UI

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Each draws text at any pixel position, in any of the game's fonts,
wrapped to a width, and most add an outline and a drop shadow. Godot's
`Label` has an outline size and shadow offsets in its theme
(https://docs.godotengine.org/en/stable/tutorials/ui/gui_using_fonts.html).
Bevy has `Text2d` for text in the world and `TextShadow` for UI text.
Defold's labels take an outline and a shadow colour.

What Shuyi has today:

- Text is cells only: `GlyphBuffer::print`, and the helpers in
  `crates/ui/src/frame/draw.rs` (`print_centred`, `print_cut`), count one
  cell per `char`.
- The frame's items are `Item::Rect` and `Item::Sprite`
  (`crates/ui/src/frame/glyph_buffer.rs:305`), and the snapshot lists
  them one per line (`crates/ui/src/frame/snapshot.rs`).
- `Game` reuses one buffer and clears it every frame with a `fill_rect`
  of the whole buffer (`crates/ui/src/game.rs`), which cuts every item.
- Each widget has a glyph look, painting a plain-data view.
  `crates/ui/src/widgets/menu/glyph.rs` says: "Another look is another
  `paint` of the same view (game ADR-0054)".

2224 adds `ProportionalFont`, its formats and the tools, and its ADR
decides how text sits in the frame: a run is ordinary sprites tagged
with the run, plus a record the snapshot prints as one line. This ticket
builds that, and fonts by id. 2329 lays widgets out in cells or pixels,
and its `Measure` needs text that measures itself.

Why it matters:

- **The game:** nothing changes. It stays all glyph cells (game ADR-0038
  §3). `GameData::fonts` has a default, so its PR that moves the pin
  needs no change, and no snapshot of it changes.
- **Other 2D games:** a score in a corner, a "+100" over an enemy, a
  title, a menu in a smooth font. An outline or a shadow keeps text
  readable over pictures. On 2026-10-20 Nick put the features most 2D
  games expect after the MVP, not parked.
- **The end goal** (2026-10-19): an agent making a game writes one call
  to put a score on the screen.

## Nick input

None.

## Scope

**In:**

- **Fonts by id** (2224's ADR), in `shuyi_assets::font`
  (`crates/assets/src/font/table.rs`, re-exported from `font`):
  - `FontId`: a font's bundle id (`&'static str`, such as `fonts/ui`),
    `Copy + Eq + Hash`, made only by a `FontTable`, like `ImageId`;
  - `Face { font: ProportionalFont, pages: Vec<ImageId> }`;
  - `FontTable::new()`, a `const fn` giving an empty table;
  - `FontTable::insert(&mut self, id: &'static str, font:
    ProportionalFont, images: &ImageTable) -> Result<FontId,
    ContentError>`: each of `font.pages` (file names beside the font
    file) joined to the font's path with `AssetPath::sibling`, as
    2224's ADR says, then found with `ImageTable::id`. A
    page the table lacks is an error that names the page and says the
    image table lacks it (a game may have left it out with the `skip`
    list of `ImageTable::load` or `load_with`);
  - `FontTable::load(bundle, formats, images: &ImageTable, ids:
    &[&'static str]) -> Result<Self, Vec<ContentError>>`: each id read
    through the registry (`Formats::load::<ProportionalFont>`), then
    `insert`ed. Every error of every font is kept;
  - `FontTable::insert_cells(&mut self, id: &'static str, &FontAtlasDef,
    page: ImageId) -> FontId`: the cell font as a text font, through a
    new `ProportionalFont::from_cells(&FontAtlasDef, page: &str)` (each
    glyph its cell, the advance `cell_w`, the line height and the
    baseline `cell_h`, no kerning). A game gets pixel-placed text in the
    font it already has by keeping its atlas PNG in the image table;
  - `FontTable::id(&str) -> Option<FontId>` and
    `FontTable::get(FontId) -> Option<&Face>`;
  - `GameData::fonts(&self) -> &FontTable` (`crates/ui/src/ctx.rs`), by
    default a `static` empty table. Its doc says a game builds its table
    once, from the image table it hands the platform as `Look::images`;
  - the toy (`crates/ui/src/toy.rs`) gets `pub fonts: FontTable`, empty
    by default, which its `fonts` returns, so tests can add a font.
- **Layout**, `shuyi_ui::frame::text` (`crates/ui/src/frame/text.rs`),
  `no_std`, integers only:
  - `TextStyle { font: FontId, color: Option<Rgb>, align: TextAlign,
    max_width: Option<u16>, max_lines: Option<u16>, outline:
    Option<Rgb>, shadow: Option<Shadow>, layer: Layer }`, with
    `Shadow { color: Rgb, dx: i8, dy: i8 }` and `TextAlign { Left,
    Center, Right }`, all `Copy + Eq + Hash`. A `color` of `None` keeps
    the page's own colours;
  - `layout(face: &Face, text: &str, style: &TextStyle) -> TextLayout`,
    with `TextLayout { glyphs: Vec<(char, PxRect)>, rect: PxRect, cut:
    bool, missing: Vec<char> }`: each drawn glyph (after the fallback
    below) and where its `Glyph::rect` lands, from the run's top-left:
    - the pen moves by each glyph's advance plus the pair's kerning, and
      a glyph's rect is at the pen plus its `offset`, from the line's
      top;
    - `\n` starts a line, and lines are `line_height` apart;
    - with `max_width`, a line breaks at the last space that fits, and a
      word wider than the width is cut between characters, as
      `shuyi_assets::text::word_wrap` does in cells. A line's width is
      where its pen ends, trailing spaces left out;
    - each line is aligned within `max_width`, or within the widest line
      when there is none;
    - lines past `max_lines` are dropped, and `cut` is set;
    - a character the font lacks is drawn as its `?` (`FALLBACK_GLYPH`)
      if it has one, or else skipped, and `missing` lists it;
    - `rect` holds every line's box (its width by `line_height`) and
      every glyph's rect, grown by one pixel on each side for an
      outline and stretched by the shadow's offset;
  - `measure(face, text, &style) -> PxRect`: `layout`'s `rect`.
- **In the frame** (`crates/ui/src/frame/glyph_buffer.rs`):
  - `TextRunId(u32)`: a run's index in `texts()`, `Copy + Eq + Hash`;
  - `Sprite` gains `text: Option<TextRunId>`. `Sprite::new` sets `None`,
    and clipping and cutting keep it. Code that writes out every field
    of a `Sprite` must add it: say so in the PR (its `Breaking:` line,
    2010);
  - `GlyphBuffer::add_text(&mut self, face: &Face, text: &str, at:
    (i32, i32), style: &TextStyle) -> Option<TextRunId>`, where `face`
    is the `Face` of `style.font`. It lays the text out and adds its
    sprites through `add_sprite`, so clipping holds, in this order:
    1. every glyph's shadow;
    2. every glyph's outline: the glyph in the outline colour at the
       eight places one pixel away;
    3. the glyphs, `Paint::Solid(color)`, or `Paint::Image` with no
       colour.
    So no outline covers a neighbouring glyph. A run with nothing
    visible adds no record and gives `None`;
  - `TextRun { text: String, rect: PxRect, style: TextStyle, cut: bool
    }`, kept in the buffer and returned by `GlyphBuffer::texts() ->
    &[TextRun]`. Its font is `style.font`, and `rect` is `measure`'s
    moved to `at`. `cut` is set when `layout` dropped lines, when the
    buffer's edge clipped one of its sprites, or when a later
    `fill_rect` or `blit` cut one. Tests and agents read this, not
    pixels (`AGENTS.md` rule 9);
  - **cutting:** `fill_rect` and `blit` cut a run's sprites as they cut
    any sprite. A run that lost any part of a sprite is marked `cut`. A
    run with no sprite left is dropped from `texts()`, and the runs
    after it are renumbered, their sprites with them. So the frame's
    clear leaves no runs;
  - **blit** copies the runs whose sprites it brings, `rect` moved by
    its offset, numbered after this buffer's runs, and marked `cut` if
    the copy clipped any of their sprites;
  - `TextRun` and `TextRunId` are re-exported from `frame`
    (`crates/ui/src/frame.rs`);
  - **Either order with 1120:** if 1120 is done, a run can also go in
    a world layer, such as names over things: its sprites placed in
    world pixels and sorted as one item, and its one snapshot line in
    the layer's list; if not, nothing here (1120 adds it).
- **The snapshot** (`crates/ui/src/frame/snapshot.rs`): a run prints one
  line in the overlays section, where its first remaining sprite is,
  and its sprites print nothing on their own:
  ```text
  over  text 11,3 48x11  fonts/ui "Score 100" solid=highlight outline=#000000 shadow=#000000:1,1 align=center
  ```
  After the layer come `text`, the run's `rect`, its font id and its
  text, quoted as Rust's `{:?}` prints a `str`. Then, each only when it
  is set: ` solid=colour`, ` outline=colour`, ` shadow=colour:dx,dy`,
  ` align=center` or ` align=right`, ` width=N` (`max_width`),
  ` lines=N` (`max_lines`), and last ` cut`. Colours are named as the
  other lines name them (a palette name, else `#rrggbb`). The module's
  docs describe the line.
- **Painters:** none change. `shuyi-app` (`crates/app/src/render.rs`),
  the `frame-png` `Painter` (`crates/cli/src/frame_png.rs`, or
  `shuyi_ui::frame::raster`) and the one-bit renderer
  (`crates/ui/src/frame/onebit.rs` and its `reference`) draw a run's
  sprites as they draw any sprite. The tests below show it for the two
  that run headless. **Either order with 2018:** if 2018 is done, a run
  with an outline and a shadow joins its render card
  (`shuyi_ui::frame::raster::card`); if not, nothing more here (2018
  puts one on its card, its clause says so). Until then the card
  already shows a run's pixels: they are sprites in `Paint::Solid`.
- **The player's text size. Either order with 2323 (parked):** if
  2323 is done, a run keeps its position, and its glyphs, advances,
  line height, and outline and shadow offsets grow by the player's
  `glyph_scale` (2323's `TextSize`), so text placed in pixels grows
  with the cells; 2323's `harness::check_text_sizes` covers runs too
  (`TextRun::cut`), with a unit test of a run's advances and line
  height at each scale. If not, nothing to do (2323's clause does it).
- **Pixel-placed UI:**
  - `widgets::TextLabel { label: Label, style: TextStyle }`
    (`crates/ui/src/widgets/text_label.rs`), with `paint(&self, ctx,
    buf, rect: PxRect) -> Option<TextRunId>`: the label in the player's
    language (`Ctx::label`) at the rect's top-left, `max_width` the
    rect's width and `max_lines` as many lines as fit its height. The
    face is `ctx.game.fonts().get(style.font)`; a font the table lacks
    draws nothing. **Either order with 2329:** if 2329 is done,
    `TextLabel` implements its `Measure` in pixels, with `measure`; if
    not, 2329 does it.
  - **The menu's text look**, `widgets::menu::text`
    (`crates/ui/src/widgets/menu/text.rs`), beside `menu::glyph`. It
    draws the same `MenuView` in pixels, with the face of
    `style.font` from `ctx.game.fonts()`; `style` gives the font, the
    outline, the shadow and the layer, and the look sets each run's
    colour, width and alignment:
    - `size(ctx, view, style: &TextStyle) -> (i32, i32)`: the widest
      label with its suffix, or note, plus 4 pixels on each side, by
      one `line_height` per shown item plus 4 pixels above and below,
      plus one line for the note when any item has one. As in the
      glyph look, it doesn't change as the focus moves or the menu
      scrolls. A font the table lacks gives `(0, 0)`;
    - `paint(ctx, view, buf, at: (i32, i32), style: &TextStyle)`, which
      draws nothing for a font the table lacks:
      - a `Panel` rectangle (an `Overlay` in `style.layer`) of that size
        behind it;
      - each shown item a run in `Text`, or `TextDim` when disabled, and
        its suffix a second run after it, in `glyph::suffix_color`;
      - the focused item's row a `Focus` bar (the `Border` colour for a
        disabled one), with its runs in `Panel`, as in the glyph look;
      - a two-pixel `Text` bar along the top or the bottom edge where
        items are scrolled out (`more_above`, `more_below`);
      - the focused item's note in a run under the items.
    Engine screens keep the glyph look. This shows an engine widget
    drawn off the grid, as 2224's ADR allows.
  - **Either order with 2355:** if 2355 is done, `menu::text::paint`
    paints a `MenuView` with `active: false` with no `Focus` bar and no
    note, as the glyph look does, and `size` doesn't change with it; if
    not, 2355 does it when it adds `active`.
  - **Either order with 2405:** if 2405 is done, `menu::text::paint`
    records hits as `menu::glyph::paint` does: its box as `Hit::AREA`,
    then each shown item's row (its pixel rectangle) as the item's
    index, under the view's `widget` key, with a test that `hit_at` on
    a row's pixel gives its item; if not, 2405 does it.
  - **Either order with 2338:** if 2338 is done and the game has a
    panel frame (`Ctx::panel_frame(BoxStyle::Single)`), the text look
    draws it over its `Panel` rectangle with `GlyphBuffer::nine_slice`,
    in `style.layer`, before the items. If not, it is a plain
    rectangle, and 2338 adds the frame when it lands (its clause says
    so).
  - **Either order with 2330:** if 2330 is done, its `Text` node takes an
    optional `font` (an id in `GameData::fonts`) and is drawn as a
    `TextLabel`; if not, 2330 adds the field.
  - **Either order with 2722 (parked):** if 2722 is done, its `vn.ron`
    `layout` takes an optional `font` (an id in `GameData::fonts`), and
    `VnSkin` draws the text box, name box and choices as `TextLabel`s in
    pixel rectangles when it names one, the reveal still wrapped once,
    on the full text, as 2722 describes; if not, nothing to do (2722
    does it).
  - **Either order with 2802:** if 2802 is done, the book's chapter on
    the frame gets a short section on text runs; if not, nothing to do
    (2802 writes it, as its chapter 4 says).
  - **Either order with 2353:** if 2353 is done, its walk
    (`shuyi_platform::harness::walk`) also reads the buffer's runs
    (`texts()`), leaving out a run inside a stop's `skip`: a run with
    `cut` set is a `Cut`, and two or more letters of its text outside a
    bracket pair are a `NotFromKey`, each at the cell under the run's
    top-left with the run's text as `row`, with a test screen that
    draws a cut run; if not, 2353 does it.
- **Test fonts, our own:** a small `ProportionalFont` built in the tests
  (in code, with a kerning pair, a glyph with an offset and a `?`), its
  page in an `ImageTable` from a PNG made in code, and the tiny cell
  atlas of the `frame-png` tests (`tiny_atlas`,
  `crates/cli/src/frame_png/tests.rs`) through `insert_cells`.

**Size.** If the PR passes about 600 changed lines (snapshots and
reference PNG files not counted), `menu::text` (the menu's text look, with
its 2355, 2338 and 2405 clauses) goes to a follow-up ticket
(`write-ticket`), After MVP, and the PR points the clauses of those
three tickets at it.

**Out (do not do):**

- rotated runs, or runs drawn at a scale the caller picks: 2335
  rotates single sprites, and 2323 (parked) scales runs only by the
  player's text size (above);
- styled spans inside a run: 2347 styles cell text only, and its Out
  leaves runs out too; a ticket of its own when a game asks;
- Japanese and Chinese breaking rules for runs (2302's rules are for
  cells; a run breaks at spaces and cuts long words); shaping,
  right-to-left text (2908, parked) and emoji;
- changing the look of any engine screen, or the game's;
- the one-bit copies of a font's pages for the Playdate: 3007 bakes
  every image, and a page's sidecar can pick `dither: threshold(N)`. On
  the GBA (3021, parked) a run's sprites count toward its 128 hardware
  sprites, and 3021 drops and counts the extra ones at run time.

## Implementation steps

1. Read 2224's ADR, `ProportionalFont` and `Glyph`
   (`crates/assets/src/font/proportional.rs`); in `glyph_buffer.rs`
   `Sprite`, `add_sprite`, `cut_items` and `blit`; `snapshot.rs`; and
   `widgets/menu/glyph.rs`.
2. In `shuyi-assets`, add `FontId`, `Face`, `FontTable` (`new`,
   `insert`, `load`, `insert_cells`, `id`, `get`) and
   `ProportionalFont::from_cells`, with their unit tests.
3. Add `GameData::fonts` with its default, and the toy's `fonts` field.
4. Write `frame::text` (`TextStyle`, `Shadow`, `TextAlign`, `layout`,
   `measure`) with its unit and property tests.
5. In `glyph_buffer.rs`, add `TextRunId`, the `text` field of `Sprite`,
   `add_text`, `TextRun` and `texts()`, then the cut and blit rules,
   with tests in a new `glyph_buffer/text_tests.rs`.
6. Add the snapshot line and its module docs.
7. Add `TextLabel` and `menu::text`, with snapshots. Then the
   either-order parts whose tickets are done: 2329's `Measure`, 2330's
   `font`, 2355's inactive look, 2338's frame, 2405's hits, 2353's
   walk, 2018's card, 2722's VN font, 2802's section and 2323's text
   size.
8. Add `insta.workspace = true` to `shuyi-cli`'s `[dev-dependencies]`
   (Apache-2.0, already a workspace dependency, allowed by
   `deny.toml`). Write the `frame-png` references as PNG snapshots of
   `insta::assert_binary_snapshot!`, as
   `examples/hello-grid/tests/it/snapshot.rs` does (in the raster's
   tests), and the one-bit test.
9. Run the gates.

## Acceptance criteria

- [ ] Every existing snapshot and reference PNG is unchanged.
- [ ] `layout` unit tests pass for: advances with a kerning pair; a
      glyph's offset; a break at a space; a long word cut; left, centre
      and right; a `max_lines` cut; a missing glyph drawn as `?` and
      listed; a missing glyph skipped when the font has no `?`.
- [ ] A run with an outline and a shadow prints one snapshot line, and
      `texts()` returns it. After a `fill_rect` over part of it, its
      line ends with ` cut`. After a `fill_rect` of the whole buffer,
      `texts()` is empty and no line prints (snapshot and unit tests).
- [ ] `blit` of a buffer with two runs into one with a run gives three
      runs, numbered in order, each sprite tagged with its own run.
- [ ] `FontTable::load` with a page the image table lacks is an error
      naming the page, and `insert_cells` of `tiny_atlas` gives a face
      whose `A` advances 8 pixels.
- [ ] A `frame-png` reference PNG of runs is committed and reviewed:
      plain, outlined, shadowed, centred in a width, and in the cell font
      through `insert_cells`.
- [ ] The one-bit renderer draws an outlined run with the right pixels
      (a test of a look).
- [ ] The menu's text look has a snapshot and a reference PNG, and the
      glyph look's snapshots don't change.
- [ ] A game that doesn't override `GameData::fonts` builds and draws as
      before (the toy game's and `examples/hello-grid`'s tests).
- [ ] `shuyi-ui` and `shuyi-assets` still build for
      `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `layout` (above); `FontTable::insert` and `load` with a page
  missing from the image table; `from_cells`; the snapshot line with
  each option; a run cut in part and cut away; `blit` renumbering runs;
  `TextLabel` in a rect too short for its lines.
- Property: with `max_width` at least the widest advance, every glyph
  the layout places lies inside the width; the same text and style lay
  out the same twice; every sprite `add_text` adds on an empty buffer
  lies inside the run's `rect`.
- Snapshot / integration: the run lines; the `frame-png` reference; the
  one-bit render; the menu's text look.

## Completion notes

*(Filled in by the session that completes the ticket.)*
