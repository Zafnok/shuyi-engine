---
id: "3218"
title: "Typed text arguments: screen text and tips choose their words by a number's plural, a value, the device and the target"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2348"]
nick_input: none
completed: 2026-11-17
---

# 3218 — Typed text arguments: screen text and tips choose their words

## Context

Split from 2348 (*Messages that choose their words*), which landed
`shuyi_assets::message` (ICU MessageFormat's plural, selectordinal and
select, ADR-0127), its plural table and `Lang::text_in` /
`Lang::data_text_in`, and stopped at 1,500 lines. This ticket is the
half a game sees: the values a screen hands its text become typed, so a
plural can choose by a number, and the engine's fills go through
`Message::walk`.

- **The game first.** game#1062
  ([`1062-playdate-the-guides-words-without-colours-or-keys.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1062-playdate-the-guides-words-without-colours-or-keys.md))
  asks for a tip's text to have a second wording on the Playdate,
  chosen by the device as the button names are. No friction review: a
  missing feature. The generic shape: one text says one thing on a
  device without colour or keys and another elsewhere. A farming game's
  tutorial sign that says "click" on a PC and "press" on a handheld
  needs the same.
- ADR-0127 decides the syntax, the typed arguments (`Arg::Int`,
  `Arg::Text`, `ToArg`), the built-in `device` and `target` arguments,
  and that a text's plurals follow the rules of the language it is in.
  It amends ADR-0020, whose label values were "already text".
- **What there is today** (after 2348): `shuyi_ui::text::fill`,
  `fill_values` and `fill_styled` (`crates/ui/src/text.rs`) take
  `&[(&str, &dyn Display)]` and walk `shuyi_core::text::split_tokens`;
  `Label::Key`'s `args` are `Vec<(String, String)>`; `Ctx::text_with`,
  `text_with_keys`, `text_styled`, `text_styled_with_keys`, `label` and
  `label_styled` (`crates/ui/src/ctx.rs`) fill through `fill_styled` with
  the player's language's text from `Lang::text`. `tip::placeholders`
  (`crates/assets/src/tip.rs`) finds names between braces by hand, so a
  pack can't translate a plural message (its "same placeholders" rule in
  `lang::entries` reads the inside of a plural as one name).
- **A draft exists.** 2348's session wrote this ticket's code before the
  split and kept it on the local branch `wip/2348-full` (commit
  `c3fa207b`) of Nick's clone: `crates/ui/**`, `examples/hello-grid/**`.
  Use it if the branch is there (check each step against `main`); else
  write it from the steps.

The game pins engine `1ccb1acc` (2026-11-16). Searches of the game:
`text_with` is called in about 25 files (`crates/ui/src/screens/battle/*`,
`input.rs`, `save.rs`), `fill_values` in `profiles.rs`, `tips.rs` and
`words.rs`, `Label::Key` in `screens/battle/mode.rs` and
`widgets/menu.rs`, and `fill_placeholders` (the game's tip fill) in
`tips.rs`, `tips/view.rs` and `screens/battle/guides.rs`.

## Nick input

None.

## Scope

**In:**

- **`shuyi_ui::text`** re-exports `Arg` and `ToArg`, and names the
  argument slice `Args<'a> = [(&'a str, &'a dyn ToArg)]`:
  - `Label::Key`'s `args` are `Vec<(String, Arg)>`;
    `Label::with_arg(name, value: impl ToArg)`, and the same for
    `MenuItem::with_arg` (`crates/ui/src/widgets/menu.rs`);
  - `fill(text, lang, args: &Args, lookup)`, `fill_values(text, lang,
    args)` and `fill_styled(text, lang, args, key_args, lookup)`:
    `lang` is the language the text is in. A text that parses is filled
    through `Message::walk`: a plural or select chooses by `args`, then
    `key_args`; a `Piece::Arg` is written from `args` (plain), else from
    `key_args` or `lookup` (a `Key` span, its spaces joined with
    `NO_BREAK_SPACE` as 2302 made them), else as `{name}`; a
    `Piece::Number` in digits. A text that doesn't parse is filled as
    today, on `split_tokens`;
  - `Ask::view_with_keys` (`crates/ui/src/widgets/ask.rs`) and the
    screens that hand key names as values (`screens.rs`,
    `screens/key_bindings.rs`, `screens/layout_picker.rs`) take
    `&dyn ToArg`.
- **`Ctx`** (`crates/ui/src/ctx.rs`):
  - `text_with`, `text_with_keys`, `text_styled` and
    `text_styled_with_keys` take `&Args`; with `label` and
    `label_styled` they get the text and its language from
    `Lang::text_in` (under `effective_lang()`), fill by that language's
    rules, and give the built-in `device` and `target` after the
    caller's values and before key names;
  - `Device::message_value(self) -> &'static str`
    (`crates/ui/src/input/state.rs`): `keyboard`, or the pad's kind, one
    of `shuyi_assets::message::DEVICES`;
  - `target` is `ctx.platform.target`;
  - `Ctx::fill_text(&self, text, lang, args) -> String`: a text the
    caller has (a tip's, data text from `Lang::data_text_in`), filled as
    `text_with` fills a key's. A game's tips go through it.
  - `numbered_profile` fills by `Lang::text_in`'s language.
- **`tip::placeholders`** gives a message's argument names
  (`Message::names`), and for a text that isn't a message the names
  between braces through `split_tokens`. So `lang::entries`' "same
  placeholders" rule lets a translation wrap a `{n}` in a plural or
  drop a plural its language doesn't need, and
  `check_key_placeholders` checks a key named inside a branch.
- **The engine's own text:** `profile.message.too_many` in
  `crates/ui/src/toy/en.ron` and `examples/hello-grid/files/en.ron`
  becomes `There can be at most {n, plural, one {# profile} other {#
  profiles}}.`; the profiles screen still says "There can be at most 8
  profiles." The toy and hello-grid read their English as a plain map,
  so a unit test in each checks that every text parses.
- **Docs:** `docs/formats.md`'s language pack paragraph shows a text
  that chooses by `device`; the book's chapter on text (the docs map
  names it) shows `with_arg` with a number.
- **For the game:** breaking. `Label::Key`'s args, `Label::with_arg`,
  `MenuItem::with_arg`, `text::fill`, `fill_values`, `fill_styled`,
  `Ctx::text_with`, `text_with_keys`, `text_styled`,
  `text_styled_with_keys` and `Ask::view_with_keys` take typed values,
  and `fill`, `fill_values` and `fill_styled` take the text's language.
  Integers and strings keep compiling; a value of the game's own
  `Display` type needs `.to_string()`. The completion notes carry a
  `Breaking:` line for each, in 2010's form. The game's PR that moves
  its pin follows this one, and game#1062 can then write a tip's second
  wording as `{device, select, playdate {…} other {…}}` and show it
  through `Ctx::fill_text`.
- Public types (ADR-0039): nothing new but `Args`, a type alias.
- **Either order with 2261:** if 2261 is done, English that doesn't parse
  is already a load error where it loads through `english_from_source`,
  and the engine's plural `profile.message.too_many` passes it; if not,
  nothing to do (2261 checks it).
- **Either order with 2351:** if 2351 is done, `profile.message.too_many`
  lives in the engine's `crates/ui/lang/en/ui.ron` (hello-grid's copy is
  gone): the plural form goes there and the parse test reads that file;
  if not, change the two files above, and 2351 moves the key.
- **Either order with 2390:** if 2390 is done, the fallback for a text
  that doesn't parse is its `split_tokens` code as it is, and
  `tip::placeholders`' fallback is its `tokens`; if not, build both on
  `split_tokens` (2390 then has nothing left of them).
- **Either order with 2330:** if 2330 is done,
  `DocScreen::set_arg(id, name, value)` takes `value: impl ToArg` and
  keeps an `Arg`, as `Label::with_arg` does; if not, nothing to do (2330
  types it with `ToArg`).
- **Either order with 2352:** if 2352 is done,
  `profile.message.too_many` becomes a plural message in each engine
  pack (`crates/ui/lang/<code>/ui.ron`) too, written by machine with the
  categories of the pack's language and still `made_by: Machine`, so
  2352's upkeep test passes; if not, nothing to do (2352 writes its
  packs from the plural English).
- **Either order with 2758:** if 2758 is done, a call of `fill` or
  `Ctx::text_with` in its code takes the new arguments as every
  caller's does; if not, nothing to do.

**Out (do not do):**

- The load checks (a pack select that drops a branch, an unknown device,
  a plural category the language lacks), tips checked by form, the
  overflow list by form and `missing_forms`: 2261.
- Numbers formatted by language: `{n, number}` writes plain digits
  (ADR-0127).
- Dialogue lines and captions: they keep their `{tokens}`.
- Reading the device anywhere but the `Ctx::device` field.

**Estimated size:** about 650 lines (code 330, tests 220, docs 100):
the draft measured 490.

## Implementation steps

1. Read ADR-0127, `crates/ui/src/text.rs`, `crates/ui/src/ctx.rs`
   (`text`, `label`, `text_with` and the styled ones, `help_keys`,
   `numbered_profile`), `shuyi_assets::message` and
   `crates/assets/src/tip.rs`.
2. `shuyi_ui::text`: `Args`, the re-exports, `Label`, `fill`,
   `fill_values`, `fill_styled` on `Message::walk` with the fallback.
3. `Ctx`: the typed methods, `Lang::text_in`, `device` and `target`,
   `fill_text`; `Device::message_value`.
4. Fix the callers in `shuyi-ui` (`menu.rs`, `ask.rs`, `screens.rs`,
   `key_bindings.rs`, `layout_picker.rs`) and the tests that call
   `fill` (`text.rs`, `text/styled_tests.rs`). The menu test that
   compares `with_arg("name", 3)` with `with_arg("name", "3")`
   (`crates/ui/src/widgets/menu/tests.rs`) compares `Arg::Int(3)`.
5. `tip::placeholders` on `Message::names`.
6. The engine's English, the two parse tests, the docs.
7. The gates, and the game's call sites checked with the searches
   above.

## Acceptance criteria

- [x] With the player's language `ru` and a `ru` pack that lacks a key,
      `Ctx::text_with` on that key's English
      `{n, plural, one {# key} other {# keys}}` gives "21 keys" for 21,
      and `Ctx::label` of `Label::key(…).with_arg("n", 21)` the same.
- [x] `{device, select, playdate {A} other {B}}` shows A when the
      `Ctx::device` field is `Device::Pad(PadKind::Playdate)` and B with
      the keyboard, through both `Ctx::text_with` and `Ctx::fill_text`
      (tests in `crates/ui/src/ctx/tests.rs`); a value the caller passes
      as `device` wins; `{target}` gives `ctx.platform.target`.
- [x] Every `Device` has a `message_value` in `DEVICES`, and every value
      of `DEVICES` is some device's.
- [x] A key's name in a plural's branch is a `TextRole::Key` span.
- [x] `fill` gives today's output for text that isn't a valid message:
      the existing tests in `text.rs` pass with only the new `lang`
      argument added, and
      `a_key_name_of_two_words_is_never_split_across_lines` unchanged.
- [x] `tip::placeholders` of `{Confirm} {n, plural, one {#} other {#
      {Cancel}}}` is `Confirm`, `n`, `Cancel`; of `{a}{b} {` it is
      `a`, `b` as today.
- [x] Every text of the toy's and hello-grid's English parses (the new
      tests).
- [x] The profiles screen still says "There can be at most 8 profiles."
      (`crates/ui/src/screens/profiles/tests.rs`), and every engine
      screen's `frame-png` snapshot is unchanged.
- [x] The completion notes have a `Breaking:` line for each break under
      *For the game*, and a `Game:` line.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `fill` and `fill_styled` with a plural, a select, a key in a
  branch, and text that isn't a message; `Ctx::text_with`,
  `Ctx::label` and `Ctx::fill_text` by the text's language; `device` and
  `target`; `Device::message_value`; `tip::placeholders`; the two parse
  tests.
- Property: `fill` never panics on random text.
- Existing: the profiles screen's message test and every frame
  snapshot, unchanged.

## Completion notes

Done on 2026-11-17 from the draft on `wip/2348-full` (`c3fa207b`), which
applied cleanly to `main`; each step checked against `main`.

- `shuyi_ui::text` re-exports `Arg` and `ToArg` and names `Args<'a>`.
  `fill`, `fill_values` and `fill_styled` take the text's language and
  walk `Message::walk` for a text that parses (a plural or select
  chooses by `args`, then `key_args`; `{name}` from `args` plain, else a
  key span from `key_args` or the lookup, else `{name}`; `#` in digits),
  and `split_tokens` as before for one that doesn't. `Label::Key`'s args
  are `Vec<(String, Arg)>`; `Label::with_arg` and `MenuItem::with_arg`
  take `impl ToArg`.
- `Ctx::text_with`, `text_with_keys`, `text_styled` and
  `text_styled_with_keys` (and so `label` and `label_styled`) take
  `&Args`, read the text and its language from `Lang::text_in` under
  `effective_lang()`, and give the built-in `device`
  (`Device::message_value`) and `target` (`ctx.platform.target`) after
  the caller's values. `Ctx::fill_text(text, lang, args)` fills text the
  caller has (a tip's). `numbered_profile` fills by `text_in`'s language.
- `tip::placeholders` gives `Message::names` for a message and the
  `Part::Token`s of `split_tokens` otherwise, so `lang::entries`' "same
  placeholders" rule and `check_key_placeholders` see inside a branch.
- `profile.message.too_many` is a plural in the toy's and hello-grid's
  English; a test in each checks that every English text parses.
- Tests: a plural, a select, a key in a branch and a non-message through
  `fill` and `fill_styled`; a property test that `fill` never panics;
  `Ctx::text_with`, `label` and `fill_text` by the text's language with
  a `ru` player and a pack lacking the key; `device`, a caller's
  `device` winning, and `target`; every `Device` against `DEVICES`;
  `tip::placeholders` of a message; a misspelled action inside a
  plural's branch fails `check_key_placeholders`. The profiles screen's
  message test and the frame snapshots pass unchanged.
- Docs: `docs/formats.md` says which texts have `device` and `target`;
  the book's chapter 6 (*Games and screens*, text by key) shows a plural
  chosen by a number through `text_with` and `with_arg`, `device` and
  `target`, and `ctx.fill_text`.

Deviations:

- The draft also checked tips by form in `tip::from_source` and let
  `device` and `target` through its placeholder check. Both are under
  this ticket's *Out* (2261), so they are left out; 2261's *In* lists
  them, and its clause on 3218 now says so. Until 2261 lands, a game's
  tip that is a `device` select needs its `placeholder` rule to admit
  `device`, and its raw text is measured whole.
- The draft named the messages ADR "ADR-0124"; it is ADR-0127 (ADR-0124
  is key caps).
- Merged with 3217 (done), which added `fill_styled_with` (a lookup
  that gives spans, for the cursor keys drawn as one arrows picture) and
  moved `Ctx` onto it: it takes the text's language and `&Args` too, and
  walks the message as `fill_styled` does, which now calls it.
- Either-order clauses: 2261, 2351, 2390, 2330, 2352 and 2758 are open,
  so the *if not* branch of each was taken: the two English files
  changed, both fallbacks built on `split_tokens`, and nothing else.
  Their clauses on 3218 now say what holds.

Breaking: shuyi-ui: text::Label::Key, text::Label::with_arg, widgets::menu::MenuItem::with_arg: a label's args are `Vec<(String, Arg)>` and `with_arg` takes `impl ToArg`; integers and strings compile as before, a value of the game's own `Display` type is passed as `value.to_string()`, and a game that builds `Label::Key { args, .. }` itself holds `Arg`s.

```migration
since: 0.1.0
what: A label's values are typed (an Arg), so a plural can choose by a number; with_arg takes impl ToArg.
old: "Label::with_arg(name, value: impl Display); Label::Key { args: Vec<(String, String)> }"
new: "Label::with_arg(name, value: impl ToArg); Label::Key { args: Vec<(String, Arg)> }"
before: |
  let label = Label::key("unit.class").with_arg("class", unit.class);
  let raw = Label::Key { key: "a".into(), args: vec![("n".into(), n.to_string())] };
after: |
  let label = Label::key("unit.class").with_arg("class", unit.class.to_string());
  let raw = Label::Key { key: "a".into(), args: vec![("n".into(), Arg::Int(n))] };
find: ["n.to_string())] }"]
```

Breaking: shuyi-ui: text::fill, text::fill_values, text::fill_styled, text::fill_styled_with: each takes the language the text is in after the text, and its values as `&Args` (`&[(&str, &dyn ToArg)]`); a game passes the tag (`Lang::text_in` gives it with the text) and casts a value to `&dyn ToArg`, or passes its own `Display` type as a `String`.

```migration
since: 0.1.0
what: fill, fill_values and fill_styled take the text's language, whose plural rules a message follows, and typed values.
old: "fill_values(text: &str, args: &[(&str, &dyn Display)])"
new: "fill_values(text: &str, lang: &str, args: &Args)"
before: |
  let line = fill_values(text, &[("n", &n as &dyn Display)]);
after: |
  let line = fill_values(text, lang, &[("n", &n as &dyn ToArg)]);
find: ["as &dyn Display"]
```

Breaking: shuyi-ui: Ctx::text_with, Ctx::text_with_keys, Ctx::text_styled, Ctx::text_styled_with_keys: their values are `&Args` (`&dyn ToArg`); integers and strings compile as before, a slice typed `&dyn Display` becomes `&dyn ToArg`, and a value of the game's own `Display` type is passed as a `String`.

```migration
since: 0.1.0
what: A screen's text takes typed values, so a plural chooses by a number.
old: "Ctx::text_with(key, args: &[(&str, &dyn Display)])"
new: "Ctx::text_with(key, args: &Args)"
before: |
  let values = [("count", &n as &dyn Display), ("name", &unit.name)];
  ctx.text_with("battle.line", &values)
after: |
  let values = [("count", &n as &dyn ToArg), ("name", &unit.name.to_string())];
  ctx.text_with("battle.line", &values)
find: ["as &dyn Display"]
```

Breaking: shuyi-ui: widgets::ask::Ask::view_with_keys: its values are `&[(&str, &dyn ToArg)]`; a game casts to `&dyn ToArg` where it cast to `&dyn Display`.

```migration
since: 0.1.0
what: A question's view takes typed values, as Ctx::text_with does.
old: "Ask::view_with_keys(ctx, keys, args: &[(&str, &dyn Display)])"
new: "Ask::view_with_keys(ctx, keys, args: &[(&str, &dyn ToArg)])"
before: |
  ask.view_with_keys(ctx, keys, &[("name", &name as &dyn Display)])
after: |
  ask.view_with_keys(ctx, keys, &[("name", &name as &dyn ToArg)])
find: ["as &dyn Display"]
```

Breaking: hello-grid: tests/golden/hello-grid.ron "asset en.ron" and "fingerprint files": the example's English has the plural `profile.message.too_many`; only the example's table moves (and `tests/sessions/stage.session.ron`'s content stamp with it), and a game changes nothing.

Behaviour: shuyi-ui: a screen's text that parses as a message is written by ICU's rules: two apostrophes show one, a quoted brace shows a brace, and `{n, number}` shows the digits; a text that doesn't parse is filled as before.

Behaviour: shuyi-ui: `{device}` and `{target}` in screen text are filled by the device pressed last (`keyboard`, `xbox`, `playdate`…) and the target's id unless the caller passes values of those names; they were left as written.

Behaviour: shuyi-ui: a key's text a pack lacks shows the source's text by the source language's plural rules (`Lang::text_in`), not the player's.

Behaviour: shuyi-assets: `tip::placeholders` of a message gives its arguments' names, each once, inside branches too (`{n, plural, one {#} other {# {Cancel}}}` gives `n` and `Cancel`), where it gave the raw inside of each pair of braces.

Behaviour: shuyi-ui: the engine's `profile.message.too_many` is a plural message in the toy's and hello-grid's English; it still reads "There can be at most 8 profiles."

Searches of the game (`gh search code --repo Zafnok/visions-of-shuyi`):
`fill_values(` in `crates/ui/src/tips.rs`, `profiles.rs` and `words.rs`;
`Label::Key` in `widgets/menu.rs` and `screens/battle/mode.rs`;
`fill_placeholders` in `tips.rs`, `tips/view.rs` and
`screens/battle/guides.rs`; `dyn Display` in `tips.rs`, `words.rs`,
`screens/preparations.rs`, `screens/class_change.rs` and
`screens/battle/` (`magic.rs`, `rewind.rs`, `mod.rs`, `map_menu.rs`).
Each is a call the breaks above change. `{device}`: no hit. `{target}`:
four `battle.rewind.*` texts in `assets/lang/en/ui.ron`, and
`screens/battle/rewind.rs` passes its own `target` in each call, which
wins over the built-in. No doubled apostrophe in the game's English.

Game: game#1062, whose PR moves the pin to this change, fixes the calls above and shows its tip's second wording through `Ctx::fill_text`.

Nick: nothing to decide.
