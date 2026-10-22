---
id: "2511"
title: "The short path: engine defaults for every GameData part, a prelude, run_game and BasicTools, and the blank template made minimal on them"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2511 — The short path: engine defaults, a prelude, run_game, and a minimal blank template

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and the 2D engines GameMaker, Defold and Construct 3. Nick
chose "Split by group"
([`docs/decisions.md`](../../docs/decisions.md), 2026-10-20, *a second
comparison*), and this ticket is in the group that goes in the MVP: the
book's quick start (2802), the tutorial (2803) and the skill pack
(1134) should teach a short path, not today's long one.

**What the benchmarks have:** a new game is a few lines.

- **Bevy:** `App::new().add_plugins(DefaultPlugins).run()` opens a
  window with every default plugin, and since 0.15 required components
  fill in what an entity needs (<https://bevy.org/news/bevy-0-15/>).
- **Defold:** a new project runs from the bootstrap collection that
  `game.project` names (<https://defold.com/manuals/project-settings/>).
- The gap also cites LÖVE, whose games start from three callbacks
  (`love.load`, `love.update`, `love.draw`), and PICO-8, which caps a
  cartridge's code at 8192 tokens
  (<https://www.lexaloffle.com/dl/docs/pico-8_manual.html>).

**What Shuyi has today** (read 2026-10-20). `shuyi new blank` writes one
title screen in 437 lines of Rust that the game's author owns. That
counts lines that are not blank and not only a comment, in
`crates/game/src` and `crates/game/examples`, with the `#[cfg(test)]`
modules left out. There are also 129 lines of tests and 53 lines of data
(`files/*.ron`, the generated font map left out). Most of it is the same
in every game:

- `src/lib.rs` (130 lines):
  - `include_dir!`, `file`, `text` and `gather`;
  - four loads by hand: `parse_ron` for `en.ron`,
    `KeymapDef::from_source`, `PaletteDef::from_source` and
    `FontAtlasDef::from_source`;
  - a `GameData` impl with four associated types and the trait's seven
    required methods (`lang`, `keymap_def`, `audio_manifest`,
    `palette_def`, `console`, `theme_def`, `voice_variant`;
    `crates/ui/src/ctx.rs`). `theme_def` returns `None`, and
    `voice_variant` returns `OnlyClip`, a placeholder `ClipVariant`
    copied into `templates/blank`, `templates/grid` and
    `examples/hello-grid`.
- `src/action.rs` (55): six actions that map one to one onto
  `UiAction`'s six moves (`crates/assets/src/keymap/action_set.rs`). The
  grid template's and hello-grid's are the same six.
- `src/settings.rs` (42): the engine's settings alone, saved as one
  record. The grid template's file is the same, byte for byte.
- `src/main.rs` (57): `Window`, `STORAGE_NAME`, three no-op sound
  functions, `AppConfig`, `look`, `start`, `window_conf` and
  `#[macroquad::main]`.
- `src/tools.rs` (84): a `GameTools` with `frame`, `lang`, `check`,
  `session` and `controls` (`crates/cli/src/game.rs`), the same shape in
  every game.
- `src/title.rs` (64) is the game's own screen. It has its own `line`
  helper, which `shuyi_ui::screens::print_centred` already is.

The engine already has the pieces this needs:

- the format registry: `Formats`, `KeymapRon`, `PaletteRon::requiring`,
  `FontAtlasRon::requiring`, `Ron<T>` and `ImageTable::load_with`
  (ADR-0019). hello-grid already loads through it
  (`examples/hello-grid/src/lib.rs`, `formats` and `load_from`);
- bundles: `Bundle`, `Layer::dir`, `bundle::install`;
- `Start`, `Setup` and the `Harness` in `shuyi-platform`.

**What is planned:** 2003 makes the `shuyi` crate, with re-exports only.
2501 turns the templates into parts. 2401, 2314 and 2330 each take some
code out of games, one feature at a time. Nothing gives `GameData`
defaults, one call to run, or a smaller template.

**Why it matters:**

- **The game first.** Nothing changes for Visions of Shuyi. It keeps
  its long form: its own `GameData`, `AppConfig` and `GameTools`.
  Everything here is new items and default bodies, so its next pin bump
  needs no change.
- **Other 2D games** start from at most 120 lines (about 80) instead of
  437. They grow by adding their own parts, not by rewriting the start.
- **The end goal** (Nick, 2026-10-19): an agent makes snake "in not too
  many LOC", and every new API is judged against it. Snake waits for
  real time, but the MVP's book, tutorial and skills are written now.
  Without this ticket they teach the long form, and their CI locks it in.
  2512 then counts the lines and caps them.

## Nick input

None.

## Scope

**In:**

- **No settings and no voices are `()`.** The engine already uses `()`
  for "none" (`ActionSet::Group`, `GameSettings::Theme`):
  - `impl ClipVariant for ()` in `crates/assets/src/voice.rs`: the
    suffix `""`, and `split` empty;
  - `impl GameSettings for ()` in `crates/ui/src/settings.rs`, with
    `Theme = ()`. The saved form is the templates' record of today
    (`Settings(text_speed, fullscreen, music_volume, sound_volume)`,
    with `serde(default)`), plus the picked layout as the toy saves it
    (`layout: Option<String>`). So a save written by today's blank game
    loads unchanged.
- **Default bodies on `GameData`** (`crates/ui/src/ctx.rs`):
  - `theme_def` returns `None`;
  - `voice_variant` returns `Self::Voice::default()` (`ClipVariant`
    already requires `Default`).

  A game's own impls still override them. The other five methods stay
  required.
- **`UiAction` as an `ActionSet`**
  (`crates/assets/src/keymap/action_set.rs`):
  - `Group = ()`, and `ALL` is the six moves a game plays with: `Up`,
    `Down`, `Left`, `Right`, `Confirm` and `Cancel`, the templates' six
    of today. `Debug` stays out of `ALL`, and so does any `UiAction`
    added later (2316's `DebugPause` and `DebugStep`, 2357's
    `DebugBack`). A keymap file must list every action of `ALL`
    (`KeymapDef::from_source`, "missing action"), so a new action in
    `ALL` would break the `keymap.ron` of every game on the short path
    at its next pin bump. A short-path game has no debug key, as
    today's templates have none; a game that wants one writes its own
    action enum;
  - `name` is the variant's name (`"Up"`, …, `"Debug"`);
  - the four moves repeat;
  - the six are required and rebindable. `Debug` is neither, like
    `keymap::sample`'s `Debug`;
  - `ui` is `Some(self)`;
  - the fixed key is plain `Escape` for `Cancel`, as in every template.

  The keys stay data: a game's `keymap.ron` names them (rule 8). The
  blank template's `keymap.ron` doesn't change.
- **An `English` kind** in `shuyi_assets::lang`: the screen text by key
  (`#[serde(transparent)]` over a `BTreeMap<String, String>`). It is read
  with `Ron::<English>::new()`, as hello-grid's own `English` is.
- **`shuyi_ui::basic::BasicData<A: ActionSet, T = ()>`** (new
  `crates/ui/src/basic.rs`; `T: Debug + 'static`, as `GameData`
  asks). It is a `GameData` with `Action = A`, `Colors = EngineColor`,
  `Voice = ()` and `Settings = ()`:
  - **What it holds:** the text, the keymap, an empty `AudioManifest`,
    the palette and the console. Public fields hold the `font`, the
    `images`, the `bundle` it was read from, and `own: T`, the game's
    own part.
  - **`BasicData::formats() -> Formats`:** `Formats::standard()`, then,
    as hello-grid's `formats` does:
    - `Formats::replace` with `PaletteRon::requiring` the `EngineColor`
      names and with `FontAtlasRon::requiring(BASIC_GLYPHS)`, in place
      of the standard palette and font formats (`register` would refuse
      them: the standard ones have their ids);
    - `Formats::register` of `KeymapRon::<A>::new()` and
      `Ron::<English>::new()`.

    `BASIC_GLYPHS` is printable ASCII and the box and shape glyphs the
    engine's widgets draw: the templates' `REQUIRED_GLYPHS` of today.
  - **`BasicData::load(bundle, console, own)`**, giving
    `Result<Self, Vec<ContentError>>`. `own` is
    `fn(&Bundle) -> Result<T, Vec<ContentError>>`. It reads these names
    through the registry:
    - `en.ron`, `keymap.ron` and `palette.ron`;
    - `font/atlas.ron`, with `font/atlas.png` beside it (consts
      `FONT_RON` and `FONT_PNG`);
    - every other picture the registry claims (`ImageTable::load_with`,
      with the font's picture left out).

    `own(&Bundle)` reads the game's part. Every problem of every file
    comes back at once, each naming its file, as `gather` does today.
  - `shuyi-ui` stays `no_std`: a `Layer::table` bundle works without
    `std`.
- **`shuyi_platform::BasicGame<A, T = ()>`** (new
  `crates/platform/src/basic.rs`). It holds what the engine needs to
  know and can't read from the files, and the binary, the tools and the
  tests share it:
  - **Constructors:**
    - `BasicGame::new(name, storage, files: Layer, first)`, for a game
      with no part of its own (`T = ()`);
    - `BasicGame::with_own(name, storage, files, own, first)`, for a
      game with one (`own` as in `BasicData::load`).

    `name` and `storage` are `&'static str`. `first` is `fn(&mut
    Ctx<BasicData<A, T>>) -> Box<dyn Screen<BasicData<A, T>>>`.
    Non-capturing closures coerce to it.
  - **Builders:**
    - `console(ConsoleSpec)`: the default is `ConsoleSpec::text(48,
      24)`, the templates' console;
    - `screens(fn)`: every screen the game can show, for `check
      --target`. The default is the first screen alone.
  - **Methods:**
    - for the backends, which can't read private fields: `name()`,
      `storage_name()`, `console_spec()` and `bundle()` (a `Bundle` of
      the one layer);
    - `data() -> Result<BasicData<A, T>, Vec<ContentError>>`, from
      `bundle()`;
    - `ctx() -> Result<Ctx<BasicData<A, T>>, String>`: the data,
      `Ctx::new`, and the keymap file's first layout
      (`KeymapDef::first_layout`), as the templates' `use_layout(LAYOUT)`
      does today. The `Err` holds every problem, one per line;
    - `game(ctx)`: `Game::new` with the first screen and
      `GameHooks::default()`;
    - `start()`: a `Start` with `Card::text(name)`, the font and the
      pictures as its look, and a load through `Setup::ctx`. A file
      that doesn't load is the load's `Err`, which the platform shows
      in place of the game;
    - `harness()`, behind the `harness` feature. It panics with
      `ctx()`'s problems, for tests.
  - Private fields, two constructors and builders (either order with
    2013, below). `Layer`, `ConsoleSpec` and fn pointers are `Copy`, so
    `BasicGame` is too: `Clone` and `Copy` by hand, since a derive would
    ask `T: Copy`.
- **`shuyi_platform::prelude`** (public): what a game on the short path
  names, and the start types a game names when it leaves the short path
  (`Game`, `GameHooks`, `Card`, `Start`, `Setup`, `Look`). Nothing
  else: a module path stays the way to the rest.
  - From `shuyi-assets`: `ContentError`, `Bundle`, `Layer`,
    `ActionSet`, `UiAction`.
  - From `shuyi-ui`: `Ctx`, `Game`, `GameData`, `GameHooks`, `Screen`,
    `ScreenResult`, `FrameInput`, `Transition`, `ConsoleSpec`,
    `EngineColor`, `Rgb`, `BoxStyle`, `Cell`, `GlyphBuffer`, `Rect`,
    `print_centred`, `BasicData`.
  - From `shuyi-platform`: `BasicGame`, `Card`, `Start`, `Setup`,
    `Look`.

  The crate's private `no_std` prelude (`mod prelude` in `lib.rs`, used
  by 6 files) is renamed `alloc_prelude`. The public one goes here, not
  in `shuyi-ui`, whose private `prelude` 73 files use. Nor does it go in
  `shuyi-app`: that is a backend, and a Playdate build of a game's
  library can't depend on it. 2003's `shuyi::prelude` re-exports this
  one.
- **`shuyi_app::run_game(game: BasicGame<A, T>)`**
  (`crates/app/src/lib.rs`), a plain function that a plain `fn main`
  calls:
  - **The window:** the console at twice its size, with a 16-pixel
    margin (784 × 784 for 48 × 24, as today), titled with the name.
  - **`AppConfig`:**
    - the storage name;
    - `files`, reading the game's bundle, which `run_game` installs with
      `bundle::install`. Only `run_game` installs it: the tools and the
      tests read the bundle `BasicData` holds, since one test process
      runs many games;
    - `font_png: FONT_PNG`;
    - no sounds: the template's three no-op functions move here;
    - no voices and no system keyboard.
  - **Then:** `macroquad::Window::from_config(window_conf(&window),
    run(config, game.start()))`. That is what `#[macroquad::main]`
    expands to (`macroquad_macro` 0.1.8, in `Cargo.lock`). `Window` is
    `#[doc(hidden)]` in macroquad 0.4.16, so it changes only with the
    macro. The ADR says so; parked 3034 (a backend without macroquad)
    would replace it.
  - Two pure functions make the window and the config, with unit tests.
    `run_game` only joins them.
  - `run`, `AppConfig` and `window_conf` don't change: the long form
    compiles as it is.
- **`shuyi_cli::BasicTools<A, T = ()>`** (`crates/cli/src/game.rs`, or
  a new `basic.rs` beside it), made with `BasicTools::new(game:
  BasicGame<A, T>)`. Its field is private, as 2013 asks of a new type
  that may grow. It is a `GameTools` with:
  - `frame`: no options; the harness, the script, and a `Painter` from
    the data's font, pictures and bundle;
  - `lang`;
  - `check`: the load's errors;
  - `session`: `Headless` over the harness;
  - `controls`: the keymap and `screens`.

  The other methods keep their defaults. A game that adds a scene to
  its session (`Headless::with_scene`), `replay`, `bots`, `importers` or
  `bake` writes its own `GameTools` and calls `BasicTools`' methods for
  the rest.
- **The blank template on the short path** (`templates/blank`):
  - `src/action.rs`, `src/settings.rs` and `src/tools.rs` go.
  - `src/lib.rs` keeps three things:
    - `FILES` (`include_dir!`);
    - `pub type Data = BasicData<UiAction>`;
    - `pub fn game() -> BasicGame<UiAction>`, with the name and the
      storage name (`"{{name}}"`, `"{{crate}}"`) and the title as its
      first screen.
  - `src/main.rs` keeps the `windows_subsystem` line and
    `fn main() { shuyi_app::run_game(game::game()); }`.
  - `src/title.rs` uses `UiAction` for its actions and `print_centred`
    for its lines. It looks the same.
  - `examples/tools.rs` calls `main_with` with
    `BasicTools::new(game::game())`.
  - `files/keymap.ron` stays as it is: it lists the six actions of
    `UiAction`'s `ALL`.
  - `crates/game/Cargo.toml` lists only the crates it uses. `macroquad`,
    `serde` and `ron` go if nothing names them, from the template's
    `[workspace.dependencies]` too.
  - In `shuyi.ron`, `check_keys.key_files` loses
    `crates/game/src/action.rs`.
  - `tests/it/main.rs` keeps the same tests, on `game::game().harness()`
    and `BasicTools`.
  - `AGENTS.md` and `README.md` describe the files as they now are. A
    short section, "When the game outgrows the short path", lists four
    ways on: its own `ActionSet` enum, a part of its own (`with_own`),
    its own `GameTools` (`shuyi new grid` shows one), or the long form
    (`GameData` by hand, `AppConfig` and `run`).
  - It is the minimal template. No third template is added: `blank` is
    already in `Template::ALL` and CI's templates matrix, and 2009's
    dependency count and 2501's `base` plan on it.
- **An ADR** (the next free number, the `write-adr` skill), "The short
  path: ready-made parts beside the long form", with its row in
  `docs/adr/README.md`. It covers:
  - `()` for none;
  - `UiAction` as a ready-made action set, whose `ALL` stays the six
    moves: a `UiAction` added later gets its match arms and stays out of
    `ALL`, so no game's keymap file breaks;
  - `BasicData`'s file names and its own part;
  - `BasicGame` as one description for the binary, the tools and the
    tests;
  - where the prelude lives, and why;
  - `run_game` and `Window::from_config`;
  - no macros, and the long form untouched;
  - the font stays a data file;
  - every new API is judged against the minimal template's lines
    (2512's gate).
- **Docs:**
  - `docs/contract.md`: *What a game gives* names `BasicTools`, and *The
    templates* says `blank` is the short path;
  - `crates/cli/src/contract/new.rs`'s module doc: the placeholders are
    now in `lib.rs`'s strings, not in the window's title;
  - `crates/app/src/lib.rs`'s module doc: the short form first, then the
    long one.
- **Either order:**
  - **2013** (`#[non_exhaustive]`, constructors and builders): if it is
    done, `run_game` builds its `Window` and `AppConfig` with 2013's
    `Window::new` and `AppConfig::new`, and the new types follow its
    ADR. If not, they already have private fields, constructors and
    builders, and 2013's ADR gives `BasicGame` as its example (its
    clause says so).
  - **2010** (the `api` job): if it is done, its summary lists only added
    items and default bodies. If it calls a finding breaking, the
    ticket's `Breaking:` line says why.
  - **2321** (the frame cap): if it is done, the `()` saved form keeps
    `frame_cap` with `serde(default)` as the toy's does, and `run_game`'s
    window sets `vsync` to 2321's default. If not, 2321 does both (an
    edit request there).
  - **2351** (built-in English): if it is done, `BasicData::load` builds
    its `Lang` with `with_engine_english(shuyi_ui::text::engine_english())`,
    and the blank template's `en.ron` drops its `keys.*` lines. If not,
    2351 does that (its clause says so).
  - **2006** (check-keys on the engine's code): its key files are the
    key pipeline, the keymap format (`shuyi-assets::keymap`) among them,
    so `UiAction`'s fixed key in `keymap/action_set.rs` is allowed in
    either order. If 2006 is done and lists the files one by one, check
    that `action_set.rs` is on the list.
  - **2316 and 2357** (new debug `UiAction`s): if either is done, its
    actions get their arms in the impl (neither required nor
    rebindable, `ui` is `Some(self)`) and stay out of `ALL`. If not,
    they do the same when they land, as the ADR says (2357's text says
    so too).
  - **2339** (transition looks): if it is done, the blank template's
    `frame` turns idle motion on and adds a running transition to
    `Shot::shows`; `BasicTools::frame` does the same. If not, 2339 puts
    it in `BasicTools::frame` (its clause says so).
  - **2513** (setup on Linux): if it is done, the blank template keeps
    `scripts/setup-linux.sh`, `.devcontainer/` and their rows in its
    `README.md`. If not, 2513 adds them to it.
  - **2114** (RNG helpers): if it is done, the prelude also re-exports
    its `Bag` and `RandomSource`, and `shuyi-platform` depends on
    `shuyi-core` (already in its tree through `shuyi-ui`). If not, 2114
    adds them.
  - **2510** (compiler messages): if it is done, update its notes that
    the defaults make wrong, as its clause says.
  - **2003** (the `shuyi` crate): if it is done, the blank template
    depends on `shuyi` and imports `shuyi::prelude::*`. If not, 2003
    moves it (an edit request there).
- **For the game:** no API change. This ticket adds new items, default
  bodies, and impls of the engine's own traits for `()` and `UiAction`,
  which the orphan rule keeps a game from writing. Say so in the PR. The
  game's PR that moves the pin needs nothing for it.

**Out (do not do):**

- the line counter and its CI cap (2512);
- moving the grid template onto these parts: 2501 does it when it makes
  the grid stage a part over `base` (an edit request there);
- moving hello-grid. It keeps its long form on purpose, to show it, as
  the game does, and it keeps its own `English` type;
- macros: no attribute, derive or `macro_rules!` writes a `Screen`, an
  `ActionSet`, a `GameData` or the entry point. A derive for `ActionSet`
  is a follow-up if 2512's counts show games paying for their own enum;
- changes to layers or determinism, and to `GameData`'s required
  methods beyond the two default bodies;
- sounds on the short path: the manifest is empty and there are no sound
  files (2225 adds them; an edit request there);
- a font in the engine's crates. The font stays a data file in the
  template (`files/font/`, Terminus under OFL-1.1, with its license
  beside it), so no engine crate links one into a Playdate or GBA build;
- themes, voices, `GameHooks` and settings of the game's own on the
  short path: the long form has them;
- the engine's English for its own text keys (2351);
- `shuyi::prelude` (2003);
- the book's quick start (2802), the tutorial (2803) and the skill pack
  (1134) on the short path: they wait for this ticket (edit requests
  there);
- taking more code out of games one feature at a time: a game's keys
  kept when a profile has no layout (2401), enum option rows (2314) and
  screens in a `.ui.ron` (2330);
- the window's saved size (3044), the crash report hook (3046) and the
  game's identity (3048) in `run_game`: each adds itself when it lands
  (their scopes say so). 3048 changes `BasicGame::new` and `with_own` to
  take its `AppInfo`, a break its PR names in a `Breaking:` line.

## Implementation steps

1. Read these files:
   - `crates/ui/src/ctx.rs` (`GameData`) and `crates/ui/src/settings.rs`;
   - `crates/assets/src/keymap/action_set.rs` and `keymap/sample.rs`;
   - `crates/platform/src/app.rs` (`Start`, `Setup`);
   - `crates/app/src/lib.rs` and `crates/cli/src/game.rs`;
   - `examples/hello-grid/src/lib.rs` (`formats`, `load_from`);
   - `crates/cli/src/frame_png.rs` (`Painter::new`) and
     `crates/cli/src/contract/session.rs` (`Headless`);
   - the whole blank template.

   Make a game from it as CI's templates job does (`cargo run -q -p
   shuyi-cli -- new blank <dir> --name Demo --engine-path <this
   checkout>`), and in `<dir>` keep its title's picture (`cargo shuyi
   frame-png title.png`) to compare at the end.
2. Write `()` as `ClipVariant` and as `GameSettings`, the two default
   bodies, `UiAction` as an `ActionSet`, and `English`, each with its
   tests.
3. Write `BasicData`, its formats and its load, with fixtures in a
   `Layer::table`.
4. Rename `shuyi-platform`'s private prelude to `alloc_prelude` in
   `lib.rs` and its 6 users. Write `BasicGame` and the public prelude.
5. Write `run_game` and its two pure functions, then `BasicTools`.
6. Move the blank template. `cargo fmt --all` doesn't reach `templates/`:
   run `rustfmt --edition 2024` on its `.rs` files (the placeholders are
   only in strings). Make a game from it again in a new folder, with
   `--name Demo` as in step 1, and run the templates job's commands
   there (`cargo fmt --all
   --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, `cargo shuyi check`, `cargo shuyi check
   --target all`, `cargo shuyi check-keys`). Compare its `frame-png`
   with step 1's. Count the template's lines by the rule in *Context*,
   before and after, for the PR description.
7. Write the ADR and the docs, and settle the *Either order* clauses.
   Run the gates.

Size: expect well over 600 changed lines (about 400 of them template
deletions). If the diff runs long, steps 4 to 6 (`BasicGame`, the
public prelude, `run_game`, `BasicTools` and the blank template) go to
a follow-up MVP ticket written in this PR and blocked by it; a ticket
blocked by this one for those parts is then blocked by the follow-up.

## Acceptance criteria

- [ ] The blank template's Rust outside tests is at most 120 lines by
      the rule in *Context* (437 today). The PR description gives both
      counts and says how they were made.
- [ ] Its `src/action.rs`, `src/settings.rs` and `src/tools.rs` are
      gone. `main.rs` is one call, and `lib.rs` names the game's files,
      its data type and its first screen.
- [ ] `UiAction::ALL` is the six moves, and a keymap file that names
      `Debug` is refused as an unknown action.
- [ ] A game from `shuyi new blank` passes CI's templates job (fmt,
      clippy, the tests, `check`, `check --target all`, `check-keys`).
      Its title's `frame-png` is the same bytes as in step 1.
- [ ] A settings record written by today's blank game (its
      `Prefs::to_ron`) reads back as the same settings through `()`, in a
      test with the text inline.
