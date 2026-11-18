---
id: "2266"
title: "A language's texts are parsed once, when they load, and a fill reuses what the load learned"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2266 — A language's texts are parsed once, when they load

## Context

Found by the retro of 2026-11-17 (PRs #379–#484), its problem 3: 3218
(#476) made every text fill run `Message::parse` (ADR-0127), and the
game's Playdate cursor sweep went over its budget (2,050,631
instructions against 2,000,000). 3219 takes the cost off plain text
with a fast check of the text on each fill. This ticket removes the
work from the frame altogether: the load already parses every text,
and keeps nothing of it.

What the load does today, checked in the code on 2026-11-17 (`main` at
`5a1b6e29`), since #482 (2261):

- `english_and_places_from_source` (`crates/assets/src/lang.rs`, line
  562) runs `messages::source_problems` on each English text, which
  parses it (`crates/assets/src/lang/messages.rs`, `message`, line 17)
  and drops the `Message`.
- `entries` (`lang.rs`, line 704) runs `messages::entry_problems` on
  each pack entry, which parses its text and its source (`messages.rs`,
  lines 17 and 59–65) and drops both. The pack format's load calls it
  through `pack_from_files_in` (`crates/assets/src/lang/formats.rs`,
  line 209).
- `Lang` keeps plain strings: `ui: BTreeMap<String, String>`, a pack's
  `Entry { key, source, text }`, and `SourceText`'s data. `Lang::text_in`
  hands back `(&str, &str)`, and `Ctx::text_styled_with_keys`
  (`crates/ui/src/ctx.rs`) gives that text to
  `text::fill_styled_with`, which scans it again (3219's check) and
  parses it again if it is a message.
- `Message<'a>` borrows its text (`crates/assets/src/message.rs`,
  `Node::Text(&'a str)`, `Selector::Value(&'a str)`), so `Lang` can't
  keep one beside the `String` it reads without borrowing from itself.
- `Message::walk` takes `value: impl Fn(&str) -> Option<Arg>`, an owned
  `Arg` per lookup, and `ToArg::to_arg` copies a text value into it: a
  `String` for each `{name}` filled with text, and for `device` in every
  select by device.

The size to measure against: the game's `assets/lang/en/ui.ron` has 833
keys (`grep -cE '^\s*"[a-z0-9_.]+"\s*:'` on the game's `main`,
2026-11-17; the game pins engine `e738ffc2`), 33 lines with a plural or
select and 346 with a brace; it ships English only today and plans
Japanese and Chinese (game ADR-0045). No game file comes here
(`AGENTS.md` rule 4): the test builds a synthetic set of that size and
mix.

The job in one sentence: a screen's text costs no parse in a frame,
whatever language it is shown in. A farming game on a handheld with
three languages and a few hundred menu texts gains the same.

## Nick input

None.

## Scope

**In:**
- **A parse that `Lang` can keep** (`shuyi_assets::message`): the
  parser records where each piece is in its text (byte ranges) rather
  than borrowing it, so a parsed message can be stored apart from its
  text and walked with it. `Message::parse`, `walk`, `format`, `names`
  and `args` keep their signatures. No `unsafe`, no new dependency.
- **`Lang` records each text's form when it is built**
  (`Lang::new`, `Lang::with_source`, and a pack as it is read): plain
  (no message syntax: 3219's check, or the parse finding only text and
  simple arguments), a message (its parse kept), or not a message (the
  `split_tokens` fallback, as today). This covers the source's screen
  text and data text and each pack's screen and data entries. A plain
  text costs nothing beyond its `String`: only messages keep a parse.
  `Lang` stays `Clone`, `Eq` and `Default`.
- **`Lang::form_in(code, key) -> (Form<'_>, &str)`**: `text_in` with
  the record, `Form` a `#[non_exhaustive]` enum (ADR-0039 kind 2) of
  `Plain(&str)`, `Message(…)` and `NotAMessage(&str)`; and
  `data_form_in` beside `data_text_in`. `text_in` and `data_text_in`
  stay.
- **`Ctx` fills a key's text from its form**: `text_with`,
  `text_with_keys`, `text_styled`, `text_styled_with_keys`, `label` and
  `label_styled` call `form_in` and never `Message::parse`.
- **A walk that borrows its values:** `Message::walk` gets a sibling
  that takes a value as a borrowed `ArgRef<'_>` (`#[non_exhaustive]`:
  `Int(i64)`, `Text(&str)`), and `ToArg` a provided method that hands
  one out without copying (overridden for `str`, `String`, `char`, the
  integers and `Arg`; its default goes through `to_arg`, so a game's
  own `ToArg` keeps compiling). The fill uses it, so `device` and a
  text value are never copied into a `String` to be compared or
  written.
- **The load's cost, measured:** the instructions and the heap's peak
  of building a `Lang` from a synthetic 833-key English with the
  game's mix (33 messages, about 350 texts with a placeholder) and two
  packs of it, before and after, in the PR; and Hello, Grid's
  `start_instructions` from `cargo shuyi bench` before and after.
- **An ADR** (the `write-adr` skill) amending ADR-0127 §3–§4: a text's
  form is learned at load and kept, and arguments are lent to the walk.
- Docs: the rustdoc of `Lang::form_in`, `Form`, `ArgRef` and the new
  `ToArg` method; the book's chapter on text if it shows `Lang::text_in`
  (the docs map names it).
- **For the game:** no break; added API only (`api: added`). The game
  needs no change to gain it: its `Ctx` fills go through the record.
  A game that calls `text::fill` on text it holds keeps 3219's check.
- **3219 is done (#485):** its check `has_message_syntax`
  (`crates/ui/src/text.rs`) is the test for plain at load, and its
  private value lookup (`text::fill_styled_by`) is what `Ctx`'s fills
  from a form use. It did not reach two items its scope had grown, so
  do them here, with the borrowed walk:
  - **A select compares without a `String`:** `Choice::pick` compares an
    `Arg::Text` by `&str`, and an `Arg::Int` by its digits written into a
    stack buffer. An `Int` still matches a branch by its digits: `7`
    matches `7`, not `007`.
  - **An argument is written once:** each argument is pushed into the
    fill with no `.to_string()`. A key name's spaces become
    `NO_BREAK_SPACE` as it is pushed, with no `joined` `String`.

  The labels' lists and a plain fill's sink went to 3221.
- **Either order with 3221:** if 3221 is done, lower the counts pinned
  in `examples/hello-grid/tests/frame_alloc.rs` to what it measures (it
  fails until you do); if not, nothing to do.

**Out (do not do):**
- Folding the format's check and the record into one parse: `English`
  is a public map of strings, and changing it is a break. The load
  parses a text at most twice (the check, the record), never in a
  frame.
- Text a caller holds (`Ctx::fill_text` of a tip, `text::fill` and
  `fill_values` on a game's own text): 3219's check covers it.
- `Lang::status`'s `messages::lacks`: a tool's report, not a frame.
- Any change in what a text fills to.

**Estimated size:** about 1,400 lines (code 600, tests 600, docs and
the ADR 200).

**Split:** past 1,000 lines, split off the borrowed walk (`ArgRef`,
the `ToArg` method and the fill on them: steps 6–7) into a new ticket,
and keep the stored parse, `form_in` and `Ctx` on it here.

## Implementation steps

1. Read `message.rs`'s parser and `walk`, `lang.rs`'s `Lang`,
   `text_in` and `entries`, `lang/source.rs`'s `data_text_in`, and
   `ctx.rs`'s fills, and 3219's `has_message_syntax` and
   `fill_styled_by` (done in #485).
2. Measure first: Hello, Grid's `start_instructions` and a frame of
   its options menu (`cargo shuyi bench`, in WSL or CI), and the
   synthetic load, on today's `main`.
3. The parser on byte ranges; `Message<'a>` built on them; the stored
   form walked with its text. The existing message tests pass
   unchanged.
4. `Lang`'s record, built in its constructors and as a pack is read;
   `form_in` and `data_form_in`.
5. `Ctx`'s key fills on `form_in`.
6. `ArgRef` and the `ToArg` method; the borrowed walk.
7. The fill on the borrowed walk.
8. The parse counter and the frame test; the numbers; the ADR and the
   docs; the gates.

## Acceptance criteria

- [ ] A test drives the toy (`shuyi_platform` harness) and Hello,
      Grid through screens whose texts include plain text, keys, a
      plural and a select on `device`, for several frames after the
      load, and counts zero calls to `Message::parse` in those frames
      (a counter in `shuyi-assets` behind a feature only tests turn
      on, reset after the load).
- [ ] For every text of the toy's and Hello, Grid's languages, a fill
      from its form gives byte for byte what `fill_styled` of its text
      gives, with and without values and keys.
- [ ] Building a `Lang` of only plain texts allocates no more than
      before this ticket (an allocation count with
      `shuyi-alloc-count`), so only messages cost a kept parse.
- [ ] A select on `device` and a `{name}` filled with a text value
      allocate nothing while the message is walked (the same counter).
- [ ] Two `Lang`s built from the same files are equal (`==`), and a
      clone fills the same as the original.
- [ ] The PR states the synthetic load's instructions and heap peak
      and Hello, Grid's `start_instructions` and options-menu frame,
      before and after.
- [ ] The ADR amending ADR-0127 is written and indexed.
- [ ] The book's chapter is updated (2802), or the completion notes
      have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the parser's ranges give back the same pieces as before for
  every message test; a form of each kind from `form_in`, along a
  fallback chain (a pack lacking the key, a stale entry).
- Property: for random texts of braces, commas, apostrophes, spaces,
  `#` and words, the stored form walked with its text gives what
  `Message::parse(text)` walked gives, and a text the record calls
  plain fills the same by both paths.
- Integration: the frame test above; the allocation counts, each alone
  in its test binary (the counter is the process's).
- Golden: none (no format changes).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
