---
id: "2261"
title: "Messages checked when the languages load: English that doesn't parse, packs that drop a branch or use a category their language lacks, tips and places by form, and missing forms in lang-status"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2348"]
nick_input: none
completed: 2026-11-17
---

# 2261 — Messages checked when the languages load

## Context

Split from 2348 (*Messages that choose their words*), which landed
`shuyi_assets::message` (ICU MessageFormat's plural, selectordinal and
select, ADR-0127) and stopped at 1,500 lines. A message a translator or
an agent writes wrong today fails nowhere: a text that doesn't parse is
filled as plain placeholders, a plural branch its language never picks
is never shown, and a translation that drops the Playdate's wording of a
`{device, select, …}` shows the other one there. ADR-0127 says these are
load errors, as every other mistake of a language file is (ADR-0007:
the text formats are the source of truth, checked when they load).

What there is (after 2348):

- `shuyi_assets::lang::english_and_places_from_source`
  (`crates/assets/src/lang.rs`) checks the source's keys and places,
  not its texts.
- `lang::entries` checks that an entry's text has "the same
  placeholders" as its source, through `tip::placeholders`.
  `pack_from_files_in` doesn't know the pack's own tag; `from_files_in`
  and `LangPackRon::decode` (`crates/assets/src/lang/formats.rs`) do
  (`pack_code`).
- `tip::from_source` (`crates/assets/src/tip.rs`) checks each line of a
  tip's raw text against its box (`TipLimits`).
- `Lang::overflow` (`crates/assets/src/lang/places.rs`) measures a
  pack entry's raw text against its key's `max_cells`.
- `LangStatus` (`lang.rs`) lists `missing`, `stale` and `orphans`;
  `lang-status` (`crates/cli/src/lang_status.rs`) prints them.
- `Message::args`, `Message::names`, `Message::forms`,
  `message::plural::categories` and `message::DEVICES` are what the
  checks need.

**A draft exists.** 2348's session wrote steps 2–5 before the split and
kept them on the local branch `wip/2348-full` (commit `c3fa207b`) of
Nick's clone: `crates/assets/src/lang/messages.rs` and its uses in
`lang.rs`, `formats.rs`, `places.rs` and `tip.rs`, with tests. Use it if
the branch is there (check it against `main`); else write it from the
steps. `missing_forms` (step 6) was never drafted.

## Nick input

None.

## Scope

**In:**

- **English** (`english_and_places_from_source`): each text parses as a
  message; a select on `device` uses only `DEVICES` and `other`. Each
  error names the file and the key:
  `"a.bad": the text isn't a message: at character 11: expected …`.
- **Packs** (`lang::entries`, for `ui.ron` and `data.ron`):
  - the text and its source both parse;
  - the same argument names (`Message::names`) as today's placeholder
    rule; a translation may wrap a `{n}` in a plural or drop a plural
    its language doesn't need;
  - each value of a source's select on an argument has a branch in the
    text's selects on that argument, so a translation can't drop a
    device's wording;
  - a select on `device` uses only `DEVICES` and `other`;
  - a plural or selectordinal uses only categories its language has
    (`plural::categories`; `fr` has no `few`). `pack_from_files_in`
    takes the pack's tag beside the source's: `from_files_in` and
    `LangPackRon` pass it; the public `pack_from_files` and
    `pack_from_sources`, which don't know it, check no categories.
- **Tips** (`tip::from_source`): a text or how that doesn't parse is an
  error naming the tip; each form (`Message::forms`) is checked against
  the lines and cells of `TipLimits` instead of the raw text, each
  problem once; `device` and `target` are always known placeholders,
  whatever `TipRules::placeholder` says.
- **Places** (`Lang::overflow`): each form is measured and the widest is
  reported (`Overflow::width` and `text` are its).
