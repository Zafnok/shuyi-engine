---
id: "3059"
title: "A one-language build: shuyi bake --lang and shuyi export --lang keep one language's packs and start the game in it"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["2226", "2349", "3004"]
nick_input: none
completed:
---

# 3059 — A one-language build

## Context

- **The game ticket.** game#1049
  ([`1049-playdate-one-build-per-language.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1049-playdate-one-build-per-language.md)):
  one Playdate build per language, side by side on one device, each
  starting in its language. Its bundle per language "uses the engine's
  language layers". It isn't a blocker: the game's ticket waits for
  this one.
- **The promise.** 2349 (the player's language) gives a pack's folder
  its own copies of files (`lang/<tag>/files/<path>`), and
  `FirstLanguage::Fixed` for a build that ships one language. It says a
  one-language build "ships a layer without the other packs' folders".
  Its Out left making that layer (a `--lang` flag on `shuyi bake`, or a
  choice in `shuyi export`) to a ticket of its own: this one.
- **The job:** a developer bakes or exports a build that carries one
  language, and the languages its tag falls back to, and the game
  starts in it.
- **Friction:** no friction review: a missing feature.
- **The generic shape:** a pack baked with only the language folders on
  one tag's chain. The pack records that tag, so the game answers
  `FirstLanguage::Fixed` from it.
- **Outside tactics:** a word puzzle game on a handheld with a small
  package budget, sold as one build per language; a visual novel whose
  title pictures and signs are drawn per language, where each
  language's `files/` copies weigh more than its text.
- **Shuyi today** (read 2026-10-24):
  - `shuyi bake [--target <id>] [--out <file>]`
    (`crates/cli/src/contract/bake.rs`, from 2204) bakes every file of
    the bundle the game's `GameTools::bake(target)` returns
    (`GameBake { bake, bundle, formats }`, `crates/cli/src/game.rs`).
    `Bake::bake` (`crates/assets/src/pack/bake.rs`) walks
    `bundle.files_under("")`.
  - The pack's index holds `PackInfo { stamp, target, content }`
    (`crates/assets/src/pack.rs`, `pack::VERSION` 3 since 2111; re-read
    2026-10-28). `content` is `bundle.fingerprint()` of the bundle it
    was baked from (`Bake::bake`, `pack/bake.rs`), and a game's stamp
    (`GameTools::made`, `shuyi_core::Made`) holds the same fingerprint.
    `shuyi replay` refuses a record made with other content
    (`crates/cli/src/contract/replay.rs`). The game opens its pack
    itself (`Pack::open`, as Hello, Grid's `Hello::from_pack` does), so
    it can read `Pack::info`.
  - `Bundle` (`crates/assets/src/bundle.rs`) reads layers of `'static`
    files. `Layer::leaving_out` names files a layer lacks on purpose; it
    removes none. Nothing hides a folder of a bundle.
  - The game's languages come from `GameTools::lang()` (`GameLang`, for
    `lang-status`): a `Lang` with every pack. 2226 gives `Lang::chain`,
    `Lang::source_lang`, `LangCode::parse`, and pack folders named by
    canonical tags.
  - `shuyi export <target>` (3004, done) runs the target's check and
    its exporter (`crates/cli/src/export.rs`). No exporter puts a pack
    in its package: `shuyi.ron` has no setting by which a game asks for
    one (2204 added none), so a desktop or web package holds the
    program with the game's files built in. Only the Playdate's (3010)
    will bake a pack.

## Nick input

None. Which languages a game builds alone, and how it names each build
in a store, are the game's.

## Scope

**In:**

- **A bundle without some folders.** `Bundle::without(self, dirs:
  Vec<String>) -> Self`: this bundle with every file under `dirs` gone,
  in every layer. `bytes`, `file`, `has`, `files_in`, `files_under`,
  `test_part`, `load_each` and `fingerprint` see none of them
  (`fingerprint`, in `bundle/fingerprint.rs`, walks each layer's files
  itself, so it skips them there too). A folder is a whole
  path segment: hiding `lang/pt` keeps `lang/pt-BR/ui.ron`. `Bundle`'s
  fields are private, so this is additive. It builds without `std`, as
  the rest of the file does.
- **The folders off a language's chain.**
  `shuyi_assets::lang::folders_off_chain(languages: &str, lang: &Lang,
  code: &LangCode) -> Vec<String>`, pure: `<languages>/<tag>` for every
  pack of `lang` that isn't on `lang.chain(code)`, in tag order.
  - The source's folder is never one: every chain ends at it, and it
    holds the keys' own text.
  - With packs `fr`, `pt` and `pt-BR`: `pt-BR` gives `lang/fr`; `pt`
    gives `lang/fr` and `lang/pt-BR`.
  - The game's debug language is a pack like any other: off the chain,
    it goes.
- **The pack says its language.**
  - `PackInfo::lang: Option<String>`: the canonical tag the pack was
    baked for (`shuyi bake --lang`), if any, and
    `PackInfo::lang_code(&self) -> Option<LangCode>`.
  - The index changes, so `pack::VERSION` becomes the next number (4,
    from 3 today) and the snapshot of the binary form is updated. A
    pack baked before is refused with "bake it again", as any other
    version is.
  - **The content of a one-language pack** is the fingerprint of what
    it carries: `Bake::bake` already writes `bundle.fingerprint()`, and
    the bundle it is handed is `bundle.without(…)`. So it differs from
    the full bundle's, and from another language's pack's. A game that
    ships it stamps its records and saves with that content (the
    fingerprint of a bundle over the pack is `PackInfo::content`), so
    `shuyi replay` on the full build refuses a record made on the `ja`
    build as made with other content, and runs it with `any_stamp`.
    That is on purpose: the files differ. `docs/formats.md` says so.
  - `PackInfo` becomes `#[non_exhaustive]` (ADR-0039: it has grown).
  - `Bake::bake_for_language(&self, formats, bundle, target:
    Option<&str>, lang: &LangCode) -> Result<Baked, Vec<ContentError>>`
    bakes as `Bake::bake` does and records `lang`. `bake` keeps its
    signature and records none.
- **The game names its languages' folder.**
  `GameBake::with_languages(self, dir: impl Into<String>) -> Self` and
  the field `GameBake::languages: Option<String>` (`lang`). This is
  additive: `GameBake` is `#[non_exhaustive]` with `GameBake::new`. A
  game that doesn't say can't be baked for one language.
- **`shuyi bake --lang <tag>`.**
  - `BakeInput::lang: Option<String>`, read with `LangCode::parse` (any
    case, `_` for `-`). A tag that doesn't parse is `WRONG_ARGUMENTS`
    (exit 2), with `TagError`'s words.
  - The bake takes the game's `GameBake` for the target, as today, and
    the game's `Lang` from `GameTools::lang()`. It refuses:
    - a game whose `GameBake` names no languages' folder, with
      `NOT_OFFERED` and a line on `GameBake::with_languages`;
    - a tag that isn't the source and whose chain holds no pack, with a
      new code `language` (the next free `SH3nnn`, with its row in
      `codes.rs` and its page in `crates/cli/errors/`). The message
      names the game's languages: `no pack for de or its parents; the
      game's languages are en, fr, ja`.
  - Then it bakes `bundle.without(folders_off_chain(…))` with
    `Bake::bake_for_language`. The load times run on that same bundle.
  - With `--target`, the bundle is the game's for that target, every
    layer of it. A target layer's copy of an off-chain language's file
    goes too, since a path is the same in every layer.
  - The default file is `target/shuyi/<target or game>-<tag>.shuyipak`
    (`playdate-ja.shuyipak`).
  - `PackMade` gains `lang: Option<String>` and `without: Vec<String>`
    (the folders). The text says `for playdate in ja` and lists the
    folders left out.
  - The MCP `bake` tool takes `lang` from the input's schema, with no
    code of its own.
- **Starting in the build's language.** The language reaches the game
  through the pack it opens. A game answers `GameData::first_language`
  with `FirstLanguage::Fixed(code)` when `pack.info().lang_code()` gives
  one, and with its usual policy otherwise. The docs of
  `FirstLanguage` (`crates/ui/src/settings.rs`) show those lines.
  - The game's `Lang`, loaded from that pack, holds only the source and
    the packs on the chain. So 2349's `languages_offered`, and the
    options row built on it, offer only what shipped.
  - A player who picks another of them keeps it: `Settings::language`
    comes first, in 2349's order.
- **`shuyi export <target> --lang <tag>`** (3004's operation):
  - `--lang` in its arguments and `lang` in its input. An exporter that
    puts the game's pack in its package bakes it as `bake --lang` does,
    through one function both call.
  - The package's name, and the `.debug` file beside it, gain `-<tag>`
    before the extension (`<name>-windows-ja.zip`). So builds for two
    languages sit side by side in one `--out` folder.
  - `export-report.json` gains `lang` and the folders left out.
  - An export whose package holds the game's loose files, built into
    the program, refuses `--lang` with the `language` code: their
    folders can't be left out without a pack. The message says to ship
    a pack (`docs/formats.md`, *The pack*).
- **Either order with 3010** (the Playdate's exporter): if 3010 is
  done, its `playdate` exporter takes `--lang`. It bakes the `playdate`
  pack for that language, as `bake --target playdate --lang` does, and
  names the `.pdx` per language as the other exports do. Its `pdxinfo`
  is per language too: `bundleID` is the game's id with the tag after a
  dot, in lower case (`com.example.game.ja`), and `name` is the game's
  name with the tag in brackets (`Game (ja)`). Check the SDK's docs for
  the characters a bundle id may hold, and record it. Builds for two
  languages then sit side by side on one device, and each keeps its
  saves in its own data folder. A unit test reads the `pdxinfo` of a
  `--lang ja` export, and `docs/targets/playdate.md` says how to make
  one build per language. If not, nothing to do: 3010 does it (its
  clause says so).
- **The schemas.** `bake`'s and `export`'s input and answer change in
  one contract version: `cargo shuyi schema --out crates/cli/schemas`,
  `contract::VERSION` up by one, and its line in `versions.json`
  (`docs/contract.md`, *The schemas*).
- **Docs.**
  - `docs/formats.md`, *The pack*: the language in the info, the new
    `pack::VERSION`, and a one-language pack: what it keeps, its
    content fingerprint, and how the game starts in it.
  - `docs/contract.md`: the `bake` and `export` rows with `--lang`, the
    `language` code, and the `bake` row of the game's tools table
    (`GameBake::with_languages`).
  - The module docs of `contract/bake.rs`.
- **For the game:**
  - `pack::VERSION` up by one, so its packs are baked again.
  - A one-language build's records and saves carry that build's
    content, not the full bundle's.
  - `PackInfo` gains a field and becomes `#[non_exhaustive]`. The
    completion notes carry a `Breaking:` line for each break CI's `api`
    job finds (ADR-0038).
  - `GameBake::with_languages` is additive. A game that bakes one
    language per build calls it in its `GameTools::bake`, and answers
    `first_language` from the pack.
  - The game's PR that moves the pin follows this one, and builds
    game#1049 on it.

**Out (do not do):**

- A command-line language switch for a shipped game (as Unity's
  command-line selector): a ticket of its own when a game asks.
- One-language builds of a game that ships its loose files, built into
  the program: such an export refuses `--lang` (above). A ticket of its
  own when a game asks.
- Leaving out the source language: every chain ends at it.
- Some languages in one build but not all (`--lang ja --lang zh-Hans`):
  a ticket of its own when a game asks. `FirstLanguage::Fixed` takes
  one.
- A store or launcher name the game writes per language (the Playdate's
  `name` gets the tag only): a ticket of its own when a game asks.
- `check --baked --lang`: a test here proves a one-language pack equal
  to its sources, and `check --baked` keeps proving the whole pack.
- Releases from a tag that build one package per language: a game that
  wants them runs `export --lang` once per language.

## Implementation steps

1. Read 2349's result (`FirstLanguage`, `GameData::first_language`,
   `languages_offered`, a pack's `files/`); 2226's `Lang::chain`,
   `Lang::source_lang` and `LangCode::parse`; and 3004's `export`
   operation, its `Exporter` trait (no step puts a pack in a
   package yet). Read `crates/cli/src/contract/bake.rs`,
   `crates/assets/src/pack.rs`, `pack/bake.rs` and `bundle.rs`. Check
   whether 3010 is done, for its clause.
2. Add `Bundle::without`, with its tests in
   `crates/assets/src/bundle.rs`.
3. Add `lang::folders_off_chain` in `crates/assets/src/lang.rs`.
4. Add `PackInfo::lang` and `lang_code`, `#[non_exhaustive]`, the next
   `pack::VERSION` and `Bake::bake_for_language`. Update the snapshot
   of the binary form (`cargo insta review`) and the tests that build a
   `PackInfo` (`crates/assets/src/pack/tests.rs`).
5. Add `GameBake::with_languages`, then `--lang` in `contract/bake.rs`:
   `BakeInput`, `parse_args`, `usage`, `out_file`, `PackMade` and
   `text`. Add the `language` code with its row and page. Give the fake
   game of `contract/fake.rs` a languages' folder with packs `fr`, `ja`,
   `pt` and `pt-BR`, a `files/` copy in `ja`, and a `lang()` that loads
   them.
6. Add `--lang` to `export` as above.
7. Regenerate the schemas and raise `contract::VERSION`.
8. Write the docs. Run the gates.

## Acceptance criteria

- [ ] `Bundle::without(vec!["lang/fr".into()])` gives no
      `lang/fr/ui.ron` through `bytes`, `has`, `files_in` and
      `files_under`, in any layer; hiding `lang/pt` keeps
      `lang/pt-BR/ui.ron` (unit tests).
- [ ] With packs `fr`, `pt` and `pt-BR`, `folders_off_chain` gives
      `["lang/fr"]` for `pt-BR`, `["lang/fr", "lang/pt-BR"]` for `pt`,
      and every pack's folder for the source (unit tests).
- [ ] `shuyi bake --lang ja` on the fake game writes
      `target/shuyi/game-ja.shuyipak`. It holds the source's folder and
      `lang/ja/` with its `files/` copy, and nothing under `lang/fr/`,
      `lang/pt/` or `lang/pt-BR/`. Its answer lists those folders, and
      the pack's `Pack::info().lang` is `ja` (test in
      `contract/bake/tests.rs`).
- [ ] `--lang JA` gives the same pack as `--lang ja`; `--lang 7` exits
      2 with `WRONG_ARGUMENTS`; `--lang de` gives the `language` code
      and names the game's languages; a game with no languages' folder
      gives `NOT_OFFERED` (tests).
- [ ] `Bake::check` finds no difference between the one-language pack
      and the bundle without the other folders (unit test).
- [ ] A pack of version 3 is refused with "bake it again", and the
      snapshot of the binary form holds the new index.
- [ ] The `ja` pack's `Pack::info().content` equals
      `bundle.without(folders).fingerprint()` and differs from the full
      bundle's `fingerprint()`; a hidden file's bytes changed leave it
      the same (unit tests).
- [ ] A `Ctx` whose game loads its `Lang` from the `ja` pack, and
      answers `first_language` from `PackInfo::lang_code`, starts in
      `ja`; `languages_offered` gives the source and `ja` only
      (integration test in `crates/ui/tests/it/`).
- [ ] `shuyi export` with `--lang ja`, through a test exporter that
      ships a pack, names its package `…-ja`, puts the one-language
      pack in it, and writes `lang` in `export-report.json`. Through
      one that ships loose files it refuses with the `language` code
      (unit tests).
- [ ] The schemas are regenerated and `contract::VERSION` is one
      higher (the schema test passes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Bundle::without` (each reader, the segment boundary, every
  layer); `folders_off_chain` along chains; `PackInfo::lang` written and
  read back; `bake --lang`'s arguments, refusals, default file and
  answer; the export's name per language, and its refusal.
- Integration: a one-language pack read back equals its sources
  (`Bake::check`); a game started from it is in its language and offers
  only what shipped.
- Snapshot: the pack's binary form at the new version.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
