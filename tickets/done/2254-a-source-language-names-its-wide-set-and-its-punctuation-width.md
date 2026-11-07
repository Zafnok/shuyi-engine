---
id: "2254"
title: "A source language names its wide set and its punctuation's width, as a pack's lang.ron does"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 2254 — A source language names its wide set and its punctuation's width

## Context

- **From 2303 (done):** a language pack's `lang.ron` names the font
  atlas's wide set it draws from (`LangInfo::wide_font`) and whether
  `“ ” ‘ ’ … —` are wide in it (`LangInfo::wide_punctuation`).
  `LangInfo::widths` makes the language's width table from the atlas's
  (`CellWidths::in_lang`), which `Ctx::widths`, the glyph buffer, the
  pack's limits and `font::undrawn` all use
  ([ADR-0086](../../docs/adr/0086-wide-glyph-sets-named-beside-the-unnamed-one-the-languages-set-and-widths-carried-in-cellwidths.md)).
- **The gap:** the source language has no `lang.ron`. A game declares it
  in code (`SourceLang::new(code, name)`, `Lang::with_source_lang`), so
  a game written in Chinese, or in Japanese beside a Chinese pack, has
  no way to say its set or its wide punctuation: its text draws from the
  unnamed set (or any), its quotes narrow, and `undrawn` checks it
  against any set.
- **Outside tactics:** a visual novel written in Chinese and translated
  into Japanese.

## Nick input

None.

## Scope

**In:**
- `shuyi_assets::lang::SourceLang` (`#[non_exhaustive]`, ADR-0039):
  two fields, `wide_font: Option<String>` and `wide_punctuation: bool`,
  with builder methods; additive.
- The source's table wherever a pack's is made: `Ctx`'s tables
  (`lang_widths` in `crates/ui/src/ctx.rs`), `font::undrawn`,
  `font-atlas`'s `AtlasChars::of_lang`, and the source's own limits.

**Out (do not do):**
- A `lang.ron` for the source's folder: the source stays declared in
  code.
- Changing a pack's `lang.ron`.

## Implementation steps

1. Add `SourceLang::wide_font` and `wide_punctuation`, and
   `SourceLang::with_wide_font(set)` / `with_wide_punctuation(on)`.
   Add `SourceLang::widths(&self, base: &CellWidths) -> CellWidths`, as
   `LangInfo::widths` does.
2. One helper, `Lang::widths(&self, code: &LangCode, base) ->
   CellWidths`: a pack's `info.widths(base)`, the source's
   `source_lang().widths(base)`, else `base`. Use it in
   `crates/ui/src/ctx.rs` (`lang_widths` holds the source's too),
   `font::undrawn` (the `None` language is the source) and
   `font_atlas::AtlasChars::of_lang` (the source's set and punctuation).
3. Where the source's own text is checked against limits with the
   atlas's widths, measure with the source's table.
4. Document the fields in `docs/formats.md` (the source language's
   screen text) and the book's chapter 5, beside the pack's.

## Acceptance criteria

- [x] A game whose source names set `zh-Hans` with wide punctuation:
      `Ctx::widths()` in the source has `“` wide and names `zh-Hans`
      (unit test).
- [x] `font::undrawn` fails the source's wide character its set lacks
      (unit test).
- [x] `font-atlas` keeps the source's wide characters in its set
      (unit test).
- [x] A source with neither field behaves as before (every existing test
      unchanged).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the source's table; `undrawn` with a source set; the tool's
  per-set characters with a source set.
- Snapshot / integration: none new.

## Completion notes

- `SourceLang` has `wide_font` and `wide_punctuation`, set with
  `with_wide_font` / `with_wide_punctuation` (neither by default), and
  `SourceLang::widths(base)` as `LangInfo::widths`.
- `Lang::widths(code, base)`: a pack's table, the source's, else `base`.
  `Ctx`'s `lang_widths` now holds the source's table too, so
  `Ctx::widths()` and the glyph buffer follow it in the source.
  `font::undrawn` checks the source's text against its set (a set the
  atlas lacks is one error on the source's folder, as `TextAt::path`
  names it), and `font-atlas`'s `AtlasChars::of_lang` keeps the source's
  characters and wide punctuation in its set. Both read the set and the
  punctuation through the helper's `CellWidths`.
- Tests: `Lang::widths` and the builders (`shuyi-assets`), `undrawn`
  with a source set, `Ctx::widths()` in a source naming `zh-Hans` (via a
  test-only `Toy::with_source_lang`), `font-atlas`'s per-set characters,
  and the book's snippet.
- Deviation, step 3: no engine code measures the source's own text
  against limits with the atlas's widths; a game does, through the
  widths it hands its tip and dialogue rules. The book and
  `docs/formats.md` say to hand them `source.widths(&CellWidths::of(&atlas))`.
  `Lang::overflow` stays a pack's, as before.
- Docs: `docs/formats.md` (the pack's `lang.ron` paragraph) and the
  book's chapter 5, with a compiled snippet (`site/snippets/src/assets.rs`,
  anchor `source_lang`).
- No break: `SourceLang` is `#[non_exhaustive]`; the fields and methods
  are additions.
