# ADR-0036: The short path: ready-made parts beside the long form

- **Status:** Accepted
- **Date:** 2026-10-23
- **Related tickets:** 2511 (this), 2512 (the line counter and its cap),
  2003 (the `shuyi` crate), 2013 (types that can grow), 2316 and 2357
  (debug `UiAction`s), 2501 (templates of parts), 3048 (the game's
  identity)
- **Extends:** ADR-0019 (the format registry), ADR-0024 (the platform
  boundary)

## Context

`shuyi new blank` wrote one title screen in 439 lines of Rust that the
game's author owned (counted as `cargo xtask template-lines` will, 2512:
lines neither blank nor only a comment, in `crates/game/src` and
`crates/game/examples`, test modules left out). Most of it was the same
in every game: the four loads by hand, a `GameData` with seven required
methods, six actions that are `UiAction`'s six moves, the engine's
settings saved as one record, a placeholder voice variant, the window,
the storage name, three silent sound functions, `#[macroquad::main]`,
and a `GameTools` of the same shape in every game. Bevy's
`App::new().add_plugins(DefaultPlugins).run()`, LÖVE's three callbacks
and Defold's bootstrap collection start a game in a few lines. Nick's end
goal (2026-10-19) is an agent making snake "in not too many LOC", and the
MVP's book, tutorial and skill pack must teach a short path.

The engine already had the pieces: the format registry (ADR-0019),
bundles, and `Start`, `Setup` and the Harness (ADR-0024). The game the
engine was carved from writes its own `GameData`, `AppConfig` and
`GameTools`, and must not change at its next pin bump.

## Decision

1. **`()` means none.** As `ActionSet::Group` and `GameSettings::Theme`
   already do: `()` is a `ClipVariant` (suffix `""`, no split: no
   voices) and a `GameSettings` (no settings of the game's own). `()`'s
   saved form is the record the blank template wrote,
   `Settings(text_speed, fullscreen, music_volume, sound_volume)` with
   `serde(default)`, plus the layout picked by name
   (`layout: Option<String>`), so an old save reads unchanged.
2. **Default bodies.** `GameData::theme_def` returns `None` and
   `GameData::voice_variant` the variant's default. The other five
   methods stay required. A game's own impls still override them.
3. **`UiAction` is a ready-made `ActionSet`.** `Group = ()`; `ALL` is the
   six moves (`Up`, `Down`, `Left`, `Right`, `Confirm`, `Cancel`), each
   required and rebindable; the four moves repeat; `ui` is `Some(self)`;
   plain `Escape` is the fixed key for `Cancel`. `Debug` is not in `ALL`,
   and **no `UiAction` added later ever is**: a keymap file must list
   every action of `ALL`, so a new one there would break every short-path
   game's `keymap.ron`. A later `UiAction` (2316's `DebugPause` and
   `DebugStep`, 2357's `DebugBack`) gets its match arms (neither required
   nor rebindable, `ui` is `Some(self)`) and stays out. A game with a
   debug key writes its own action enum.
4. **`shuyi_ui::basic::BasicData<A, T = ()>`** is a `GameData` with
   `Action = A`, the engine's colours, `Voice = ()` and `Settings = ()`.
   It reads fixed names from one bundle through the registry
   (`BasicData::formats`): `en.ron` (`shuyi_assets::lang::English`),
   `keymap.ron`, `palette.ron` (with every `EngineColor`),
   `font/atlas.ron` with `font/atlas.png` beside it (with `BASIC_GLYPHS`,
   printable ASCII and the widgets' box and shape glyphs), and every
   other picture the registry claims. Its sound manifest is empty. The
   game's own part `T` is read by a `fn(&Bundle) -> Result<T, _>` and
   kept in `own`. Every problem of every file comes back at once. It
   keeps the bundle it read, so the tools and the tests paint from it
   without a process-wide bundle. It stays `no_std` (a `Layer::table`
   bundle).
5. **`shuyi_platform::BasicGame<A, T = ()>`** is the one description of a
   short-path game that its binary, its tools and its tests share: its
   name, its storage name, its files (one `Layer`), its first screen
   (`fn(&mut Ctx) -> Box<dyn Screen>`) and, by builders, its console
   (48 × 24 text cells until said), its own part (`with_own`) and every
   screen it can show (its first alone until said). It gives `data`,
   `ctx` (in the keymap file's first layout), `game`, `start` (a text
   card with the name; a file that doesn't load is the load's error) and,
   with `harness`, the Harness. Its fields are private and it is `Copy`
   (by hand: a derive would ask `A` and `T` to be). 2013's rule for types
   that may grow holds for it either way.
6. **The prelude lives in `shuyi-platform`** (`shuyi_platform::prelude`):
   what a short-path game names (the assets' `Bundle`, `Layer`,
   `ContentError`, `ActionSet`, `UiAction`; the UI's `Ctx`, `Screen`,
   `FrameInput`, `Transition`, `ScreenResult`, `GameData`, `BasicData`,
   the console and colour types, the glyph buffer's `Cell`, `Rect` and
   `BoxStyle`, `print_centred`) and the start types of the long form
   (`Game`, `GameHooks`, `Card`, `Start`, `Setup`, `Look`). Nothing else:
   a module's path stays the way to the rest. Not in `shuyi-ui`, which
   has no `BasicGame` and whose private `prelude` 73 files use; not in
   `shuyi-app`, a backend a Playdate build of a game's library can't
   depend on. The crate's private `no_std` prelude is now
   `alloc_prelude`. 2003's `shuyi::prelude` re-exports this one.
7. **`shuyi_app::run_game(game)`** is a plain function a plain `fn main`
   calls. It installs the game's bundle (only the binary does: one test
   process runs many games), makes the window (the console at twice its
   size and a 16-pixel margin, titled with the name) and the `AppConfig`
   (the storage name, files from the installed bundle, the font's
   picture, no sounds, no voices, no system keyboard) with two pure
   functions, and calls `macroquad::Window::from_config(conf, run(config,
   game.start()))`: what `#[macroquad::main]` expands to (macroquad_macro
   0.1.8). `Window` is `#[doc(hidden)]` in macroquad 0.4.16, so it
   changes only with the macro; a backend without macroquad (parked 3034)
   replaces it. `run`, `AppConfig` and `window_conf` don't change.
