---
id: "2261"
title: "Messages checked when the languages load: English that doesn't parse, packs that drop a branch or use a category their language lacks, tips and places by form, and missing forms in lang-status"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2348"]
nick_input: none
completed:
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
- **Either order with 3218:** nothing to do either way: 3218 types the
  arguments the UI hands a message and changes nothing these checks
  read.

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

- [ ] Each of these is a load error naming the file and the key, through
      `from_files`: an English text that doesn't parse; a pack select
      that drops a branch value of its source; a select on `device` with
      an unknown value; a plural category the pack's language lacks.
- [ ] A pack entry may wrap its source's `{n}` in a plural with its
      language's categories, or drop its source's plural, and loads.
- [ ] A tip whose text is a select of two short wordings passes the box
      check, and one whose longer form is too wide fails, naming the
      tip; a tip text that doesn't parse fails naming the tip.
- [ ] `Lang::overflow` reports a message's widest form.
- [ ] `lang-status` lists a `ru` pack plural that lacks `few` under
      missing forms, and the pack still loads (a fixture pack); a `fr`
      plural without `many` isn't listed. The existing tests in
      `lang_status.rs` pass unchanged.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each check, with its message; `from_files` naming the file and
  key; tips by form; the overflow by form; `missing_forms` and its
  `lang-status` lines.
- Existing: every test of `lang`, `tip` and `lang_status` passes
  unchanged but those whose fixtures need a message's text.

## Completion notes

*(Filled in by the session that completes the ticket.)*
