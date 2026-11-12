---
id: "3300"
title: "Desktop and web exports ship a baked pack beside the program, not the files folder embedded as text"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3099"]
nick_input: none
completed: 2026-11-11
---

# 3300 — Desktop and web exports ship a baked pack

## Context

The default template embeds the whole `files/` folder with `include_dir!`
(`templates/shuyi-default/base/crates/game/src/lib.rs:22`). A desktop or
web export therefore carries every RON file as written, with its original
path: `strings` on the exe or the Wasm lists every rule table, map and
line of text. Only the Playdate export bakes a pack (`crates/cli/src/export/playdate.rs`,
`BAKED`), which the device reads from its `.pdx`
(`crates/playdate/src/backend.rs::read_pack`).

The pack exists since 2204 ([ADR-0023](../../docs/adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md)):
a postcard index of paths and kinds, baked postcard sections per kind,
LZ4 where smaller, proven equal to its sources by `Bake::check`, and
measured 2.5× (native) to 4× (web) faster to load. ADR-0023 says "a game
that ships a pack loads faster and ships less". This ticket makes the
desktop and web exports do it.

Research ticket 2917 ([report](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md),
sections 1 and 3) is why it is in the public-demo-defenses track: the
history of Godot, Unreal, RPG Maker and Ren'Py says a pack's obscurity
buys hours at most and encryption buys nothing a public engine can keep,
so this ticket claims no protection. Its value is tidiness and speed: no
RON text and no paths in the program, baked data instead of parsed text,
and the pack a single file the disclosure gate (3081) already lists.

Related: 2215 (parked) replaces `include_dir` with an in-house build-script
table for the embedded path. **Either order with 2215:** if 2215 is done,
the template's unpacked path uses its table and this ticket touches only
the packed path; if not, the unpacked path keeps `include_dir!` and 2215
replaces it later, the packed path being unaffected either way.

**From 3082 (done):** `shuyi.ron`'s `editions` build a public demo from its
own folder of files (`content`), handed to the build as `SHUYI_CONTENT`, which
the template's `build.rs` points `include_dir!` at
([ADR-0100](../../docs/adr/0100-editions-named-builds-in-shuyi-ron-a-content-root-handed-to-the-build-and-default-features-always-said.md)).
The packed path must bake an edition's folder, not `files/`.

**From 3304 (done):** `BakeInput::content` (`bake --content <folder>`) bakes
a content root by the game's tool program built with it as `SHUYI_CONTENT`, in
the Cargo profile `shuyi-content`
([ADR-0101](../../docs/adr/0101-an-editions-pack-is-baked-by-the-game-s-tool-program-built-with-its-content-root.md)).
Bake an edition's pack through it, as `playdate::bake_input` does for the
Playdate.

## Nick input

None.

## Scope

**In:**
- A `packed` Cargo feature in the template's game crate: with it, the
  game's `files` layer comes from a pack file rather than `FILES`
  (`Layer::dir`). The desktop program reads `<name>.pack` beside itself
  (and the macOS bundle's `Resources/`); the web page fetches it once
  beside `index.html`, through the existing loader, before the first
  frame.
- `shuyi export windows|linux|macos|web` in release bakes the pack with
  `contract::bake` for the target's profile (as the Playdate exporter
  does), builds the game with `packed`, places the pack in the package,
  and runs `Bake::check` against the source files before packaging.
