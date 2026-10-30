---
id: "2736"
title: "Parked: Example: a match-3 on the turn path, with swaps, chains, refills from a seed and a move limit"
type: feature
milestone: "Engine: examples"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2114"]
nick_input: none
completed:
---

# 2736 — Example: a match-3 on the turn path

> **Parked** (Nick, 2026-10-20, choosing a gallery of small games as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal Engine 5.8 and Construct 3. Each teaches kinds of game with
small working games it keeps up to date:

- Unity's 2D sample projects include a match-3, Gem Hunter Match;
- Godot's demo projects hold small 2D games (`pong`,
  `dodge_the_creeps`, `bullet_shower`):
  https://github.com/godotengine/godot-demo-projects/tree/master/2d;
- Bevy keeps small games among its examples
  (`examples/showcase/breakout.rs`, `mines.rs`);
- Construct 3 opens on an Example Browser of tagged projects.

What Shuyi has today:

- one example, `examples/hello-grid`: a turn game on a grid map, with a
  Harness playthrough, bots, replay determinism and `frame-png`
  snapshots (`tests/it/`). Its code is about 1 900 lines and its tests
  about 1 800 (2512's count);
- the turn path: `TurnRules` (`crates/core/src/turn.rs`); `Sides`,
  `Bot` and `play` (`crates/core/src/bots.rs`); `Grid<T>` with
  `neighbors4` and `positions` (`crates/core/src/grid.rs`); `SimRng`
  with `next_u32`, `roll_percent` and `roll_below`, and `ScriptedRng`
  for tests (`crates/core/src/rng.rs`); `state_hash`
  (`crates/core/src/hash.rs`);
- a game's own data kind: an `AssetKind` read by
  `shuyi_assets::ron::Ron<T>` and `Formats::load`
  (`crates/assets/src/format.rs`), as hello-grid reads its `English`;
- the grid kit draws any board. `MapScene<U>`
  (`crates/ui/src/grid/scene.rs`) holds the tiles, the game's own units
  (`MapUnit`), named ranges (`RangeName`, tinted with the palette's
  colour of that name), the cursor and an animation clock
  (`clock_ms`). A game's `UnitPainter` (`crates/ui/src/grid/skin.rs`)
  paints its units. The glyph skin's tile is two cells of 8 × 16;
- the one-bit renderer (`crates/ui/src/frame/onebit.rs`) draws the
  Playdate's 50 × 15 cells. It draws a colour that is not an ink white
  or black by its brightness, so a tint alone may not show;
- the `wasm` job of `.github/workflows/ci.yml` and
  `.github/workflows/pages.yml` name `examples/hello-grid` by hand, so
  each new example needs steps there;
- the short path a small game is written on (2511, ADR-0036):
  `BasicGame::with_own`, whose `Own<T>` reader is
  `fn(&Bundle) -> Result<T, Vec<ContentError>>`, with `.console` and
  `.screens` (`crates/platform/src/basic.rs`); `BasicData` and
  `BASIC_GLYPHS` (`crates/ui/src/basic.rs`); `BasicTools`
  (`crates/cli/src/basic.rs`); and `shuyi::prelude` with `run_game`.
  `UiAction` is an `ActionSet` of hello-grid's six actions. `run_game`
  plays no sounds; 2225 gives the short path its sounds.

What is planned:

- 2114 (After MVP): named streams (`SimRng::stream`), and `index`,
  `pick` and `shuffle` on `RandomSource`;
- 2803: the tutorial's box-pushing puzzle, the other puzzle on a grid;
- 2735, 2737 and 2738 (parked): the roguelike and the rest of the
  gallery; 2739: snake;
- 2801: the website's Examples section, one card per example; 2524: the
  agent eval, whose tasks a reference example passes.

Why it matters:

- **The game first:** Visions of Shuyi needs none of this, so it waits
  for the launch.
- **Other 2D games:** Nick said on 2026-10-14 that the engine serves new
  developers' games, "not constrained to tactical rpgs", and on
  2026-10-20 he picked a gallery of small games. A match-3 is a turn
  game where one move sets off a chain of events, the shape of most
  puzzle games, which hello-grid doesn't show. It uses the grid kit
  for a board that is not a map, which tests the kit beyond tactics.
- **The end goal** (2026-10-19): an agent asked for a match-3 reads this
  example first, and an eval task grades against it.

Nothing comes from an existing match-3 (no names, gem art, levels or
scoring tables), and nothing from Visions of Shuyi (`AGENTS.md` rule
4).

## Nick input

None.

## Scope

The whole example is about three PRs of about 600 lines each (hello-grid
is far larger). This ticket's PR is part 1: the rules, pure and tested.
The same PR writes parts 2 and 3 as tickets, from the scope below, so
each part is one PR.

**In (part 1, this PR):**
- **`examples/match-three`** (package `match-three`), a library crate
  and workspace member (`members = ["crates/*", "examples/*"]`). It
  depends on the `shuyi` crate alone for the engine, and on `serde`
  and `include_dir`, as `examples/hello-grid/Cargo.toml` names them,
  with `proptest` as a dev-dependency.
- **Levels as data:** `files/levels.ron`, embedded as hello-grid embeds
  its files (`FILES`, `include_dir!`, and `layer()` as
  `Layer::dir("files", &FILES)`).
  - `Levels` (an `AssetKind`, `NAME` `"levels"`) holds a list of
    `Level`, each with `width` and `height` (5 to 10), `kinds` (3 to
    6), `moves`, and `goal` in points, both at least 1.
  - `pub fn levels(bundle: &Bundle) -> Result<Levels, Vec<ContentError>>`:
    `Formats::standard()` with `Ron::<Levels>::new()` registered, then
    `Formats::load::<Levels>(bundle, "levels.ron")`, then the range
    checks. A value out of range is a `ContentError` naming the file and
    the level. This is the shape of 2511's `BasicGame::with_own`
    function, so part 2 passes it as it is.
  - Three levels, each harder than the last.
- **The rules** (`src/rules.rs`, pure: no I/O, no clock, no float; it
  uses `shuyi-core` and `serde` alone). `Board::new(level: &Level,
  seed: u64)` implements `TurnRules` and `Sides` (one side, `Player`;
  `turn` is the moves made). `Board` derives `Serialize` and
  `Deserialize`, for `state_hash` and records:
  - the board is a `Grid<Gem>`, where `Gem(u8)` is a kind below the
    level's `kinds`;
  - **the start:** the gems come from `SimRng::stream(seed, "gems")`
    (2114), kept in the state. Each tile, row by row, takes `pick` among
    the kinds that make no line of three with the two tiles to its left
    or the two above it, so the start has no line. A start with no
    legal swap is drawn again from the same stream;
  - **`Command::Swap(Pos, Dir)`** swaps the gem at `Pos` with its
    neighbour to the `Right` or `Down`. A screen sends a swap to the
    left or up as the same swap from the other gem, so each swap has
    one command. `check` refuses `Left` and `Up` (`Error::Direction`),
    a swap off the board (`Error::OffBoard`), a swap that makes no line
    (`Error::NoMatch`), and any swap once the game is over
    (`Error::Over`);
  - **`legal_commands`:** every swap `check` accepts, in board order,
    `Right` before `Down` on a tile;
  - **a swap resolves at once, in `apply`:**
    1. find every line of three or more of one kind, across and down; a
       gem in two lines counts once;
    2. clear them and score `10 × gems × chain`, where `chain` counts
       1, 2, 3 within one swap;
    3. let the gems above fall into the gaps;
    4. fill the gaps from the top with gems drawn from the stream;
    5. repeat while there are lines;
    6. count the move;
    7. if the game goes on and the settled board has no legal swap,
       draw it again from the stream (`Event::Redrawn`), keeping the
       score.
  - the fill draws through `&mut impl RandomSource`, so tests feed it a
    `ScriptedRng`;
  - **events, in order:** `Swapped { a, b }`; per chain step,
    `Cleared { tiles, chain, points }`, `Fell { moves: Vec<(Pos, Pos)> }`
    and `Filled { gems: Vec<(Pos, Gem)> }`; then `Redrawn` if it
    happened. Part 3 plays them back; here, tests read them;
  - **outcome:** `Won { score, moves_left }` when the score reaches the
    goal, `Lost { score }` when the moves run out first;
  - **`Greedy`**, a `Bot`: the swap whose first step clears the most
    gems, ties to the first in `legal_commands`.
- **The two follow-up tickets** below, written with the `write-ticket`
  skill. Each keeps the milestone this ticket has once unparked and
  `status: todo`. Part 2 is blocked by this ticket; part 3 by part 2.
  Each says what changed since this ticket was written (which
  either-order tickets are done).

**Part 2 (a follow-up ticket): the match-3 played.**
- **The game on 2511's short path:** `pub fn game() -> BasicGame<UiAction,
  Levels>` made with `BasicGame::with_own(.., levels, ..)` and
  `.console(ConsoleSpec::text(50, 15))`; `src/main.rs` calls
  `shuyi_app::run_game(match_three::game())`.
- **The board screen** (`src/board.rs`), the game's only screen, glyph
  skin only. The first level's seed is `ctx.music_seed`, as hello-grid
  seeds its stages (the platform sets it from the clock; the Harness
  keeps `DEFAULT_MUSIC_SEED`, 0):
  - a `MapScene<GemView>` built each frame, as hello-grid's
    `src/screens/stage.rs` builds its scene: one floor terrain, each
    gem a unit (`GemView` implements `MapUnit`) that the game's
    `UnitPainter` paints, the cursor as its `CursorView`, and the picked
    gem in a range named `picked`;
  - after a swap it shows the settled board at once;
  - a side panel: the level, the score against the goal
    (`GlyphBuffer::meter`) and the moves left. When the level ends, the
    panel says won or out of moves, with the score. `Confirm` starts the
    next board: the next level after a win (level 1 after the last), the
    same level after a loss. Every new board takes the next seed;
  - it declares the six actions it reacts to, as hello-grid's screens
    do, so `check --target` checks them.
- **The actions:** `UiAction`'s six. The directions move the cursor.
  `Confirm` picks the gem under it. With a gem picked, a direction
  swaps it with that neighbour; a refused swap plays `MenuSound::Denied`
  (`ctx.audio.menu`) and keeps the gem picked. `Cancel` puts it down.
  `files/keymap.ron` has `targets` tables for `playdate` (the d-pad, A
  and B) and `steam-deck`, as `examples/hello-grid/files/keymap.ron`
  does. Nothing names a key (the `input-actions` skill). **Either order
  with 2405:** if it is done, the board takes the pointer as its grid
  maps do; if not, nothing. No action needs the mouse.
- **The look:** each kind has its own glyph (`♦ ● ▲ ■ ♥ ♣`, all in
  `examples/hello-grid/files-src/ter-u16n.bdf`) and its own palette
  colour (`gem_0` to `gem_5`), so the board reads without colour. The
  picked gem also gets a mark in its tile's second cell, so it reads in
  1-bit. The atlas is made with `cargo shuyi font-atlas
  files-src/ter-u16n.bdf files/font` from a copy of the BDF, with 2511's
  `BASIC_GLYPHS` and the six in `shuyi.ron`'s
  `font_atlas.required_glyphs`, `Terminus-LICENSE.txt` beside it, and a
  row in `THIRD_PARTY_ASSETS.md`.
- **Sounds:** if 2225 is done, `files/audio.ron` with its menu presets.
  If not, the example is silent, as 2511's `run_game` is, its README
  says so, and 2225's PR adds the sounds (its item on the gallery
  examples says so).
- **The targets:** `ConsoleSpec::text(50, 15)` is the Playdate's
  400 × 240, and a 10-wide level (20 columns) fits beside the panel.
  `shuyi.ron` lists `desktop`, `web`, `steam-deck` and `playdate`, each
  `(frame: (400, 240), scale: integer, text_px: 16)`, with a
  `check_keys` section as the templates' have, the `game` tool program
  and a `web` section (`title: "Match Three"`). It has a
  `palette@onebit.ron` (3006) and a `frame-png --target playdate`
  picture. **From 3010 (done):** `shuyi export playdate` is there, and
  builds a game that does what *What a game adds* in
  `docs/targets/playdate.md` says (a `playdate` feature and
  `shuyi_playdate::main!`, as Hello, Grid's `src/playdate.rs`). This
  example does that, `shuyi export playdate` builds it, and its report
  gives the `.pdx` size against the profile's `package_bytes`.
- **The tools** (`src/tools.rs`, and `examples/tools.rs` calling
  `shuyi_cli::main_with`): its own `GameTools`, which calls 2511's
  `BasicTools` for `frame`, `lang`, `check` and `controls`, and adds
  `session` (`Headless::with_scene`: the board as text, and `score`,
  `moves_left` and `outcome` in the scene's data), `replay` (a record
  of the level, the seed and the commands, through
  `shuyi_cli::contract::replay::turns`, as hello-grid's `StageRecord`)
  and `bots` (`Greedy`). If 2527 is done, its soak method too.
- **Tests:** a Harness playthrough from seed 0 through actions only
  (`Harness::act`): each move walks the cursor to the first legal
  swap's gem, picks it and swaps, until the level ends, and the panel
  shows the rules' score; a refused swap asks for the `Denied` cue
  (`Harness::audio_requests`); `cargo shuyi replay` on a record from
  `cargo shuyi bots 1 --json`; `frame-png` pictures of level 1 at the
  start, with a gem picked, and at a level's end
  (`insta::assert_binary_snapshot!`, as hello-grid's
  `tests/it/snapshot.rs`); `cargo shuyi check`, `check --target all`
  and `check-keys`. **From 2116 (done):** a golden table of one
  record, as Hello, Grid's (`tests/golden.rs`, the window behind a
  default `window` feature, and its own `cargo test --target
  wasm32-wasip1 -p <example> --no-default-features --test golden` step
  in the `determinism` job). If 2527 is done, `shuyi soak` over 20
  seeds.
- **CI and Pages,** either order with the other examples (2735, 2737,
  2738, 2739 and 2740): if the `wasm` job of `.github/workflows/ci.yml`
  and `.github/workflows/pages.yml` still name each example by hand
  (today `examples/hello-grid` alone), change both to a loop over every
  `examples/*/shuyi.ron` that has a `web` section:
  `cargo shuyi check --target all` and `cargo shuyi web --release` in
  each folder, an uploaded artifact per example, and in `pages.yml`
  each `dist/web` copied to a folder named after the example, with a
  line on the index page from its `web.title`. The `run-gates` skill's
  web-build step loops the same way. If an earlier example made the
  loop, nothing changes. The example gets its card,
  `site/content/examples/<name>.md` (2801, done); 2019's `web-render` job (done) boots it too: it loops over the examples as `pages.yml` does.
- **A README** in hello-grid's form: how to run it, its files, its
  targets, what each part of the engine does in it, and its own lines
  without tests and data (`cargo xtask template-lines` if 2512 is done;
  counted, and said how, if not). The engine's `README.md` names the
  example beside hello-grid.

**Part 3 (a follow-up ticket): the match-3 finished.**
- a title with a `Menu` (`shuyi_ui::widgets::Menu`): Play, the engine's
  options (`shuyi_ui::screens::options`) and Quit; an end screen with
  the score; and on `Cancel` with nothing picked, a screen that asks
  whether to leave and pops with its answer, as hello-grid's `Leave`;
- the chain played back from the events, one step a beat (`BEAT_MS`,
  about 120 ms, from the scene's `clock_ms`): cleared gems in a range
  named `cleared` for a beat, falling gems one row a beat, then the new
  gems. Input waits while it plays, and `Confirm` skips to the settled
  board. A `frame-png` picture of a chain mid-way, and an animated one
  if 2514 is done;
- a hint: after 5 seconds with no input (`Harness::wait` in its test),
  the first legal swap's two gems in a range named `hint`;
- the eval: if 2524 is done, `evals/tasks/match-three.ron` with
  `reference: Project("examples/match-three")`, targets `desktop` and
  `web`, and checks `Gates`, `Check`, `Targets`, `Web`, `Replay` and a
  `Scene` check of `/score` and `/moves_left`; `cargo xtask
  eval-reference match-three` passes. If not, add that task to 2524's
  first tasks.

**Out (do not do):**
- in this PR, anything of parts 2 and 3 beyond writing their tickets: no
  screen, binary, tools, `shuyi.ron`, CI or README;
- special gems (line clears, bombs), blockers, goals other than points,
  and timed modes: follow-ups if a game asks;
- smooth falls on a sprite skin (2109's tweens), and shake or flash on a
  long chain (2340): follow-ups if a game asks;
- saves and a level select: the levels play in order, and nothing is
  kept;
- search bots (2113): `Greedy` is enough for an example;
- a `kind` answer in `docs/modules.ron` (2003), a guide (2004) or a
  template part (2501): the example proves the default engine, not a
  module (ADR-0009 §7);
- the box-pushing puzzle (2803), the roguelike (2735), the card battle
  (2737);
- anything from an existing match-3 or from Visions of Shuyi;
- new engine features. A gap the example shows becomes a ticket (the
  `write-ticket` skill), named in the Completion notes.

## Implementation steps

1. Read `examples/hello-grid` (`src/lib.rs`, `src/rules.rs` and
   `src/rules/tests.rs`), `crates/core/src/turn.rs`, `bots.rs` and
   `rng.rs` (with 2114's helpers), and `Formats::load` in
   `crates/assets/src/format.rs`.
2. Make `examples/match-three`: `Cargo.toml`, `src/lib.rs` with
   `FILES`, `layer()`, `Levels`, `Level` and `levels`, and
   `files/levels.ron`.
3. Write `src/rules.rs`: `Gem`, `Board::new` and the start, lines,
   clears, falls, fills, the redraw, `check`, `apply`,
   `legal_commands`, `outcome`, `Sides`, then `Greedy`.
4. Write `src/rules/tests.rs` and the property tests.
5. Write the part 2 and part 3 tickets (the `write-ticket` skill),
   checking which either-order tickets are done by then.
6. Run the gates. Say the PR's changed lines in its body.

## Acceptance criteria

- [ ] `levels(&Bundle::new(vec![layer()]))` loads the three levels, and
      a level with a value out of range gives a `ContentError` naming
      the file and the level.
- [ ] On 1 000 seeds of each level, the start has no line of three and
      at least one legal swap.
- [ ] After every swap, the settled board is full, holds no line of
      three, and has a legal swap or the game is over (property test).
- [ ] `Greedy` plays 50 seeds of each level through `shuyi_core::play`
      with no rules error, each game ending within its moves.
- [ ] A game's commands, applied twice to `Board::new` with the same
      level and seed, give the same events and the same `state_hash`.
- [ ] The part 2 and part 3 tickets are in `tickets/open/`, with the
      scope above and their `blocked_by`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`src/rules/tests.rs`): lines across, down, crossing and longer
  than three; a gem in two lines counted once; falls; fills from a
  `ScriptedRng`; a chain of two with its points; a redraw of a
  hand-built board with no swap; each refused swap (`Left`, off the
  board, no line, after the end); the move limit and both outcomes;
  `Greedy`'s pick and its tie; `legal_commands` in its order.
- Unit (`src/lib.rs`'s tests): `levels` on the shipped file, and on a
  bundle whose `levels.ron` has a value out of range.
- Property (proptest): the start over random seeds; the settled board
  after random legal swaps; the score never goes down.
- Integration (`tests/`): `Greedy` over 50 seeds of each level; the
  replay of a game's commands twice.

## Completion notes

*(Filled in by the session that completes the ticket.)*
