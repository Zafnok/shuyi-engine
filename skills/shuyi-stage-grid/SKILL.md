---
name: shuyi-stage-grid
description: Build a turn-based grid stage on Shuyi Engine, with its rules in a pure crate over TurnRules, its map in the engine's `.map` format, and its screen painted by the grid kit's glyph skin from a MapScene. Use when writing or changing a grid game's rules, commands, terrain, map or stage screen, or the scene data its tools show. Not for a real-time stage (shuyi-stage-field), for bots and balance (shuyi-balance-bots), or for a bug a record reproduces (shuyi-debug-replay).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# A turn-based grid stage

A grid stage is three things kept apart: the **rules** (a pure crate:
the state, the commands it takes, the events they make), the **map** (a
text file the rules read), and the **screen** (it sends the player's
actions to the rules as commands, and paints what they say through a
skin). The same seed and the same commands always play the same game,
so tests, bots and replays all rest on the rules alone.

`shuyi new <dir> --template grid` makes one: `crates/rules/src/lib.rs`
(the rules), `crates/game/files/stage.map` (the map) and
`crates/game/src/stages/top_down_grid.rs` (the screen, with its tools in
`top_down_grid/tools.rs`). Start from it.

## When to use it

- Writing or changing a grid game's rules: a command, a terrain, a way
  to win or lose.
- Changing the map, or reading a map's errors.
- Changing what the stage shows, or the scene data its tools answer
  with.

## When not to

- A stage that moves in real time: `shuyi-stage-field`.
- Bots that play the stage, and balancing it: `shuyi-balance-bots`.
- A record that plays differently than it should: `shuyi-debug-replay`.
- A screen that isn't a map (a menu, a title): `shuyi-screen`.

## Steps

1. **Change the rules first, with a unit test**, in `crates/rules`. The
   state implements `shuyi_core::TurnRules`:
   - `check(&self, &Command)`: whether the command may be sent now, or
     the `Error` that says why not;
   - `apply(&mut self, &Command)`: the command played, its `Event`s
     returned; it refuses what `check` refuses;
   - `legal_commands(&self)`: every command `check` takes now, in a
     fixed order (bots pick from it);
   - `outcome(&self)`: how the game ended, once it has.

   and `shuyi_core::Sides` says whose turn it is (`side_to_move`) and
   which turn (`turn`). Run the rules' tests alone:

   ```bash
   cargo test -p <game>-rules
   ```

2. **Change the map** in `files/stage.map`, and read it back:

   ```bash
   cargo shuyi import crates/game/files/stage.map
   cargo shuyi check
   ```

3. **Change the screen** last: what a command does on screen comes from
   the events and the state, never from the screen's own sums.

4. **Look at it.** `scene` answers with the map as text and the stage's
   data; `frame-png` draws it:

   ```bash
   cargo shuyi scene --script "Enter Right" --json
   cargo shuyi frame-png stage.png --keys "Enter Right"
   ```

5. **Run the gates** (`shuyi-gates`): `lint` checks that the rules stay
   pure.

## The rules

- **The state changes only through commands.** A command is plain data
  (`Step(Dir)`); `apply` is the only `&mut self` method the game calls.
  Anything that happens is an event the screen reads afterwards.
- **No luck but the `SimRng` kept in the state.** Seed it when the state
  is made (`SimRng::new(seed)`) and draw from it with `roll_below(n)` or
  `roll_percent()`. Never the `rand` crate (its minor releases may
  change its values) and never a clock. Rules that take any luck can
  take a `RandomSource`, so a test hands them a `ScriptedRng` with the
  rolls it wants.
