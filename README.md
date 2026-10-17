# Shuyi Engine (being carved out)

This folder holds the code of **Shuyi Engine**, the open-source 2D engine
that is being carved out of the game in this repository (ADR-0063 §1, §3).
Code moves here one layer at a time (tickets 1103–1117), with no change in
behavior; new generic work after a layer has moved is written here
directly. Once the carve is done and an example game runs from engine
crates alone (1118), ticket 1131 moves this folder to the engine's own
repository with its history.

The engine knows nothing about the game: no game crate, no game asset
folder, no class, character, story or Combat Art. `cargo xtask
check-engine-boundary` fails CI when something here does (ADR-0063 §4).

## Plans, ADRs and tickets

They live in the engine's repository,
[Zafnok/shuyi-engine](https://github.com/Zafnok/shuyi-engine): its ADRs
("engine ADR-NNNN"), its engine-only tickets, its roadmap and product
decisions. Only the carve's tickets (1102–1118, 1131) stay in this
repository, because they change this repository's code.

## The crates

Each goes in `engine/crates/<name>/` and sits under the game crate of the
same layer (ADR-0063 §2). ADR-0004's rules hold for each: the core is pure
and deterministic, only the app touches the platform, time is an argument,
and no `unsafe` outside the app. An engine crate never depends on a game
crate.

| Crate | Folder | Owns |
| ----- | ------ | ---- |
| `shuyi-core` | `core` | The seeded RNG, the simulation traits, and the tactics kit: grid, paths, turns. No I/O. |
| `shuyi-assets` | `assets` | Bundle layers, images, fonts, palette, audio cues, languages, keymaps, tilesets, importers, error reports. |
| `shuyi-ui` | `ui` | The frame, screens, input actions, widgets, skins, cinematics and the test Harness. |
| `shuyi-app` | `app` | The window, blitting, input devices, the audio backend, storage and the run loop. |
| `shuyi-cli` | `cli` | Tools: `frame-png`, `font-atlas`, `check-keys`, `check-text`, `web`, and the rest. |

Carved so far:

- `shuyi-assets` (1103): the bundle as layers the game gives
  (`bundle`), content errors with file, line and column (`error`), and
  RON loading (`ron`). 1116: the `.map` text format of a grid map
  (`grid_map`), with a hook for a game's tile features. 1036: a layer
  that leaves files out on purpose (`Layer::leaving_out`,
  `Bundle::left_out`, which the audio check honors), a watcher of every
  file read (`watch`) and a thread reading another bundle (`reading`).
- `shuyi-assets` (1104): the image table (`image`), font atlases
  (`font`), named colors and themes (`palette`), one-bit inks (`ink`),
  lights (`lighting`), the audio manifest (`audio`) and the credits
  (`credits`). The game names its files and what it requires.
- `shuyi-assets` (1105): languages and packs (`lang`), the names table
  (`names`), the `.dlg` dialogue format with its checks and `@if`
  (`dialogue`), voice clips by line id (`voice`) and tips (`tip`). The
  game gives its directives, tokens, limits, line variants and triggers.
  `shuyi-core` (1105): character ids (`character`) and the `{token}`
  splitter (`text`).
- `shuyi-core` (1114): the seeded RNG (`rng`), the grid and tile sets
  (`grid`), paths over a cost function (`path`), the `TurnRules` trait
  (`turn`), a game's history over it (`history`) and save versions
  (`save`). 1116: playtest bots over any `TurnRules` (`bots`): the
  `Bot` trait, the play loop and a seeded random bot.
- `shuyi-assets` (1107): keymap files (`keymap`): keys, chords,
  controller buttons, named layouts, repeat and stick settings, generic
  over a game's actions (`ActionSet`, `UiAction`). `keymap::sample` is a
  sample game's actions and keymap file for tests (the `sample-actions`
  feature).
- `shuyi-ui` (1107): input (`input`): the keymap, held keys and repeat,
  controllers, the player's saved bindings, fixed keys and the names of
  keys and buttons in help text. The game gives its actions, its keymap
  file and the words. 1034: button chords (two pad buttons together,
  `ButtonChord`, with the firing rules in `input::state`) and a keymap
  file's `playdate` table.
- `shuyi-ui` (1108): the frame (`frame`): the glyph console
  (`glyph_buffer`, with items, layers and the backdrop), its size
  (`console`, which the game gives, and letterboxing), colors by name
  (`color`: a game's `ColorSet` and the engine's own `EngineColor`s), the
  text form snapshot tests read (`snapshot`), framing a shot (`cinema`),
  fitting a picture to cells (`portrait`) and the Playdate's one-bit
  renderer (`onebit`). `float` is the float maths `core` lacks, for
  `no_std` builds.
- `shuyi-ui` (1109): screens (`screen`: `Screen`, `Transition`,
  `ScreenStack`), the context they share (`ctx`: `Ctx<G>`, with the
  game's part in `ctx.game` and what the engine reads from it,
  `GameData`) and the frame loop (`game`: `Game<G>`, `GameHooks` for the
  screen the first key opens and the debug menu, `RawInputEvent`,
  `FrameOutput`); with them the player's settings (`settings`: the
  engine's and a game's `GameSettings`, one saved record in the game's
  form), profiles (`profiles`), storage (`storage`), audio requests and
  the music (`audio`) and `{name}` placeholders (`text`). A toy game of
  two screens (`toy`, tests only) runs them.
- `shuyi-app` (1110): the platform shell. `run` opens the window and
  runs the loop around `Game::frame`: the keyboard (`keys`) and
  controllers (`pads`, gilrs and the web's `web/gamepad.js`, whose
  functions are `shuyi_pad_*`), blitting the frame (`render`), the
  sounds, music and voices (decoded off the main thread on native) and
  storage (`storage`: a file per key in the OS's data folder, or
  `localStorage` on the web). The game's binary keeps macroquad's entry
  point and gives an `AppConfig`: the window, the storage's name, its
  embedded files, font and images, its sound files and voice folder;
  `Platform::setup` puts the storage, voices and music seed into its
  context.
- `shuyi-ui` (1111): the test Harness (`harness`, the `harness`
  feature): any game driven without a window by key, typing, controller
  and wait scripts in frames of 1/60 s, with the audio each frame asked
  for, a stand-in for the music player and snapshots. The toy game
  (`toy`, the `toy` feature) gained a layout picker the first key opens
  and a gallery of pictures the Debug key opens; the Harness's tests drive
  it (`tests/it`).
- `shuyi-assets` (1040, written here): one-bit pictures (`bitmap`): the
  `.1bit` file a one-bit screen draws in place of a PNG, and the
  converter that makes one (`bitmap::convert`: enlarge, stretch, levels,
  Bayer dither, dark lines; ADR-0085). The game's table says which
  pictures and how.

## Third-party crates the engine may use

An engine crate may depend on another engine crate and on the crates
below, and nothing else. `cargo xtask check-engine-boundary` reads this
block. Adding one is a change to this list in the same PR, after checking
its license against ADR-0013.

```engine-deps
serde
ron
thiserror
hashbrown
include_dir
once_cell
libm
macroquad
quad-snd
gilrs
quad-storage
proptest
insta
tempfile
png
```

## License

While they are in this repository, the files in this folder are under this
repository's `LICENSE`, like the rest of it. When 1131 moves them to the
engine's repository they are published under MIT OR Apache-2.0, the
license Nick chose for the engine.
