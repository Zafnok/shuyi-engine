---
id: "2351"
title: "Built-in English for the engine's text keys, beneath the game's text, which overrides it"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2351 — Built-in English for the engine's text keys

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Engines bring the words of their own screens:

- Godot's built-in controls, such as `FileDialog`, show their own
  English without the game writing it
  (https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html).
- Unreal keeps the engine's own text in a localization target of its
  own (`Engine`), apart from a game's targets.
- Bevy's 2D template, `bevy_new_2d`, comes with its title, settings and
  pause menus already worded.

Shuyi today (checked 2026-10-21):

- The engine's screens name **87 text keys** in its own code: constants
  such as `NOT_MAPPED_TEXT` and `PROFILE_NUMBERED_TEXT`
  (`crates/ui/src/ctx.rs:40` and `:52`), `CAPTURE_PROMPT`
  (`crates/ui/src/screens/key_bindings.rs:105`), the constants of
  `screens/options.rs`, `screens/profiles.rs`,
  `screens/layout_picker.rs`, `widgets/name_entry.rs` and `screens.rs`
  (`TEXT_BOX_HELP`), and literals in the ready-made rows
  (`OptionRow::text_speed` and the others in `screens/options.rs`,
  `TextSpeed::key` in `crates/ui/src/settings.rs`). All start with
  `keys.`, `text_box.`, `options.`, `key_bindings.`, `layout_picker.`,
  `profile.` or `name_entry.`.
- The engine has no words for them that a game can use.
  `Lang::text` (`crates/assets/src/lang.rs:229`) returns the bare key
  when the game's English lacks it, and `Ctx::text`
  (`crates/ui/src/ctx.rs:369`) panics in debug builds.
- So every game copies them by hand. `examples/hello-grid/files/en.ron`
  holds 84 of the 87. Both templates
  (`templates/blank/crates/game/files/en.ron`,
  `templates/grid/crates/game/files/en.ron`) copy the three `keys.*`
  and leave out `profile.numbered`, so a new game's first profile is
  called "profile.numbered" (`numbered_profile`,
  `crates/ui/src/ctx.rs:859`).
- The words exist, for the engine's tests only:
  `crates/ui/src/toy/en.ron` (read at `crates/ui/src/toy.rs:367`) has
  83 of the keys, and the toy's `ENGLISH` array (`toy.rs:339`) the
  other four. The file's header says a game's own language files give
  them their words.
- Each engine ticket that adds a row puts its English in two files
  (2321, 2333). The tickets that add engine keys have an either-order
  clause with this ticket: 2340, 2342, 2346, 2349 and 3043, and the
  parked 2366, 2404, 2408 and 3054. So do 2226, 2353 and 2511.

Why it matters:

- **The game first.** Visions of Shuyi has every key in its English
  today, so nothing changes for it. But each new engine row or screen
  needs new English in the game before it shows, at every pin bump.
  With this ticket the engine's words show until the game writes its
  own.
- **Other games** get the options, Key bindings and profiles screens
  without copying about 85 lines, and an agent can't forget a key.
- **The end goal** (Nick, 2026-10-19, `docs/decisions.md`): a game made
  "in not too many LOC".

The engine's other languages are 2352 (parked, a question for Nick).
This ticket is English only.

## Nick input

None.

## Scope

**In:**
- **The engine's English:** `crates/ui/lang/en/ui.ron`, a map of key to
  text in the shape of a game's `en/ui.ron` (`docs/formats.md`). It is
  not a new format: it is read with
  `shuyi_assets::lang::english_from_source`. It holds exactly the keys
  the engine's own code names (87 today), **moved** from
  `crates/ui/src/toy/en.ron` and the toy's `ENGLISH` array with the
  same words, so the toy's snapshots stay the same.
  `shuyi_ui::text::engine_english() -> BTreeMap<String, String>` reads
  it (`include_str!("../lang/en/ui.ron")` in `crates/ui/src/text.rs`;
  it works without `std`). If the file doesn't parse it panics with the
  file's errors, through `unwrap_or_else(|e| panic!(..))` as `Toy::new`
  does (the workspace lints warn on `unwrap` and `expect`). A test keeps
  it parsing. **Either order with 2383:** if 2383 is done, its talk
  screen's `talk.next` is one of the keys the engine's code names: give
  it its English here with the others. If not, nothing to do (2383 adds
  it here once this is done, by its own clause).