- **Count turns, never time.** A rule that waits ("the door opens on
  turn 5") counts turns or commands. The rules never read a frame's
  `dt` or the clock.
- **Ordered maps only**: `BTreeMap` and `BTreeSet`. The state's hash
  (`shuyi_core::state_hash`, which replays compare) must be the same on
  every run.
- **Whole numbers only**: tiles, turns, hundredths. No `f32` or `f64`.
- The rules crate depends on `shuyi-core` and `shuyi-assets` alone, and
  never reads a file: the game crate hands it the map's text.

`crates/rules/clippy.toml`, the `deny(clippy::float_arithmetic)` at the
top of `lib.rs` and `tests/layers.rs` turn these into errors, and
`cargo shuyi lint` reports them (`shuyi-gates`).

## The grid kit

All in `shuyi_core` (`shuyi::core` from the game crate):

| Item | What it is |
| ---- | ---------- |
| `Pos`, `Dir` | a tile (`Pos::new(x, y)`, `step(dir)`, `Pos::manhattan`), and the four steps, `Dir::ALL` in a fixed order |
| `Grid<T>` | a rectangle of tiles: `get`, `get_mut`, `in_bounds`, `positions`, `neighbors4` |
| `TerrainId` | a terrain, a number the game names (`TERRAIN_NAMES` in the template) |
| `TileSet` | a set of tiles of one grid |
| `reachable(w, h, start, budget, cost)` | where a walker can go on a budget, as a `Reach`: `cost`, `contains`, `tiles`, `path_to` |
| `flow_field(w, h, goals, cost)` | how far every tile is from the nearest goal |
| `strike_area(stops, ranges)`, `strike_area_from(w, h, at, ranges)` | the tiles a piece can strike from where it can stop, or from where it stands; `ranges` are inclusive Manhattan `(min, max)` pairs |
| `Threats<P>` | each piece's `Threat` (its id, its area): `reaches(tile)`, `safe(tile)`, `zone()`, and `danger(tile, forecast)` |
| `GoalDistance<K>` | how far a piece walks from a tile to the nearest goal, one `flow_field` per movement kind, a goal's tile free to enter |
| `History<R>` | the first state and every command since: `push`, `state_at(n)`, `replay`, `rewind_to(n)` |

A cost function answers what entering a tile costs, or `None` when it
can't be entered. Both searches break ties the same way every time.

## The map

The `.map` format (`shuyi_assets::grid_map`): a RON header, a `---`
line, then one character per tile, top row first.

```text
(
    name: "The stage",
    legend: { '.': "floor", '#': "wall", '>': "goal" },
    features: { (1, 6): Hero, (6, 3): Wanderer },
)
---
############
#.........>#
#....##....#
############
```

- `legend` maps each character to a terrain's name, which the rules'
  `terrain_named` turns into a `TerrainId`.
- `features` puts the game's own things on tiles (`Hero`, a sign, a
  chest): a serde type of the rules' that implements `MapFeature`.
- A `MapFormat` says which tile looks a map may name and how big it may
  be. `grid_map::parse(file, text, &FORMAT, terrain_named)` reads it,
  every problem with its line and column.

A map can read and still make no stage (no goal, two heroes). The
rules' constructor (`Stage::new`) says so, and the stage's `own`
function hands that to `check` as the file's error.

## The screen

The stage screen holds the rules' state and sends commands:
`UiAction::Right` becomes `Command::Step(Dir::Right)`. It keeps the
commands and events it sent, for its tools. It never moves a piece
itself.

What it shows is plain data, and a skin paints it:

- `MapScene<U>` (`shuyi::ui::grid`): the visible tiles' terrain
  (`set_terrain`) and the units on them (`push_unit`), each a `MapUnit`
  of the game's (`id`, `pos`, `text`). `Camera::centred_on` picks the
  visible part.
- `GlyphSkin` paints a scene, two glyph cells a tile, through `Looks`
  (the palette, the game's `Terrains`, the map's lights). The game's
  `UnitPainter` paints its units between the skin's layers.
- The lines round the map are a view of their own (`StageView`), their
  text asked for by key from `files/en.ron`.

Tests read the scene, the view or the state, never cells and colors,
unless they test a look.

## The stage's scene in the tools

The stage's `Tools` (`top_down_grid/tools.rs`) give the contract its
scene: `Scene::new(screen, "grid map", scene.to_text(&Terrain))`, with
`data` set to the state as JSON (the turn, the pieces, the outcome).
`scene --json` answers with it at `/result/scene`, and `run --json` too.
Add a field there when a test or an agent needs to read it.

## When it fails

Branch on the `code`, and read its page with `cargo shuyi explain
<code>`:

- `SH2008` from `import`: a character the legend lacks, a feature the
  rules don't know; the error has its line and column.
- `SH2001` from `import`: the header's RON doesn't read.
- `check` with an error and no code: the map reads, but the rules make
  no stage of it (its message says why). `import` finds the line of a
  map's own errors.
- `SH2103`, `SH2104` from `lint`: the rules broke their fence
  (`shuyi-gates`).
- `SH3001`: the game's tools don't offer the operation: a game with no
  stage's `Tools` has no `bots` or `replay`. A `scene` with no scene to
  give isn't an error: its `limits` say why.

## Worked example

[`example.ron`](example.ron) runs on a new `grid` game: `import` reads
the map; a character the legend lacks fails it with `SH2008` at its
line and column; a second goal makes `check` fail; then a new rule, no
step straight back, goes into the rules with its unit test, `scene`
shows the step refused, the bots still play, and `lint`, `fmt`, clippy
and the tests pass.
