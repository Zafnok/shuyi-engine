# Top-down grid, with turns

- **Covers:** `perspective/top-down-grid`, `kind/tactics`, `time/turns`
- **Status:** ready
- **Tickets:** 2117, 2739

A board seen from above, cut into square tiles, where the game waits for
the player: tactics, board games, puzzles, a roguelike's moves. The
player picks a piece and a tile, the rules answer, and nothing moves
between turns. It is the direction the engine was carved for, and the
one it builds first.

## What starts it

The engine's template, with its `grid` short name:

```bash
cargo shuyi new ../my-game --template grid --name "My Game"
```

It writes the game's rules in a crate of their own,
`crates/rules/src/lib.rs`, the stage's map,
`crates/game/files/stage.map`, and the stage's screen,
`crates/game/src/stages/top_down_grid.rs`, with a bot, records and the
tests that replay them. The `shuyi-new-game` skill walks through it, and
the `shuyi-stage-grid` skill grows the stage.

## What it uses

All of it is in the default engine: no module to turn on.

- **The rules**, in the game's rules crate over
  [`shuyi-core`](../../crates/core/): `TurnRules`, the state, the
  commands it takes and the events they make. The same seed and the same
  commands always play the same game, so tests, bots and replays rest on
  the rules alone.
- **The grid kit**, in `shuyi_core`: `Grid`, `Pos`, `Dir` and `TileSet`
  (`grid`); where a piece can go on a budget and the cheapest path there,
  `reachable`, and how far every tile is from a goal, `flow_field`
  (`path`); the tiles each piece can strike, and the danger at a tile
  (`threat`).
- **Timers in turns**, `Countdown` and `Every` (`timer`): a foe that waits
  two turns, a door that opens every third.
- **Bots** that play the rules to the end (`bots`): a random one, a
  rollout bot, a planner of a whole turn, playtests and an autobalance
  search. The `shuyi-balance-bots` skill uses them.
- **The map**, a `.map` text file, read by
  [`shuyi-assets`](../../crates/assets/)
  ([`docs/formats.md`](../formats.md)).
- **The screen**, in [`shuyi-ui`](../../crates/ui/): the grid
  kit's `MapScene`, which the rules' state builds and a skin paints, in
  glyphs or tiles.

The rules in a few lines: where a piece on the top-left tile can go with
eight steps to spend, around a wall, and a foe that waits two turns.

```rust
use shuyi::core::{Countdown, Pos, reachable};

// A 5 × 3 room; the wall stands on (2, 0) and (2, 1).
let wall = |p: Pos| p.x == 2 && p.y < 2;
let reach = reachable(5, 3, Pos::new(0, 0), 8, |p| (!wall(p)).then_some(1));

// Round the wall by the bottom row: eight steps, nine tiles.
assert_eq!(reach.cost(Pos::new(4, 0)), Some(8));
let path = reach.path_to(Pos::new(4, 0)).unwrap();
assert_eq!(path.len(), 9);
assert!(path.iter().all(|&p| !wall(p)));

// The foe waits two turns, then acts.
let mut wait = Countdown::new(2);
assert!(!wait.advance(1));
assert!(wait.advance(1));
```

## What looks related, and isn't for it

- **`TickSim` and fixed ticks** (`shuyi_core::tick`): a world that moves
  on its own, a fixed number of steps a second. A turn game never needs
  it; see *In real time* below.
- **The `shuyi-stage-field` skill**: a real-time stage, which the engine
  doesn't build yet.
- **`shuyi-physics`** (parked, ticket 1128), and the movers
  and collision kit: bodies that slide and bounce in a free world, for
  [top-down free](top-down-free.md) and [side view](side-view.md). A
  piece on a grid steps from tile to tile.
- **Floats and `dt`.** The rules never read the clock or a float: a turn
  is a command, and a timer counts turns.
- **The other grids.** [Isometric](isometric.md) draws a square grid at an
  angle, and [hex grids](hex-grid.md) have six neighbours; neither is
  built, and this kit's paths step in four directions.

## Which example to read first

[Hello, Grid](../../examples/hello-grid/README.md): a title menu, a
scene, and a 12 × 10 stage where you walk your piece to the goal while a
foe steps toward you, with its rules over `TurnRules` and `Sides`, a bot,
a playtest, an autobalance search and the tool contract. Then the book's
project, a box-pushing puzzle built step by step
([`site/tutorial/`](../../site/tutorial/)).

## In real time (planned)

A grid that steps in real time, a snake, waits for ticket 2739, which
builds on the tick simulation (`TickSim`, ticket 2137). Until then a turn
game times its rules in turns, with `shuyi_core::timer`'s `Countdown` and
`Every`, never with `dt`.