- **Beneath the game's English, in `shuyi_assets::lang`:**
  - `merged_english(engine: &BTreeMap<String, String>, game:
    BTreeMap<String, String>) -> BTreeMap<String, String>`: every key
    of both, with the game's text where both have it;
  - `Lang::with_engine_english(self, engine: BTreeMap<String, String>)
    -> Self`: merges `engine` into the `Lang`'s own English map,
    beneath the game's keys. So `has`, `english`, `text` and `status`,
    and anything else that walks that map (2350's scan, 2353's
    `with_pseudo`), see the merged English. A private
    `BTreeSet<String>` in `Lang` remembers the keys only the engine's
    English has;
  - `Lang::is_engine(&self, key: &str) -> bool`: the key's English is
    the engine's (the game's English doesn't have it);
  - packs: `from_files_with_engine(engine, files, data, display,
    rules)`, with `from_files` calling it with an empty map, and
    `LangPackRon::with_engine_english(self, engine) -> Self` beside
    `with_data`. A pack's `ui.ron` is checked against the merged
    English, so a pack may translate an engine key that the game's
    English never writes.
  - All of it is new API, and `Lang` and `LangPackRon` already have
    private fields. A game that calls none of it works exactly as
    today.
- **`Ctx::text`'s debug message** says how to get the engine's words:
  build the `Lang` with `with_engine_english(shuyi_ui::text::engine_english())`.
- **`lang-status`** (`crates/cli/src/lang_status.rs`): the keys whose
  English is the engine's (`Lang::is_engine`) get their own part,
  `Part::Engine`, headed "engine screen text", after the game's screen
  text. `--only engine` shows it alone; `--only ui` shows the game's
  screen text alone. The parse error and both usage lines
  (`crates/cli/src/lib.rs:66` and `:245`) name `engine`.
  - `Part` is a public enum without `#[non_exhaustive]`, so the new
    variant is a break by the Cargo book's rules. No code in this
    repository outside `lang_status.rs` matches on it. The PR says so
    (see 2010 below).
- **The users:**
  - the toy (`Toy::new`, `crates/ui/src/toy.rs:451`) builds its `Lang`
    with `with_engine_english`. Its debug pack (`DEBUG_UI`, which
    translates `keys.arrows`) is checked against `merged_english`. The
    header of `toy/en.ron` says what is left in it: the toy's own keys;
  - hello-grid: `Hello::load_from`
    (`examples/hello-grid/src/lib.rs:180`) builds its `Lang` (line 222)
    with `with_engine_english(shuyi_ui::text::engine_english())`, and
    `files/en.ron` drops each engine key whose words equal the engine's
    (82 today). Its comments say which engine keys it keeps and why. It
    keeps its own words for `profile.title` ("Profiles") and
    `profile.question.delete`, which shows that a game's key wins;
  - both templates. The grid template's `Data::load`
    (`templates/grid/crates/game/src/lib.rs:95`, its `Lang::new` at
    line 119) makes the call. The blank template gets it through
    `BasicData::load` (`crates/ui/src/basic.rs`, 2511, its `Lang::new`),
    which it uses. Both `en.ron` drop the three `keys.*`
    lines. `layout.arrows` stays: it names their keymap's layout
    (`files/keymap.ron`). This also gives a new game's first profile
    its name, "Player 1".
- **An ADR** (the next free number, `write-adr` skill): the engine's
  screens bring their own English beneath the game's; the game's key
  wins; the engine's keys are exactly those its code names; a game opts
  in with one call, since a `Lang` is the game's and `Ctx` has no
  globals; how this sits with ADR-0020 (labels are keys).
- `docs/formats.md`, its **A language pack** bullet: one sentence on
  `LangPackRon::with_engine_english`.
