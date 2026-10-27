---
id: "3039"
title: "Web builds carry the notices they owe: a NOTICES.txt with the crates compiled in, the vendored scripts' licenses and the game's license texts, published with the Pages examples"
type: feature
milestone: MVP
model: sonnet-5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-26
---

# 3039 — Web builds carry their notices

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Godot documents what a game owes for the engine and its
third-party parts
(<https://docs.godotengine.org/en/stable/about/complying_with_licenses.html>)
and hands a game the texts at run time (`Engine.get_license_info`).
Bevy leaves it to each game, which usually runs `cargo-about`. Nick,
2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): "license
notices in web builds" is one of the protections in the MVP.

What Shuyi does today (checked 2026-10-20):

- `cargo shuyi web` (`crates/cli/src/web.rs`: `SHELL`, `run`,
  `package`) writes `index.html`, `gamepad.js` and three vendored
  scripts into the project's `web.dist`: `mq_js_bundle.js` (macroquad),
  `sapp_jsutils.js` and `quad-storage.js`. All three are MIT. None has a
  license header, and their texts (`crates/cli/web/*-LICENSE-MIT.txt`)
  stay in the engine's repository.
- The wasm binary holds about 70 crates (Hello, Grid's graph for
  `wasm32-unknown-unknown` from `cargo metadata`): MIT, Apache-2.0,
  Zlib and Unicode-3.0 among them. Six have no license file in their
  package (`include_dir`, `include_dir_macros`, `quad-rand`,
  `quad-snd`, `quad-storage-sys`, `sapp-jsutils`), and three of those
  write their license the old way, `MIT/Apache-2.0`. The engine's own
  crates have none in their folders either: the texts are
  `LICENSE-MIT` and `LICENSE-APACHE` at the repository's root.
  `LICENSE-APACHE` is the standard text, with no copyright line.
- Hello, Grid embeds the Terminus font's atlas
  (`examples/hello-grid/files/font/`, OFL-1.1, whose text must travel
  with the font). `Terminus-LICENSE.txt` beside it never reaches
  `dist/web`.
- `.github/workflows/pages.yml` publishes `examples/hello-grid/dist/web`
  as it is. The live Pages build ships all of the above with no notices.
- `THIRD_PARTY_ASSETS.md` says a game "ships some of these too", and
  that "its own registry lists them". Nothing gives a game that list.

MIT, Apache-2.0, Zlib and OFL-1.1 each ask for their notice or text in
the copies handed out, and a web build is handed out. This ticket makes
`shuyi web` write one `NOTICES.txt` beside the page. The Pages examples,
2801's site and every game with a `web` part then carry it. 3004 writes
the same file into every package later, through the module this ticket
adds.

