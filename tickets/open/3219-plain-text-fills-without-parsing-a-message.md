---
id: "3219"
title: "Plain text fills without parsing a message: the cost 3218 added to every frame's hints"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3219 — Plain text fills without parsing a message

## Context

A request from the game.

1. **The game ticket:** game#1285, which moves the game's pin past this
   one and checks its Playdate bench is under budget again
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1285-the-playdate-bench-under-budget-after-the-text-fill-fix.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** filling a screen's text (a help bar, a
   label) costs what it cost before messages when the text has no
   plural, ordinal or select in it.
3. **The friction numbers:** no friction review: a performance
   regression. The game's pin move across engine PRs #460–#479
   (game PR #732, 2026-11-17) made every frame of its `cargo shuyi
   bench` scenarios dearer on every target: an idle battle frame from
   1,882,790 to 1,962,514 instructions on the Playdate (+80k, +4%) and
   from 1,159,227 to 1,248,598 on desktop and web; the Playdate's cursor
   sweep from 1,982,686 to 2,050,631, over its 2,000,000 budget. Most
   of it is #476 (3218): `text::fill_styled_with` now runs
   `Message::parse` on every text it fills, before the plain path it
   took before, and `Ctx::fill_styled` copies its `args` into a new
   `Vec` with `device` and `target` on every call. A game builds many
   such texts each frame (its help bars, labels and panels). Measured on
   the desktop, release build: `Message::parse` of a 110-character help
   line takes about 200 ns of the 1.5 µs `fill` takes.
4. **The generic shape:** `shuyi_ui::text::fill_styled_with` (and so
   `fill`, `fill_values`, `fill_styled` and `Ctx`'s text) does no
   message work for a text that has none, and a fill allocates only
   what it hands back. Same output for every text, byte for byte.
5. **An example from a game that isn't tactics:** a platformer's pause
   menu and its button hints, filled each frame with the player's keys,
   pay the same cost on a handheld target.

**Why the MVP.** The retro of 2026-11-17 (PRs #379–#484) moved this
ticket from *After MVP* to the MVP. 3218, which made the cost, blocks
the first release (2005), and it put the game's Playdate cursor sweep
over budget (2,050,631 instructions against 2,000,000). The engine's
first user must not ship a target over budget at the release, so the
fix goes with the change that caused it.

**What else costs per fill**, each checked in the code on 2026-11-17
(`main` at `5a1b6e29`):

- `Ctx::label` and `Ctx::label_styled` (`crates/ui/src/ctx.rs`, lines
  591–594 and 670–673) collect a `Label::Key`'s `Vec<(String, Arg)>`
  into a new `Vec<(&str, &dyn ToArg)>` on every call. Every menu row is
  one (`widgets/menu/glyph.rs`, `screens/profiles/glyph.rs`,
  `dialogue/text_box/glyph.rs` call `ctx.label` for each item each
  frame).
- `Ctx::fill_styled` (lines 651–657) copies `args` into a new `Vec` to
  push `device` and `target` after them.
- `text::fill` (`crates/ui/src/text.rs`, line 143) builds a whole
  `StyledText`, a `Vec` of spans each with its own `String`, and then
  joins it with `.plain()`. `Ctx::text_with_keys` and `Ctx::fill_text`
  do the same.
- Each argument is written through `value.to_arg()` and then
  `.to_string()` (lines 203–217): a text value is copied twice, an
  integer formatted into a `String` before it is copied into the span.
- `joined` (line 246) makes a `String` for each key name before it is
  pushed.
- A select picks its branch by `arg.to_string()`
  (`crates/assets/src/message.rs`, `Choice::pick`, line 397): a `String`
  per select per fill, and the built-in `device` is a select in every
  text that chooses by it.

## Nick input

None.

## Scope

**In:**
- **The fast path** in `crates/ui/src/text.rs`: a check of the text
  that picks the old plain path (`split_tokens`, filling each `{name}`
  as before) when the message path would give the same: every `{…}`
  holds a name with no whitespace, `,` or `{` in it (`Message::parse`
  trims a name's spaces where `split_tokens` keeps them, so `{ n }`
  must go to the message path), and no `'` sits next to `{`, `}` or
  another `'`. A text the check doesn't call plain goes to
  `Message::parse` as today. The property test below decides the rule
  where this paragraph is wrong.
- **No list per call in `Ctx`:** `fill_styled`, `label` and
  `label_styled` hand their values to the fill without a new `Vec`.
  The fill reads its values through a private lookup (a small trait or
  closure in `text.rs`) that `&Args`, a `Label::Key`'s
  `[(String, Arg)]` and "these, then `device` and `target`" all give.
  Public signatures don't change.
- **A plain fill writes a `String`:** `fill`, `fill_values`,
  `Ctx::text_with`, `text_with_keys`, `fill_text` and `label` write
  into the `String` they return through a private sink that a
  `StyledText` also takes, rather than building spans and joining them.
- **An argument is written once:** match on the `Arg` that `to_arg`
  gives and push its text (or write an integer's digits) straight into
  the sink, with no `.to_string()`; a key name's spaces become
  `NO_BREAK_SPACE` as it is pushed, with no `joined` `String`.
- **A select compares without a `String`:** `Choice::pick` compares an
  `Arg::Text` by `&str` and an `Arg::Int` by its digits written into a
  stack buffer (or an equivalent that never allocates), keeping
  today's rule that an `Int` matches a branch by its digits (`7`
  matches `7`, not `007`).
- **Numbers:** the instructions per fill before and after (a
  micro-benchmark under Callgrind, or `cargo shuyi bench` on Hello,
  Grid in WSL or CI), and the allocations per fill before and after, in
  the PR.
- Public types (ADR-0039): none new; the lookup and the sink are
  private.
- **Either order with 2266:** if 2266 is done, a text from `Lang`
  already says whether it is plain or a parsed message: `Ctx`'s fills
  use that record and the fast check covers only text from outside the
  language files (`fill_values`, `fill_text` of data text, a game's own
  `fill`); the lookup, the sink and the other items above are still
  this ticket's. If not, the fast check covers every fill, and 2266
  later moves `Ctx`'s fills onto its record.
- **Either order with 3221:** if 3221 is done, lower the counts pinned
  in `examples/hello-grid/tests/frame_alloc.rs` to what it measures (it
  fails until you do); if not, nothing to do.

**Out (do not do):**
- Caching parsed messages per key, and a walk that borrows its
  arguments instead of taking an owned `Arg` per lookup (the copy
  `ToArg::to_arg` makes of a text value, and so of `device` in every
  select by device): 2266 does both.
- Any change in what a text fills to.
- The key names' own cost (`Game::key_text_styled` building the cursor
  keys for each key it names): not this ticket's; it is the same
  before and after 3218.
- `split_tokens` and the other placeholder parsers: 2390 unifies them,
  and the fast path calls the fallback that is already there.

No API break: a `Behaviour:` line only if an output changes, and none
should. The game's PR that moves the pin follows this one (game#1285).

**Estimated size:** about 900 lines (code 380, tests 460, docs 60).

**Split:** past 1,200 lines, move the select's comparison and the
argument written once (the `Choice::pick` and `to_arg` items) to 2266,
whose borrowed walk replaces that code anyway, and keep the fast path,
the lookup and the sink here.

## Implementation steps

1. Read `text::fill_styled_with`, `split_tokens`, `Message::parse` and
   `Choice::pick`, and `Ctx::fill_styled`, `label`, `label_styled`,
   `text_with_keys` and `fill_text`.
2. Measure first: the instructions and allocations of `Ctx::text_with`
   on a 110-character help line with two keys, of `Ctx::label` on a
   `Label::Key` with two arguments, and of a text with a `device`
   select, on today's `main`. Keep the numbers for the PR.
3. The private lookup: one trait (or closure type) that finds a value
   by name, with impls for `&Args`, `[(String, Arg)]` and a chain of
   two. Move `fill_styled_with`'s body onto it; the public functions
   call it with `&Args`.
4. `Ctx::fill_styled`, `label` and `label_styled` on the lookup, with
   no `Vec`.
5. The private sink: `push_plain`, `push_key` and `push(text, role)`,
   for `String` (roles dropped) and `StyledText`. `fill`, `fill_values`
   and `Ctx`'s plain fills write into a `String`.
6. Arguments written once and keys joined as they are pushed.
7. The fast check, and the fill routed by it.
8. `Choice::pick` without `to_string`.
9. Tests, the numbers, the gates.

## Acceptance criteria

- [ ] A text the check calls plain is never given to `Message::parse`
      (a test with a counting wrapper or the path's own test hook).
- [ ] For every text of the toy's and Hello, Grid's languages and of
      the text tests, `fill_styled` and `fill` give byte for byte what
      they gave before this ticket, with and without values and keys
      (the old functions kept in the test as the reference).
- [ ] `Ctx::label` of a `Label::Key`, `Ctx::text_with` and
      `Ctx::fill_styled` allocate no `Vec` of arguments: an allocation
      count test (`shuyi-alloc-count`'s `global` counter, as
      `crates/sound/tests/mix_alloc.rs` does) pins each one's count,
      and each is lower than step 2's by at least the lists removed.
- [ ] `fill` of a plain text with no placeholders allocates once (the
      `String` it returns), by the same counter.
- [ ] A select on an `Arg::Int` or an `Arg::Text` allocates nothing in
      `Choice::pick`, and picks the same branch as before for `7`,
      `-7`, `007` and a text value.
- [ ] Every existing text test passes unchanged.
- [ ] The PR states the instructions and the allocations per fill
      before and after, for step 2's three cases.
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not (nothing a game writes
      changes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the check on `{a}`, `{ a }`, `{a, plural, …}`, `'{a}'`, `''`,
  `don't`, a lone `}`, an unclosed `{`, `{}`; `Choice::pick` on the
  values above.
- Property: over random texts of braces, commas, apostrophes, spaces,
  `#` and words, the fast path and the message path give the same
  `StyledText` whenever the check picks the fast one; and `fill`'s
  `String` equals `fill_styled(…).plain()` for every text.
- Equivalence: the language-file sweep above against the old code.
- Allocation counts: an integration test in `crates/ui/tests/`, alone
  in its binary since the counter is the process's
  (`shuyi-alloc-count` with `global` as a dev-dependency of
  `shuyi-ui`; add the edge to `tools/layers.ron` if `cargo xtask
  check-layers` asks for it).

## Completion notes

*(Filled in by the session that completes the ticket.)*