- **Either order:**
  - **2511** (done): `BasicData::load` builds its `Lang` with
    `with_engine_english`, as the bullet above says.
  - **2226** (language tags and a source language the game declares):
    if it is done, the engine's English sits beneath the game's source
    text whatever language the game declares; if not, 2226 keeps it
    there.
  - **2348** (plurals in text): if it is done,
    `profile.message.too_many` uses its plural form; if not, 2348
    changes it with the rest.
  - **3047** (parked, crash reports; adds four engine keys:
    `options.row.crash_reports`, `options.value.ask`,
    `options.value.always`, `options.value.never`): if it is done, they
    are in the toy's `en.ron`, and this PR moves them into the engine's
    file with the rest. If not, nothing to do (3047 puts them straight
    in the engine's file).
  - **From 2321 (done):** its three keys, `options.row.frame_cap`
    ("Frame rate"), `options.value.fps` ("{n} FPS") and
    `options.value.unlimited` ("Unlimited"), are in the toy's `en.ron`
    and hello-grid's. This PR moves them into the engine's file with the
    rest, and hello-grid drops them.
  - **2333, 2340, 2342, 2346, 2349 and 3043, and the parked
    2323** (each adds engine keys): any done first put its English in
    the toy's `en.ron`, and some in hello-grid's too. This PR moves
    those keys into the engine's file with the rest, and hello-grid
    drops the ones whose words equal the engine's. Any done after puts
    its English straight in the engine's file.
  - **2404** (parked: hold or toggle, and the input timings set by the
    player) adds engine keys too: if it is done first, its rows' keys
    are in the toy's `en.ron`, and this PR moves them with the rest; if
    not, nothing to do (2404 puts them straight in the engine's file).
  - **2353** (a pseudo-language): if it is done, a test builds a `Lang`
    with `with_engine_english` and then `with_pseudo`, and the pseudo
    pack has an entry for an engine-only key; `with_pseudo`'s doc says
    to call it after `with_engine_english`. If not, 2353 covers the
    merged map.
  - **2330** (UI declared in a `.ui.ron`): if it is done and an engine
    screen's text keys now sit in a `.ui.ron` under `crates/ui`, the
    integration test also collects those keys, so the engine's file
    keeps them. If not, 2330 extends this test when it moves the
    screen.
  - **From 2006 (done):** the root `shuyi.ron`'s `check_keys.assets`
    gets `crates/ui/lang`, and `cargo run -q -p shuyi-cli -- lint`
    passes on it.
  - **2010** (done, public API diffs): the completion notes and the PR
    body have the `Breaking:` line its check asks for, for
    `Part::Engine`.
  - **2802** (the book): if it is done, chapter 2 gets a paragraph on
    the engine's words and on overriding one; if not, 2802 writes it.
  - **2227** (parked, PO files): if it is done, `write_po` writes a
    key whose English is the engine's (`Lang::is_engine`) with
    `#: engine` in place of the game's `ui.ron`; if not, nothing to do
    (2227's clause does it).

**Out (do not do):**
- The engine's words in other languages (2352, parked).
- Keys a game names, which stay the game's: `key_bindings.action.*` (a
  `KeyRow`'s label), `key_bindings.dialogue_log` (a `RowGroup::Shared`
  heading), `layout_picker.legend.*`, layout names from the keymap file
  (`layout.arrows`, `layout_picker.right_handed`), and the labels of
  rows the game adds (`options.row.key_bindings`, `options.row.theme`,
  `options.row.layout`, `options.row.look`, `options.help.none`). A
  ready-made `OptionRow::key_bindings()` is not this ticket.
- The `dialogue.*` keys in the toy's and hello-grid's `en.ron`. The
  text box view takes words (`SkipView`, `WaitView::log_hint` in
  `crates/ui/src/dialogue/text_box.rs`), and the screen that fills it
  is the game's, so they are the game's keys and stay where they are.
- Deleting the game's copies of the engine's keys: a game PR after its
  pin moves, if Nick's game wants the engine's words.
- Checking that a game's own code names only keys its English has
  (`check-text` counts literals; it doesn't resolve keys).

## Implementation steps

1. Read ADR-0020, `crates/assets/src/lang.rs`,
   `crates/assets/src/lang/formats.rs`, `crates/cli/src/lang_status.rs`,
   `crates/cli/src/lib.rs` (lines 60 to 70 and 239 to 260) and
   `crates/ui/src/toy.rs` (lines 339 to 478). Check which of
   2226, 2330, 2333, 2340, 2342, 2346, 2348, 2349, 2353, 2511,
   2802 and 3043 are done.
2. In `shuyi_assets::lang`, write `merged_english`,
   `Lang::with_engine_english`, `Lang::is_engine`,
   `from_files_with_engine` and `LangPackRon::with_engine_english`,
   with their tests.
3. Create `crates/ui/lang/en/ui.ron` by moving the engine's keys out of
   `crates/ui/src/toy/en.ron` and the toy's `ENGLISH`. Write
   `shuyi_ui::text::engine_english()` and the integration test in
   *Tests required* (`mod engine_english;` in
   `crates/ui/tests/it/main.rs`).
4. Build the toy's `Lang` with it, and update `toy/en.ron`'s header.
5. Change `Ctx::text`'s debug message.
6. Add `Part::Engine` to `lang-status`, its `--only engine`, the parse
   error and the two usage lines.
7. Move hello-grid and both templates to the call, and trim their
   `en.ron`. Add the templates' profile test.
8. Write the ADR and the line in `docs/formats.md`; do the either-order
   items that apply.
9. Run the gates, the templates job's commands included.

## Acceptance criteria

- [ ] `crates/ui/lang/en/ui.ron` holds exactly the text keys the
      engine's code names: the string literals that start with an
      engine prefix (`keys.`, `text_box.`, `options.`, `key_bindings.`,
      `layout_picker.`, `profile.`, `name_entry.`) in the non-test,
      non-toy code under `crates/ui/src`, leaving out comments,
      `#[cfg(test)]` items and literals with a `{` in them. No key
      missing, none extra (a test, both ways).
- [ ] The toy's `en.ron` and `ENGLISH` hold none of the engine file's
      keys, every toy test passes, and no frame snapshot changes.
- [ ] With `with_engine_english`, a key both have gives the game's text,
      a key only the engine has gives the engine's, `is_engine` says
      which, and a pack's `status` lists an engine key it lacks as
      missing.
- [ ] A pack that translates an engine key the game's English lacks
      loads through `from_files_with_engine` and
      `LangPackRon::with_engine_english`, and is refused as today
      ("no such key in English") through `from_files` and
      `LangPackRon` without them.
- [ ] `lang-status` shows the "engine screen text" part, `--only engine`
      and `--only ui` split the two, and its usage lines name `engine`.
- [ ] hello-grid's `en.ron` has no line equal to the engine's, and its
      snapshots don't change.
- [ ] A game made from each template names its first profile
      "Player 1", and the CI templates job passes.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] The ADR is merged. The PR says what changes for the game:
      nothing breaks in `shuyi-assets` or `shuyi-ui`, `lang_status::Part`
      gains a variant, and the game may later delete its copies.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-assets`): `merged_english` (game wins, union of keys);
  `with_engine_english` over `has`, `english`, `text` and `status`;
  `is_engine`; `from_files_with_engine` and
  `LangPackRon::with_engine_english` accepting a pack entry for an
  engine key, and the plain ones refusing it.
- Unit (`shuyi-ui`): `engine_english()` parses and isn't empty; each
  public text-key constant (`NOT_MAPPED_TEXT`,
  `options::TITLE` and the rest) is in it.
- Unit (`shuyi-cli`): `parse_args` reads `--only engine`; a
  `lang-status` report with an engine key missing from a pack, and with
  `--only engine`.
- Integration: `crates/ui/tests/it/engine_english.rs` reads the `.rs`
  files under `CARGO_MANIFEST_DIR/src` and collects the string
  literals that start with an engine prefix. It skips `tests.rs`,
  `*_tests.rs`, `tests/`, `grid/testing.rs`, `toy.rs` and `toy/`,
  comments, and each `#[cfg(test)]` item from the attribute to the end
  of its braces, as `check-keys` does (`settings.rs`'s test module
  builds `format!("options.value.{name}")`). It also skips a literal
  with a `{` in it. It checks the keys it found are exactly the keys of
  `crates/ui/lang/en/ui.ron`. It is under `tests/`, so
  `cargo xtask check-no-std` (which reads `src/`) doesn't see its
  `std::fs`.
- Snapshot: the toy's and hello-grid's frame snapshots, unchanged.
- Template: in each template's `crates/game/tests/it/main.rs`, a fresh
  harness's first profile (`h.ctx().profiles().all()[0].name`) is
  "Player 1".

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
