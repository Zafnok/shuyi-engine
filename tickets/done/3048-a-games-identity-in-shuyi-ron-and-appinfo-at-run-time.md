---
id: "3048"
title: "A game's identity in shuyi.ron: its name, a reverse-DNS id, version, build, publisher and one icon, checked by check --target, and an AppInfo the game's binary reads"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: done
blocked_by: ["2013", "2511"]
nick_input: none
completed: 2026-10-30
---

# 3048 — A game's identity in shuyi.ron, and AppInfo at run time

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, with GameMaker and Defold. Each keeps a game's name, id,
version, publisher and icon once, as data, and every export reads them:

- Unity's Player settings hold a company name, a product name, a version
  and an application identifier. The company and product names also
  choose where saves go
  (<https://docs.unity3d.com/ScriptReference/Application-persistentDataPath.html>).
- Godot's project settings hold the name, version and icon, and its
  export presets read them.
- Defold's `game.project` holds the title, version, bundle ids and icons
  (<https://defold.com/manuals/project-settings/>); GameMaker has its
  Game Options.
- Bevy has nothing of its own: a game sets its window title in code and
  adds an exe icon with a build-script crate.

What Shuyi has (read on 2026-10-20):

- `Config` in `crates/cli/src/project.rs` (the parts of `shuyi.ron`):
  `command`, `game`, `check_keys`, `check_text`, `font_atlas`, `web`,
  `glyph_tiles`, `private_assets`, `targets` and `target_profiles`. No
  name, id, version, publisher or icon.
- The name is typed twice: `Window::title` in the game's `main.rs` and
  `web.title` in `shuyi.ron` (`examples/hello-grid/src/main.rs`,
  `examples/hello-grid/shuyi.ron`).
- `shuyi_app::Window::icon` (`crates/app/src/lib.rs`) takes three
  hand-made PNG files (`Icon { small, medium, big }`: 16, 32 and 64 px). Both
  templates and hello-grid set it to `None`.
- `AppConfig::storage_name` names the save folder (`storage::data_dir`,
  `crates/app/src/storage/native.rs`) and the prefix of every
  `localStorage` key on the web (`storage::web_prefix`,
  `crates/app/src/storage/mod.rs`). Both templates write
  `const STORAGE_NAME: &str = "{{crate}}";`
  (`templates/blank/crates/game/src/main.rs`,
  `templates/grid/crates/game/src/main.rs`). So two new games named
  "Snake" on one computer share `%APPDATA%/snake`, and two on one web
  origin share every key under `snake/`. Each overwrites the other's
  settings and saves.
- `CARGO_PKG_VERSION` is read only for hello-grid's `PACK_STAMP`
  (`examples/hello-grid/src/lib.rs`).
- `TargetCheck` (`crates/cli/src/contract/target.rs`) has three
  variants (`Fits`, `Text`, `Controls`) and is exhaustive.
- Plans that need these facts and have no source for them:
  - 3004's `export-report.json` has a `version`;
  - 3010 step 3 writes `pdxinfo` "from the game manifest", which
    doesn't exist;
  - 3046's crash reports take the game's version;
  - 2111 (done) stamps the template's records with its crate's name and
    version (`made()` in `base/crates/game/src/lib.rs`);
  - 3015 (parked) needs a bundle id for the stores.

Two MVP tickets change these files first, and this one builds on both:

- **2013** makes `Window`, `Icon` and `AppConfig` `#[non_exhaustive]`,
  with `Window::new(title, width, height)`, `#[must_use] const fn
  Window::with_icon(self, icon: Option<Icon>)`, `Icon::new(small,
  medium, big)` and `AppConfig::new(..)`. Its ADR moves an existing
  type under its rule in the PR that next adds to it.
- **2511** puts the blank template on the short path. Its `lib.rs` has
  `pub fn game() -> BasicGame<UiAction>`, built with
  `BasicGame::new("{{name}}", "{{crate}}", files, first)`, and its
  `main.rs` is one `shuyi_app::run_game(game::game())` call.
  `BasicGame` (`crates/platform/src/basic.rs`) holds the name and the
  storage name, and `run_game` titles the window and names the storage
  with them. 2511 leaves the identity in `run_game` to this ticket.

This ticket adds the data, its checks and a constant the game's binary
reads. 3049 writes the same data into each package (the exe's icon and
version, Info.plist and `.icns`, a `.desktop` file, web icons).

Why it matters:

- **Visions of Shuyi:** nothing changes until it adds the part. Then its
  Playdate package (3010) and its desktop and web packages (3004, 3049)
  read one place.
- **Other 2D games:** a new game gets a save folder of its own, and
  every port gets the same name, version and icon.
- **The end goal** (Nick, 2026-10-19, `docs/decisions.md`): an agent
  makes a game "with a web ver, a playdate ver, etc." in under an hour.
  The facts every port needs are written once, in one file an agent
  edits, and checked.

## Nick input

None. The id's rule, the placeholder id and the defaults are technical
choices.

## Scope

**In:**
- **The `app` part of `shuyi.ron`:** `AppPart` in
  `crates/cli/src/project.rs`, as `Config::app: Option<AppPart>`, with
  `deny_unknown_fields`:
  - `name: String`: the name players see;
  - `id: String`: reverse-DNS, as `com.example.moon-garden`;
  - `version: Option<String>`: semver. Without it, the crate's version
    (`env!("CARGO_PKG_VERSION")` in the generated file);
  - `build: u32`, default 1: a number that only grows, for Playdate's
    `buildNumber` (3010) and macOS's `CFBundleVersion` (3049);
  - `publisher: String`, default empty;
  - `icon: Option<String>`: one square PNG, at least 512 px a side;
  - `storage: Option<String>`: the storage name. Without it, the id;
  - `code: Option<String>`: where `app-info` writes the Rust file, as
    `crates/game/src/app_info.rs`;
  - `window_icons: Option<String>`: the folder where `app-info` writes
    the window's 16, 32 and 64 px PNG files.

  `Config` gains a field here, so it moves under 2013's ADR in this PR:
  `#[non_exhaustive]` (a type with a `Default` that is parsed, never
  built by a literal outside its crate).
- **The rules,** in `crates/cli/src/app_info.rs`:
  `problems(part, root, command) -> Vec<AppProblem>`, each with a
  `Level` (`contract/target.rs`), its setting (`app.id`) and a message:
  - `name`: not empty, no control character (error);
  - `id`: two or more parts between dots; each part ASCII letters,
    digits and `-`, starting with a letter (error). These are the
    characters Apple allows in a bundle id, with a letter first as
    Android asks (3015 maps the `-` when it is unparked). Every crate
    name `crate_name` (`contract/new.rs`) makes passes, so
    `com.example.<crate>` always does;
  - an id that starts with `com.example.` (warning): "two games with
    this id share their saves on one computer and on one web origin; set
    your own before you hand out a build, and before the first release,
    since the save folder follows it";
  - `version`, when given: `MAJOR.MINOR.PATCH` in digits, then an
    optional `-pre-release` and `+build` (error). A small parser, no
    crate;
  - `build`: 1 or more (error);
  - `storage`, or the id when it is unset: one folder name. ASCII
    letters, digits, `.`, `-`, `_` and spaces, with no `.` or space
    first or last (error);
  - `icon`: the file exists, decodes as PNG (`codec::decode_png`, or
    `png_pixels` once 2565 is done: the clause below), is
    square, and is at least 512 px a side (error);
  - `icon` and `code` set without `window_icons` (error);
  - a file `app-info` would write differs from the one on disk (error:
    "crates/game/src/app_info.rs is out of date: run
    `cargo shuyi app-info`").
- **`shuyi app-info [--check]`,** a tool command beside `font-atlas`
  (`crates/cli/src/lib.rs`: the `run` match and the usage text):
  - it writes `code`, a Rust file:
    - its first line is ``// Generated by `<command> app-info` from
      shuyi.ron's `app`. Do not edit.``, the form of `font-atlas`'s
      header (`crates/cli/src/font_atlas.rs`), which 2512's line count
      leaves out;
    - then a `//!` line, and one `pub const APP:
      shuyi_platform::AppInfo` with a doc comment and
      `#[rustfmt::skip]`, so `cargo fmt` never rewrites the file and
      `--check` stays its only judge;
    - `APP` is `AppInfo::new(name, id, version)`, then one setter for
      each field whose value isn't `new`'s default, in the order
      storage, build, publisher, window icon;
    - each string is in quotes, with only `"`, `\` and control
      characters escaped. `check_name` (`contract/new.rs`) refuses all
      three in a game's name, so the template's file as `new` fills it
      is the file `app-info` writes, whatever the name;
    - the window icons are `include_bytes!` paths relative to the file,
      with `/`;
  - with `icon`, it writes `<window_icons>/16.png`, `32.png` and
    `64.png`, downscaled from the icon, with `codec::encode_png`;
  - `--check` writes nothing, lists every file that differs, and exits 1
    if one does;
  - without `code` it writes no Rust. A game that keeps its constants by
    hand still gets its part checked, and its packages (3049).
- **Downscaling in integers:**
  `app_info::downscale(w, h, rgba, size) -> Vec<u8>`:
  - each output pixel averages the source pixels of its square, whose
    bounds are `x * w / size` to `(x + 1) * w / size` (the same for `y`);
  - in premultiplied alpha, rounded half up, so a transparent edge
    doesn't darken the colour beside it;
  - the same input gives the same bytes everywhere;
  - it runs only in the tool, never at run time. 3049 uses it for every
    package size.
- **`shuyi_platform::AppInfo`** (`crates/platform/src/app_info.rs`,
  `no_std`, re-exported from the crate's root), a config type under
  2013's ADR:
  - `#[non_exhaustive]`, `Debug, Clone, Copy`, with public fields
    `name`, `id`, `version`, `publisher`, `storage: &'static str`,
    `build: u32` and `window_icon: Option<WindowIcon>`;
  - `pub const fn new(name, id, version) -> AppInfo`: the storage is the
    id, the build 1, the publisher empty, and no window icon;
  - `#[must_use] pub const fn` setters `with_storage`, `with_build`,
    `with_publisher` and `with_window_icon(self, icon:
    Option<WindowIcon>)`;
  - `WindowIcon { small, medium, big: &'static [u8] }` moves here from
    `shuyi_app::Icon`, as 2013 left it: `#[non_exhaustive]`, with `pub
    const fn new(small, medium, big)`. `shuyi-app` keeps `pub use
    shuyi_platform::WindowIcon as Icon;`, so `shuyi_app::Icon`,
    `Icon::new`, `Window::icon` and `Window::with_icon` don't change;
  - its docs say the storage name never changes after a release, as
    `AppConfig::storage_name` says today.
- **`BasicGame` and `run_game`** (2511):
  - `BasicGame::new(app: AppInfo, files, first)` and
    `BasicGame::with_own(app, files, own, first)` take an `AppInfo` in
    place of the name and the storage name. `name()` and
    `storage_name()` read it, and a new `app()` returns it;
  - `run_game` titles the window with `app.name`, gives it
    `app.window_icon` (2013's `with_icon`), and names the storage
    `app.storage`. 2511's two pure functions get a test for each;
  - `start()`'s card stays `Card::text` of the name.
- **`check --target`** (`crates/cli/src/contract/check.rs`,
  `crates/cli/src/contract/target.rs`):
  - a new `TargetCheck::App`. `TargetCheck` gains a variant here, so it
    moves under 2013's ADR in this PR: `#[non_exhaustive]` (a set that
    grows);
  - each target's report holds the part's problems, with `setting`
    `app.<field>`;
  - a project without `app` gets one line in `limits`, not a problem:
    "the identity isn't checked: shuyi.ron has no `app`". A game that
    hasn't added the part sees no new warning;
  - the `limits` line "only the frame's fit, the text's size and the
    controls are checked yet" names the identity too;
  - raise `contract::VERSION` (`crates/cli/src/contract.rs`),
    regenerate the schemas, and add the version's line to
    `crates/cli/schemas/versions.json` (ADR-0022 §5).
- **The templates** (`templates/blank`, `templates/grid`):
  - `shuyi.ron` gets `app: (name: "{{name}}", id:
    "com.example.{{crate}}", code: "crates/game/src/app_info.rs",
    window_icons: "crates/game/icon")`, a comment on each field, and the
    icon commented out (`// icon: "art/icon.png", // square, 512 px or
    more`). `crates/game/icon` is outside `files/`, so the icons aren't
    embedded twice;
  - `crates/game/src/app_info.rs` is the file `app-info` writes for that
    part, with the placeholders in its strings. `lib.rs` declares
    `pub mod app_info;`;
  - the blank template: `game()` in `lib.rs` calls
    `BasicGame::new(app_info::APP, files, first)`. Its `main.rs` (one
    `run_game` call) doesn't change;
  - the grid template's `main.rs`: `WINDOW` is `Window::new(APP.name,
    48 * 8 * 2 + 16, 24 * 16 * 2 + 16).with_icon(APP.window_icon)`,
    `config()` passes `APP.storage` to `AppConfig::new`, the loading
    card is `Card::text(APP.name)`, and `STORAGE_NAME` goes;
  - so a new game's saves are in a folder named by its id, not by its
    crate. A game made from today's templates keeps its own code and its
    saves. If it adds the part, it sets `storage` to its old storage
    name, as the ADR and `docs/contract.md` say;
  - `crates/cli/src/contract/new.rs`'s module docs: the game's Rust
    holds placeholders only in the strings of `app_info.rs`.
- **hello-grid** (`examples/hello-grid`): an `app` part with
  `name: "Hello, Grid"`, `id: "io.github.zafnok.shuyi-engine.hello-grid"`,
  `storage: "shuyi-hello-grid"` (today's, so saves keep loading) and
  `code: "src/app_info.rs"`, and no icon. `src/lib.rs` declares
  `pub mod app_info;`, and `main.rs` reads `APP` as the grid template's
  does. Its `web.title` stays until 3049.
- **An ADR** (the next free number in `docs/adr/`), listed in
  `docs/adr/README.md`:
  - the identity is one part of `shuyi.ron`;
  - the tools read it, and write a Rust constant the binary compiles,
    checked like `font-atlas`'s output. Never a build script that parses
    RON in every game: it would slow every build (2009);
  - the storage name follows the id unless it is set;
  - what is checked, and why the placeholder id is only a warning.
- **Docs:** `docs/contract.md`'s `check --target` row and its *The
  templates* section.
- **Either order:**
  - **Either order with 2565:** if 2565 is done, `codec::decode_png` is
    gone: the icon check decodes with
    `shuyi_assets::bitmap::convert::png_pixels`, which reads every kind
    of PNG (a grey icon too); if not, use `codec::decode_png` and 2565
    moves the call onto `png_pixels`.
  - **From 3004 (done):** `export-report.json`'s `version` is the
    package's version, which `metadata` in `crates/cli/src/export.rs`
    reads with `cargo metadata --no-deps` into `ExportCx::version` (it
    names the package's files too, `ExportCx::base`). This ticket makes
    it `app.version` when the part sets one, else the package's
    version, as before, with a test.
  - **3044** (the window): if it is done, the grid template's and
    hello-grid's `WINDOW` is `Window::console(APP.name, CONSOLE, 2,
    16).with_icon(APP.window_icon)`, and they call
    `window_conf(&WINDOW, APP.storage)`. 2013 made `Window`
    `#[non_exhaustive]`, so neither a struct literal nor `..` compiles
    outside `shuyi-app`. The blank template's window is `run_game`'s.
    If 3044 isn't done, it makes that change (its scope says so).
  - **3046** (crash reports): if it is done,
    `CrashReports::new(env!("CARGO_PKG_VERSION"))` becomes
    `CrashReports::new(APP.version)` in the grid template and
    hello-grid, `run_game` takes the version from the `AppInfo`, and
    3046's `BasicGame::version` builder goes, with its call in the
    blank template's `lib.rs`. If not, 3046 does it (its scope says
    so).
  - **From 2111 (done):** the template's stamp, `made()` in
    `base/crates/game/src/lib.rs`, becomes `format!("{} {}", APP.id,
    APP.version)` (`Made::new` takes `impl Into<String>`), not the
    crate's name and version (`concat!(env!("CARGO_PKG_NAME"), " ",
    env!("CARGO_PKG_VERSION"))`). This ticket changes it.
  - **2501** (templates as data, done): the `app` part and
    `app_info.rs` go into the built-in template's `base` part, with its
    placeholders (`{{crate_name}}`), and a test that a composed game
    passes `cargo shuyi app-info --check`.
  - **2502** (the wizard): if it is done, it asks for the id after the
    name (default `com.example.<crate>`, checked by this ticket's rule),
    with an `--id` flag, and this ticket adds that. If not, 2502 does
    (an edit request there).
  - **3050** (releases from a tag): if it is done, `export --plan`'s
    `version`, which its tag check compares with the tag, is
    `app.version` when the part sets one (a test). Each template's
    `README.md` and `AGENTS.md` "Releasing" sections (the `base`
    part's, `templates/shuyi-default/base`, since 2501) then say to set the version there when
    the part has one, else in `[workspace.package]` in `Cargo.toml`.
    If not, nothing to do (3050 does both, its scope says so).
- **For the game:** Visions of Shuyi has no `app` part, so `check
  --target` adds only a `limits` line. `shuyi_app::Icon` becomes a
  re-export of the same struct. The game doesn't use `BasicGame`. Its
  pin bump needs a change only where its own code builds `Config` by a
  literal or matches `TargetCheck` without a wildcard. Say so in the PR.
  The completion notes carry 2010's `Breaking:` lines:
  - `Breaking: shuyi-cli: Config and TargetCheck are #[non_exhaustive],
    and TargetCheck gains App`;
  - `Breaking: shuyi-platform: BasicGame::new and BasicGame::with_own
    take an AppInfo in place of the name and the storage name` (and
    `BasicGame::version` goes, if 3046 is done).
- **Size:** if the PR passes about 600 changed lines, the icon work
  (the `icon` field and its check, `downscale`, `window_icons`,
  `WindowIcon`'s move into `shuyi-platform`, and the window icon in
  `AppInfo` and `run_game`) goes to a follow-up ticket (the
  `write-ticket` skill, After MVP, blocked by this one), and 3049, which
  uses `downscale`, then waits for that ticket too.

**Out (do not do):**
- writing the identity into packages (the exe's icon and version
  resource, Info.plist and `.icns`, a `.desktop` file and its icons, web
  icons) and the web page's title from `app.name`: 3049;
- Playdate's `pdxinfo`, launcher icon and card: 3010 (an edit request
  there);
- platform facts at run time (the target, quitting, full screen, the
  devices): 3053;
- a `copyright` field and icons drawn per size: added when a target
  needs them;
- mobile manifests (3015, parked), signing (3051, parked), store art;
- showing the version on a screen: a game reads `APP.version` where it
  wants it.

## Implementation steps

1. Read `crates/cli/src/project.rs`, `font_atlas.rs` (its generated
   header), `codec.rs`, `contract/check.rs`, `contract/target.rs` and
   `contract/new.rs`; `crates/app/src/lib.rs` (`Window`, `Icon`,
   `run_game`) and `crates/platform/src/basic.rs`; 2013's ADR; and the
   templates' and hello-grid's `lib.rs`, `main.rs` and `shuyi.ron`.
   Check whether 3044, 3046, 2111, 2501, 2502 and 3050 are done.
2. Add `AppPart` to `Config`, mark `Config` `#[non_exhaustive]`, and
   write the parse tests.
