---
id: "2301"
title: "Wide glyphs: a character two cells wide in the glyph buffer, drawn from a wide font in the atlas"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2350"]
nick_input: none
completed:
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
   2350's `font::undrawn` counts a character with a wide glyph as
   drawn.
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
   clause says so).
5. **Renderers.** `shuyi-app` draws a wide glyph as one 16 × 16 quad
   over both cells and skips continuation cells. The one-bit renderer
   draws its two halves. `frame-png` draws the same as `shuyi-app`.
   The software painter is
   `crates/ui/src/frame/raster.rs` (2332). **Either order with 2018:** if 2018 is done, add a
   line of wide and narrow glyphs to its render card
   (`shuyi_ui::frame::raster::card`), so its `render` CI job compares
   the window with the raster; if not, compare a screenshot of the
   window with `frame-png` by hand, note it in the PR, and 2018 puts
   wide glyphs on the card.
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

- [ ] Printing `"あA"` fills three cells: wide, continuation, narrow
      (unit test).
- [ ] Overwriting either half of a wide glyph leaves no orphan half
      (unit test).
- [ ] A frame PNG with wide text matches a committed reference (test).
- [ ] `font-atlas --wide` takes exactly the wide characters the text
      uses plus the project's set, and 2350's check (`font::undrawn`)
      counts a wide glyph as drawn and fails `shuyi check` when a pack
      gains a wide character the atlas lacks (test).
- [ ] An atlas without wide glyphs, every existing snapshot and every
      narrow atlas cell are unchanged.
- [ ] The fixture font is our own (no license row needed) or its license
      is listed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: buffer printing, overwriting and blitting; `text_width`; atlas
  validation; the tool's choice of characters.
- Property: printing any mix of narrow and wide characters never leaves
  a continuation cell without its first half.
- Snapshot / integration: the frame PNG; the one-bit render of a wide
  glyph (a test of a look, reading pixels).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