- The pack carries no original paths it does not need: the index keeps
  the path (loaders look files up by it) but the exporter fails if a path
  holds a build-machine prefix (ADR-0058's search, extended to the pack).
- The disclosure gate (3081) lists the pack as one `Shipped` entry and
  also scans its entries' paths and raw (non-baked) bytes for the
  manifest's canaries, since a pack hides nothing from the gate.
- `shuyi run` and `cargo run` stay on the embedded files: a developer
  never needs a pack to play.
- Measurements in `docs/formats.md`: package bytes and load time of
  `hello-grid`, embedded against packed, native and web.
- An ADR extending ADR-0023 (ADR-0109): packs are the shipping form on
  every target; the embedded folder is the development form. ADR-0023's
  status line names it.

**Out (do not do):**
- Encrypting or signing the pack, or hiding the index. Section 3 of
  2917's report says why; 2603 excludes a client-held key.
- Removing `include_dir` from the template's development path (2215).
- Changing the pack format's `VERSION` unless a path check needs it; if it
  does, a `Breaking:` line and migration block (ADR-0038).
- Playdate, GBA or NDS exporters: unchanged.

## Implementation steps

1. Read `contract::bake`, `Bake`, `Pack::into_layer` and the Playdate
   exporter's bake step; decide where the pack lands per target
   (`<base>/<name>.pack` on Windows and Linux, `Contents/Resources/` on
   macOS, `dist/<name>.pack` on the web) and write it in `ExportReport`.
2. Template: add `packed = []` to `crates/game/Cargo.toml`; in `lib.rs`,
   `game()` builds its `files` layer from `FILES` without the feature and
   from `shuyi::app::pack_beside_program(APP)` with it. Add that helper to
   `shuyi-app` (native: read the file next to `current_exe`; web: the
   loader's fetch, see `crates/cli/web/` and `crates/app/src/backend.rs`
   for where the page hands bytes to Rust). Each is additive public API
   (ADR-0039).
3. Exporters: in `export/desktop.rs` and `export/web.rs`, before the cargo
   build in release, call `bake` as `playdate.rs:823-845` does with the
   target's profile; pass `--features <package>/packed`; copy the pack into
   the package; run `Bake::check`; fail with the first difference.
4. Extend ADR-0058's path search (`ExportCx::search`, `export/paths.rs`)
   to the pack's index paths.
5. 3081's gate: a `FileKind::Pack` entry; open it with `Pack::open` and
   scan entry paths and raw bytes for canaries; add the fixture game's
   canary inside a pack to the gate's tests.
6. Measure and write `docs/formats.md`; write the ADR extending ADR-0023; update
   the book's export chapter and the `shuyi-export` skill if one exists
   (the `document` skill's map says where).
7. CI's `export` job exports `hello-grid` packed for the host and the web
   and runs it headless one frame (the web through the existing Playwright
   gate from 2914's PR, if still present, else `check.sh`).

## Acceptance criteria

- [x] A release desktop or web export of `hello-grid` contains a pack and
      no RON text of `files/` in the program (`strings` finds none of a
      known key from `files/keymap.ron`).
- [x] The exported game plays the same: the capture plan of
      `hello-grid` (2018) passes on the packed build.
- [x] `Bake::check` runs in every release export and a changed source file
      after baking fails the export with its path.
- [x] A canary planted inside the pack fails 3081's gate with the pack's
      path and the entry's path.
- [x] `docs/formats.md` has the embedded-against-packed measurements.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `pack_beside_program` finds the pack next to the program and
  reports a missing or foreign pack with ADR-0023's "bake it again".
- Property: for the fixture games, `decode(source) == decode(pack)` for
  every baked kind (already `Bake::check`; run it in the exporter's test).
- Integration: the packed export of `hello-grid` for the host and the web
  in CI's `export` job; the disclosure gate over a pack with a canary.

## Completion notes

- **The pack beside the program.** A game's package with a `packed`
  feature reads its files from `<package>.pack` beside the program in place
  of embedding them; the template's game crate and Hello, Grid have one.
  `shuyi_app::pack_beside_program(name, stamp)` reads it beside the program
  (and in a `.app`'s `Contents/Resources`); on the web the page's loader
  fetches it before the game starts (`crates/cli/web/pack-file.js`,
  `index.html`'s `%PACK%`) and hands its bytes over, so the call is the same
  everywhere. A missing or foreign pack is a `PackFileError` that says to
  bake it again, shown in place of the game
  (`BasicGame::files_unreadable`, Hello, Grid's `pack_problem`).
- **The export.** A `--release` export of `windows`, `linux`, `macos` or
  `web` of a package with `packed` turns it on (`ExportCx::features_on`),
  bakes the pack for the target's profile in `prepare`
  (`ExportCx::bake_pack`, an edition's content root through ADR-0101's
  path), proves the file with `bake --check` (new: `BakeInput::check`), puts
  it in the package, and searches it for the build machine's paths
  (`paths::check_pack`: its bytes, every index path, every file it unpacks
  to). `export-report.json` names it (`ExportOutput::pack`). A package
  without the feature exports as before. Contract 48.
- **A game on the short path** bakes its English, palette and font atlas
  (`BasicData::bake`, `BasicGame::bake`, `BasicTools::bake`), stamped
  `AppInfo::pack_stamp` (its id and version, which `BasicGame::made`
  already used); `BasicData::formats` reads both forms. The grid stage's
  tools pass `bake` through, so `shuyi bake` works on a new game.
- **The disclosure gate** lists the pack as one `pack` file with its
  entries, as it did the Playdate's, and now also scans each entry's path
  for the canaries: a canary in an entry's path fails with the pack's path
  and the entry's.
- **Checked by hand on this machine**, on this branch over `b6dc5c22`.
  The merges of `main` after them brought ticket 2066's xtask and docs
  changes (`1eacb69e`), then 2514's `frame-png` frames and 2057's
  Dependabot list (`d1ded901`), none in what they ran; the contract
  became 48 after 2514's 47:
  - `cargo shuyi export windows --release` of Hello, Grid: the zip holds
    `hello-grid.pack`, the exposure gate passes, and
    `tools/export-check/check.sh windows` passes, its new check included
    (no `layout.arrows` of `files/keymap.ron` in the `.exe`).
  - Hello, Grid built with `capture,packed`, its pack baked beside it: the
    capture plan passes 3 of 3; without the pack it reports "the game's
    files, hello-grid.pack, aren't beside the program".
  - `cargo shuyi export web --with packed` (debug: no `wasm-opt` here)
    served and opened in the desktop app's browser: the page fetched
    `hello-grid.pack` and the title screen drew from it; the `.wasm` holds
    no `layout.arrows`.
  - A game from `shuyi new --template grid` on this checkout: its tests,
    fmt, clippy (with and without `packed`), `lint`, `bake --target
    desktop`, and `export windows --release`, which ships `demo.pack`; its
    packed program loads from the pack, and without it shows the message.
- **Measurements** in `docs/formats.md` (*The pack a release ships*):
  the program shrinks by about the pack's size, the zip grows by 9,520
  bytes (deflate shrinks text more than LZ4 and postcard), and loading
  is 2.3× faster natively in a release build.
- **CI.** The `export` job's lists name the pack, and `check.sh` refuses a
  program holding `files/keymap.ron`'s text or a report naming no pack.
  The `render` job runs Hello, Grid's plan on its packed build, its raster
  pictures the same bytes as the embedded build's. The `web-render` job
  boots the packed release web export in headless Chromium. The render
  card's and sound probe's hand-filled pages blank `%PACK%`.

Deviations:

- **The ticket, fixed before the code:** step 4 named `export.rs:370-376`,
  which had moved (`ExportCx::search`); the scope's "ADR-0023 gains a
  paragraph" became ADR-0109, which extends it, since an accepted ADR's
  decision isn't edited (the `write-adr` skill); ADR-0023's status line
  names it.
- **The pack's name** is `<package>.pack` (the ticket said `<name>.pack`):
  the game's library knows its package as `CARGO_PKG_NAME`, and the
  exporter knows it, while a program's name can differ by edition.
- **The web's pack is fetched by the page**, not by the Rust program
  through macroquad's asynchronous `load_file`: one synchronous call on
  every target, before `BasicGame` is built (ADR-0109's alternatives).
- **A short-path game's keymap is not baked.** `UiAction`, the template's
  action set, has no serde form, and baking `KeymapDef<A>` in `BasicData`
  would bound every game's `A`. It ships as its own bytes in the pack, out
  of the program. No ticket: Hello, Grid shows a game whose actions have
  serde bakes its keymap through its own `GameTools::bake`.
- **"A changed source file after baking fails the export"** holds for the
  file that ships: `bake --check` reads the pack back and proves it against
  the sources the bake read, and an edition's check runs its tool program
  again, which a changed file rebuilds. Without a content root the export's
  own tool program embedded its files when it was built, so an edit made
  during the build isn't seen; the unit test changes the sources the tools
  give between the bake and the check. No ticket: the shipped program
  embeds nothing, so what it reads is what was proven.
- **The web's load time** was not measured again: no `wasm-opt` here for a
  release web export, and the pack's read is the code 2204 measured 4×
  faster on the web. The web's program size was measured, before
  `wasm-opt`. No ticket: CI's `web-render` boots the packed release page.
- **Step 7's "runs it headless one frame"** is the `render` job's capture
  plan on the packed build (Xvfb), and the `web-render` job's boot of the
  packed release page, rather than a step of the `export` job: the export
  job's sniper container has no display.

Follow-up tickets: none. 2215 (parked) now says its unpacked path is the
template's `files()` and Hello, Grid's `layer()`; 3302 says the pack's
entries must stay in path order; 3052 says how `index.html` now starts the
game.

Breaking: shuyi-cli: `web::Options`: a new public field, `pack`; code that builds one with a struct literal adds `pack: None`.

```migration
since: 0.1.0
what: The web build's options gain the pack a packed build ships beside the page.
old: shuyi_cli::web::Options { release, features, require_wasm_opt, no_default_features, content }
new: shuyi_cli::web::Options { release, features, require_wasm_opt, no_default_features, content, pack }
before: |
  let options = Options { release: true, features, require_wasm_opt: true, no_default_features: false, content: None };
after: |
  let options = Options { release: true, features, require_wasm_opt: true, no_default_features: false, content: None, pack: None };
```

Breaking: shuyi-cli: `contract::export::ExportOutput`: a new public field, `pack`; code that builds one with a struct literal adds `pack: None`.

```migration
since: 0.1.0
what: An export's report names the pack it shipped beside the program.
old: shuyi_cli::contract::export::ExportOutput { …, edition }
new: shuyi_cli::contract::export::ExportOutput { …, edition, pack }
before: |
  let report = ExportOutput { edition: None, ..report };
after: |
  let report = ExportOutput { edition: None, pack: None, ..report };
```

Book: `site/content/learn/book/targets-and-export.md` says what a release
build ships and that `cargo run` still embeds the files.

Docs: `docs/formats.md`, `docs/contract.md`, `docs/targets/desktop.md`,
`crates/cli/web/README.md`, the `shuyi-ship` skill and its worked example,
the template's `AGENTS.md` and `README.md`, ADR-0109 and the status lines
of ADR-0023, ADR-0058 and ADR-0101.

Behaviour: shuyi-cli: every web build ships `pack-file.js` beside `index.html`, whose last line now starts the game through `shuyi_fetch_pack("<pack, or empty>", …)`; a list of the web build's files (the game's `scripts/check-release-package.sh`) adds `pack-file.js`.

Behaviour: shuyi-cli: a `--release` export of `windows`, `linux`, `macos` or `web` whose package has a `packed` feature bakes and ships `<package>.pack` beside the program and builds with `packed`; a package without the feature exports as before.

Behaviour: shuyi-ui, shuyi-cli: `BasicData::formats` also registers the baked forms of the English, the palette and the font atlas, and `BasicTools` offers `bake`, which answered `SH3001` before.

Behaviour: shuyi-cli: the contract is version 48 (`bake`'s `check`, `export`'s `pack`), and the disclosure gate also finds a canary in a pack entry's path, naming the pack and the entry.

Searched the game (`gh search code --repo Zafnok/visions-of-shuyi`) for
`BasicData`, `BasicTools`, `packed`, `web::Options`, `ExportOutput`,
`pack_beside_program`, `SH3001`, `index.html` and `page-files.js`: hits
only in its copies of the skill pack, `docs/engine/pins.md` and
`scripts/check-release-package.sh`, whose web file list the first
`Behaviour:` line names. Its game package has no `packed` feature, so its
exports otherwise build as before; adopting the pack is its own change.

Nick input: none.