- [ ] The grid template, hello-grid, the toy and every test outside the
      blank template compile with no change: the long form still works.
- [ ] `shuyi-assets`, `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf` (CI's `no-std` job), and
      `cargo xtask check-no-std` passes.
- [ ] The ADR is written and listed, and the docs say what *Scope*
      says.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `shuyi-assets`:
  - `UiAction` as an `ActionSet`: names round trip; `ui` and `for_ui`;
    which actions repeat, are required and are rebindable; the fixed
    key; `ALL` is the six moves and leaves `Debug` out;
  - a keymap file naming the six parses as `KeymapDef<UiAction>`; one
    that also names `Debug` is refused;
  - `()`'s suffix and split;
  - `English` through the registry.
- Unit, `shuyi-ui`:
  - `()` settings: a round trip; today's template record; an empty
    `Settings()`; unreadable text refused;
  - a `GameData` that leaves out `theme_def` and `voice_variant` gets
    `None` and the default;
  - `BasicData::load`: on good files; with each standard file missing
    (each problem named, all at once); with a palette that lacks an
    engine colour; with an own part that fails.
- Unit, `shuyi-platform`: `BasicGame` starts on its first screen, in the
  keymap's first layout; `screens` defaults to the first screen;
  `start()` run by `App` on `HeadlessPlatform` shows the card, then the
  first screen; with a broken file, `ctx()` and `start()`'s load give
  every problem; the harness plays a key.
- Unit, `shuyi-app`: the window for 48 × 24 cells is 784 × 784; the
  config's storage name, font path and silent sounds.
- Unit, `shuyi-cli`: `BasicTools`' `frame` (its `shows` is the first
  screen's name), `lang`, `check` on good and on broken files,
  `session` and `controls`.
- Integration: the blank template's own tests, in CI's templates job.

## Completion notes

*(Filled in by the session that completes the ticket.)*
