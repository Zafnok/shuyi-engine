---
id: "2301"
title: "Wide glyphs: a character two cells wide in the glyph buffer, drawn from a wide font in the atlas"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2350"]
nick_input: none
completed: 2026-11-04
---

# 2301 — Wide glyphs: two cells in the glyph buffer, from a wide font in the atlas

## Context

- **The game ticket:** game#0236
  ([`0236-wide-glyphs-japanese-font-in-the-atlas.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0236-wide-glyphs-japanese-font-in-the-atlas.md)).
  It adds a Japanese font to the game; this ticket is the engine half it
  waits for.
- **The job:** draw Japanese or Chinese text on a cell console, where
  each of those characters is twice as wide as a Latin letter.
- **Friction:** no friction review: a missing feature, found when the
  game's ticket was split between the two repositories.
- **The generic shape:** a font atlas holds a second, wide font next to
  the narrow one (16 × 16 glyphs beside 8 × 16 ones). A wide character
  takes two cells of the `GlyphBuffer`: the first holds the character,
  the second a continuation mark, and the renderers draw one glyph over
  both. Every place that measures text for layout counts it as two.
- **Outside tactics:** a roguelike or a visual novel in Japanese on a
  cell console; any game with CJK text that draws through the glyph
  buffer.

Today a cell holds one `char` and one atlas cell (`shuyi_assets::font::
FontAtlasDef`: `cell_w`, `cell_h`, `columns`, `glyphs`). `shuyi_assets::
lang::text_width` counts one cell per character; its doc already says it
is the one place that will learn wide glyphs. `shuyi-cli font-atlas`
turns BDF fonts into `atlas.png`, `atlas.ron` and the one-bit copy
`atlas_1bit.bin` (8 pixels wide at most per cell). After 2350 it also
keeps every narrow character the game's text uses (`AtlasChars`, read
through `GameTools::lang`), and `shuyi check` fails on text the atlas
can't draw (`shuyi_assets::font::undrawn`).

## Nick input

None.

## Scope

**In:**
- The atlas format: wide glyphs and their cells.
- `font-atlas --wide <font.bdf>`. **Either order with 2224:** if 2224
  is done, `--wide` also takes a TTF or OTF, through its
  `font_atlas::read_any` with `--px` and the wide cell as its `--cell`
  (and `one_bit` still takes cells up to 8 pixels wide, so each half is
  one); if not, nothing more here, and 2224 makes `--wide` read through
  `read_any`.
- `GlyphBuffer` printing, overwriting and blitting with wide characters.
- `text_width` from the atlas's widths, and the engine's own layout
  code that counts characters (centring, right-aligning, clipping).
- The renderers: `shuyi-app`'s (`crates/app/src/render.rs`), the
  one-bit renderer (`crates/ui/src/frame/onebit.rs`) and `frame-png`.
- The text form of snapshots (the Harness): a continuation cell prints
  as nothing, so a row with wide text reads naturally.
- A small BDF fixture of our own with a few 16 × 16 glyphs, for the
  tests.
- What text the cells draw, said in `docs/formats.md` (step 8).

**Out (do not do):**
- Line breaking for text without spaces (2302).
- More than one wide font, a character drawn differently by language,
  and punctuation whose width depends on the language (2303).
- Any font a game ships; the game picks and licenses its own.
- Changing any narrow glyph or the narrow atlas cells.
- The engine's own screens in a language with wide glyphs.
  **Either order with 2352** (parked: the engine's screens in other
  languages, machine-translated): if 2352 is done, nothing to do (an
  engine pack in such a language is a ticket of its own, if Nick
  asks); if not, nothing to do (Nick may then pick one for 2352, as
  its clause says).
- Combining marks (a decomposed `é`, Thai and Devanagari vowel signs),
  right-to-left text and shaped scripts: 2908 (parked research), which
  may change what a cell holds.

**For the game:** the API changes (`text_width` takes the widths; see
step 4). The game's PR that moves its pin follows this one and adds its
font (game#0236).

## Implementation steps

1. **The atlas.** `FontAtlasDef` gains the wide glyphs: a map from
   `char` to the cell index of its left half, the right half being the
   next cell in the same atlas row. Validation: both cells of a wide
   glyph fit the PNG and the same row; no character is both narrow and
   wide. `glyph_rect` stays narrow; add `wide_rect(char)` for the
   16 × 16 rectangle. An atlas with no wide glyphs reads as today.
   `FontAtlasDef::draws` and `drawn` (added by 2350) take the wide
   glyphs too, so 2350's `font::undrawn`,
   2346's `Ctx::font_chars` and 2353's `can_draw` count a character
   with a wide glyph as drawn. Nothing else reads the glyph maps to
   answer it.
2. **The tool.** `font-atlas --wide <font.bdf>`: the wide characters
   taken are exactly the characters of the game's text that the narrow
   fonts lack (2350's `AtlasChars::used`, which `font-atlas` already
   reads through `GameTools::lang`), plus a set the project file names
   (a new `font_atlas.wide_glyphs` string, empty by default), so name
   entry and tests don't depend on a translation. They are laid out after the
   narrow glyphs, two cells each, never split across a row end. The
   one-bit copy stores each half as an ordinary 8-pixel cell. If a BDF
   uses a JIS encoding, convert its code points to Unicode in the tool.
3. **The buffer.** `GlyphBuffer` learns which characters are wide from
   a width table set where the buffer is made (`GlyphBuffer::new` and
   `with_spec` keep "nothing is wide", so existing tests don't change;
   add a way to give the table, e.g. `with_widths`). `print` and the
   functions built on it advance two cells for a wide character: the
   first cell holds it, the second a reserved continuation `char`
   (a constant, never drawn) with the same colors. Test for it only
   through one function (e.g. `Cell::is_continuation()`), never by
   comparing `glyph` with the constant at each use, so that 2908
   (parked) can change what a cell holds in one place. Overwriting either
   half clears the other to a space. A wide character that would start
   in the last column isn't drawn. `blit` keeps pairs whole.
4. **Widths.** One width table type (e.g. `CellWidths`, built from a
   `FontAtlasDef`, with `CellWidths::NARROW` for "nothing is wide").
   `text_width(text, &CellWidths) -> usize` counts a wide character as
   two. Every caller in the engine passes the table it has. Replace
   `chars().count()` used for layout in the engine's screens and
   widgets with it. **Either order with 2347:** if 2347 is done,
   `StyledText::width` and `StyledText::cut` (`crates/ui/src/text.rs`)
   take the `CellWidths` too, and `GlyphBuffer::print_styled` advances
   two cells for a wide glyph, as `print` does (it goes through the same
   `print_with`); if not, 2347 does it. **Either order with 2346 and
   2353:** if 2346 is done, `NameRules` counts a name's length with the
   table; if 2353 is done, `pseudo_text` and `with_pseudo` measure with
   it and its walk skips continuation cells. If not, each does it (its
   clause says so). **Either order with 2239:** if 2239 is done,
   `dialogue::box_fit` (`crates/assets/src/dialogue/check.rs`) measures
   with `longest_width` and `lines_at_longest`: give it the `CellWidths`
   too, and its two callers (`Checker::step`, and `add_lines` in
   `crates/assets/src/lang/lines.rs`) pass the table; if not, nothing to
   do (2239 passes it, as its clause says).
5. **Renderers.** `shuyi-app` draws a wide glyph as one 16 × 16 quad
   over both cells and skips continuation cells. The one-bit renderer
   draws its two halves. `frame-png` draws the same as `shuyi-app`.
   The software painter is
   `crates/ui/src/frame/raster.rs` (2332). Add a line of wide and
   narrow glyphs to the render card (`shuyi_ui::frame::raster::card`,
   2392), so 2018's `render` CI job compares the window with the
   raster.
6. **Snapshots.** The Harness's text snapshot prints a continuation
   cell as nothing; say so in the Harness's docs.
7. A `frame-png` reference PNG of a line with wide and narrow glyphs,
   drawn with the fixture font, committed like the existing references.
8. Say in `docs/formats.md`, at the font atlas and at the language
   pack, what text the cells draw: left to right, one character to a
   cell (two for a wide one), with no combining marks, no joining and
   no reordering. Right-to-left and shaped scripts (Arabic, Hebrew,
   Thai, Devanagari) are 2908's (parked research).

## Acceptance criteria

- [x] Printing `"あA"` fills three cells: wide, continuation, narrow
      (unit test).
- [x] Overwriting either half of a wide glyph leaves no orphan half
      (unit test).
- [x] A frame PNG with wide text matches a committed reference (test).
- [x] `font-atlas --wide` takes exactly the wide characters the text
      uses plus the project's set, and 2350's check (`font::undrawn`)
      counts a wide glyph as drawn and fails `shuyi check` when a pack
      gains a wide character the atlas lacks (test).
- [x] An atlas without wide glyphs, every existing snapshot and every
      narrow atlas cell are unchanged (but the render card's snapshot,
      which gains step 5's line of wide glyphs).
- [x] The fixture font is our own (no license row needed) or its license
      is listed.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: buffer printing, overwriting and blitting; `text_width`; atlas
  validation; the tool's choice of characters.
- Property: printing any mix of narrow and wide characters never leaves
  a continuation cell without its first half.
- Snapshot / integration: the frame PNG; the one-bit render of a wide
  glyph (a test of a look, reading pixels).

## Completion notes

**What was done.**
- **The atlas** (`shuyi_assets::font`): `FontAtlasDef::wide`, each wide
  glyph's left cell (its right half the next, in the same row), read as
  empty from an atlas without it. `wide_rect`; `draws` and `drawn` take
  the wide glyphs too, so `font::undrawn` and `shuyi check` count one as
  drawn. `problems` names a character both narrow and wide, a wide glyph
  in a row's last cell and a cell two glyphs share; `image_problems` a
  right half past the picture.
- **Widths:** `font::CellWidths` (`NARROW`, `of(&atlas)`, `wide(chars)`,
  `is_wide`, `of_char`), cheap to clone (the set is shared).
  `lang::text_width(text, &CellWidths)`.
- **The buffer** (`shuyi_ui::frame::glyph_buffer`): `with_widths`,
  `set_widths`, `widths`, `text_cells`; `new` and `with_spec` stay
  narrow. `print`, `print_fg` (and so everything built on `print_with`)
  write a wide character in two cells, the second a continuation in the
  same colours (`Cell::is_continuation`, the one test of it; the
  reserved `char` is private). Replacing either half (`set`, `print`,
  `fill_rect`, `draw_box`, `blit`) makes the other a space, keeping its
  colours; `dim` and `blend_bg` leave pairs alone. A wide character the
  buffer's edge would cut isn't drawn; `blit` copies pairs whole or not
  at all. `glyphs()` gives the renderers each glyph and whether it is
  drawn over two cells.
- **Where the widths come from:** `Game::set_widths` gives the frame and
  the context (`Ctx::widths`) the atlas's widths and draws again;
  `shuyi_platform::App` calls it after the game loads, with its look's
  atlas, on every backend.
- **Layout:** `frame::draw::cut` takes the widths; `print_centred`,
  `print_cut`, `panel`, `screens::print_centred`, the key bindings,
  options, profiles and debug screens, the text box, the ask, menu and
  name-entry widgets and the crank hint measure with the frame's widths
  (the menu's `size` with `Ctx::widths`, as it has no frame).
- **Renderers:** `shuyi-app` and the raster draw a wide glyph as one
  quad over both cells and skip the continuation; a wide glyph without
  its continuation draws the fallback. The one-bit renderer and its
  reference draw a half in each cell (`Font::cell_rows`), and a partial
  redraw (`render_changed`) redraws a pair's other half; the speed
  tests' property tests now print wide glyphs too, and the fast
  renderer still equals the reference. The render card has a line of
  wide and narrow glyphs and one wide glyph without its continuation.
- **Snapshots:** a continuation prints as nothing in the glyph rows; the
  colour rows keep a key a cell (`frame::snapshot` and
  `Harness::snapshot` say so).
- **The tool:** `font-atlas --wide <font.bdf>` and `shuyi.ron`'s
  `font_atlas.wide_glyphs`. The wide glyphs are the characters the text
  uses that the narrow fonts lack, and the project's, laid out after the
  narrow ones, two cells each, never across a row's end
  (`build_wide_atlas`, `AtlasChars::with_wide`, `wants_wide`,
  `check_wide` for a font twice as wide). `atlas.ron` leaves `wide` out
  when there are none, so the template's and the examples' atlases are
  unchanged byte for byte. The one-bit copy keeps each half as a cell.
  A BDF in JIS X 0208 (`CHARSET_REGISTRY "JISX0208…"`) is read as
  Unicode through `encoding_rs`' EUC-JP table, with `shuyi-cli`'s `jis`
  feature; without it such a font is refused, saying to turn it on.
- **Fixtures:** `crates/cli/tests/fixtures/fonts/wide-16.bdf` and
  `wide-16-jis.bdf` (four CJK characters made of straight lines, drawn
  for this ticket: MIT OR Apache-2.0, no license row) and the reference
  `wide-line.png`, drawn by the raster with the template's Terminus and
  the fixture. When it changes on purpose its test writes
  `wide-line.new.png` beside it (ignored by git) to look at.
- **Docs:** `docs/formats.md` (the font atlas and the language pack:
  what the cells draw), the book's *Assets and formats*, the
  `shuyi-import-art` skill's row, the rustdoc.

**Deviations.**
- Step 4's "every caller in the engine passes the table it has": the
  text checks (`longest_width`, `lines_at_longest`, `Names::longest`,
  the tips' check) run as the text loads, before anything has the
  atlas, so they pass `CellWidths::NARROW`. Ticket **2252** gives them
  the atlas's widths through `PackRules` and `TipRules`, and makes a hot
  reload of the atlas change the frame's widths (today it keeps those it
  had at load). 2239's clause now points at 2252.
- `Ctx` gets the widths too (`Ctx::widths`, set only by
  `Game::set_widths`): the menu's size is measured from the context,
  before it has a frame.
- `dialogue/text_box.rs` counts characters for the typewriter's reveal,
  not for layout, and `profiles.rs` and `name_entry.rs` count a name's
  stored length (2346 measures it); they still count characters.
- The JIS conversion needs JIS X 0208's table: `encoding_rs` 0.8
  (`(Apache-2.0 OR MIT) AND BSD-3-Clause`, all in `deny.toml`'s allow
  list; `default-features = false, features = ["alloc"]`) rather than a
  table of 7,000 rows in the repository. It brings five crates, so it is
  optional, behind `shuyi-cli`'s `jis` feature (on for this crate's
  tests): a new game's tree stays at 144 crates, and only a game whose
  font is in JIS turns it on.
- The one-bit renderer's partial redraw draws a continuation again when
  its first cell changes; the other way round can't happen (replacing a
  continuation makes its first cell a space), so it has no rule.
- The pack's binary form of a font atlas has the new field:
  `pack::VERSION` is 8.
- Either-order clauses: 2224, 2239, 2346, 2347, 2353 were not done, so
  each took its other branch; their clauses now say what holds.

**Follow-ups.** 2252 (above).

**For the game.** game#0236 adds its Japanese font with `--wide` at the
pin bump that follows this PR; that game PR isn't open yet. The bump's
changes are below: none if the game builds no `FontAtlasDef`, calls no
`text_width` and no `cut`.

Breaking: shuyi-assets: `font::FontAtlasDef`: new field `wide`; a game that builds one adds `wide: BTreeMap::new()`.

```migration
since: 0.1.0
what: A font atlas may hold wide glyphs; one built in code says it has none.
old: FontAtlasDef { cell_w, cell_h, columns, glyphs }
new: FontAtlasDef { cell_w, cell_h, columns, glyphs, wide }
before: |
  let atlas = FontAtlasDef { cell_w: 8, cell_h: 16, columns: 32, glyphs };
after: |
  let atlas = FontAtlasDef { cell_w: 8, cell_h: 16, columns: 32, glyphs, wide: BTreeMap::new() };
```

Breaking: shuyi-assets: `lang::text_width`: takes the widths; pass the frame's (`buf.widths()`), the context's (`ctx.widths()`) or `&CellWidths::NARROW`.

```migration
since: 0.1.0
what: text_width counts a wide character as two cells, so it takes the widths it measures with.
old: lang::text_width(&str)
new: lang::text_width(&str, &CellWidths)
before: |
  let w = text_width(label);
after: |
  let w = text_width(label, buf.widths());
find: ["text_width(label)"]
```

Breaking: shuyi-ui: `frame::draw::cut` (and `screens::cut`): takes the widths; pass `buf.widths()`.

```migration
since: 0.1.0
what: cut measures text in cells, a wide character two.
old: cut(&str, usize)
new: cut(&str, usize, &CellWidths)
before: |
  let shown = cut(name, 12);
after: |
  let shown = cut(name, 12, buf.widths());
find: ["cut(name, 12)"]
```

Breaking: shuyi-assets: `pack::VERSION` and `formats.ron` "font atlas: font-atlas": a font atlas's binary form has `wide`, and a baked pack is of version 8; every game bakes again.

```migration
since: 0.1.0
what: A pack baked before this engine is refused; bake it again.
old: pack::VERSION 7
new: pack::VERSION 8
before: |
  cargo shuyi bake
after: |
  cargo shuyi bake
```

Breaking: shuyi-cli: `font_atlas::run` takes the wide font's path after the fonts' (`None` for none), and `project::FontAtlas` has `wide_glyphs`.

```migration
since: 0.1.0
what: font-atlas may read a wide font; a caller without one passes None.
old: font_atlas::run(&rules, command, &fonts, out_dir, &chars)
new: font_atlas::run(&rules, command, &fonts, None, out_dir, &chars)
before: |
  font_atlas::run(&rules, "shuyi", &fonts, out, &chars)
after: |
  font_atlas::run(&rules, "shuyi", &fonts, None, out, &chars)
find: ["&fonts, out, &chars"]
```

Book: updated (`assets-and-formats`, after *When a file is wrong*: wide
glyphs, `--wide`, `wide_glyphs` and measuring with the frame's widths).
