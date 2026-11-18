# ADR-0127: Messages choose their words: a subset of ICU MessageFormat 1, our own plural table, and typed arguments

- **Status:** Accepted; §4 and §9 amended by ADR-0131 (a text's form learned when it loads)
- **Date:** 2026-11-16
- **Related tickets:** 2348; 3218 (typed arguments in `shuyi-ui`), 2261 (the load checks), game#1062
- **Amends:** ADR-0020 (a label's values are typed, not "already text")

## Context

A text in a language file can't choose between wordings. `{n} keys` is
wrong for 1 in English, and Russian, Polish and Ukrainian have three or
four plural forms where English has two; French counts 0 as singular.
The game asks for a tip that says one thing on the Playdate and another
elsewhere (game#1062), chosen by the device as the key names are. Every
engine compared on 2026-10-20 has a way: Unreal's `FText` plural and
gender forms, Unity's Smart Strings (a Plural formatter on CLDR's rules,
a Choose formatter), Godot's `tr_n()` and gettext's plural forms. Bevy
has none built in (`bevy_fluent` is a third-party crate).

ADR-0020 made a label's values "already text", so a number is a string
before the skin paints and no rule can choose by it. ADR-0002 §6 keeps
every player sentence out of code: the choice has to live in the
language file, where an agent translating someone else's game can get it
right. The game's own languages (English, Japanese, Chinese, game
ADR-0045) need no plural rules, and game ADR-0045 turned down Fluent
and gettext as more machinery than a keyed table needs.

## Decision

1. **The syntax is a strict subset of ICU MessageFormat 1**
   (`shuyi_assets::message`):
   - `{name}`, as before: a value, or a key's name (`{Confirm}`);
   - `{name, plural, …}` and `{name, selectordinal, …}`: branches
     `=N {…}` and CLDR's categories (`zero`, `one`, `two`, `few`,
     `many`, `other`); `#` in a branch is the number of the nearest
     plural around it;
   - `{name, select, value {…} … other {…}}`;
   - `{name, number}` is reserved: it writes plain digits, like
     `{name}`, until a follow-up formats numbers by language;
   - every plural, selectordinal and select has an `other` branch;
     branches nest, at most 8 deep; a message shows at most 64 texts
     (`Message::forms`);
   - `offset:`, `choice`, `date`, `time`, `spellout`, `duration`, a
     number's style or skeleton (`::`) and any other type are parse
     errors, with the character offset and what was expected;
   - a lone `}` outside every argument is a letter, as the old fill
     treated it.
2. **Quoting is ICU's "double optional" apostrophe mode:** `''` is one
   apostrophe; a `'` right before `{`, `}` or (in a plural's branch) `#`
   starts quoted text up to the next lone `'`; every other apostrophe is
   a letter. "l'épée" and "don't" need nothing. MessageFormat 1's strict
   mode, where every apostrophe quotes, trips translators up.
3. **Typed arguments:** `Arg::Int(i64)` and `Arg::Text(String)`
   (`#[non_exhaustive]`: fixed point comes with 1119), handed in through
   `ToArg`, which every integer type, `str`, `String`, `char`, `Arg` and
   references to them implement. An integer too big for `i64` becomes
   `Arg::Text` of its digits. A plural or selectordinal on an `Int` picks
   `=N` by its value, else its category by its absolute value; on a
   `Text`, or with no value, it picks `other`. A select compares text,
   an `Int` by its digits. `{name}` and `#` write an `Int` in plain
   digits, as `to_string()` did, so no text a game shows changes.
4. **The walk is the caller's:** `Message::walk(lang, value, each)` hands
   out `Piece::Text`, `Piece::Number` (a `#`) and `Piece::Arg` (a
   `{name}`) of the chosen branches, so the UI decides how a simple
   argument is written: a key's name in a key span (ADR-0122) from any
   branch. `Message::format` is the plain walk, writing an argument with
   no value back as `{name}`.
5. **Built-in arguments:** `device` (`DEVICES`: `keyboard`, `xbox`,
   `playstation`, `playstation4`, `nintendo`, `playdate`, `generic`), the
   device pressed last, and `target`, the target profile's id (ADR-0008,
   `ctx.platform.target`). A value the caller passes under the same name
   wins. A branch by device changes the words, never names a key: keys
   still come from `{Action}` placeholders (`AGENTS.md` rule 8). The UI
   fills them (3218).
