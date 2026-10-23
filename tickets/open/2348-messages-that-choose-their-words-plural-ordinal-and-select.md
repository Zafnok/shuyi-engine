---
id: "2348"
title: "Messages that choose their words: plural, ordinal and select in a subset of ICU MessageFormat, and typed label arguments"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2348 — Messages that choose their words: plural, ordinal and select

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

**What the benchmarks have:**

- Unreal: `FText` format patterns have plural and ordinal forms and
  gender forms, and `FText::AsNumber` writes numbers by culture
  (https://dev.epicgames.com/documentation/en-us/unreal-engine/text-localization-in-unreal-engine).
- Unity: the Localization package's Smart Strings have a Plural
  formatter on CLDR's rules, and a Choose formatter
  (https://docs.unity3d.com/Packages/com.unity.localization@1.5/manual/Smart/Plural-Formatter.html).
- Godot: `tr_n()` and the plural forms of gettext PO files
  (https://docs.godotengine.org/en/stable/tutorials/i18n/localization_using_gettext.html).
- Bevy: nothing built in; `bevy_fluent` is a third-party crate.

**What Shuyi has today:**

- `shuyi_ui::text::fill` (`crates/ui/src/text.rs`, line 38) replaces
  each `{name}` with a `Display` value, or with what `lookup` gives (a
  key's name through `GameData::key_text`). Nothing in a text can choose
  between wordings.
- `Label::Key { key, args: Vec<(String, String)> }` (`text.rs`, lines
  78 to 83). ADR-0020 made the values "already text", so the number is
  gone before the skin paints, and no rule can pick a word by it.
  `Ctx::label`, `Ctx::text_with` and `Ctx::text_with_keys`
  (`crates/ui/src/ctx.rs`, lines 379 to 409) all fill through `fill`.
- `Ctx` fills only text it looks up by key. A tip's text is data text,
  so a game fills it with its own code (game#1062 names the game's
  `fill_placeholders`).
- `Lang::text` (`crates/assets/src/lang.rs`, line 229) and
  `Lang::data_text` (`crates/assets/src/lang/source.rs`, line 150) give
  English when a pack has no current entry for a key, and don't say
  which language they gave.
- `shuyi_assets::lang`: `entries` (`lang.rs`, line 406) checks that a
  pack entry's text has "the same placeholders" as its source, through
  `tip::placeholders`. `PackRules::variants` gives a dialogue line one
  `text_<variant>` field per variant the game names
  (`crates/assets/src/lang/lines.rs`). Screen text (`ui.ron`) and data
  text (`data.ron`) have nothing like it.
- `english_from_source` (`lang.rs`, line 306) checks English screen
  text only when the languages load through `from_files` or
  `LangPackRon`. The toy (`crates/ui/src/toy.rs`, line 453), hello-grid
  (`Ron<English>`, `examples/hello-grid/src/lib.rs`) and both templates
  (`parse_ron`) read their `en.ron` as a plain map.
- `shuyi_assets::tip::from_source` (`crates/assets/src/tip.rs`) checks
  a tip's raw text against its box (`TipLimits`).
- `HelpKeys` (`crates/ui/src/input/help_keys.rs`) names keys or a pad's
  buttons by the device the player pressed last (the `Ctx::device`
  field, `Device` in `crates/ui/src/input/state.rs`, `PadKind` in
  `crates/ui/src/input/pad.rs`, which has a `Playdate` kind). Only key
  names follow the device; the words around them can't.
- The engine's own text needs a plural: `profile.message.too_many`,
  "There can be at most {n} profiles." (`crates/ui/src/toy/en.ron` and
  `examples/hello-grid/files/en.ron`, filled at
  `crates/ui/src/screens/profiles.rs`, line 382).

**What is planned:** nothing in `tickets/` plans plural rules, a choice
by argument, or numbers by language. 2302 measures pack text against a
width (`max_cells`). 2202 brings Yarn Spinner, whose markup has its own
plural and select. 2347 builds `fill` on a styled `fill_styled`. 2351
moves the engine's keys into an English file of the engine's own.

**Why it matters:**

- **The game first.** game#1062
  ([`1062-playdate-the-guides-words-without-colours-or-keys.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1062-playdate-the-guides-words-without-colours-or-keys.md))
  asks for a tip's text to have a second wording on the Playdate,
  chosen by the device as the button names are. Tips are an engine
  format (`shuyi_assets::tip`), so the engine gives the way to choose.
  No friction review: a missing feature. The generic shape: one text in
  the language file says one thing on a device without colour or keys
  and another elsewhere. A farming game's tutorial sign that says
  "click" on a PC and "press" on a handheld needs the same.
- The game's own languages (English, Japanese, Chinese: game ADR-0045,
  game#0237, game#0239) need no plural rules. Japanese and Chinese have
  one form, and the game writes English's two with two keys
  (game#0234). Game ADR-0045 turned down Fluent and gettext as more
  machinery than a keyed table needs.
- **Other games.** Russian, Polish and Ukrainian have three or four
  plural forms, and French counts 0 as singular, so "{n} keys" is
  wrong there. An agent translating someone else's game can't get it
  right without engine support. Choosing in the language file keeps
  ADR-0002 §6 (no player sentence in code) true in every language.
- **The end goal** (2026-10-19): one game with many ports, made and
  translated by agents. One text file serves the PC and the Playdate,
  and ICU MessageFormat is the syntax LLMs, Crowdin and Weblate already
  know.

## Nick input

None. The game's languages are fixed in game ADR-0045, and the syntax
and the plural table are technical.

## Scope

**In:**

- **An ADR** (the next free number in `docs/adr/`; other tickets of the
  2026-10-20 batch also write ADRs), "Messages choose their words".
  It amends ADR-0020, whose label values are "already text". It
  decides:
  - **The syntax**, a strict subset of ICU MessageFormat 1:
    - `{name}` as today: a value, or a key's name (`{Confirm}`,
      `{Cursor}`);
    - `{name, plural, …}` and `{name, selectordinal, …}`: branches
      `=N {…}` and CLDR's category names (`zero`, `one`, `two`, `few`,
      `many`, `other`), with `#` in a branch for the number;
    - `{name, select, value {…} … other {…}}`;
    - `{name, number}`: reserved. It parses and writes plain digits,
      like `{name}`, until a follow-up formats numbers by language;
    - every plural, selectordinal and select has an `other` branch;
    - branches may nest;
    - `offset:`, `choice`, `date`, `time`, `spellout`, `duration`,
      skeletons (`::`) and any other type are errors when a language
      file loads.
  - **Quoting**, ICU's "double optional" apostrophe mode: `''` is one
    apostrophe; a `'` right before `{`, `}`, or `#` inside a plural
    branch starts quoted text up to the next lone `'`; every other
    apostrophe is a plain letter. So "l'épée" and "don't" need nothing.
    MessageFormat 1's strict apostrophe rule trips translators up.
  - **Typed arguments:** `Arg::Int(i64)` and `Arg::Text(String)`. A
    plural or selectordinal on an `Int` picks by its absolute value; on
    a `Text` it picks `other`. A select compares text (an `Int` by its
    digits). `{name}` and `#` write an `Int` as plain digits, as
    `to_string()` does today, so no snapshot changes.
  - **Built-in arguments:** `device`, the device pressed last
    (`keyboard`, `xbox`, `playstation`, `playstation4`, `nintendo`,
    `playdate`, `generic`), and `target`, the target profile's id
    (ADR-0008), reserved until 3053 gives the target at run time. A
    value the caller passes under the same name wins. A branch by
    device changes the words, never names a key: keys still come from
    `{Action}` placeholders (`AGENTS.md` rule 8).
  - **Whose rules:** the rules of the language the text is in, not the
    player's. A key a `ru` pack lacks shows English, with English's
    rules, so 21 gives "21 keys", not `ru`'s `one` branch.
  - **The plural rules:** our own integer table, written from CLDR's
    `plurals.xml` and `ordinals.xml` (the release current on the day,
    named in the source), for `en`, `fr`, `de`, `es`, `it`, `pt`, `ru`,
    `uk`, `pl`, `ja`, `zh` and `ko`. A language not in the table has
    only `other` and `=N`. Arabic waits for right-to-left text (2908).
    Adding a language is a row and its CLDR samples.
  - Whether `LangStatus::missing_forms` (below) leaves out a category
    that only numbers of a million or more select (CLDR's integer
    `many` in French, Spanish, Italian and Portuguese), which a game
    rarely shows. Recommended: leave it out of the list, and still
    accept it in a pack.
  - **Alternatives:** MessageFormat 2 (Unicode's newer syntax, which
    fewer tools and models know); Fluent (a file format of its own, and
    game ADR-0045 turned it down); the `icu_plurals` crate (Unicode-3.0,
    which `deny.toml` allows, but it brings ICU4X's data provider, and
    the web and Playdate builds count bytes, while we need integers in a
    dozen languages); gettext's plural expressions; Unity-style smart
    strings.
- **`shuyi_assets::message`**, `no_std` with `alloc`
  (`crates/assets/src/message.rs`, the table in
  `crates/assets/src/message/plural.rs`):
  - `Message::parse(&str) -> Result<Message, MessageError>`; an error
    gives the character offset and what was expected;
  - `Message::walk(&self, lang: &str, value: impl Fn(&str) -> Option<Arg>, each: impl FnMut(Piece<'_>))`:
    picks each plural, selectordinal and select branch by `value`, and
    hands out what the chosen branches hold, in order:
    `Piece::Text(&str)` for literal text, `Piece::Number(i64)` for a
    `#`, and `Piece::Arg(&str)` for each simple `{name}`. A plural,
    selectordinal or select with no value takes `other`. The caller
    decides how a simple argument is written, so `fill` can look up key
    names (and 2347 can make them key spans);
  - `Message::format(&self, lang: &str, value: impl Fn(&str) -> Option<Arg>) -> String`,
    built on `walk`; a simple argument with no value is written back as
    `{name}`, as today;
  - `Message::args()`: each argument the message uses, with its kind
    (simple, plural, selectordinal, select with its branch values);
  - `Message::forms()`: every text the message can show, each argument
    and `#` written as `{name}`. More than 64 forms is a parse error
    ("too many forms");
  - `Arg`, and a `ToArg` trait implemented for every integer type,
    `str`, `String`, `char`, `Arg` and references to them. A `u64`,
    `usize`, `u128` or `i128` too big for `i64` becomes `Arg::Text` of
    its digits;
  - `plural::{Category, cardinal(lang, n), ordinal(lang, n), categories(lang, Kind)}`
    over `u64`.
- **`shuyi_assets::lang`:** `Lang::text_in(&self, code, key) -> (&str, &str)`
  and `Lang::data_text_in(&self, code, key, english) -> (&str, &str)`:
  the text and the code of the language it is in (`en` when it falls
  back). `Lang::text` and `Lang::data_text` stay, built on them.
- **`shuyi_ui::text`** (re-exports `Arg` and `ToArg`):
  - `Label::Key`'s `args` are `Vec<(String, Arg)>`;
    `Label::with_arg(name, value: impl ToArg)`, and the same for
    `MenuItem::with_arg` (`crates/ui/src/widgets/menu.rs`, line 114);
  - `fill(text, lang, args: &[(&str, &dyn ToArg)], lookup)` and
    `fill_values(text, lang, args)` format a message through `walk`:
    each `Piece::Arg` by its value in `args`, else by `lookup`, else
    written back as `{name}`. A text that doesn't parse is filled as
    today (simple placeholders only), so text from outside the language
    files never panics.
- **`Ctx`** (`crates/ui/src/ctx.rs`):
  - `Ctx::text_with` and `Ctx::text_with_keys` take
    `&[(&str, &dyn ToArg)]`, so call sites that pass `&n` and `&"text"`
    keep compiling. They, and `Ctx::label`, get the text and its
    language from `Lang::text_in` (under `effective_lang()`), use that
    language's rules, and fill `device` after the caller's values and
    before key names. `Device` gets the method that gives its value;
  - `Ctx::fill_text(&self, text: &str, lang: &str, args: &[(&str, &dyn ToArg)]) -> String`:
    a text the caller already has (a tip's, other data text from
    `Lang::data_text_in`) filled as `text_with` fills a key's: `lang`'s
    rules, `device`, and key names. A game's tips go through it.
- **Checks when the languages load:**
  - `english_from_source`: each English text parses; an error names
    the file, the key and the problem. The toy and hello-grid read their
    English as a plain map, so a unit test in each checks that every
    text of `crates/ui/src/toy/en.ron` and
    `examples/hello-grid/files/en.ron` parses;
  - `entries` (packs' `ui.ron` and `data.ron`):
    - `text` and `source` both parse;
    - the same argument names as today's placeholder rule, now through
      `Message::args`. A translation may wrap a simple `{n}` in a plural
      or drop a plural its language doesn't need;
    - a select in the source keeps every branch value in the text's
      select on the same argument, so a translation can't drop a
      device's wording;
    - a select on `device` uses only the device values (a typo is an
      error);
    - a plural or selectordinal uses only categories its language has
      (`fr` has no `few`). The pack's code reaches `entries` from
      `from_files` and `LangPackRon`, which know its directory.
  - `LangStatus` gets `missing_forms`: each pack entry whose plural or
    selectordinal lacks a category its language has, with the
    categories it lacks. It fails nothing, as `missing` and `stale` do
    (a missing form shows `other`). `lang-status`
    (`crates/cli/src/lang_status.rs`) prints it as a `missing forms:`
    list after the stale list, only when it isn't empty, so today's
    reports don't change.
  - Tips: `tip::placeholders` gives a message's argument names, and
    `tip::from_source` checks each form's lines against `TipLimits`
    instead of the raw text. A tip text that doesn't parse is an error
    naming the tip. `device` and `target` are always known placeholders,
    whatever `TipRules::placeholder` says.
- **The engine's own text:** `profile.message.too_many` in
  `crates/ui/src/toy/en.ron` and `examples/hello-grid/files/en.ron`
  becomes a plural message. The profiles screen's text doesn't change
  (`MAX_PROFILES` is 8).
- **Docs and credit:**
  - `docs/formats.md`, the language pack paragraph: the syntax in a few
    lines, an example, and a link to the ADR;
  - ADR-0020's status says it is amended, and `docs/adr/README.md`
    lists the new ADR;
  - CLDR's rules are Unicode data under Unicode-3.0. Add a row to
    `THIRD_PARTY_ASSETS.md`, with the license text beside the table
    (`crates/assets/src/message/CLDR-LICENSE.txt`, from
    `https://www.unicode.org/license.txt`).
- **Either order:**
  - **2351** (the engine's own English): if it is done,
    `profile.message.too_many` lives in the engine's
    `crates/ui/lang/en/ui.ron` (hello-grid's copy is gone), so the
    plural form goes there, and the parse test reads that file. If not,
    change the two files above, and 2351 moves the key.
  - **2347** (styled text): both change `fill`. If 2347 is done, `fill`
    is `fill_styled(…).plain()`: build `fill_styled` on `walk`, so a
    key's name in any branch is still a `Key` span, and its `args` take
    `&dyn ToArg`. If not, 2347 builds `fill_styled` on `walk`.
  - **2302:** if it is done, its `max_cells` overflow list measures each
    form (`Message::forms`) and reports the widest; do that here. If
    not, 2302 does it.
  - **3053:** if it is done, fill `target` from `ctx.platform.target`,
    beside `device`. If not, `target` stays reserved, and 3053 fills
    it.
  - **2226:** if it is done, look the plural table up by
    `LangCode::language()` (`pt-BR` uses `pt`), and `Lang::text_in`
    gives the code of whichever language in the fallback chain gave the
    text. If not, look it up by `LangCode` as it is, and 2226 moves the
    lookup to the subtag.
  - **2353** (a pseudo-language): if it is done, its `pseudo_text`
    changes only a message's literal text and keeps its syntax (a
    plural's `other` and its braces stay as written). If not, 2353 does
    that through `walk` or `forms`.
  - **Public types (ADR-0038):** `Arg`, `Piece`, `Category`, `Kind`,
    `MessageError` and the new `LangStatus` field follow it
    (`#[non_exhaustive]` and constructors).
- **For the game:** breaking. `Label::Key`'s args,
  `Label::with_arg`, `MenuItem::with_arg`, `text::fill`,
  `text::fill_values`, `Ctx::text_with` and `Ctx::text_with_keys` take
  typed values, and `fill` and `fill_values` take the text's language.
  Integers and strings keep compiling; a value of the game's own
  `Display` type needs `.to_string()`. `LangStatus` has a new field.
  Where English loads through `english_from_source`, English text that
  no longer parses (an unclosed `{`, a comma inside braces) is now a
  load error naming its key; elsewhere it is filled as today. The
  completion notes carry a `Breaking:` line for each in 2010's form.
  The game's PR that moves its pin follows this one, and game#1062 can
  then write a tip's second wording as
  `{device, select, playdate {…} other {…}}` and show it through
  `Ctx::fill_text`.

**Out (do not do):**

- Numbers formatted by language (digit grouping, decimal marks, other
  digits): `{n, number}` is reserved and writes plain digits. A
  follow-up comes when a game asks; separators such as U+202F need
  glyphs the cell fonts may lack (2350).
- Fixed-point arguments: `Fx` comes with 1119 (parked). Decimals,
  currencies, dates and times, and gender inference.
- Dialogue lines and captions: they keep their `{tokens}` and
  `text_<variant>` fields (`PackRules::variants`). The syntax in
  dialogue is a follow-up, with 2202's Yarn markup.
- Right-to-left and shaped scripts (2908), and Arabic's plural rules
  with them.
- Translator exchange files (2227, parked): a message goes out to PO as
  it is written, its branches inside the string.
- The atlas check (2350): it walks `Message::forms` when it comes.
- A value for `target` before 3053; reading the device anywhere but
  the `Ctx::device` field.
- Moving the templates' or the game's English onto
  `english_from_source`.

## Implementation steps

1. Read ADR-0020, `crates/ui/src/text.rs`, `crates/ui/src/ctx.rs`
   (`text`, `label`, `text_with`, `text_with_keys`, `help_keys`,
   `numbered_profile`), `crates/assets/src/lang.rs`
   (`english_from_source`, `entries`, `pack_from_files`, `from_files`,
   `Lang::text`, `Lang::status`), `crates/assets/src/lang/source.rs`
   (`data_text`), `crates/assets/src/lang/formats.rs`,
   `crates/assets/src/tip.rs` and `crates/cli/src/lang_status.rs`.
2. Write the ADR.
3. Write `shuyi_assets::message`: the parser with quoting, `walk`,
   `format`, `args` and `forms`. Then the plural and ordinal table, with
   CLDR's integer samples for each language and category as a fixture.
4. Add `Lang::text_in` and `Lang::data_text_in`, with `text` and
   `data_text` built on them.
5. Make the arguments typed in `shuyi_ui::text`, `Ctx` and the menu, add
   `device` and `Ctx::fill_text`, and fix the callers in `shuyi-ui`.
   The menu test that compares `with_arg("name", 3)` with
   `with_arg("name", "3")` (`crates/ui/src/widgets/menu/tests.rs`,
   line 231) now compares `Arg::Int(3)`. hello-grid and the templates
   pass integers and strings, so they compile unchanged.
6. Add the load checks, `LangStatus::missing_forms` and its
   `lang-status` lines, and the tip checks by form.
7. Change the English of `profile.message.too_many`, add the toy's and
   hello-grid's parse tests, the docs and the credit, and do the
   either-order items that apply.
8. Run the gates (they build `shuyi-assets` and `shuyi-ui` for
   `thumbv7em-none-eabihf`).

The PR should stay near 600 changed lines without the fixture. If it
passes about 800, move step 6's `LangStatus::missing_forms` and its
`lang-status` lines to a follow-up ticket (`write-ticket` skill, After
MVP, blocked by 2348), and say so in the completion notes.

## Acceptance criteria

- [ ] The ADR is in `docs/adr/` and its README, and ADR-0020 says it
      is amended.
- [ ] In `en`, `{n, plural, one {# key} other {# keys}}` gives "1 key"
      and "2 keys". In `ru`, a message with `one`, `few`, `many` and
      `other` picks `one` for 1 and 21, `few` for 2 and 22, and `many`
      for 5 and 11. In `fr`, 0 and 1 pick `one` (unit tests).
- [ ] In `en`, `{n, selectordinal, one {#st} two {#nd} few {#rd} other {#th}}`
      gives 1st, 2nd, 3rd, 4th, 11th, 12th, 13th, 21st and 112th.
- [ ] For every language in the table, each integer sample CLDR lists
      for a category, cardinal and ordinal, gets that category (the
      fixture).
- [ ] `=0` wins over a category; `#` writes the number; `'{'` and `''`
      come out as `{` and `'`; "l'épée" needs no quoting.
- [ ] With the player's language `ru` and a `ru` pack that lacks a key,
      `Ctx::text_with` on that key's English
      `{n, plural, one {# key} other {# keys}}` gives "21 keys" for 21.
- [ ] `{device, select, playdate {A} other {B}}` shows A when the
      `Ctx::device` field is `Device::Pad(PadKind::Playdate)` and B with
      the keyboard, through both `Ctx::text_with` and `Ctx::fill_text`
      (tests in `crates/ui/src/ctx/tests.rs`).
- [ ] Each of these is a load error naming the file and the key: an
      English text that doesn't parse; a pack select that drops a
      branch value of its source; a select on `device` with an unknown
      value; a plural category the pack's language lacks.
- [ ] `lang-status` lists a `ru` pack plural that lacks `few` under
      missing forms, and the pack still loads (a fixture pack). The
      existing tests in `lang_status.rs` pass unchanged.
- [ ] A tip whose text is a select of two short wordings passes the box
      check, and one whose longer form is too wide fails, naming the
      tip.
- [ ] `fill` gives today's output for text that isn't a valid message:
      the existing tests in `text.rs` pass with only the new `lang`
      argument added.
- [ ] Every text of the toy's and hello-grid's English parses (the new
      tests).
- [ ] The profiles screen still says "There can be at most 8
      profiles." (`crates/ui/src/screens/profiles/tests.rs`, line 232),
      and every engine screen's `frame-png` snapshot is unchanged.
- [ ] The completion notes have a `Breaking:` line for each break
      under *For the game*.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the parser (each construct, each error, nesting, quoting);
  `walk` and `format` with each kind of argument and with none; `args`;
  `forms` and its cap; `ToArg` on a `u64` past `i64::MAX`; the plural
  and ordinal table; `Lang::text_in` and `Lang::data_text_in` with and
  without a current pack entry; `device`; `Ctx::fill_text`; each load
  check; the tip check by form; the `lang-status` missing forms list;
  the toy's and hello-grid's English parsing.
- Property: `Message::parse` and `fill` never panic on random text (as
  ADR-0007 asks of every format's input); `cardinal` and `ordinal`
  always give a category in `categories(lang, kind)`; for a parsed
  message and any values, `format` gives one of `forms()` with its
  arguments filled in.
- Fixture: CLDR's integer samples, per language, category and kind.
- Existing: the profiles screen's message test and every frame
  snapshot, unchanged.

## Completion notes

*(Filled in by the session that completes the ticket.)*