3. Write `app_info.rs`: the rules, `downscale`, the generator and
   `--check`. Wire the command in `lib.rs`.
4. Add `AppInfo` and `WindowIcon` to `shuyi-platform`, and the
   re-export in `shuyi-app`. Run the no-std build of `shuyi-platform`
   (`run-gates` skill).
5. Move `BasicGame` onto `AppInfo`, and `run_game` with it.
6. Add `TargetCheck::App` and `#[non_exhaustive]` to `check --target`,
   raise the version, and regenerate the schemas.
7. Change the templates and hello-grid, and write their files with
   `app-info`. `cargo fmt --all` doesn't reach `templates/`: run the
   template gate as CI's `templates` job does (`new` each template,
   then `cargo fmt --all --check`, clippy, the tests and `cargo shuyi
   check --target all` in it).
8. Write the ADR and the docs, the `Breaking:` lines in the completion
   notes, and run the gates.

## Acceptance criteria

- [x] A `shuyi.ron` with an `app` part parses, and an unknown field in
      it is refused.
- [x] Each rule above has a test that breaks it and gets its error or
      warning, naming `app.<field>` (the icon's rules are 3072's).
- [x] ~~`cargo shuyi app-info` on a fixture with a 512 px icon writes the
      Rust file and three PNG files of 16, 32 and 64 px.~~ (not done: the
      icon is 3072's) Running it twice gives the same bytes. `--check`
      then exits 0, and after a change to the part it exits 1, naming the
      file.
- [x] The generated file's first line holds `Generated by` and `Do not
      edit`, and `cargo fmt --check` leaves it as it is.
- [x] ~~`downscale` of a picture of one colour gives that colour; of a
      2 × 2 checkerboard to 1 × 1, the rounded mean; a fully transparent
      pixel adds nothing to its neighbours' colour.~~ (not done:
      `downscale` is 3072's)
- [x] A game made by `shuyi new` from each template passes
      `app-info --check`, with an ASCII name and with `Lune Café`. Its
      `check --target all` passes, and the only new warning is the
      placeholder id.
- [x] `AppInfo`, ~~`WindowIcon`~~, `Config` and `TargetCheck` are
      `#[non_exhaustive]`; `AppInfo::new` and its setters (three: the
      window icon's is 3072's) are `const fn`s. (not done: `WindowIcon`
      is 3072's)
- [x] `BasicGame::new(APP, ..)` gives `APP.name` and `APP.storage` from
      `name()` and `storage_name()`. ~~`run_game`'s window has
      `APP.window_icon`.~~ (not done: the window's icon is 3072's)
- [x] hello-grid's window title and storage name are what they were
      (`Hello, Grid`, `shuyi-hello-grid`), now read from `APP`.
- [x] `shuyi-platform` still builds for `thumbv7em-none-eabihf` (CI's
      no-std job).
- [x] The schemas and the version line are regenerated, and the schema
      test passes.
- [x] The ADR is written and listed in `docs/adr/README.md`.
- [x] The completion notes carry the `Breaking:` lines above.
- [x] All gates in the `run-gates` skill pass (mutation testing and the
      CI-only jobs aside).

## Tests required

- Unit: `AppPart` parsing; each rule; the id and version parsers on
  good and bad input; `downscale`; the generated text for a part with
  and without an icon, a version and a storage name, and for a name
  with `é` (snapshots of the text); `--check`'s list of files that
  differ; `AppInfo::new` and its setters; `BasicGame` with an
  `AppInfo`; `run_game`'s two pure functions with a window icon and a
  storage name.
- Integration: `new` from both templates, then `app-info --check`
  (`crates/cli/src/contract/new/tests.rs`); `check --target` on a
  project with a bad id and on one with no `app`; the templates' own
  tests (`check --target all` passes) in CI's template job.

## Completion notes

**Done.** `shuyi.ron`'s `app` part (`AppPart`: `name`, `id`, `version`,
`build`, `publisher`, `storage`, `code`) and its rules
(`crates/cli/src/app_info.rs`); `shuyi app-info [--check]`, which writes
`APP` as a Rust constant; `shuyi_platform::AppInfo` (in the short path's
prelude); `BasicGame::new`/`with_own` and `run_game` on it;
`TargetCheck::App` in `check --target` (contract 30, schemas
regenerated); `export`'s version from `app.version`; the template, the
book's quick start and chapter 9, hello-grid and the skill pack moved
onto it. ADR-0059.

