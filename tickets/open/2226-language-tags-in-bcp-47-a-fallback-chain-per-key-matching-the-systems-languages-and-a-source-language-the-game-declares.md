---
id: "2226"
title: "Language tags in BCP 47 (pt-BR, zh-Hans), a fallback chain per key, matching the system's languages, and a source language the game declares"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2226 — Language tags, a fallback chain and the source language

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

**The benchmarks:**

- Godot names a locale `language_Script_COUNTRY_VARIANT`: Brazilian
  Portuguese and Simplified Chinese are locales of their own
  (https://docs.godotengine.org/en/stable/tutorials/i18n/locales.html).
- Unity's Localization package gives a locale a fallback locale, used
  when an entry is missing
  (https://docs.unity3d.com/Packages/com.unity.localization@1.5/manual/Locale.html).
- Unreal has a native culture (the language the game is written in)
  and cultures with regions.
- Bevy has no localization of its own.

**Shuyi today** (`crates/assets/src/lang.rs` and its submodules):

- `LangCode::new` takes 2 to 8 lowercase letters only (`MAX_CODE_CHARS`).
  So `pt-BR`, `es-419` and `zh-Hans` can't name a pack, and 2303, whose
  Simplified Chinese set is `zh-Hans`, is blocked by this ticket.
- English is the source, fixed in code:
  - `ENGLISH = "en"` and `LangCode::english()`;
  - the `en/ui.ron` path in `from_files` and in `LangPackRon::decode`
    (`lang/formats.rs`);
  - `Lang`'s private `english` map, and `Lang::english(key)`;
  - in `crates/ui/src/ctx.rs`, `Ctx`'s private `english` code for
    `effective_lang`, `lang: LangCode::english()` in `Ctx::new`, and
    `numbered_profile`, which reads the profile name in English.
- A key a pack lacks (or has stale) shows English. Nothing sits
  between: a small `pt-BR` pack over a full `pt` pack can't exist.

**Plans that build on this:** 2349 (the player's language setting, the
system's languages at first start, files per language) and 2303 (wide
glyph sets, with a Simplified Chinese pack named `zh-Hans`). 2346, 2348,
2350, 2351, 2352 and 2353 have either-order clauses with this ticket.

**Why it matters:**

- **The game first.** Visions of Shuyi plans Japanese and Simplified
  Chinese (game#0236 and game#0239, through 2301 and 2303) and a
  language row (game#0825, through 2349). Its Chinese folder should be
  `zh-Hans` from the start, so nobody migrates a `zhhans` folder and
  saved settings later.
- **Other 2D games** ship Brazilian Portuguese, Latin American Spanish
  and both Chinese scripts, all standard Steam languages. A studio that
  writes first in Japanese can't make Japanese the source today.

## Nick input

None.

## Scope

**In:**

- **Tags.** `LangCode` holds a BCP 47 tag (RFC 5646), in this subset:
  - the language: 2 to 8 letters, as today, so every code that works
    today still does (`en`, `ja`, the toy's `dbg`);
  - an optional script: 4 letters (`Hans`);
  - an optional region: 2 letters (`BR`) or 3 digits (`419`);
  - any variants, each once: 5 to 8 letters or digits, or a digit and
    3 letters or digits (`1996`);
  - an optional private-use part: `x-` and one or more subtags of 1 to
    8 letters or digits (`en-x-pseudo`);
  - at most 35 characters in all (`MAX_CODE_CHARS` becomes 35).

  Refused, each with its reason: extension subtags (`-u-`, `-t-`),
  extlang subtags (3 letters right after the language), grandfathered
  tags (`i-klingon`), empty subtags.
- **One spelling per tag.** The canonical case is: language, variants
  and private use in lower case, the script in title case, the region in
  upper case, `-` between subtags.
  - `LangCode::new(text) -> Option<Self>` takes the canonical form only,
    as it does today.
  - `LangCode::parse(text) -> Result<Self, TagError>` is for text from
    outside (the system's languages in 2349, the command line): any
    case, `_` for `-`, and a POSIX locale's `.charset` and `@modifier`
    dropped (`pt_BR.UTF-8` gives `pt-BR`). `C` and `POSIX` are refused.
    `TagError` says in plain words what is wrong: a `#[non_exhaustive]`
    enum with `Display` (2013's rule for a set that will grow).
  - `language()`, `script()` and `region()`.
  - `parent() -> Option<LangCode>`: the tag without its last subtag. A
    private-use part goes all at once (`en-x-pseudo` gives `en`). A bare
    language has none.
- **Pack folders are canonical tags.** `from_files` and
  `LangPackRon::decode` read a folder named by a canonical tag. Any other
  name is an error, in place of today's "must be named with 2 to 8
  lowercase letters":
  - when `LangCode::parse` gives a tag, the error names it:
    `lang/pt-br: name a language's folder by its tag as BCP 47 writes
    it: pt-BR`;
  - otherwise it gives `TagError`'s words: `lang/x1: name a language's
    folder by a BCP 47 tag (en, pt-BR, zh-Hans): <what is wrong>`.

  Windows and macOS file systems ignore case, but bundle paths don't,
  so `pt-br` must never quietly become a second language.
- **The source language the game declares.**
  - `SourceLang { code: LangCode, name: String }` in
    `shuyi_assets::lang`: the language the game is written in, and its
    own name for itself. `Default` is `en`, "English". The fields are
    public, and the struct is `#[non_exhaustive]` with
    `SourceLang::new(code: LangCode, name: impl Into<String>)`, since a
    follow-up may give the source a wide set or a name grid (Out).
  - `Lang` keeps it: `Lang::source_lang() -> &SourceLang`, and
    `Lang::with_source_lang(self, source: SourceLang) -> Self`.
    `Lang::new` and `Lang::with_source` keep English, so a game that
    says nothing changes nothing.
  - `from_files_in(source: &SourceLang, files, data, display, rules)`
    reads `<code>/ui.ron` as the source's screen text. `from_files`
    calls it with the default.
  - `LangPackRon::source_lang(self, source: SourceLang) -> Self` reads
    `<languages>/<code>/ui.ron` and refuses the source's own folder as a
    pack.
  - The source's folder still holds `ui.ron` only, and a `lang.ron`
    there is still an error: its name comes from the game's code. (2346
    relies on this.)
  - Renames, the old names kept one release as `#[deprecated]` aliases:
    - `Lang::english(key)` becomes `Lang::source_text(key)`;
    - `LangCode::english()` and `ENGLISH` are deprecated, their note
      pointing at `Lang::source_lang()`. `LangCode::default()` stays
      `en`.

    No code in `crates/`, `examples/` or `templates/` calls a deprecated
    name, tests included, except one test per alias. That test is
    marked `#[expect(deprecated)]` and checks the alias gives what the
    new name gives, so mutation testing of the diff has a test that
    kills each alias's mutants. `english_from_source` keeps its name;
    its docs say it reads the source's screen text in any language.
  - `Ctx` (`crates/ui/src/ctx.rs`): the private `english` field goes.
    `effective_lang` falls back to `&self.game.lang().source_lang().code`,
    so it follows the game's `Lang` and nothing is cached. `lang` starts
    at the source in `Ctx::new`; `numbered_profile` reads the source's
    text.
  - Messages that say "English" where they mean the source name it by
    its tag (`no such key in the source language (en)`).
    `cargo shuyi lang-status` (`crates/cli/src/lang_status.rs`) prints
    `source:` where it printed `english:`, and reads its argument with
    `LangCode::parse`.
- **A fallback chain per key.**
  - `Lang::chain(&self, code: &LangCode) -> Vec<LangCode>`: `code`, then
    each `parent()`, keeping only tags that have a pack, then the
    source, which always ends it. With packs `pt` and `pt-BR`,
    `chain(pt-BR)` is `pt-BR, pt, en`; without `pt`, it is `pt-BR, en`.
  - Every lookup walks it, and takes the first pack's entry made from
    today's source text: `Lang::text`, `data_text`, `name`, `names`,
    `found`, `line` and `scene` (`lang.rs`, `lang/source.rs`,
    `lang/lines.rs`). A stale entry is skipped as today, so a fresh
    entry in `pt` beats a stale one in `pt-BR`.
  - The lookups run on every drawn text, so they walk the chain without
    allocating. A parent is always a prefix of its tag (`pt-BR` to
    `pt`), so they look packs up by `&str` slices of the tag
    (`impl Borrow<str> for LangCode`, sound because `LangCode` compares
    and hashes as its text). `chain` is the same walk, collected.
  - `Lang::status(code)` stays what that one pack lacks; the source
    lacks nothing, as English does today.
- **Matching the system's languages.** A pure function in
  `shuyi_assets::lang`, `best_match(preferred: &[LangCode], offered:
  &[LangCode]) -> Option<LangCode>`, which 2349 uses at first start
  with the source and the packs as `offered`. For each preferred tag in
  order:
  1. the tag, then each of its `parent()`s, if offered (`en-GB` matches
     an `en` source);
  2. Chinese without a script takes one from its region (`CN` and `SG`
     give `Hans`; `TW`, `HK` and `MO` give `Hant`), then step 1 again
     (`zh-CN` matches `zh-Hans`);
  3. the only offered tag with the same language subtag, unless both
     have a script (written, or from step 2) and they differ (`pt-PT`
     matches a lone `pt-BR`; `zh-TW` doesn't match `zh-Hans`).

  The first preferred tag that matches wins. `None` if none does (the
  caller then uses the source).
- **Either order:**
  - **2348** (plurals in text): if it is done, its plural table is
    looked up by `LangCode::language()` here (`pt-BR` uses `pt`'s), and
    `Lang::text_in` and `Lang::data_text_in`, which `text` and
    `data_text` are built on, walk the chain and give the code of the
    language whose entry they return (the source's when they fall
    back); if not, 2348 does that.
  - **2351** (the engine's own English): if it is done, its English
    stays beneath the source's screen text whatever language the source
    is; if not, 2351 builds on the `SourceLang` this ticket adds.
  - **2346** (name grids from the pack): if it is done, `Ctx::name_grid`
    takes the first pack along `Lang::chain` that has a grid; if not,
    2346 does that.
  - **2350** (characters the atlas lacks): if it is done, its errors
    name the source's folder by `Lang::source_lang()`'s code where they
    say `en`; if not, 2350 does that.
  - **2353** (the pseudo-language): if it is done, its `PSEUDO` code
    becomes `qps-ploc`, the BCP 47 pseudo-locale; if not, 2353 uses
    that code.
  - **2352** (parked: the engine's own packs): if it is done, its packs
    take part in the chain as its clause says (a `pt-BR` game pack falls
    back to the engine's `pt` pack before the source); if not, 2352
    does that.
  - **2227** (parked: PO files): if it is done, `write_po`'s `msgid`
    is `Lang::source_text`, its `#:` names `<source tag>/ui.ron`, its
    `Language:` is the tag with `_` for `-` as gettext writes it
    (`pt_BR`), and `lang-export` and `lang-import` read tags with
    `LangCode::parse`, so a PO saved with `Language: pt_BR` matches the
    folder `pt-BR`; if not, nothing to do (2227's clause does it).
  - **Either order with 2238:** if 2238 is done, change the folder-name
    check and its two messages once, in `pack_code`
    (`crates/assets/src/lang.rs`), which both readers call, and walk the
    chain over `fresh`: each lookup, and `pack_names` (what `Lang::names`
    and `pack_from_files` fill names from), takes the first pack along
    the chain whose entry `fresh` keeps, with `pack_from_files` passing
    its own pack alone since it reads one pack; if not, nothing to do
    (2238 merges them after this ticket, as its clause says).
- **Docs:** the language pack bullet in `docs/formats.md`, and the
  module docs of `lang.rs`: the tag subset, the chain and the source.
  The module docs say why the engine parses the subset itself rather
  than taking `icu_locale_core`: a tag check is a short function, and
  the engine wants none of CLDR's data.

**Out (do not do):**

- The player's language setting, the options row, reading the system's
  languages, a language's own copies of files: 2349.
- CLDR data: likely subtags beyond step 2's five regions, aliases (`iw`
  for `he`), parent locales other than the shorter tag.
- A `lang.ron` for the source, and so a source in a wide-glyph language
  with its own wide set (2303) or name grid (2346): a follow-up when a
  game writes first in such a language.
- Plural and select rules, numbers by language (2348).
- Right-to-left and shaped scripts (2908, parked).
- Translator exchange files (2227, parked), beyond its item above.

**For the game:** additive for a game whose packs are named with
lowercase letters and whose source is English, since those names are
canonical tags. The deprecated names warn, so the game's PR that moves
the pin renames its calls (`Lang::english` to `Lang::source_text`).
Error messages and `lang-status` output that said "English" change, and
so does the message for a badly named folder. Nothing is removed, so
the PR needs no `Breaking:` line (2010).

## Implementation steps

1. Read `crates/assets/src/lang.rs`, `lang/formats.rs`, `lang/source.rs`
   and `lang/lines.rs`, `crates/cli/src/lang_status.rs`, and `Ctx::new`,
   `effective_lang` and `numbered_profile` in `crates/ui/src/ctx.rs`.
2. Write the tag parser: `new`, `parse`, `parent`, the accessors and
   `TagError`, with their tests. `no_std` with `alloc`, like the rest of
   the crate.
3. Add `SourceLang` to `Lang`, `from_files_in` and
   `LangPackRon::source_lang`. Move every use of `ENGLISH`,
   `LangCode::english()` and `Lang::english` in `crates/`, `examples/`
   and `templates/` to the source (`lang_status.rs`, `ctx.rs`,
   `ctx/tests.rs`, `lang/pack_tests.rs`, and hello-grid's
   `tests/it/snapshot.rs` among them), then add the deprecated aliases
   and their one test each.
4. Add `Lang::chain` and the non-allocating walk, and make the seven
   lookups use it.
5. Change the folder-name check in both readers, and the messages of
   `a_badly_named_directory_is_reported` (`lang.rs`) and of the `x1`
   test in `crates/assets/src/format/builtin_tests.rs`.
6. Write `best_match`.
7. Update `Ctx`, the messages, `lang-status` and the docs. Run the
   gates.

**Size:** about 450 to 550 changed lines before tests. If the diff runs
past about 600, `best_match` (step 6, with its tests) moves to a
follow-up ticket of the same milestone (the `write-ticket` skill,
blocked by this one), added to 2349's `blocked_by` since 2349 is its
only user. The completion notes say so.

## Acceptance criteria

- [ ] `en`, `ja`, `dbg`, `pt-BR`, `es-419`, `zh-Hans`, `zh-Hant-TW`,
      `sr-Latn-RS`, `de-CH-1996` and `en-x-pseudo` are canonical tags;
      `pt-br`, `PT-BR`, `pt_BR`, `zh-hans`, `en-u-ca-gregory`,
      `i-klingon`, `en--US` and the empty string are not (unit tests).
- [ ] `LangCode::parse` turns `pt_br.UTF-8`, `ZH-hans-cn` and
      `en_US@euro` into `pt-BR`, `zh-Hans-CN` and `en-US`, and refuses
      `C` (unit tests).
- [ ] With packs `pt` and `pt-BR`: a key only `pt` has shows `pt`'s
      text in `pt-BR`; a key neither has shows the source's; a stale
      `pt-BR` entry shows `pt`'s fresh one (unit tests over `text`,
      `data_text`, `line` and `scene`).
- [ ] A game written in Japanese (`ja/ui.ron`, a pack `en`) loads
      through `from_files_in` and through `LangPackRon::source_lang`,
      shows Japanese for a key the pack lacks, and refuses a
      `ja/lang.ron` (unit tests). Through `Ctx::text` too (a test game
      in `crates/ui/src/ctx/tests.rs`).
- [ ] A folder `pt-br` is refused by both readers with a message naming
      `pt-BR`, and a folder `x1` with `TagError`'s words (unit tests).
- [ ] `best_match` gives `pt` for `[pt-BR]` over `[en, pt]`, `zh-Hans`
      for `[zh-CN]` over `[en, zh-Hans]`, `None` for `[zh-TW]` over
      `[en, zh-Hans]`, and `fr` for `[de, fr-CA]` over `[en, fr]` (unit
      tests).
- [ ] No code in `crates/`, `examples/` or `templates/` calls
      `Lang::english`, `LangCode::english` or `ENGLISH`, except the
      one `#[expect(deprecated)]` test of each alias.
- [ ] Every existing language test passes, with only the messages that
      said "English" and the badly named folder's message changed.
- [ ] `shuyi-assets` builds for `thumbv7em-none-eabihf` (the no-std
      gate).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: as in the criteria; `parent` on every shape of tag; `chain`
  with and without the middle pack, and for the source itself;
  `best_match`'s step 2 for each of its five regions, and step 3 with
  two offered tags of one language (no match); `POSIX` refused by
  `parse`; each deprecated alias gives what its new name gives.
- Property (proptest, tags built from the subset's parts): for any tag
  `t` that `LangCode::new` takes, `LangCode::parse` of `t` in upper
  case, and of `t` with `_` for `-`, gives `t`; following `parent` ends
  within as many steps as `t` has subtags; `chain` is never empty, ends
  with the source and holds no tag twice.
- Unit (`crates/cli/src/lang_status.rs`): `report` asked for `pt-br`
  reports the `pt-BR` pack.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