- **`LangStatus::missing_forms`**: each pack entry whose plural or
  selectordinal lacks a category its language has, with the categories
  it lacks. It fails nothing, as `missing` and `stale` don't (a missing
  form shows `other`). CLDR's integer `many` of French, Spanish,
  Italian and Portuguese (numbers of a million or more) is left out of
  the list, and still accepted in a pack (ADR-0127). `lang-status`
  prints a `missing forms:` list after the stale list, only when it
  isn't empty, so today's reports don't change. A new public field
  (ADR-0039: `LangStatus` gets a constructor or stays built only here,
  as it is).
- **Docs:** `docs/formats.md`, the language pack paragraph: what a pack's
  message must keep; `lang-status`'s doc comment.
- **For the game:** where English loads through `english_from_source`,
  English text that no longer parses (an unclosed `{`, `{}`, a comma
  inside braces) is a load error naming its key; a pack entry whose
  text or source doesn't parse is one too. `Behaviour:` lines say so. A
  search of the game's `assets/` for `{}` and unclosed braces before
  landing says whether its text needs a change. `LangStatus` has a new
  field.
- **From 3218 (done):** nothing to do: it typed the arguments the UI
  hands a message and changed nothing these checks read. `tip::from_source`
  still checks each raw line and knows no built-in `device` or `target`:
  the tip checks under *In* do both.

**Out (do not do):**

- Dialogue lines and captions: they keep their `{tokens}`.
- Typed arguments in the UI (3218).

**Estimated size:** about 1,000 lines (code 450, tests 450, docs 100):
the draft of steps 2–5 measured 490.

**Split:** past 900 lines, split off step 6 (`missing_forms` and its
`lang-status` lines) into a new ticket.

## Implementation steps

1. Read ADR-0127, `shuyi_assets::message`, `crates/assets/src/lang.rs`
   (`english_and_places_from_source`, `entries`, `pack_from_files_in`,
   `from_files_in`, `Lang::status`), `lang/formats.rs`,
   `lang/places.rs`, `tip.rs` and `crates/cli/src/lang_status.rs`.
2. `crates/assets/src/lang/messages.rs`: what is wrong with a source's
   text, and with a pack entry's (its text, its source, its language's
   tag or none). Use it in `english_and_places_from_source` and
   `entries`; thread the pack's tag through `pack_from_files_in`.
3. Tips by form.
4. `Lang::overflow` by form.
5. The tests of steps 2–4, through `from_files` for the file and key.
6. `LangStatus::missing_forms` and its `lang-status` lines.
7. Docs and the gates.

## Acceptance criteria

- [x] Each of these is a load error naming the file and the key, through
      `from_files`: an English text that doesn't parse; a pack select
      that drops a branch value of its source; a select on `device` with
      an unknown value; a plural category the pack's language lacks.
- [x] A pack entry may wrap its source's `{n}` in a plural with its
      language's categories, or drop its source's plural, and loads.
- [x] A tip whose text is a select of two short wordings passes the box
      check, and one whose longer form is too wide fails, naming the
      tip; a tip text that doesn't parse fails naming the tip.
- [x] `Lang::overflow` reports a message's widest form.
- [x] `lang-status` lists a `ru` pack plural that lacks `few` under
      missing forms, and the pack still loads (a fixture pack); a `fr`
      plural without `many` isn't listed. The existing tests in
      `lang_status.rs` pass unchanged.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each check, with its message; `from_files` naming the file and
  key; tips by form; the overflow by form; `missing_forms` and its
  `lang-status` lines.
- Existing: every test of `lang`, `tip` and `lang_status` passes
  unchanged but those whose fixtures need a message's text.

## Completion notes

Steps 2–5 are the draft of `wip/2348-full` (`c3fa207b`), taken onto
`main` and checked against it: its `messages.rs` and its uses in
`lang.rs`, `formats.rs`, `places.rs` and `tip.rs`, with their tests.
Its `text_in`, `data_text_in` and `placeholders` had landed with 2348
and 3218, so only the checks came over. Step 6 is new:

- `message::plural::expected` (crate-private): a language's categories
  without the cardinal `many` of `fr`, `es`, `it` and `pt`.