**Deviations:**

- **The icon work went to 3072**, by this ticket's *Size* clause: the
  rest alone is about 800 changed lines with the tests and the ADR. Left
  for 3072: `icon`, `window_icons`, their checks, `downscale`,
  `WindowIcon`'s move into `shuyi-platform`, and the window's icon in
  `AppInfo` and `run_game`. 3049 now waits for 3072 too, and so does
  3010 (it bakes `app.icon`). The acceptance criteria on the icon PNGs,
  `downscale` and `WindowIcon` are 3072's.
- **The templates:** 2501 and 2511 made one composed template,
  `templates/shuyi-default`, and both short names run through
  `run_game`. So there is no grid `main.rs`, `WINDOW` or `config()` to
  change: the base part's `lib.rs` passes `app_info::APP` to
  `BasicGame::with_own`, and `made()` stamps `"<id> <version>"`.
- **The generated file names `shuyi::platform::AppInfo`**, not
  `shuyi_platform::AppInfo`: a game made by `shuyi new` depends on the
  `shuyi` crate alone. It also carries a `check-text: not player text`
  marker above its `const`, so a game's `check-text` (the grid stage's
  scans `crates/game/src`) never counts its strings.
- **The version is parsed by the `semver` crate**, already a dependency
  of `shuyi-cli`, not by a hand-written parser: no new crate, and a
  second grammar would only drift (ADR-0059, *Alternatives*).