8. **`shuyi_cli::BasicTools`** is the `GameTools` of a `BasicGame`:
   `frame` (painted from the data's font, pictures and bundle), `lang`,
   `check`, `session` (`Headless` over the Harness, no scene) and
   `controls` (the keymap and every screen). A game that adds a scene, a
   record, bots, importers or a bake writes its own `GameTools` and calls
   `BasicTools`' methods for the rest.
9. **No macros.** No attribute, derive or `macro_rules!` writes a screen,
   an action set, a `GameData` or the entry point: everything is plain
   types and functions a reader can follow. A derive for `ActionSet` is
   a follow-up only if 2512's counts show games paying for their own
   enum.
10. **The long form is untouched.** Everything here is a new item, a
    default body, or an impl of the engine's own traits for `()` and
    `UiAction` (which the orphan rule keeps a game from writing). Hello,
    Grid and the grid template keep their long form; the game the engine
    came from needs no change.
11. **The font stays a data file.** The template keeps Terminus in
    `files/font/` (OFL-1.1, its license beside it). No engine crate links
    a font into a Playdate or GBA build.
12. **The minimal template is the yardstick.** The blank template is the
    short path (71 lines by the rule above). Every new API is judged by
    what it does to those lines, and 2512 caps them in CI.

## Consequences

- A new game starts from about 70 lines it owns instead of 439, and
  grows by adding a part (its own actions, `with_own`, its own
  `GameTools`), not by rewriting its start.
- The book's quick start, the tutorial and the skill pack teach the short
  path; the long form stays for what it has that the short one doesn't
  (sounds, voices, themes, settings of the game's own, hooks).
- `UiAction`'s `ALL` is frozen at the six moves: every new `UiAction`
  carries the rule in its ticket.
- `BasicData`'s file names are a contract with every short-path game:
  renaming one is a break.
- `run_game` leans on a `#[doc(hidden)]` macroquad item; a macroquad
  upgrade checks it (the window tests and the templates job build it).
- `shuyi_platform`'s private module is renamed, which no game sees.

## Alternatives considered

- **A `#[shuyi::main]` attribute or a `game!` macro** — fewer lines
  still, but hides what runs, fails in ways an agent can't read, and is
  the opposite of the plain code ADR-0005 asks for. Kept out (decision 9).
- **The prelude in `shuyi-ui` or `shuyi-app`** — `shuyi-ui` can't name
  `BasicGame`, and its private `prelude` is used by 73 files; `shuyi-app`
  is a backend, which a library built for the Playdate can't depend on.
- **`Debug` in `UiAction`'s `ALL`** — every short-path keymap would need
  a debug key a player can't use, and every later debug action would
  break every game's keymap file at its next pin bump.
- **A third template, `minimal`** — `blank` is already in
  `Template::ALL`, CI's matrix, 2009's dependency count and 2501's
  `base`; a second near-empty template would split them.
- **The font in an engine crate** — saves a folder in the template, but
  links a font into every build, a Playdate's and a GBA's included.
- **Defaults for every `GameData` method** — `lang`, `keymap_def`,
  `audio_manifest`, `palette_def` and `console` have no default a game
  could mean; `BasicData` gives them instead.
