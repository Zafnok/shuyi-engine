---
id: "2239"
title: "Translated dialogue lines get the text box's page check that English lines get"
type: bug
milestone: After MVP
model: implementation
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2239 — Translated dialogue lines get the text box's page check

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). A speech or narration line must fit the dialogue text box
(2209, done): `DialogueLimits` (`crates/assets/src/dialogue.rs`) gives
`max_text`, and also `box_width` and `box_lines`, one page of the box.
Two checks keep a line to them, and only one checks the page:

- **English** (`Checker::step`, `crates/assets/src/dialogue/check.rs`,
  lines 435 to 460): a line over `max_text` fails ("text is N
  characters; the limit is L"); else a line that takes more than
  `box_lines` lines of `box_width` fails too ("text takes N lines of W
  characters; the box shows B"). It counts the lines with
  `lines_at_longest`, through `Checker::pages`.
- **A language pack** (`add_lines`, `crates/assets/src/lang/lines.rs`,
  lines 227 to 279): it measures each text with `longest_width` (line
  266) and compares it with `limit(kind, limits)` (line 196:
  `max_text` for `LineKind::Speech`, `max_option` for `LineKind::Reply`,
  none for a caption). It never calls `lines_at_longest`; nothing under
  `crates/assets/src/lang*` names `box_lines`.

So a translation as short as `max_text` that wraps to one line more
than the box holds loads, and the text box
(`shuyi_ui::dialogue::TextBox`, `crates/ui/src/dialogue/text_box.rs`,
which wraps with `word_wrap` and pages every `per_page` lines) shows it
as one page more than the scene was written for. With the test limits
(`box_width` 40, `box_lines` 2, `max_text` 80), three words of 24
letters are 74 cells: English refuses them
(`a_line_must_fit_the_box_once_its_tokens_are_longest`,
`crates/assets/src/dialogue/tests.rs`) and a pack takes them.

This matters most for 2302: its line breaking for Japanese reaches the
page check through `word_wrap` and `lines_at_longest`, and Japanese
text only ever comes in a pack.

Replies and captions agree already: English checks a reply only against
`max_option` (no page) and a caption not at all, as the pack does.

## Nick input

None.

## Scope

**In:**
- **One rule, two callers.** In `crates/assets/src/dialogue/check.rs`:

  ```rust
  /// How a speech or narration line fits the text box.
  pub(crate) enum BoxFit {
      Fits,
      /// Wider than `max_text`, in cells (`longest_width`).
      Long { cells: usize },
      /// Narrow enough, but more than `box_lines` lines (`lines_at_longest`).
      Lines { lines: usize },
  }

  pub(crate) fn box_fit(
      text: &str,
      tokens: &dyn Tokens,
      longest_name: Option<usize>,
      limits: &DialogueLimits,
  ) -> BoxFit
  ```

  It checks the width first and the pages only when the width fits, as
  `Checker::step` does today. Both are `pub(crate)`: the two callers
  are in `shuyi-assets`.
- `Checker::step` uses `box_fit` with the names table's longest name
  (`self.names.map(Names::longest)`), and words its two messages
  exactly as today ("characters"; the game's tests compare them).
- `add_lines` uses `box_fit` for a line whose English is
  `LineKind::Speech`, on every text of the entry (`LineText::all`, so
  every variant), with the pack's longest name (`Some(names.longest())`,
  as today). Its messages: `Long` keeps today's wording, "text is N
  cells; the limit is L" (the game's tests compare it), and `Lines` is
  new: "text takes N lines of W cells; the box shows B". A
  `LineKind::Reply` keeps its `max_option` check; a caption keeps none.
- **Either order with 2302:** if 2302 is done, `lines_at_longest`
  already breaks Japanese text by its rules, so add a unit test here: a
  pack's Japanese speech line within `max_text` that wraps past
  `box_lines` fails with the `Lines` message; if `word_wrap` or
  `lines_at_longest` took a width table there, `box_fit` takes it too
  and passes it on. If not, nothing more to do here (2302's rules reach
  the pack check through `lines_at_longest`, and 2302 adds that test).
- **Either order with 2301:** if 2301 is done, `longest_width` and
  `lines_at_longest` take its `CellWidths`: `box_fit` takes it too, and
  its two callers pass the table they pass to those today. If not,
  nothing to do (2301 passes its table through `box_fit` with every
  other caller of `text_width`).
- **Either order with 2241:** if 2241 is done, `add_lines` also runs
  the game's `speech_problem` on a speech line's `width`, after the
  `limit` check: keep that call after the `match` on `box_fit`, on the
  same `width`, whatever `box_fit` answered; if not, nothing to do
  (2241 adds its call after `box_fit`).
- **For the game:** no public API change (`box_fit` and `BoxFit` are
  crate-private; the English messages and the pack's width message are
  unchanged). A behaviour change: a pack's speech line too long for one
  page of the box now fails to load where it loaded. The game's PR that
  moves its pin fixes any such line in its packs.

**Out (do not do):**
- Translated tips: `PackRules::data_problem` (`crates/assets/src/lang.rs`,
  line 352) hands a pack's data text, "a tip over its box" included, to
  the game's own rule on purpose. No `TipLimits` check for packs here.
- Changing the English check's messages, or what counts as fitting.
- A page check for replies or captions (English has none).
- The overflow list of screen text per `max_cells` (2302).

## Implementation steps

1. Read `Checker::step` and `Checker::pages` in
   `crates/assets/src/dialogue/check.rs`, and `add_lines` and `limit`
   in `crates/assets/src/lang/lines.rs`. Check whether 2301 and 2302
   are done (the clauses above).
2. Add `BoxFit` and `box_fit` beside `lines_at_longest` in
   `check.rs`, with a doc comment.
3. Rewrite the `if len > limit … else if self.pages(text) > …` block of
   `Checker::step` as a `match` on `box_fit`, with the same messages.
   Drop `Checker::pages` if nothing else uses it.
4. In `add_lines`, replace the `match` on `limit(…)` with one on the
   English line's kind: `Speech` goes through `box_fit`, `Reply`
   compares `longest_width` with `max_option` as today. Drop `limit`
   and its unit test if nothing else calls it, and import `box_fit`
   from `crate::dialogue` (re-export it there as `pub(crate)` if the
   module path needs it).
5. Settle the 2301 and 2302 clauses with what is done.
6. Write the tests below, and run the gates.

## Acceptance criteria

- [ ] `grep -n "lines_at_longest\|box_fit" crates/assets/src/dialogue/check.rs`
      shows `Checker::step` calling `box_fit`, and
      `grep -n "box_fit" crates/assets/src/lang/lines.rs` finds the
      pack's call.
- [ ] A pack's speech line of three 24-letter words (74 cells, within
      `max_text` 80) fails with "text takes 3 lines of 40 cells; the box
      shows 2" (unit test), and the same text in a variant
      (`text_b`) fails the same way.
- [ ] `a_dialogue_entry_keeps_to_the_rules_of_english`,
      `a_name_token_counts_as_the_packs_longest_name`
      (`crates/assets/src/lang/pack_tests.rs`) and
      `a_line_must_fit_the_box_once_its_tokens_are_longest`
      (`crates/assets/src/dialogue/tests.rs`) pass unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/assets/src/lang/pack_tests.rs`, with the tests' `LIMITS`
  and `RULES`): a speech line that wraps to three lines fails, in `text`
  and in one variant; a line that fits two lines with English's name for
  `{n:king}` but takes three with the pack's longer name fails (the page
  check counts the pack's longest name); a reply and a caption of the
  same text get no page error.
- Unit (`check.rs`): `box_fit` gives `Fits`, `Long` and `Lines` on one
  text each, and `Long` (not `Lines`) for a text both too wide and too
  many lines.
- The existing dialogue and pack tests pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
