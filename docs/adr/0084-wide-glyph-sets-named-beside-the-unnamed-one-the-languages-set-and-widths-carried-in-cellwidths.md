# ADR-0084: Wide glyph sets: named sets beside the unnamed one, and the language's set and widths carried in `CellWidths`

- **Status:** Accepted
- **Date:** 2026-11-05
- **Related tickets:** 2303 (extends 2301's wide glyphs and 2302's line breaking)

## Context

Ticket 2301 gave the font atlas one wide font: `FontAtlasDef::wide`, a map
from `char` to the left cell of a glyph two cells wide, and
`CellWidths`, the set of characters that take two cells, handed by the
platform to `Game::set_widths` and from there to `Ctx` and the
`GlyphBuffer`. Every renderer (`shuyi-app`, the raster that `frame-png`
uses, the one-bit renderer and its reference copy) looks a wide glyph up
in the atlas when the buffer says the cell has a continuation.

A game in Japanese and Simplified Chinese needs more (ticket 2303):

- Unicode gives the two languages one code point for a shared character
  (直, 骨, 角), and each reader expects their own shape. One atlas has to
  hold two glyphs for one `char`.
- `“ ” ‘ ’ … —` are narrow in English and wide in Chinese with the same
  code points. 2301 made "narrow or wide, never both" a rule of the
  atlas, and widths were the atlas's alone, not the language's.
- Which language is on is `Ctx::lang`, a public field any screen may
  change; the renderers see only the `GlyphBuffer` and the atlas.

Games already have atlases from 2301 and packs that predate the change.

## Decision

1. **The atlas keeps its unnamed set and gains named ones.**
   `FontAtlasDef::wide` stays as it is (the unnamed set). A new field,
   `wide_sets: BTreeMap<String, WideSet>`, holds named sets, each laid
   out as `wide` is (`WideSet = BTreeMap<char, u32>`). Both are
   `#[serde(default)]`, and `font-atlas` leaves an empty one out of
   `atlas.ron`, so an atlas of 2301 reads and is written unchanged. A
   set's name is the atlas's own word, not a language tag. Validation
   runs over every set: cells overlap nowhere, a glyph never straddles a
   row's end, and a character is narrow or wide in a set, never both,
   except `font::WIDE_PUNCTUATION`.
2. **A pack names its set.** `LangInfo` (`lang.ron`) gains
   `wide_font: Option<String>` and `wide_punctuation: bool`, both
   `#[serde(default)]`. `LangPackRon::widths_of(atlas)` refuses a set the
   atlas lacks; `font::undrawn` checks a pack's wide characters against
   its own set (any set when it names none) and reports a missing set
   once, on the pack's `lang.ron`.
3. **One look-up, in `shuyi-assets`.** `FontAtlasDef::wide_cell(set, c)`
   looks in the named set `set` first, then the unnamed set, then the
   other named sets in name order. `wide_rect_in` wraps it; `wide_rect(c)`
   is `wide_rect_in(None, c)`. Every renderer calls it. It lives on the
   atlas type rather than in `shuyi-ui` (where ticket 2303 put it)
   because the tool, the check and the renderers all hold a
   `FontAtlasDef`, and the one-bit renderer runs on the Playdate without
   the rest of the UI's state.
4. **`CellWidths` is a language's table.** Beside its set of wide
   characters it carries `wide_punctuation` and the language's set name
   (`Rc<str>`). `CellWidths::of(atlas)` is every set's characters but the
   six marks the atlas also has narrow; `in_lang(set, wide_punctuation)`
   makes a language's table, and `LangInfo::widths(base)` applies a
   pack's `lang.ron`. A renderer reads the set from
   `buf.widths().wide_font()`, so nothing new crosses the platform
   boundary.
5. **The context keeps one table per pack.** `Ctx` keeps the atlas's
   table and a map of each pack's (`lang_widths`), rebuilt by
   `Game::set_widths`; `Ctx::widths()` returns the table of
   `effective_lang()`. `Game::redraw`, which every frame calls, hands the
   buffer that table first, so a language picked in a frame is drawn in
   its own shapes from that frame's drawing.
6. **A pack's checks use its own table.** `pack_from_files` measures a
   pack's limits with `info.widths(rules.widths)`, and `Lang::overflow`
   likewise.
7. **Wrapping.** `text::word_wrap` adds Chinese's punctuation to its
   no-start and no-end sets (`，` `》` `〉` and `《` `〈`), and keeps
   `” ’ … —` from starting a line and `“ ‘` from ending one only where
   the table has them wide, which also keeps `……` and `——` whole.
   English wrapping is unchanged.

The baked pack's binary form changes (`pack::VERSION` 9).

## Consequences

- A 2301 atlas and every existing pack keep working; a game's pin bump
  needs a re-bake but no new atlas until it adds a second wide font.
- One character can draw two ways; the language row of a languages menu
  shows each pack's name in its own shapes whatever language is on.
- `CellWidths` now says more than widths: it is the language's view of
  the font. Code that compared two tables compares the set name too.
- The source language has no `lang.ron`, so it has no set of its own and
  narrow punctuation. A game written in Chinese would need
  `SourceLang` to carry both (not done here).
- A struct literal of `FontAtlasDef` or `LangInfo` must add the new
  fields (a breaking change, written in 2303's notes).

## Alternatives considered

- **`wide` becomes a map by set name, the unnamed set under `""`** —
  every 2301 atlas would fail to parse, or need an untagged reader; and
  an empty name reads as a mistake.
- **Key the sets by language tag** — a set is a font, not a language:
  `zh-Hans` and `zh-SG` share one, and a pack chooses by name. The
  ticket asked for the atlas's own word.
- **Pass the language's set to each renderer beside the buffer** — it
  would change `Platform::present`, the one-bit `render` and the
  raster's `render` at every caller. The buffer already carries its
  widths to every renderer.
- **Make `Ctx::widths()` compute the table on each call** — it returns a
  reference today, used everywhere; a value would break each caller.
  The tables are few and rebuilt only when the atlas or the packs
  change.
