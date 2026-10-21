---
id: "2303"
title: "Wide glyph sets per language: one character drawn by language, punctuation wide in one language only, and Chinese line breaking"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2301", "2302", "2226"]
nick_input: none
completed:
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
   several sets.
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

- [ ] The same character (直) draws from one set with a pack that names
      it and from another set with a pack that names the other (a frame
      PNG test against two committed references, with fixture fonts).
- [ ] With `wide_punctuation`, `“好”` is six cells wide and `……` is
      four; without it, `“ok”` is four cells (unit tests).
- [ ] A Chinese paragraph wraps within the width with no line starting
      with `，` or `。`, none ending with `“`, and `……` never split (unit
      tests with real sentences).
- [ ] A pack that uses a wide character its set lacks fails 2350's
      check in `shuyi check` (test).
- [ ] Every existing snapshot, the narrow atlas cells and an atlas with
      one wide set are unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: set look-up and fall-back; widths by language; each new breaking
  rule; atlas and pack validation; 2350's check with two sets.
- Property: 2302's wrap properties (width, round trip) hold with
  `wide_punctuation` on.
- Snapshot / integration: the frame pictures; the look tests read cells, as
  a test of a look may.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