6. **Whose rules:** a text's plurals follow the rules of the language the
   text is in, not the player's. `Lang::text_in` and `Lang::data_text_in`
   give a text with the tag of the language that gave it, walking the
   fallback chain as `text` and `data_text` do (2226),
   so a key a `ru` pack lacks shows English with English's rules: 21
   gives "21 keys", not Russian's `one` branch.
7. **The plural rules are our own integer table**
   (`shuyi_assets::message::plural`), written from CLDR 49's
   `plurals.xml` and `ordinals.xml`, for `de`, `en`, `es`, `fr`, `it`,
   `ja`, `ko`, `pl`, `pt` (and `pt-PT`), `ru`, `uk` and `zh`, over
   `u64`. A tag is looked up whole, then by its language (`pt-BR` uses
   `pt`). A language not in the table has only `other` (and `=N`).
   Adding one is a row and its CLDR samples in the fixture
   `plural_samples.txt`, which a test checks every sample of. Arabic
   waits for right-to-left text (2908). CLDR's data is Unicode-3.0
   (`CLDR-LICENSE.txt` beside the table, a row of
   `THIRD_PARTY_ASSETS.md`).
8. **Checks when the languages load** (2261): English and pack texts
   parse; a pack keeps its source's arguments and select branches, and
   uses only its language's categories; a select on `device` names
   devices; tips and `max_cells` are measured by form. A missing
   category is listed by `lang-status`, not an error (a missing form
   shows `other`), and the integer `many` of French, Spanish, Italian
   and Portuguese (only numbers of a million or more pick it) is left
   out of that list and still accepted.
9. **A text that doesn't parse** is filled as before messages (each
   `{name}`, an unclosed `{` as text), so text from outside the language
   files never panics.

## Consequences

- One language file serves the PC and the Playdate, and Russian or
  Polish text can be right; the choice stays in the file, where
  translators and agents work, in a syntax LLMs, Crowdin and Weblate
  already know.
- A label's values are typed (ADR-0020 amended): a game passing its own
  `Display` type writes `.to_string()`. Integers and strings keep
  compiling (3218 carries the `Breaking:` lines).
- English that no longer parses (`{}`, an unclosed `{`, a comma inside
  braces) becomes a load error where it loads through
  `english_from_source` (2261).
- The table is a dozen rows, no data provider and no new crate: the web
  and Playdate builds stay small. A thirteenth language is a row and its
  samples.
- Numbers are plain digits: grouping and other digits wait for a game
  that asks, and for glyphs the cell fonts may lack (2350).
- Dialogue lines and captions keep their `{tokens}`; Yarn's markup
  (2202) brings choices to dialogue.

## Alternatives considered

- **MessageFormat 2** (Unicode's newer syntax): fewer tools and models
  know it, and its `.match` blocks are longer for the same choice.
- **Fluent:** a file format of its own, which game ADR-0045 turned down.
- **The `icu_plurals` crate** (Unicode-3.0, which `deny.toml` allows):
  it brings ICU4X's data provider and data, while we need integers in a
  dozen languages, and the web and Playdate builds count bytes.
- **gettext's plural expressions** (`tr_n`): a C expression per language
  and two keys per text, and no select.
- **Unity-style smart strings:** a syntax of one engine's, which models
  and translation tools know less.
- **MessageFormat 1's strict apostrophes:** every `'` quotes, so
  "l'épée" breaks; the double optional mode is ICU's own default since
  ICU 4.8.