Why it matters: Visions of Shuyi's web build goes through the same
command, so the game gets the crates' and the web shell's notices
without work of its own, and names its own license texts (its fonts')
in one line of its `shuyi.ron`. The end goal (2026-10-19) is a game an
agent makes "with a web ver" in under an hour; that build should be
fine to publish as it comes out.

## Nick input

None. Meeting these licenses' notice duties isn't a product choice.

## Scope

**In:**
- **`crates/cli/src/notices.rs`** (`pub mod notices` in `lib.rs`), a
  module `web` uses now and `shuyi export` (3004) uses later:
  - `Work { section, name, version, license, source, texts: Vec<Text> }`,
    `Text { label, body }`, and `Section` with `Crates`, `Shell` and
    `Game` (2223 adds its credits as one more);
  - `crates(root, package, triple, features) -> Result<Vec<Work>,
    String>`:
    - runs `cargo metadata --format-version 1 --filter-platform
      <triple>` in `root` (through the `CARGO` variable, as
      `web::target_dir` does), with each feature as `--features
      <package>/<feature>`;
    - parses only the fields it needs with `serde_json` (already a
      dependency of `shuyi-cli`; no new crate);
    - walks `resolve.nodes` from `package` over normal edges only (a
      `dep_kinds` entry whose `kind` is null). Dev- and
      build-dependencies don't ship (`insta`, and `autocfg`, which only
      build scripts use, are left out of Hello, Grid's list). Proc-macro
      crates stay in: listing one crate too many is harmless, leaving
      one out is not;
    - leaves out the game's own crates: the `workspace_members` whose
      `repository` isn't the engine's. They are the game's work, not a
      notice it owes, and a template's crates have no `license` at all.
      In this repository every member is the engine's, so Hello, Grid
      and the engine's crates stay listed;
    - `cargo metadata` unifies features over the workspace, so a crate
      a dev-dependency's feature turns on can be listed (Hello, Grid's
      list holds `shuyi-cli`, which its tests turn on through `tools`).
      That is harmless; don't try to be exact;
    - lists each package once, sorted by name, then version string.
  - **Each crate's texts:**
    - the files at the top of its folder (`manifest_path`'s parent)
      whose name starts with `LICENSE`, `LICENCE`, `COPYING`, `NOTICE`,
      `COPYRIGHT` or `UNLICENSE`, in any case, and its `license_file`
      (relative to that folder);
    - the engine's crates (`repository` equal to `shuyi-cli`'s own,
      `env!("CARGO_PKG_REPOSITORY")`) with no such file get the engine's
      `LICENSE-MIT` and `LICENSE-APACHE`, embedded with `include_str!`
      from the repository's root (`contract/new.rs` already embeds
      `templates/` from there);
    - any other crate with no file gets a text for each id of its
      `license` expression that `shuyi-cli` can write. The ids are split
      on `OR`, `AND`, `WITH`, brackets and the old `/`. `Apache-2.0` is
      the engine's `LICENSE-APACHE` (the standard text). `MIT` is the
      engine's `LICENSE-MIT` with its `Copyright` line replaced by
      `Copyright (c) <authors>`, the crate's `authors` joined by `, `.
      The text's label says it was written from the crate's declared
      license because the package has no file;
    - a crate left with no text at all is still listed, and `shuyi web`
      prints one warning line naming it.
  - `game_texts(root, notices) -> Result<Vec<Work>, String>`: one
    `Section::Game` work per file of the `notices` part below, named by
    its path, with the whole file as its one text. A missing file is an
    error naming its path.
  - `write(dist, title, works) -> Result<(), String>` writes
    `NOTICES.txt`:
    - a heading with the game's title (`web.title`);
    - the works by section (the crates, the web shell's files, the
      game's files), each with its name, version, license and source;
    - then every text once, headed by the works it covers. An Apache-2.0
      text forty crates share is printed once. Texts are compared and
      written with `\r\n` made `\n` and trailing blank lines dropped;
    - the same inputs give the same bytes.
- **The web shell's files,** in `web.rs` beside `SHELL`: a table with a
  row for every file of `SHELL`. A vendored file's row has its work,
  version, license and committed license text (`include_str!` of
  `crates/cli/web/*-LICENSE-MIT.txt`). `index.html`, `gamepad.js` and,
  if 3052 is done, `shell.js` are listed as the engine's own, with the
  engine's texts. `package` writes each vendored script with a first
  line `/*! <file>: <work> <version>, MIT. See NOTICES.txt. */` (a
  comment before `"use strict";` keeps the file strict). The header is
  added only in `dist`: the committed files under `crates/cli/web/` don't
  change, so the vendoring notes in `crates/cli/web/README.md` (and
  `quad-storage.js`'s one local patch) stay true. `web/index.html` gains
  `<link rel="license" href="NOTICES.txt">` in its `<head>`.
- **The game's own license texts:** a `notices` part in `shuyi.ron`:
  `Notices { texts: Vec<String> }` (`deny_unknown_fields`, `default`)
  and `Config.notices: Option<Notices>` in `crates/cli/src/project.rs`,
  for example `notices: (texts: ["files/font/Terminus-LICENSE.txt"])`.
  Each file goes into `NOTICES.txt` whole, under "The game's files". A
  missing file fails the build, naming its path.
- **`web::run` and `package`** write `NOTICES.txt` into `web.dist` on
  every build, debug or release:
  - `run` takes one more argument, `game: &[notices::Work]`, the works
    the project adds. `web` in `crates/cli/src/lib.rs` builds it with
    `notices::game_texts` from `project.config.notices` (none without
    the part) and passes it; 2223 adds the game's credits to the same
    list;
  - `run` gets the crates from `notices::crates` after `cargo build`,
    with `rules.package`, `TARGET` and `options.features`. If `cargo
    metadata` fails, `web` fails with its message: no web build is
    written without its notices;
  - `package` takes every work (the crates, the shell's files from the
    table, the game's) as an argument, so its tests need no cargo;
  - the module's doc comment names `NOTICES.txt`.
- **Hello, Grid** (`examples/hello-grid/shuyi.ron`) gets the `notices`
  part with its Terminus text. **The templates**
  (`templates/grid/shuyi.ron`, `templates/blank/shuyi.ron`) get theirs
  (`crates/game/files/font/Terminus-LICENSE.txt`), ready for 3004's
  exporters: they have no `web` part today.
- **Pages and CI:** the index `pages.yml` writes links
  `hello-grid/NOTICES.txt` beside the example. The `wasm` job in
  `.github/workflows/ci.yml` checks that the `dist/web/NOTICES.txt` it
  built names `macroquad`, `Terminus` and `mq_js_bundle.js`.
- **Docs:**
  - `crates/cli/web/README.md` gets a section "Notices", and its
    *Files* list says the vendored scripts get a header in `dist`;
  - `THIRD_PARTY_ASSETS.md`'s line about a game's "own registry" says
    instead that `shuyi web` writes the web shell's notices into
    `NOTICES.txt`, and that the controller database (native builds
    only) is 3004's;
  - a test keeps every file of `SHELL` in the table, and every vendored
    one in `THIRD_PARTY_ASSETS.md`, so no shell file, this ticket's or
    a later one's, ships without its notice.
- **The PR** says the API changes for the game's next pin bump
  (`AGENTS.md`): `web::run` takes the project's works, and `Config` gains
  `notices`.
- **Size:** if the diff, fixtures and snapshots aside, passes about 600
  changed lines, the vendored scripts' headers and the `rel="license"`
  link (step 2, but not the table) become a follow-up ticket in this
  milestone, blocked by this one (the `write-ticket` skill); the PR says
  so.

**Either order:**
- **2801** (the website): if it is done, its `pages.yml` copies each
  example's `dist/web` whole into `public/examples/<name>/`, so
  `NOTICES.txt` goes with it. Put the link on its Examples page instead
  of the hand-written index. If 2801 isn't done, link it from today's
  index; 2801's step 4 moves that index into its Examples section.
- **3052** (the web shell's progress bar and errors) changes the same
  `index.html`. Whichever lands second keeps the `rel="license"` link
  and the scripts' headers. If 3052 lands first, `shell.js` goes in the
  table as the engine's own; if second, 3052 adds it.
- **2601** (HTTP requests, parked): its web backend, a follow-up
  ticket 2601 writes, adds `http.js` to `SHELL`. If that ticket is done
  first, `http.js` goes in the table as the engine's own here; if not,
  it adds its row, and the test above fails until it does.
- **Other tickets that add a file to `SHELL`** (3043's `lifecycle.js`,
  2519's `download.js`, 2349's `languages.js`, 2377's `motion.js`,
  2522's `net.js`, 3033's `storage.js` and others): if one is done
  first, its file goes in the table as the engine's own here. If this
  ticket is done first, that ticket adds its row, and the test above
  fails until it does. 3033 (parked) also removes `quad-storage.js` and
  `sapp_jsutils.js`: if 3033 is done, list only the vendored files
  `SHELL` still holds; if not, 3033 drops their rows.
- **From 1134 (done)** (the skill pack): its `shuyi-ship` skill
  (`skills/shuyi-ship/SKILL.md`) says that `shuyi web` writes
  `NOTICES.txt` beside the page, what it lists, and that a game names
  its own license texts in the `notices` part of `shuyi.ron`;
  `check-skills` passes.
- **From 2501 (done):** the templates' `notices` part goes in the
  `base` part's `shuyi.ron` in `templates/shuyi-default/`, not in the
  two folders above (which are gone).
- **2739** (parked, the snake example): if 2739 is done, its
  `examples/snake/shuyi.ron` gets the `notices` part with its Terminus
  text, as Hello, Grid's does, and Pages links its `NOTICES.txt` beside
  it. If not, nothing to do (2739 does it).
- **3058** (parked, screen reader output on the web): its plugin
  `access.js`, if it writes one, joins `SHELL`. If 3058 is done first,
  `access.js` goes in the table as the engine's own here; if not, 3058
  adds its row, and the test above fails until it does.

**Out (do not do):**
- packages other than the web build, and the SDL_GameControllerDB that
  native builds compile in: 3004 (and 3010 through it), which reuses
  this module;
- the game's credits in the notices, and which licenses a game allows
  (2223);
- a screen that shows the notices in the game;
- choosing one license of an `OR` expression, or any legal judgment
  beyond copying what the packages say;
- scanning where a file came from;
- changing which licenses the engine may ship (`deny.toml`, ADR-0002
  §8).

## Implementation steps

1. Write `notices.rs` and its tests on a small hand-written
   `metadata.json` fixture: four packages (the root, a normal
   dependency with a license file, an `MIT/Apache-2.0` one with none, a
   dev-dependency), a build edge and a second workspace member from
   another repository, with `manifest_path`s pointing into a temporary
   folder that holds their license files. Add `insta.workspace = true`
   to `shuyi-cli`'s `[dev-dependencies]` for the snapshot (`insta` is
   already the workspace's, Apache-2.0).
2. Add the shell's table, the headers and the `rel="license"` link
   (`web.rs`, `web/index.html`).
3. Add `Notices` to `Config`, and the part to Hello, Grid and the
   templates.
4. Wire `run`, `package` and `web` in `lib.rs`. Update the `package_*`
   tests in `web.rs` that read `dist`'s files (the vendored scripts now
   start with their header).
5. Update `pages.yml`, `ci.yml`, `crates/cli/web/README.md` and
   `THIRD_PARTY_ASSETS.md`.
6. Run the gates.

## Acceptance criteria

- [x] `cargo shuyi web --release` in `examples/hello-grid` writes
      `dist/web/NOTICES.txt`. It lists `macroquad`, `shuyi-core`, the
      three vendored scripts and Terminus, with the MIT, Apache-2.0,
      Zlib and OFL-1.1 texts.
- [x] It doesn't list `insta` (a dev-dependency) or `autocfg` (only
      build scripts use it).
- [x] `quad-snd`, which has no license file and declares
      `MIT/Apache-2.0`, is listed with an MIT text naming its author and
      the Apache-2.0 text.
- [ ] Each vendored script in `dist/web` starts with its `/*!` header.
      **Moved to 3066** (the Size rule), with the `rel="license"` link.
      The committed copies under `crates/cli/web/` are unchanged.
- [x] Each text is printed once, and two runs on the same inputs give
      the same bytes (test).
- [x] A `notices.texts` file that doesn't exist fails `web`, naming its
      path.
- [x] A file added to `SHELL` without a row in the table fails a test.
- [x] The Pages index links `hello-grid/NOTICES.txt`, and the `wasm`
      CI job checks the file.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit:
  - the graph walk on the fixture: normal edges only, each crate once,
    sorted, the other repository's workspace member left out;
  - finding license files by name, and `license_file`;
  - the engine's texts for the engine's crates;
  - splitting a license expression, the old `/` form included, and the
    standard MIT text with the authors;
  - grouping identical texts, `\r\n` and `\n` alike;
  - `game_texts`, and its error for a missing file;
  - parsing the `notices` part, and refusing an unknown field in it.
- Snapshot: `NOTICES.txt` from the fixture (insta).
- Integration:
  - `crates` on Hello, Grid's real graph for `wasm32-unknown-unknown`
    holds `macroquad` and `quad-snd`, and not `insta` or `autocfg`;
  - `package` writes `NOTICES.txt` and the headers;
  - the shell's table has a row for every file of `SHELL`, and its
    vendored files match `THIRD_PARTY_ASSETS.md`.

## Completion notes

- **`crates/cli/src/notices.rs`** (`pub mod notices`): `Work`, `Text`
  and `Section` (`#[non_exhaustive]`, with `Work::new` and `Text::new`,
  ADR-0039), `crates` (`cargo metadata` for the triple, normal edges
  from the package, the game's own members left out, sorted by name and
  version), each crate's texts (its license files by name and its
  `license_file`; the engine's `LICENSE-MIT` and `LICENSE-APACHE` for
  the engine's crates; else Apache-2.0 and MIT-with-its-authors written
  from the declared license), `game_texts`, `engine_texts` and `write`
  (by section, then each text once, `
` and trailing blank lines made
  alike). Tests on a hand-written `metadata.json` fixture, an insta
  snapshot of the file, and the walk on Hello, Grid's real graph.
- **`web.rs`:** `SHELL_NOTICES`, a row per file of `SHELL` (the three
  vendored scripts with their work, version, source and committed MIT
  text; `index.html` and `gamepad.js` as the engine's own), and
  `shell_works`. `run` takes the project's works, gets the crates after
  `cargo build`, warns once naming crates with no text, and `package`
  writes `NOTICES.txt` from every work on every build. A test keeps
  `SHELL` and the table equal, and the vendored rows equal to
  `THIRD_PARTY_ASSETS.md`'s (version and license file included).
- **`shuyi.ron`:** `Config.notices: Option<Notices>` (`Notices { texts }`,
  `deny_unknown_fields`, `#[non_exhaustive]`). Hello, Grid names its
  Terminus text; the template's `base/shuyi.ron` names
  `crates/game/files/font/Terminus-LICENSE.txt` (`plans.golden`
  re-blessed).
- **CI and Pages:** the `wasm` job checks that `NOTICES.txt` names
  `macroquad`, `Terminus` and `mq_js_bundle.js`; the Pages index links
  `hello-grid/NOTICES.txt` (2801 isn't done).
- **Docs:** `crates/cli/web/README.md` (a *Notices* section),
  `THIRD_PARTY_ASSETS.md` (the line about a game's registry), and the
  `shuyi-ship` skill's web step.
- **Hello, Grid's real build:** 86 crates, 60 distinct texts, about
  190 KB. Every crate had a text, so no warning was printed.

**Deviations:**
- **Split by the Size rule.** The diff, fixtures and snapshots aside,
  came to about 1,100 lines (the module, its tests and the web tests
  were bigger than the ticket guessed), so the vendored scripts' `/*!`
  headers and the page's `rel="license"` link moved to **3066** (MVP,
  blocked by this one), as the Size line says. Its acceptance line is
  left unticked above. The clauses of 3049 and 3052 about that link now
  name 3066.
- A crate that declares MIT but names no author gets no written MIT
  text (it has no holder to name), and so may end with none: it is
  listed with "(no license text found)" and named in the warning.
- A crate's source is its `repository`, else its crates.io page for a
  crates.io crate, else empty.

**Either-order tickets:** none of 2801, 3052, 2601, 3043, 2519, 2349,
2377, 2522, 3033, 2739 or 3058 was done, so each kept its own row;
their clauses now say "From 3039 (done)" and name `SHELL_NOTICES`.

Breaking: shuyi-cli: web::run: takes a fourth argument, `game: &[notices::Work]`, the project's works for `NOTICES.txt` (pass `&[]` for none)
Breaking: shuyi-cli: project::Config: gains `notices: Option<Notices>`; a literal adds `notices: None` or ends with `..Config::default()`
