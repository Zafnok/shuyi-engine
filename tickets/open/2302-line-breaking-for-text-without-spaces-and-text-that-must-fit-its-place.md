---
id: "2302"
title: "Line breaking for text without spaces (Japanese), and a report of translated text too long for its place"
type: feature
milestone: After MVP
model: opus-5.5
effort: medium
status: todo
blocked_by: ["2301"]
nick_input: none
completed:
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

Where wrapping happens in the engine today: `shuyi_ui::widgets::
word_wrap` (`crates/ui/src/widgets/wrap.rs`), used by the dialogue text
box's paging (`crates/ui/src/dialogue/text_box.rs`, so a page is
`box_lines` lines of `word_wrap`). Length checks on pack text use
`shuyi_assets::lang::text_width` (dialogue limits, tips, names).

## Nick input

None.

## Scope

**In:**
- The breaking rules in `word_wrap`, measuring in cells.
- The dialogue text box's paging gets them through `word_wrap` (a page
  is still `box_lines` lines).
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
   If 2348 is done, `fill` formats messages through
   `shuyi_assets::message`; join a key name's words where it writes the
   key's name. **Either order with 2347:** if 2347 is done, `fill` is
   `fill_styled(…).plain()` (`crates/ui/src/text.rs`): make the join in
   the shared code, so the key's `Key` span holds the joined name; if
   not, 2347 builds `fill_styled` on whatever `fill` does then.
4. `ui.ron`: an optional `max_cells` and an optional `note` per key of
   the source language; a key without `max_cells` has no limit. A pack
   can set neither (they describe the place and the English, not the
   translation).
5. `lang-status`: after the missing and stale lists, an overflow list:
   key, its `max_cells`, the pack text's width in cells and the text.
   It fails no gate, like the other lists.

## Acceptance criteria

- [ ] A Japanese paragraph wraps within the width, with no line starting
      with `。` or `、` and none ending with `「` (unit tests with real
      sentences).
- [ ] English wrapping is unchanged (the existing tests and snapshots).
- [ ] A key name of two words filled into a placeholder is never split
      across lines (unit test).
- [ ] `lang-status` reports an entry made too long on purpose as overflow
      (test with a fixture pack).
- [ ] A key with `max_cells` and a `note` loads, and `Lang::note` gives
      the note (unit test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each breaking rule; mixed Latin and Japanese; placeholders;
  `max_cells` and `note` parsing.
- Property: every wrapped line is at most the width in cells, and joining
  the lines gives back the text apart from the spaces broken at.
- Snapshot / integration: the text box paging a Japanese line.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
