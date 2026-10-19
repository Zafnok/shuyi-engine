# The carve

The engine's code was carved out of Visions of Shuyi one layer at a time,
in the game's repository (its `engine/` folder), with no change in
behavior: the game's tickets 1102–1118 and 1154 (game ADR-0063). Ticket
1131 moved the folder here on 2026-10-19 with its history: `git log` on a
file shows the commits that carved it, and their messages name the game's
pull requests (`Zafnok/visions-of-shuyi#NNN`).

Ticket numbers below are the game's (`github.com/Zafnok/visions-of-shuyi/tree/main/tickets`),
and so are the ADRs named "game ADR-NNNN".

## What moved, and the ticket that moved it

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
  controllers (`pads`, gilrs and the web's `crates/cli/web/gamepad.js`, whose
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
- `shuyi-ui` (1112): widgets (`widgets`: a menu and word wrap), screens
  with the game's rows (`screens`: the Key bindings screen, the layout
  picker, the options screen), the debug menu with the game's tools and
  the glyph sampler (`debug`), the dialogue player with the game's hooks
  and its text box (`dialogue`), the tips seen (`tips`), and the menu
  sounds with the game's cues (`audio::AudioConfig`). Each screen's logic
  says what it shows as a view and a glyph skin paints it with the
  engine's colors. The toy game has a lobby of them and a scene read in
  the text box (game ADR-0087).
- `shuyi-assets` (1040, written here): one-bit pictures (`bitmap`): the
  `.1bit` file a one-bit screen draws in place of a PNG, and the
  converter that makes one (`bitmap::convert`: enlarge, stretch, levels,
  Bayer dither, dark lines; game ADR-0085). The game's table says which
  pictures and how.
- `shuyi-assets` and `shuyi-ui` (1113): cinematics. The file
  (`shuyi_assets::cinematic`): an id, a length from a music cue or in
  seconds, shots at times in order, and lists the shots may share; the
  game's shots plug in with their own checks (`ShotCheck`). The player
  (`shuyi_ui::cinematic`): the timeline (`Timeline`, `Moment`), easing, a
  pan over a backdrop (`Pan`) and `CinematicPlayer<S>`, which draws any
  moment from the time alone and fades it; each shot kind draws itself
  (`ShotPainter`). A toy cinematic of two shot kinds plays in `tests/it`.
- `shuyi-core`, `shuyi-assets` and `shuyi-ui` (1115): the tactics kit's
  map (game ADR-0088). `shuyi_core::TerrainId` (a terrain is an index in
  the game's table); `shuyi_assets::image::{ImageRect, Picture}` and
  `shuyi_assets::tileset` (terrain tiles, looks, corner and tile layers,
  the mix rule). `shuyi_ui::grid`: the scene of a game's units
  (`MapScene<U>`, `MapUnit`, ranges as named `RangeLayer`s, the cursor,
  path and threat), where tiles go (`Grid`, `Projection::TopDown`), the
  corner rule, the path and threat arrows, the `GridSkin` trait with the
  glyph skin and the sprite skin (ground from terrain tiles in the map's
  look and light, or the glyph skin's ground), which paint the game's
  units by its `UnitPainter` and take the game's palette, terrains and
  lights as `Looks`; and the stage's pieces: the tile `Camera`, the
  virtual `Cursor`, `steer` and walk timing, with walk speeds and colours
  as skin settings. The toy's grid game (`toy::grid`: a 10 × 8 `Field`,
  two pieces, terrain tiles) is played in both skins in `tests/it/grid.rs`.
- `shuyi-ui` (1154): a whole grid map painted once to pan over
  (`grid::MapPicture`: by a `GridSkin` and the game's `UnitPainter`, or
  by any painter through `MapPicture::painted`); a pan's frame and paint
  over such a picture (`cinematic::PanFrame`, `paint_pan`); and the
  scene camera (`debug::SceneCameraScreen`), a debug tool that shows a
  game's picture through a window, panned by the pixel and zoomed in
  whole steps. The toy opens it on its field in `tests/it/scene_camera.rs`.
- `shuyi-cli` (1117): the tools, binary `shuyi` (`cargo shuyi <command>`):
  `frame-png`, `font-atlas`, `check-keys`, `check-text`, `lang-status`,
  `web` and `private-assets`, on the project of a `shuyi.ron` (`project`:
  the paths each check scans, the atlas's required glyphs, the web
  build's package and the private repository). `frame-png` and
  `lang-status` run the game: a game gives its `GameTools` (its Harness
  started with its own options, its languages) in a tool program that
  calls `main_with`, which `shuyi.ron` names and `shuyi` runs with
  `cargo run`. The web shell is here (`cli/web/`, built into `shuyi`;
  its README says how a game's binary gets `--allow-undefined`). The toy
  game is a project too (`cli/toy/shuyi.ron`, the `toy` example).

- `shuyi-cli` (1118): `glyph-tiles`, a tileset drawn from the font's
  glyphs on squares of palette colors (`glyph_tiles`), for a sprite skin
  before there is art; `shuyi.ron`'s `glyph_tiles` lists the tiles. `web`
  finds the binary in the workspace's build folder (a project may be one
  folder of a bigger workspace). `shuyi-app` (1118): a key pressed and
  let go within one frame is a tap, not a key held down (`keys::ordered`).