- `--check` ignores ``, so a checkout with CRLF line endings isn't
  stale.
- The `limits` line for a project with no `app` comes after the
  per-target lines, before the last line, so the earlier lines keep
  their places.
- `_typos.toml` allows `caf`: `lune-caf` is the crate name `shuyi new`
  makes of "Lune Café".
- The MCP session's server here was a build from before the change
  (contract 29), so the session tools couldn't start hello-grid. Nothing
  a headless player sees changed (the window's title and the storage
  name are what they were, which a hello-grid test checks); `check
  --target all` ran on hello-grid with this branch's tools.

**For the game:** Visions of Shuyi has no `app` part, so `check
--target` adds one `limits` line and nothing else. It doesn't use
`BasicGame`. Its pin bump needs a change only where its code builds
`Config` by a literal or matches `TargetCheck` without a wildcard.

**Follow-ups:** 3072 (the icon). Clauses settled in 2333, 2502, 2512,
2565, 3010, 3015, 3044, 3046, 3049, 3050 and 3051.

Breaking: shuyi-cli: Config and TargetCheck are #[non_exhaustive], and TargetCheck gains App: a game builds `Config` with `Config::parse` (or `Config::default()` and assigns fields) and matches `TargetCheck` with a wildcard arm

```migration
since: 0.1.0
what: shuyi-cli's Config and TargetCheck are #[non_exhaustive], and TargetCheck has a new variant, App.
old: "Config { command: Some(..), ..Config::default() }; match check { TargetCheck::Fits => .., TargetCheck::Text => .., TargetCheck::Controls => .., TargetCheck::OneBit => .. }"
new: "Config::parse(text) or Config::default() with fields assigned; a match with a `_ =>` arm"
before: |
  let config = Config { command: Some("cargo shuyi".to_owned()), ..Config::default() };
after: |
  let mut config = Config::default();
  config.command = Some("cargo shuyi".to_owned());
find: ["..Config::default()"]
```

Breaking: shuyi-platform: BasicGame::new and BasicGame::with_own take an AppInfo in place of the name and the storage name: a game passes `AppInfo::new(name, id, version).with_storage(storage)`, keeping its old storage name so its players' saves keep loading

```migration
since: 0.1.0
what: BasicGame::new and BasicGame::with_own take who the game is, an AppInfo, in place of its name and its storage's name.
old: "BasicGame::with_own(name: &'static str, storage: &'static str, files, own, first)"
new: "BasicGame::with_own(app: AppInfo, files, own, first)"
before: |
  const NAME: &str = "Moon Garden";
  const STORAGE_NAME: &str = "moon-garden";
  BasicGame::with_own(NAME, STORAGE_NAME, files, stages::own, first)
after: |
  const APP: AppInfo = AppInfo::new("Moon Garden", "com.example.moon-garden", env!("CARGO_PKG_VERSION"))
      .with_storage("moon-garden");
  BasicGame::with_own(APP, files, stages::own, first)
find: ["STORAGE_NAME, files"]
```
