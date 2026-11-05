---
id: "2302"
title: "Line breaking for text without spaces (Japanese), and a report of translated text too long for its place"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["2301"]
nick_input: none
completed: 2026-11-04
---

# 2302 — Line breaking for text without spaces, and text that must fit its place

## Context

- **The game ticket:** game#0237
  ([`0237-japanese-line-breaking-and-text-that-fits.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0237-japanese-line-breaking-and-text-that-fits.md)).
  The game wraps its own few remaining texts with the engine's wrap and
  sets the widths of its places; the breaking rules and the report are
  this ticket.
- **The job:** wrap a Japanese paragraph in a text box the way a reader
  of Japanese expects, and tell a translator which translated labels
  are too long for the place they are drawn in.
- **Friction:** no friction review: a missing feature, found when the
  game's ticket was split between the two repositories.
- **The generic shape:** `word_wrap` breaks at spaces today, and
  Japanese has none. Between two wide characters (2301) a line may break
  anywhere, except before closing punctuation and after an opening
  bracket (the usual *kinsoku* rules). Widths are cells, measured with
  `text_width`. A language pack's screen text can say how many cells
  its place has, and `lang-status` lists the entries of a pack that
  don't fit.
- **Outside tactics:** a visual novel or an adventure game translated
  into Japanese: the same text box, the same labels on fixed-width
  buttons.

Where wrapping happens in the engine today: `shuyi_assets::text::
word_wrap` (`crates/assets/src/text.rs`; 2209 moved it there, and
`shuyi_ui::widgets::word_wrap` re-exports it). It has two callers: the
dialogue text box's paging (`crates/ui/src/dialogue/text_box.rs`, so a
page is `box_lines` lines of `word_wrap`), and the dialogue check's
`lines_at_longest` (`crates/assets/src/dialogue/check.rs`), which
`check_scene` uses to report a line longer than the box. Length checks
on pack text use `shuyi_assets::lang::text_width` (dialogue limits,
tips, names).

## Nick input

None.

## Scope

**In:**
- The breaking rules in `word_wrap`, measuring in cells.
- The dialogue text box's paging gets them through `word_wrap` (a page
  is still `box_lines` lines).
- **From 2209 (done):** `word_wrap` has a second caller, the dialogue
  check's `lines_at_longest`, which gets the new rules through it, so
  the check and the box keep agreeing. It writes each token as that
  many `W`s: under the new rules that is a Latin run kept whole, the
  worst case (a typed Latin name of that length), so keep it. If
  `word_wrap` takes 2301's width table, pass it on. A unit test:
  `lines_at_longest` on a Japanese line with a name token.
- **From 2252 (done):** `lines_at_longest` takes a `&CellWidths`, and
  until `word_wrap` measures in cells it writes each wide character as
  `WW` before wrapping, so the count of cells is right but a break
  between two wide characters can't happen. When `word_wrap` takes the
  width table, pass `widths` to it and drop the `WW`s, so the check
  and the box break at the same places.
- **Either order with 2239:** if 2239 is done, the language-pack check
  (`add_lines`, `crates/assets/src/lang/lines.rs`) counts a translated
  speech line's pages through `dialogue::box_fit` and
  `lines_at_longest`, so it gets the new rules too: add a unit test of a
  pack's Japanese speech line within `max_text` that wraps past
  `box_lines`, and if `word_wrap` takes 2301's width table, pass it
  through `box_fit`; if not, nothing to do (2239 adds the pack's page
  check and that test).
- An optional width per screen-text key in the source language's
  `ui.ron` (e.g. `max_cells`), parsed and checked in `shuyi_assets::lang`.
- Beside it, an optional `note` per key: what a translator should know
  (where the text shows, what a placeholder holds). It has the same
  value form as `max_cells`, is kept by `Lang` (`Lang::note(key)`) and
  is never shown in the game. The parked 2227 writes notes and
  `max_cells` into PO files, and a translating agent reads them in
  `ui.ron`.
- `lang-status <code>`: an **overflow** list, every entry of the pack
  whose text is wider than its key's `max_cells`.
- **Either order with 2348** (messages with plural and select branches):
  if 2348 is done, the overflow list measures each form of a message
  (`shuyi_assets::message::Message::forms`) and reports the widest form;
  if not, measure the raw text as planned (no text has branches yet),
  and 2348 makes the list measure forms.
- **Either order with 2352** (parked: the engine's screens in other
  languages): if 2352 is done, give the engine's English keys
  (`crates/ui/lang/en/ui.ron`) a `max_cells` where their place on the
  engine's screens has a fixed width, and make 2352's upkeep test fail
  on an engine pack entry wider than its key's `max_cells`; if not,
  nothing to do (2352 does it, as its clause says).

**Out (do not do):**
- Chinese punctuation, its wide quotes, and rules that depend on the
  language (2303).
- Furigana, vertical text, hyphenation of Latin words.
- Any game's text, widths or layouts.
- Finding text cut on drawn screens: 2353's walk with the
  pseudo-language is the runtime half of this ticket's overflow list.

**For the game:** `word_wrap`'s signature may gain the width table of
2301; the game's PR that moves its pin follows this one (game#0237).

## Implementation steps

1. Latin words break at spaces as today; a word longer than the width is
   still cut. Measure every length with `text_width` (2301), so a wide
   character counts as two cells.
2. Between two wide characters, or a wide one and a Latin word, a break
   is allowed anywhere, except:
   - never start a line with `、。，．・：；？！）」』】〕ー` or a small
     kana (`ぁぃぅぇぉっゃゅょ` and their katakana `ァィゥェォッャュョ`);
   - never end a line with `（「『【〔`;
   - keep a run of Latin letters or digits inside Japanese text together.
   Where obeying a rule leaves no break that fits, break anyway (a line
   never runs past the width).
3. A key name filled into a placeholder (`text::fill`) is never split:
   for example, `fill` joins the words of a multi-word key name with a
   no-break space (U+00A0) that `word_wrap` never breaks at and the
   renderers draw as a space. Say what you chose in the completion notes.
   **Either order with 2348:** if 2348 is done, `fill` formats messages
   through `shuyi_assets::message`; join a key name's words where it
   writes a `Piece::Arg` by `lookup`. If not, join them in today's
   `fill`, and 2348 keeps the join (its clause says so).
   **Either order with 2347:** if 2347 is done, `fill` is
   `fill_styled(…).plain()` (`crates/ui/src/text.rs`): make the join in
   the shared code, so the key's `Key` span holds the joined name; if
   not, 2347 builds `fill_styled` on whatever `fill` does then.
   **Either order with 2390:** if 2390 is done, `fill` walks
   `shuyi_core::text::split_tokens`: make the join in its `Token` arm;
   if not, make it where today's loop fills a placeholder, and 2390
   keeps the join in the `Token` arm when it rewrites the loop.
4. `ui.ron`: an optional `max_cells` and an optional `note` per key of
   the source language; a key without `max_cells` has no limit. A pack
   can set neither (they describe the place and the English, not the
   translation).
5. `lang-status`: after the missing and stale lists, an overflow list:
   key, its `max_cells`, the pack text's width in cells and the text.
   It fails no gate, like the other lists.

## Acceptance criteria

- [x] A Japanese paragraph wraps within the width, with no line starting
      with `。` or `、` and none ending with `「` (unit tests with real
      sentences).
- [x] English wrapping is unchanged (the existing tests and snapshots).
- [x] A key name of two words filled into a placeholder is never split
      across lines (unit test).
- [x] `lang-status` reports an entry made too long on purpose as overflow
      (test with a fixture pack).
- [x] A key with `max_cells` and a `note` loads, and `Lang::note` gives
      the note (unit test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each breaking rule; mixed Latin and Japanese; placeholders;
  `max_cells` and `note` parsing.
- Property: every wrapped line is at most the width in cells, and joining
  the lines gives back the text apart from the spaces broken at.
- Snapshot / integration: the text box paging a Japanese line.

## Completion notes

**What was done:**

- `shuyi_assets::text::word_wrap(text, width, widths)` measures in cells
  (`text_width`, a wide character two). Latin text breaks at spaces as
  before, and a word wider than a line is still cut. Inside a word (text
  between spaces), a line may break between two characters when either
  is wide in `widths` or Japanese or Chinese by its code point (CJK
  punctuation, kana, ideographs, fullwidth forms), so text breaks the
  same before an atlas has its wide glyphs. It never breaks before
  `、。，．・：；？！）」』】〕ー`, the small kana or ASCII closing
  punctuation, never after `（「『【〔` or an ASCII opening bracket, and
  never beside a no-break space. A run of Latin letters or digits is
  kept whole, since a break needs a wide or Japanese character on one
  side. Where the rules leave no break that fits, the piece is cut at the
  width.
- The text box pages through it: `TextBox::new` takes the width table
  (`ctx.widths()`), and so do the toy, `hello-grid` and the
  `shuyi-dialogue` skill's example.
- `lines_at_longest` passes its `widths` to `word_wrap` and no longer
  writes a wide character as `WW`, so the check and the box break at the
  same places. A token is still written as that many `W`s (an unbroken
  Latin run, the worst case). The dialogue test that paged
  space-separated runs of `あ` was rewritten around a Latin run, because
  those runs now break between characters.
- **The key-name join:** `shuyi_ui::text::fill` turns each space of what
  `lookup` gives (a key's name, `Left Shift`) into `NO_BREAK_SPACE`
  (U+00A0, `shuyi_assets::text::NO_BREAK_SPACE`). Values from `args` are
  left alone. `word_wrap` never breaks at it or beside it, and
  `GlyphBuffer::print` writes it as a space, so every renderer draws a
  space.
- `en/ui.ron`: a key's value may be `(text: …, max_cells: …, note: …)`,
  both optional (`max_cells` at least 1, a note not empty), read with
  `implicit_some`. `english_and_places_from_source` returns the text and
  the places; `english_from_source` keeps its signature and drops them.
  `from_files` gives them to `Lang` (`Lang::with_places`), and
  `Lang::place`, `Lang::max_cells` and `Lang::note` read them
  (`crates/assets/src/lang/places.rs`). A pack's entry can't set either,
  because `Entry` denies unknown fields (tested).
- `lang-status <code>`: the screen-text line counts `N too long`, and a
  `too long:` list follows the missing and stale lists: key, its width in
  cells, its place's `max_cells`, and the text (`Lang::overflow`). The
  report is measured with the game's font atlas (`GameTools::font_atlas`)
  when it gives one, else narrow. Every entry is measured, stale ones too,
  as written (placeholders unfilled). It fails no gate.

**Either-order clauses:** 2239, 2347, 2348, 2352 and 2390 were not done,
so I took each one's *if not* branch: the overflow list measures the raw
text, the join is in today's `fill` loop, and the engine's own keys get
no `max_cells` (there is no `crates/ui/lang/` yet). Each of those tickets'
clauses now says what 2302 left for it.

**Deviations:**

- The `English` asset kind (a `blank` or `grid` game's `files/en.ron`,
  read by `BasicGame`) still takes text values only. `max_cells` is read
  from the languages folder's `lang/en/ui.ron`, as the ticket says, and a
  basic game has no packs to measure.
- `lang-status`'s report takes the width table, so its signature changed
  (below).
- The `hello-grid` dialogue was not checked through the MCP session
  tools: it has only English text, which wraps exactly as before (the
  existing text box tests and snapshots pass unchanged).

**Follow-ups:** none new. Chinese rules stay with 2303, the runtime walk
for cut text with 2353, and PO notes with 2227.

**For the game:** game#0237's PR moves the pin and passes `ctx.widths()`
to `TextBox::new` and `word_wrap`.

Book: none needed beyond the change to `games-and-screens.md` (word
wrap in cells, the Japanese rules and the key-name join).

Breaking: shuyi-assets, shuyi-ui: `text::word_wrap` (re-exported as `ui::widgets::word_wrap`): takes the width table; a game passes `ctx.widths()` (or `&CellWidths::NARROW`).

```migration
since: 0.1.0
what: word_wrap measures in cells and takes the width table, so a wide character counts as two and Japanese breaks between characters.
old: 'word_wrap(text: &str, width: usize) -> Vec<String>'
new: 'word_wrap(text: &str, width: usize, widths: &CellWidths) -> Vec<String>'
before: |
  let lines = word_wrap(&text, 30);
after: |
  let lines = word_wrap(&text, 30, ctx.widths());
find: ["word_wrap(&text, 30)"]
```

Breaking: shuyi-ui: `dialogue::text_box::TextBox::new`: takes the width table; a game passes `ctx.widths()`.

```migration
since: 0.1.0
what: A text box wraps its text in cells, so it takes the width table.
old: 'TextBox::new(text: &str, width: usize, per_page: usize)'
new: 'TextBox::new(text: &str, width: usize, per_page: usize, widths: &CellWidths)'
before: |
  let text = TextBox::new(&line, WIDTH, LINES);
after: |
  let text = TextBox::new(&line, WIDTH, LINES, ctx.widths());
find: ["TextBox::new(&line, WIDTH, LINES)"]
```

Breaking: shuyi-cli: `lang_status::report`: takes the width table after the variants; a caller passes the atlas's (`CellWidths::of`) or `&CellWidths::NARROW`.

```migration
since: 0.1.0
what: lang-status's report measures screen text against its places, so it takes the width table.
old: 'lang_status::report(lang, variants, code, options)'
new: 'lang_status::report(lang, variants, widths, code, options)'
before: |
  let said = report(&lang, &variants, "ja", &Options::default())?;
after: |
  let said = report(&lang, &variants, &CellWidths::NARROW, "ja", &Options::default())?;
find: ["report(&lang, &variants, \"ja\""]
```
