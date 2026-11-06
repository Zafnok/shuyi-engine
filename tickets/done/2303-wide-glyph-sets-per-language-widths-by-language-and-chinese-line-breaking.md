---
id: "2303"
title: "Wide glyph sets per language: one character drawn by language, punctuation wide in one language only, and Chinese line breaking"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2301", "2302", "2226"]
nick_input: none
completed: 2026-11-05
---

# 2303 — Wide glyph sets per language, widths by language, Chinese line breaking

## Context

- **The game ticket:** game#0239
  ([`0239-chinese-font-a-wide-font-per-language-and-chinese-line-breaking.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0239-chinese-font-a-wide-font-per-language-and-chinese-line-breaking.md)).
  It adds a Simplified Chinese font and pack to the game; this ticket is
  the engine half it waits for.
- **The job:** show Japanese and Simplified Chinese in one game, each
  with its own shapes, and wrap Chinese the way a Chinese reader
  expects.
- **Friction:** no friction review: a missing feature, found when the
  game's ticket was split between the two repositories.
- **The generic shape:** after 2301 an atlas has one wide font, keyed by
  `char`. Two languages need more:
  - **The same character is drawn differently in the two languages.**
    Unicode gives Japanese and Chinese one code point for a shared
    character (直, 骨, 角, 海…), and each language expects its own
    shape; a Japanese font also lacks most Simplified-only characters.
    So the atlas holds a **set** of wide glyphs per font, and a language
    pack says which set it uses.
  - **Some punctuation is narrow in English and wide in Chinese** with
    the same code point: `“ ” ‘ ’ … —`. Chinese text uses `“”` for
    speech and the doubled `……` and `——`.
  - **Chinese has its own no-break punctuation**, and never splits
    `……` or `——`.
- **Outside tactics:** any game translated into both Japanese and
  Chinese: a visual novel, a farming game's shop and letters.

## Nick input

None.

## Scope

**In:**
- `font-atlas`: named wide sets, given more than once.
- The atlas format: wide glyphs per set.
- A pack's `lang.ron` (`shuyi_assets::lang::LangInfo`): which wide set it
  uses, and whether its punctuation is wide.
- 2350's check of text against the atlas (`font::undrawn`): a pack's
  wide characters against the set it names.
- Looking a wide glyph up by language, in every renderer.
- Widths by language: the width table of 2301 per language.
- The wrapping rules for Chinese in `word_wrap` (2302).

**Out (do not do):**
- Traditional Chinese; typing Chinese (2346 turns a Windows IME on in
  a name box; its composing text drawn in the box is parked 3034's
  research).
- Any game's fonts, packs or text.
- Changing any narrow glyph, or the glyphs of a single wide set.
- The engine's own screens in Japanese or Chinese. **Either order
  with 2352** (parked: the engine's screens in other languages,
  machine-translated): if 2352 is done, nothing to do (an engine pack
  in such a language is a ticket of its own, if Nick asks); if not,
  nothing to do (Nick may then pick one for 2352, as its clause says).

**For the game:** an atlas with one unnamed set (2301's) keeps working,
or the game's PR that moves its pin regenerates it (say which). That
PR follows this one (game#0239).

## Implementation steps

1. **The tool.** `--wide` takes a named set and may be given more than
   once (`--wide ja=<font.bdf> --wide zh-Hans=<font.bdf>`). A set's
   name is the atlas's own word, not a language tag; a pack names its
   set in `lang.ron` (step 3). The Simplified Chinese pack's folder is
   the tag `zh-Hans` (2226). A set's glyphs are the wide characters used
   by the packs that name it (step 3; from 2350's `Lang::texts`, by
   pack), plus that set's fixed minimum from the project file (one
   string per set name). Sets are laid out one after another after the
   narrow glyphs.
2. **The atlas.** `FontAtlasDef`'s wide glyphs become a map per set
   name. Validation: a set's cells fit the PNG; a character may be in
   several sets. `FontAtlasDef::draws` and `drawn` (2350's)
   take every set, so a character in any set counts as drawn where no
   language is named (2346's `Ctx::font_chars`); step 3's check reads
   a pack's own set.
3. **The pack.** `lang.ron` gains `wide_font: Option<String>` (a set
   name) and `wide_punctuation: bool` (default `false`). Validation: a
   pack's `wide_font` names a set the atlas has. 2350's check
   (`shuyi_assets::font::undrawn`) learns sets: a pack's wide characters
   must be in the set it names, so `shuyi check` fails when one is
   missing. It covers every set.
4. **Look-up.** One function in `shuyi-ui`: the current language's set
   first, then any other set that has the character (the language row
   of 2349 shows `日本語` and `简体中文` at the same time, whatever the
   language is). `shuyi-app`'s renderer, the one-bit renderer and
   `frame-png` all use it.
5. **Widths by language.** With `wide_punctuation`, `“ ” ‘ ’ … —` are
   wide and drawn from the pack's set; otherwise they are narrow as
   today. This relaxes 2301's "no character is both narrow and wide" to
   "except these six, by the language's flag". The buffer's width table
   and `text_width` take the table of the current language; a pack's
   length checks use that pack's table.
6. **Wrapping.** Add to the never-start-a-line set
   `，。、；：？！）》〉」』”’…—` and to the never-end-a-line set
   `（《〈「『“‘`; never break between the two halves of `……` or `——`.
   The quote and dash rules apply only where they are wide (step 5), so
   English wrapping is unchanged.
7. Document the sets, the look-up order and the six widths in the
   atlas's and the pack format's docs.

## Acceptance criteria

- [x] The same character (直) draws from one set with a pack that names
      it and from another set with a pack that names the other (a frame
      PNG test against two committed references, with fixture fonts).
- [x] With `wide_punctuation`, `“好”` is six cells wide and `……` is
      four; without it, `“ok”` is four cells (unit tests).
- [x] A Chinese paragraph wraps within the width with no line starting
      with `，` or `。`, none ending with `“`, and `……` never split (unit
      tests with real sentences).
- [x] A pack that uses a wide character its set lacks fails 2350's
      check in `shuyi check` (test).
- [x] Every existing snapshot, the narrow atlas cells and an atlas with
      one wide set are unchanged.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: set look-up and fall-back; widths by language; each new breaking
  rule; atlas and pack validation; 2350's check with two sets.
- Property: 2302's wrap properties (width, round trip) hold with
  `wide_punctuation` on.
- Snapshot / integration: the frame pictures; the look tests read cells, as
  a test of a look may.

## Completion notes

**What was done.**

- **The atlas** (`shuyi_assets::font`): `FontAtlasDef::wide` stays the
  unnamed set; `wide_sets` holds the named ones (`WideSet`), both left
  out of `atlas.ron` when empty. Validation covers every set (shared
  cells, a row's end, an empty name); a character may be in several
  sets, and `font::WIDE_PUNCTUATION` (`“ ” ‘ ’ … —`) may be narrow and
  wide at once. `draws`, `drawn` and the new `wide_chars`, `has_wide`
  and `cell_end` take every set.
- **The look-up**: `FontAtlasDef::wide_cell(set, c)` (and
  `wide_rect_in`): the language's set, the unnamed set, then the named
  ones by name. `shuyi-app`'s renderer, the raster (`frame-png`), the
  one-bit renderer and its reference all call it with the buffer's
  `widths().wide_font()`.
- **Widths by language**: `CellWidths` carries the language's set name
  and whether its punctuation is wide (`in_lang`, `wide_font`,
  `wide_punctuation`); `LangInfo::widths(base)` makes a pack's.
  `Ctx` keeps a table per pack and `Ctx::widths()` returns the effective
  language's; `Game::redraw` hands it to the buffer first.
  `pack_from_files` and `Lang::overflow` measure a pack with its own.
- **The pack**: `lang.ron` gains `wide_font` and `wide_punctuation`
  (both optional). An empty `wide_font` is refused; with the atlas,
  `LangPackRon::widths_of` refuses a set the atlas lacks.
  `font::undrawn` checks a pack's wide characters against its own set
  (any set when it names none), and a missing set once on its
  `lang.ron`.
- **The tool**: `font-atlas --wide [<set>=]<font.bdf>`, once a set
  (`font_atlas::WideFont`); a set keeps the wide characters of the
  packs naming it, their wide punctuation, and
  `shuyi.ron`'s `font_atlas.wide_set_glyphs` for it; sets are laid out
  one after another in the order given (`build_atlas_with_sets`).
- **Wrapping** (`text::word_wrap`): `，》〉` never start a line, `《〈`
  never end one; `” ’ … —` never start and `“ ‘` never end one where
  they are wide, which keeps `……` and `——` whole. English is
  unchanged.
- Fixture fonts of ours, `wide-16-ja.bdf` and `wide-16-zh.bdf` (`直` in
  two shapes, `简`, the wide punctuation), and two committed frame
  pictures, `wide-set-ja.png` and `wide-set-zh.png`.
- [ADR-0085](../../docs/adr/0085-wide-glyph-sets-named-beside-the-unnamed-one-the-languages-set-and-widths-carried-in-cellwidths.md);
  docs in `docs/formats.md`, the book's chapter 5, and the
  `shuyi-import-art` and `shuyi-screen` skills.

**Deviations.**

- The look-up lives on `FontAtlasDef` in `shuyi-assets`, not in
  `shuyi-ui` (step 4): the tool, the check and every renderer hold the
  atlas, and the language reaches the renderers in the buffer's
  `CellWidths` (ADR-0085 §3, §4).
- The unnamed set keeps the characters of the languages that name no
  set (all of them, for a game of one wide font, as before).
- A punctuation mark only wide in the atlas (2301's) stays wide in
  every language; the language's flag decides only the marks the atlas
  has both narrow and wide.

**For the game:** an atlas with one unnamed set (2301's) keeps working
as it is; nothing regenerates it. The pack format is version 9, so the
game bakes again. game#0239 moves the pin to this change and adds its
`zh-Hans` pack and fonts.

**Follow-up:** 2254 (a source language names its wide set and its
punctuation's width; the source has no `lang.ron`).

Breaking: shuyi-assets: `font::FontAtlasDef`: a struct literal adds `wide_sets`.

```migration
since: 0.1.0
what: A font atlas has named wide sets beside its unnamed one.
old: "FontAtlasDef { cell_w, cell_h, columns, glyphs, wide }"
new: "FontAtlasDef { cell_w, cell_h, columns, glyphs, wide, wide_sets }"
before: |
  FontAtlasDef { cell_w: 8, cell_h: 16, columns: 32, glyphs, wide: BTreeMap::new() }
after: |
  FontAtlasDef { cell_w: 8, cell_h: 16, columns: 32, glyphs, wide: BTreeMap::new(), wide_sets: BTreeMap::new() }
find: ["wide: BTreeMap::new() }"]
```

Breaking: shuyi-assets: `lang::LangInfo`: a struct literal adds `wide_font` and `wide_punctuation`.

```migration
since: 0.1.0
what: A pack's lang.ron may name its wide set and make its punctuation wide.
old: "LangInfo { name, made_by }"
new: "LangInfo { name, made_by, wide_font, wide_punctuation }"
before: |
  LangInfo { name, made_by: MadeBy::Machine }
after: |
  LangInfo { name, made_by: MadeBy::Machine, wide_font: None, wide_punctuation: false }
find: ["made_by: MadeBy::Machine }"]
```

Breaking: shuyi-assets: `pack::VERSION`, `formats.ron` "font atlas: font-atlas" and "language pack: lang-pack", and hello-grid's "asset font/atlas.ron": a font atlas's and a pack's info's binary forms have the new fields, and a baked pack is of version 9; every game bakes again.

```migration
since: 0.1.0
what: A pack baked before this engine is refused; bake it again.
old: pack::VERSION 8
new: pack::VERSION 9
before: |
  cargo shuyi bake
after: |
  cargo shuyi bake
```

Breaking: shuyi-cli: `font_atlas::run` takes the wide fonts as `&[WideFont]`, `AtlasChars::wants_wide` takes the set, and `project::FontAtlas` has `wide_set_glyphs`.

```migration
since: 0.1.0
what: font-atlas takes several wide fonts, each a set.
old: "font_atlas::run(rules, command, fonts, Option<&Path>, out, chars)"
new: "font_atlas::run(rules, command, fonts, &[WideFont], out, chars)"
before: |
  font_atlas::run(rules, "shuyi", &fonts, Some(&wide), &out, &chars)
after: |
  font_atlas::run(rules, "shuyi", &fonts, &[WideFont::parse(wide)], &out, &chars)
find: ["Some(&wide), &out"]
```
