---
id: "2349"
title: "The player's language: a setting per profile, an options row of the languages by their own names, the system's languages at first start, and a language's own copies of files"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2226", "2253", "2511"]
nick_input: none
completed: 2026-11-25
---

# 2349 — The player's language

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8.

- **The game tickets.** They were written in the game before the carve.
  `Settings`, the options screen and `Ctx::lang` are the engine's now,
  so their engine half is this ticket. None of them blocks it; each
  waits for it (the game's PR adds `engine#2349` to their `blocked_by`):
  - game#0825
    ([`0825-options-language.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0825-options-language.md)):
    a saved language, an Options row, and however the first launch
    picks one;
  - game#0831
    ([`0831-game-over-words-per-language.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0831-game-over-words-per-language.md)):
    a language brings its own picture of words;
  - game#1049
    ([`1049-playdate-one-build-per-language.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1049-playdate-one-build-per-language.md)):
    one Playdate build per language, which starts in it.
- **The job:** the player picks the language the game shows and it is
  kept; a new player starts in the system's language when the game has
  it; a picture with words in it can differ by language.
- **Friction:** no friction review: a missing feature.
- **The benchmarks:**
  - Unity's Localization picks the first locale through selectors in
    order: the command line, the system's locale, a saved choice, a
    fixed one
    (https://docs.unity3d.com/Packages/com.unity.localization@1.5/manual/LocaleSelector.html).
  - Godot's `TranslationServer` starts from the OS's locale, and its
    Remaps swap a resource per locale.
  - Unreal replaces an asset with the copy at the same path under
    `L10N/<culture>/`
    (https://dev.epicgames.com/documentation/en-us/unreal-engine/asset-localization-in-unreal-engine).
- **Shuyi today:**
  - `Ctx::lang` (`crates/ui/src/ctx.rs`) is documented as "English
    until the player picks another", but nothing lets them pick: only
    tests set it.
  - `Settings` (`crates/ui/src/settings.rs`) has the text speed,
    fullscreen, the theme, the volumes and the layout, and no language.
    Each game writes the saved form itself (`GameSettings::to_ron` and
    `from_ron`): the toy's `File` in `crates/ui/src/toy.rs`,
    `examples/hello-grid/src/settings.rs`, the engine's
    `impl GameSettings for ()` (`crates/ui/src/settings.rs`, 2511),
    which the blank template uses, and
    `templates/grid/crates/game/src/settings.rs`.
  - `RowValue::Choice` (`crates/ui/src/screens/options.rs`) shows text
    keys (`values: fn(&Ctx<G>) -> Vec<&'static str>`). A language's own
    name (`LangInfo::name`, from its `lang.ron`) is not a key.
  - `Setup` (`crates/platform/src/app.rs`) and the `Platform` trait
    (`crates/platform/src/platform.rs`) say nothing of the system's
    languages, and only a backend may ask (`AGENTS.md` rule 5).
  - A pack's folder may hold only `lang.ron`, `ui.ron`, `data.ron` and
    `dialogue/*.ron` (`from_files` in `crates/assets/src/lang.rs`,
    `LangPackRon::decode` in `lang/formats.rs`). A language has no copy
    of any other file.
- **Builds on 2226:** tags such as `pt-BR`, `LangCode::parse`,
  `Lang::chain` and the source language (`SourceLang`); **and on 2253:**
  `best_match`.
  **And on 2511** (MVP): the blank template's saved form is the
  engine's `()` form.
- **Outside tactics:** a farming game's shop sign drawn as a picture of
  words; a puzzle game on the web that opens in the visitor's browser
  language; a visual novel's title logo per language.

## Nick input

None. How the game picks a language at first launch, and how it labels
a machine translation, are the game's to decide (its ticket 0042). The
engine gives the pieces and a default, and the game chooses.

## Scope

**In:**

- **The setting.** `Settings::language: Option<LangCode>`: the language
  the player picked, `None` until they pick one. It is per profile, as
  every setting is.
  - `Settings::restored()` keeps it, as it keeps the layout: "Restore
    defaults" must not switch a reader's language under them.
  - The saved forms (the toy's and hello-grid's `File`, the grid
    template's `File` and the engine's `()` form from 2511) gain
    `language: Option<String>`, written from `LangCode::as_str` and
    read with `LangCode::parse`. Text that isn't a tag reads as `None`.
    Their `#[serde(default)]` reads old settings as `None`, so today
    there is no version to bump.
  - **Either order with 2110** (save migrations): if it is done, the
    engine's `()` form and the grid template's `File` have a `VERSION`.
    Raise it in each, read the version before as 2110's steps do (a
    missing `language` is `None`), and add a fixture, as its guard's
    failure says. If not, 2110's version 1 holds `language`.
- **The language in use.** `Ctx` sets `Ctx::lang` when the settings load
  (`load_settings`, so on a profile switch too), when
  `Settings::language` changes in `preview_settings` (as the theme
  does; `change_settings` and Discard go through it) and when the
  system's languages arrive. A change to another setting leaves
  `Ctx::lang` as it is, so a tool that set it (a test, 2353's
  `Harness::set_lang`) keeps it. In order:
  1. `Settings::language`, when the player picked one;
  2. else the game's policy, `GameData::first_language(&self) ->
     FirstLanguage` (`FirstLanguage` in `crates/ui/src/settings.rs`):
     - `FirstLanguage::System`, the default: `best_match` (2253) of the
       system's languages among the source and the packs, else the
       source;
     - `FirstLanguage::Source`: the source, until the player picks;
     - `FirstLanguage::Fixed(LangCode)`: that language (a build that
       ships one, game#1049).

  More rules:
  - The first-start pick isn't saved. Until the player picks, each start
    follows the system again.
  - A game that asks at first launch opens its own screen while
    `settings().language` is `None`, and saves the answer with
    `change_settings`.
  - A saved language with no pack (a pack dropped, a one-language
    build) shows through its chain: `pt-BR` with only `pt` shipped
    shows `pt`.
  - The game's debug language (`GameData::debug_lang`) is never picked
    from the system, and still shows only with debug tools
    (`effective_lang`).
  - **Either order with 2353** (the pseudo-language): if it is done,
    its `PSEUDO` pack is never picked from the system, and
    `languages_offered` offers it only with debug tools, as it does the
    debug language (the toy's row then shows `[Pseudo]` after `Debug`);
    tools still pick it with `Harness::set_lang`. If not, 2353 does
    that.
- **The system's languages.**
  - `Platform::languages(&self) -> Vec<String>`: the system's preferred
    languages, most preferred first, as the system writes them. A
    default method that returns none, so a backend that doesn't know
    builds unchanged.
  - `Setup::languages: Vec<LangCode>`: `App::load` fills it from the
    backend with `LangCode::parse`, dropping what doesn't parse.
    `Setup::ctx` hands it to a new `Ctx::set_system_languages` before
    the settings load (`with_storage`).
  - **`shuyi-app`:** `Macroquad::languages` (`crates/app/src/backend.rs`)
    calls a new `crates/app/src/languages.rs`, split by target as
    `pads.rs` is.
  - **Native:** `sys_locale::get_locales()`. `sys-locale` 0.3.2 is MIT
    OR Apache-2.0, both on `deny.toml`'s allow list (checked on
    crates.io on 2026-10-20). It has no dependencies on Windows, macOS
    or Linux (`libc` on Android only), and no default features. Add it
    under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`
    without its `js` feature: that feature pulls `wasm-bindgen`, which
    the web build can't use.
  - **The web:** a new plugin of the engine's own,
    `crates/cli/web/languages.js`, over `navigator.languages` (else
    `navigator.language`): `shuyi_lang_count()` and
    `shuyi_lang_char(i, j)` (a character code, 0 past the end). Tags
    are ASCII, so no string helper is needed.
    - The Rust side, the web half of `languages.rs`, imports exactly
      those, in the manner of `crates/app/src/pads/web.rs`: its own
      plugin name and version, a `crate_version` export, and the
      `#[allow(unsafe_code)]` on its `extern` block. The comment on
      `unsafe_code` in `crates/app/Cargo.toml` names both files.
    - `crates/cli/src/web.rs` ships the file (`SHELL`), `index.html`
      loads it between the bundle and the game, and tests like
      `gamepad_js_provides_exactly_the_functions_the_app_imports`,
      `gamepad_js_version_matches_the_app` and
      `index_html_loads_gamepad_js_between_the_bundle_and_the_game`
      check it.
    - `crates/cli/web/README.md` says what the file is.
    - **From 3039 (done)** (the web build's notices): `languages.js`
      gets a row in `SHELL_NOTICES` (`crates/cli/src/web.rs`), the
      table of `SHELL`'s files, as the engine's own beside `gamepad.js`
      (its test fails until it does).
    - **Either order with 3052** (the web shell's progress and errors):
      if it is done, `SHELL` holds its `shell.js` and the page ends
      with `if (shuyiShell.start(…)) load("%WASM%");`. `languages.js`
      still loads between the bundle and that line, and this ticket
      sets `SHELL`'s length. If not, nothing to do (3052 keeps
      `languages.js` when it changes the page).
    - **Either order with 2377** (the system's reduced motion): if it
      is done, `SHELL` holds its `motion.js`, which `index.html` loads
      after `gamepad.js`. `languages.js` loads beside it, this ticket
      sets `SHELL`'s length, and the comment on `unsafe_code` in
      `crates/app/Cargo.toml` names 2377's files and this ticket's. If
      not, nothing to do (2377 loads `motion.js` beside `languages.js`;
      its clause says so).
  - **The headless backend:** `HeadlessPlatform::with_languages(self,
    languages: &[&str]) -> Self`, as the system writes them; none by
    default.
  - **From 3053 (done):** the languages are
    `PlatformFacts::languages: Vec<LangCode>` (`crates/ui/src/platform_facts.rs`),
    filled by each backend's `facts()` (so no `Platform::languages` or
    `Setup::languages`, and the headless backend takes them through
    `HeadlessPlatform::with_facts`), and `Setup::ctx` hands them to
    `Ctx::set_system_languages` before the settings load.
  - **From 3010 (done):** `shuyi-playdate` fills the rest of its
    `facts()` from the profile (`Playdate<S>` in
    `crates/playdate/src/backend.rs`) but leaves `languages` empty,
    since the `Sdk` trait (`src/sdk.rs`) has no call for the system's
    language. This ticket adds one over the SDK's
    `system->getLanguage()` (in the C shim, `shim/shim.c`, the real
    `Device` in `src/device/mod.rs` and the fake in `src/fake.rs`), and
    `facts()` fills `languages` from it (English gives `en`, Japanese
    `ja`, anything else none), with a fake-SDK test.
- **The options row.**
  - `RowValue::Names { values: fn(&Ctx<G>) -> Vec<String>, get, set }`:
    a choice whose values show as written, not looked up as text keys
    (ADR-0020: a label that is text, not a key). It steps, previews and
    is put back like `Choice`: share that code.
    **From 2024 (done):** `RowValue` and each of its struct-like
    variants are `#[non_exhaustive]` (ADR-0039), so `Names` is too, and
    games build it with `RowValue::names(values, get, set)`, a
    `const fn` beside `RowValue::choice`.
  - `Ctx::languages_offered(&self) -> Vec<(LangCode, String)>`: the
    source, then every pack in tag order, each with its own name
    (`SourceLang::name`, `LangInfo::name`). The debug language only
    with debug tools.
  - `OptionRow::language()`: labelled by `options.row.language`,
    offering `languages_offered`, showing the language in use (the
    first tag of its chain that is offered), and setting
    `Settings::language` through `preview_settings`. Every screen
    changes at once, and Discard puts the language back.
  - A game that labels a machine translation (`LangInfo::made_by`)
    builds its own row with `RowValue::names` on `languages_offered`.
  - The toy's `option_rows` gets the row after Key bindings, so the rows
    above keep their places, and `toy/en.ron` its key.
  - **Either order with 2351** (the engine's own English): if it is
    done, `options.row.language`'s English goes in its engine file; if
    not, it goes in the toy's `en.ron`, and 2351 moves it.
- **A language's own copies of files.** Split to 2278 by the *Size*
  clause below; it holds this part now, as written here.
  - A pack's folder may hold `files/<path>`: that language's copy of the
    game's file at `<path>`, as in Unreal's `L10N/<culture>/<path>`.
    Both readers (`from_files`, `LangPackRon::decode`) accept `files/`
    and never read it as text. Any other unknown file is still an
    error.
  - **From 2229 (done, import settings in `.meta` files):** both
    readers already skip a settings file (`is_settings_file`): add the
    `files/` exception beside that skip in the same two loops, and test
    a pack holding `files/x.png` and `files/x.png.meta` in both.
  - With 2238 (one check of a language folder's files): 2278's clause
    says where `files/` is accepted.
  - `shuyi_assets::lang::localized_path(languages: &str, chain:
    &[LangCode], path: &str, has: impl Fn(&str) -> bool) -> String`,
    pure: the first `<languages>/<tag>/files/<path>` along the chain
    that `has` (the source's folder has no copies), else `path`.
  - `Ctx::localized(&self, languages: &str, path: &str, has: impl
    Fn(&str) -> bool) -> String`: the same, along the chain of
    `effective_lang`. A screen asks each time it draws, so a change of
    language shows at once:
    `images.id(&ctx.localized("lang", "title/end.png", |p| images.id(p).is_some()))`.
    `ImageTable` already lists every PNG of the bundle, copies included.
  - The pack (`crates/assets/src/pack.rs`) holds every file of the
    bundle it was baked from, so copies are baked with no change.
  - A one-language build ships a layer without the other packs'
    folders, and starts in its language with `FirstLanguage::Fixed`.
    Both readers need no pack and `localized_path` asks `has`, so
    nothing is named with `Layer::leaving_out` (which names files a
    layer lacks; it removes none).
  - **With per-target layers (3007):** the language's copy is looked
    for first, then the plain path, each through the bundle's layers in
    order. A target that needs its own copy of a language's file puts it
    at that same path in its layer
    (`assets@playdate/lang/ja/files/title/end.png`). The words in a
    picture matter more than the device's version of another
    language's picture.
- **Docs:** `docs/formats.md` (a pack's `files/`),
  `docs/platform-backends.md` (`languages`), and the module docs of
  `settings.rs` and `screens/options.rs`.
- **Size:** if the PR would pass about 600 changed lines (snapshots not
  counted), *A language's own copies of files* (step 6 and its criteria)
  goes to a follow-up ticket written with the `write-ticket` skill
  (After MVP, blocked by 2349, for game#0831), and 3007's line on
  `Ctx::localized` points at it. The PR says so.

**Out (do not do):**

- A first-launch language screen, and labels for machine translations:
  the game's (its 0042), built on `languages_offered`.
- Voices that follow the language: `VoiceFiles::lang`
  (`crates/app/src/lib.rs`) stays set at build time. The game's voices
  are in one language, and whether they follow translated text is its
  own question (game 0043). Parked 2725 plans voices per language for
  visual novels, from `VoiceFiles`' folder per language along the
  language's chain, not from a pack's `files/` (`localized_path`).
- Steam's game language: 3037 (parked), which builds on
  `Platform::languages`.
- Reading again the data a game read at start when the language
  changes: the game does that itself if it must.
- Making a one-language build's layer: 3059 (`shuyi bake --lang` and
  `shuyi export --lang`, for game#1049's Playdate builds). It keeps the
  packs on the language's chain, with their `files/`, and records the
  language in the pack for `FirstLanguage::Fixed`.
- A command-line language switch: a ticket of its own when a game asks.
- The engine's screens in other languages (2352, parked).
- Drawing a language's own name in a script the font lacks (`日本語`
  on the row): 2303's wide glyph sets. **From 2303 (done):** a wide
  glyph is drawn from the language's set first, then from any other
  (`FontAtlasDef::wide_cell`), and `font-atlas` keeps a pack's name in
  the set its `lang.ron` names, so the row shows each name in its own
  shapes with nothing to do here.
- Choice rows built from an enum setting: 2314.
- The pseudo-language itself: 2353.

**For the game:** additive. `RowValue` gains a variant, which breaks no
game: it is `#[non_exhaustive]` since 2024, so a game's `match` on it
already has a `_` arm. If CI's `api` job finds a break all the same, the
completion notes carry its `Breaking:` line (ADR-0038). A game's saved
settings should write the new field. The game's PR that moves the pin
follows this one. It builds game#0825 on `Settings::language` (an
`Option`, not the `LangCode` with a version bump that 0825 planned) and
`OptionRow::language`, and game#0831 on `Ctx::localized`.

## Implementation steps

1. Read 2226's result in `crates/assets/src/lang.rs`; `load_settings`,
   `preview_settings`, `load_profile` and `effective_lang` in
   `crates/ui/src/ctx.rs`; `OptionsScreen` and `RowValue`; `App::load`
   and `Setup::ctx` (`crates/platform/src/app.rs`); `HeadlessPlatform`;
   `crates/app/src/pads.rs`, `pads/web.rs` and `crates/cli/web/gamepad.js`
   with their tests in `crates/cli/src/web.rs`. Check whether 2110
   and 2353 are done, for their clauses.
2. Add `Settings::language`, the four saved forms and `FirstLanguage`.
   The grid template isn't formatted by `cargo fmt --all` (CI's
   template job checks it), so format it on its own.
3. Make `Ctx` set the language in use, with `set_system_languages`.
4. Add `Platform::languages`, `Setup::languages` and the headless
   backend's, then `shuyi-app` on native and on the web.
5. Add `RowValue::Names` and `RowValue::names`, `languages_offered`,
   `OptionRow::language` and the toy's row. Update the toy's Harness
   tests that count rows, if any moved.
6. Accept `files/` in both readers; add `localized_path` and
   `Ctx::localized`.
7. Write the docs. Run the gates, the web build included.

## Acceptance criteria

- [x] On the toy, the language row shows `English`, then `Debug` (the
      debug pack's own name). Stepping to `Debug` makes
      `ctx.text("home.greeting")` give `HELLO, {name}` at once; Keep
      and a relaunch from the same storage keep it; Discard gives back
      `Hello, {name}` (Harness test in `crates/ui/tests/it/screens.rs`).
- [x] Old toy settings without `language` load as `None`;
      `Settings::restored` keeps the language; each profile keeps its
      own (unit tests).
- [x] A change to another setting (the text speed) leaves `Ctx::lang`
      as a test set it (unit test).
- [x] With no pick: `System` gives `pt` for the system's `[pt-BR,
      en-US]` over a `pt` pack, and the source for `[de]`; `Source`
      gives the source; `Fixed` gives its language (unit tests on
      `Ctx`, and one through `App` on
      `HeadlessPlatform::with_languages`).
- [x] The debug language is never picked from the system, and the row
      offers it only with debug tools (unit tests).
- [x] The toy's options screen with the language row is a committed
      snapshot, reviewed with `frame-png`.
- [x] ~~`localized_path` finds `lang/pt/files/x.png` for `pt-BR`, prefers
      `lang/pt-BR/files/x.png` when that exists, and gives `x.png`
      otherwise. With `lang/pt/` left out of the bundle, it gives
      `x.png`. A copy read back from a pack equals the loose one (unit
      tests).~~ (not done: split to 2278 by the *Size* clause)
- [x] ~~A pack with `files/` passes both readers (unit tests).~~ (not
      done: split to 2278 by the *Size* clause)
- [x] The functions `languages.js` provides equal the app's imports,
      its version equals the app's, and `index.html` loads it between
      the bundle and the game (tests in `crates/cli/src/web.rs`); the
      web build passes.
- [x] `cargo deny check` passes with `sys-locale`, and `cargo tree -p
      shuyi-app --target wasm32-unknown-unknown` shows neither it nor
      `wasm-bindgen`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the order that picks the language in use; the saved forms with
  and without a language (all four); `RowValue::Names` stepping,
  previewing and Discard; `languages_offered` with and without debug
  tools; `localized_path` along a chain; both readers with `files/`.
- Unit (`crates/cli/src/web.rs`): `languages.js` against the app's
  imports, its version, and its place in `index.html`.
- Integration: a Harness run on the toy that switches the language; an
  `App` run on the headless backend with system languages.
- Snapshot: the toy's options screen with the language row.

## Completion notes

**What was done.**

- `Settings::language: Option<LangCode>`, kept by `Settings::restored`,
  and `FirstLanguage` (`System`, the default; `Source`; `Fixed`) with
  `GameData::first_language`, in `crates/ui/src/settings.rs`. The saved
  forms write it through `language_to_text` and read it with
  `language_from_text` (text that isn't a tag reads as `None`).
- `Ctx` sets `Ctx::lang` when the settings load (a launch, a profile
  switch), when `Settings::language` changes in `preview_settings`, and
  in `Ctx::set_system_languages`; another setting's change leaves it.
  The system pick is `best_match` over the source and the packs, never
  the debug language. `Ctx::widths` follows a tag with no pack along
  its chain. `Ctx::languages_offered` gives the source and the packs by
  their own names, the debug language only with debug tools.
- The system's languages are `PlatformFacts::languages` (with
  `with_languages`, serialized as text in a session record):
  `shuyi-app` fills them in `Macroquad::new` from the new
  `crates/app/src/languages.rs` (`sys-locale` on native; on the web
  the new `crates/cli/web/languages.js` plugin, in `SHELL`,
  `SHELL_NOTICES`, `index.html` and the web README, with the three
  tests); `shuyi-playdate` from a new `Sdk::language` over
  `system->getLanguage()` (shim, device, fake, and a fake-SDK test).
  `Setup::ctx` puts the facts in before the settings load.
- `RowValue::Names` and `RowValue::names`, sharing `Choice`'s
  stepping, preview and put-back; `OptionRow::language` and
  `LANGUAGE_ROW`; the toy's row after Key bindings and its key in
  `toy/en.ron`.
- Tests: the saved forms (the engine's `()`, the toy's, hello-grid's),
  `crates/ui/src/ctx/lang_tests.rs` (the order, per profile, the chain,
  the debug language, another setting's change, the languages
  arriving, `languages_offered`), the row's own test in
  `screens/options/tests.rs`, two Harness tests and the snapshot
  `it__screens__options_with_the_language_row` in
  `crates/ui/tests/it/screens.rs`, an `App` test on the headless
  backend's facts, and the facts' serde test.

**Deviations.**

- **Step 6 split to 2278**, by the ticket's *Size* clause (this PR is
  past 600 lines; `cargo xtask pr-size` counts 1,097). 2278 holds a
  pack's `files/`, `localized_path` and `Ctx::localized`; 2238's
  clause and 2725's line now name 2278, and 3059 (whose tests read a
  pack's `files/`) is blocked by 2278 too. 3007's line on
  `Ctx::localized` was already gone (3007 is done). Had the PR needed
  to split further, step 4 (the backends: `shuyi-app`'s native and
  web reads, the Playdate's) would have gone apart cleanly.
- **No `Platform::languages`, `Setup::languages` or
  `HeadlessPlatform::with_languages`**: 3053 (done) made the system's
  facts one `PlatformFacts`, so they are `PlatformFacts::languages`,
  and the headless backend takes them through `with_facts`. 3053
  planned that the facts *hold* them; this ticket added the field.
  `Setup::ctx` needed no call of its own: it already sets
  `ctx.platform` before `with_storage`, which picks the language.
- **Three saved forms, not four**: the grid template's `settings.rs`
  no longer exists (the template is `templates/shuyi-default`, on the
  engine's `()` form).
- **A toggle for `sys-locale`** (the decision of 2026-11-24, after
  this ticket was written): `shuyi-app`'s `system-languages` feature,
  on by default, and the facade's, in its defaults
  (`shuyi-app?/system-languages`; the facade now takes `shuyi-app`
  with `default-features = false`). hello-grid and snake, which pick
  the facade's features, turn it on in their `window` feature.
  `cargo xtask check-modules` counts it as an engine feature, and the
  API gate's lists name it. A stripped release hello-grid on
  Windows is 3 KB bigger with it (5,424,128 bytes against 5,421,056,
  measured on this branch). Off, a desktop game hears no system
  language and starts in its source language until the player picks;
  the web reads the browser's either way (our own plugin, no
  dependency).
- The `frame-png` review: the toy has no font atlas, so its picture is
  boxes on its test palette (as its lobby is); it shows the Language
  row in place and focused. The words are reviewed in the text
  snapshot, which shows `Language ◄ Debug ►` and the help line already
  in the debug language.

**Either-order clauses.** 2110, 2351, 2353, 2377 and 3052 now say what
holds from 2349 (done); the parked 3037 too (its clause only). Not done
when this landed: 2110 (its version 1 will hold `language`), 2353,
2351 (`options.row.language` is in the toy's `en.ron`; 2351 moves it),
3052 and 2377.

**Follow-ups.** 2278 (a language's own copies of files).

Game: game#0825 (a saved language and an Options row, on
`Settings::language` and `OptionRow::language`) adopts it in the game's
PR that moves the pin; game#0831 waits on 2278, game#1049 on 3059.

Searched the game at its pin `b69585ee`: `gh search code --repo
Zafnok/visions-of-shuyi 'ctx.lang'` (its language tests set it on a
Harness after the launch), `'debug_lang'` (its debug language is its
`test` pack) and its `assets/lang/` (only `en` and `test`): on any
system the game starts in English as before, since its only pack is the
debug language, which the system never picks. It takes `shuyi-app` with
its default features, so it gets the `system-languages` read.

Behaviour: shuyi-ui: `Ctx::lang` is set again when the settings load (`with_storage`, a profile switch) and when `Settings::language` changes: a tool that set `ctx.lang` before those sees the player's pick or the game's start pick instead. A change to another setting leaves it.
Behaviour: shuyi-ui: a game with a pack that fits the system's languages starts in it while the player hasn't picked (`FirstLanguage::System`, the default of `GameData::first_language`); answer `FirstLanguage::Source` to keep starting in the source.
Behaviour: shuyi-ui: Restore defaults (`Settings::restored`) keeps the language picked, as it keeps the layout.
Behaviour: shuyi-app, shuyi-playdate: `PlatformFacts::languages` holds the system's languages (sys-locale on a desktop, `navigator.languages` on the web, the Playdate's `getLanguage`), so a session record carries them.
Behaviour: shuyi, shuyi-app: a new default feature, `system-languages`; a game that takes the facade with `default-features = false` gets no system language on a desktop until it adds `shuyi/system-languages`.

Book: `site/content/learn/book/saves-and-settings.md` (*Settings*) says
what the language setting is, the three first-language policies and the
row.