- `lang::messages::lacks`: the categories an entry's plurals lack.
- `LangStatus::missing_forms`, a list of `MissingForms { key, lacks }`,
  the screen text's entries then the data's, in key order.
- `lang-status` prints `missing forms:` after each part's stale list,
  only when there are any, with each entry's key, what it lacks and its
  text. They are left out of the counts line, so no report without them
  changes. Test text is filtered as for the other lists, but isn't
  counted as left out a second time (`Scope::listed`).

Deviations:

- The ticket and the code named the messages ADR as ADR-0124 (the draft)
  and ADR-0129 (`message.rs`, `Lang::text_in`, `data_text_in`, the CLI's
  `notices.rs`). It is ADR-0127 (ADR-0124 is key caps, ADR-0129 the
  arrow keys). Corrected in the files this PR touches.
- `LangStatus` became `#[non_exhaustive]` beside its new field. It is a
  view only `Lang::status` builds (ADR-0039, kind 3), and the new field
  breaks a struct literal either way; with it, a field added after this
  one is no break.

The game's text (searched on its `main`, 2026-11-17): its
`assets/lang/en/ui.ron` (841 keys) loads through `english_from_source`
with these checks, and the 30 texts and hows of `assets/data/tips.ron`
all parse as messages, so no game text needs a change. `gh search code
--repo Zafnok/visions-of-shuyi LangStatus` finds `crates/content/src/lang.rs`
and `lib.rs`, which read its fields and never build one. The game's
fixtures with `{open` and `{}` are dialogue lines, which these checks
leave alone, and its data test's error text ("the text and its source
must have the same placeholders") is unchanged.

Breaking: shuyi-assets: `lang::LangStatus`: it has a new field `missing_forms` and is `#[non_exhaustive]`; a game that builds one with a struct literal compares fields instead, or starts from `LangStatus::default()`

```migration
since: 0.1.0
what: lang::LangStatus is a view only Lang::status builds; it gained missing_forms and can't be written as a literal.
old: "LangStatus { missing, stale, orphans }"
new: "lang.status(&code), its fields read"
before: |
  assert_eq!(status, LangStatus { missing: vec![], stale: vec![], orphans: vec![] });
after: |
  assert_eq!(status, LangStatus::default());
find: ["LangStatus {"]
```

Behaviour: shuyi-assets: English screen text that doesn't parse as a message (an unclosed `{`, `{}`, a comma inside braces) is a load error naming its file and key, as is a select on `device` with a value no device has (`english_from_source`, `from_files`, `LangPackRon`).

Behaviour: shuyi-assets: a pack's `ui.ron` or `data.ron` entry whose text or source doesn't parse, whose text drops a branch value of its source's select, or whose select on `device` names no device is a load error naming its file and key; read with its tag (`from_files`, `LangPackRon`), so is a plural or selectordinal category its language lacks (`fr` with `few`).

Behaviour: shuyi-assets: a pack entry may wrap its source's `{n}` in a plural or drop its source's plural: the placeholder rule compares arguments' names (`Message::names`), not the raw inside of each pair of braces.

Behaviour: shuyi-assets: `tip::from_source` fails a text or how that doesn't parse, naming the tip; it checks each form of a message against the box's lines and cells, each problem once, and takes `{device}` and `{target}` as known placeholders whatever `TipRules::placeholder` says.

Behaviour: shuyi-assets: `Lang::overflow` measures each form of a message and reports the widest: its `width` and `text` are that form's.

Behaviour: shuyi-cli: `lang-status` lists `missing forms:` after a part's stale keys when a plural lacks a category the pack's language has; a report with none is unchanged.

Book: `site/content/learn/book/assets-and-formats.md`, after "A text can choose its words", says what the languages check as they load and what `lang-status` lists.

Game: nothing to adopt. Its English and tips load unchanged (above); it reads `LangStatus`'s fields only, and `lang-status` gains a list only when a pack's plural lacks a form, which the game's packs have none of yet.
