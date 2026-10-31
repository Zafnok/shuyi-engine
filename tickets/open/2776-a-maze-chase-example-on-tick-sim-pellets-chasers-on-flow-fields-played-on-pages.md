---
id: "2776"
title: "A maze chase example on TickSim: pellets, chasers on flow fields and power pellets, played on Pages"
type: feature
milestone: "Engine: real time"
model: reasoning
effort: high
status: todo
blocked_by: ["2739"]
nick_input: none
completed:
---

# 2776 — A maze chase example on TickSim

## Context

Nick asked on 2026-10-26 for an A/B against Bevy: how easily, and in how
many lines, an agent makes a Pac-Man-style game in each engine, hosted
on Pages ([`docs/decisions.md`](../../docs/decisions.md), 2026-10-26).
That is 2562. It needs, on Shuyi's side:

- **a reference solution**, this ticket, which proves the task can be
  done on the engine's tick path and gives the eval (2524) something to
  grade, as snake (2739) does for its task;
- **the task as data**, `evals/tasks/maze-chase.ron`, which 2562 runs
  through an agent in each engine: 2572.

On 2026-10-30 Nick unparked the demo chain ("we need better demos",
`docs/decisions.md`): this example is its second game, after snake. The
eval task was split out of this ticket then, so the game plays on Pages
without waiting for the eval (2524) and the tickets before it.

A maze chase is the next arcade game after snake on a real-time grid:
2739's ADR names "a maze chase on a grid" among the games a grid tick
stage serves. It adds what snake lacks: actors that chase the player
(`shuyi_core::path::flow_field`, `crates/core/src/path.rs:118`), modes on
a timer (scatter, chase, frightened), and a player who turns early at a
corner because the turn is held (the input rule of 2137's driver).

**Not Pac-Man** (the 2026-10-26 rule): the example takes no name,
character, maze layout, sound or art from it. Its maze is drawn for this
example, its chasers are named by colour, and it is called "Maze Chase".
Nothing comes from Visions of Shuyi either (`AGENTS.md` rule 4).

## Nick input

None. When it unparks is his call (above).

## Scope

**In:**
- **`examples/maze-chase`**, a game made the way 2739 makes snake: from
  the arcade template part (`cargo shuyi new --template arcade`, 2739),
  then changed, so its lines over the template are counted the same way.
  - **the rules**, a `TickSim` (2137) in tile coordinates
    (`shuyi_core::grid`):
    - the maze is `files/maze.map` (the grid template's map format):
      walls, pellets, four power pellets, a pen in the middle and a
      tunnel through both side walls;
    - the runner moves a tile every `runner_period` steps (`Every`,
      2115) while its direction is open, and keeps the last held
      direction until a wall stops it. A direction held before a corner
      turns there: the turn is kept until it can be taken;
    - eating a pellet scores 10, a power pellet 50 and turns the
      chasers frightened for `fright_steps`;
    - four chasers leave the pen one after another. In chase mode each
      steps along a `flow_field` toward its own target tile: the runner,
      a few tiles ahead of the runner, a tile mirrored from another
      chaser, or the runner when far and its home corner when near.
      Scatter mode sends each to its home corner. The modes follow a
      table of steps in `files/tuning.ron`. Chasers never reverse except
      when the mode changes. Frightened ones pick a free direction with
      `Rng::roll_below` from the run's seed; eaten ones return to the
      pen and score 200, 400, 800, 1600;
    - a chaser on the runner's tile, not frightened, costs a life;
      three lives; every pellet eaten clears the level, and the next
      level is faster by the tuning's table;
    - events: `Ate`, `Powered`, `Caught`, `ChaserEaten`, `Cleared`,
      `GameOver`; `hash` is `state_hash`;
    - the numbers (periods, fright steps, the mode table, scores) are a
      `Tuning` in `files/tuning.ron`, read with `shuyi_assets::ron::Ron`.
  - **the screens:** a title, the stage, a pause menu on `Cancel`, and a
    game-over screen with the score that starts again on `Confirm`,
    built as snake's are, through 2137's driver and a `MapScene`.
  - **the actions:** `Up`, `Down`, `Left`, `Right`, `Confirm`, `Cancel`,
    as snake's. Nothing names a key (the `input-actions` skill).
  - **the tools** (`src/tools.rs`): a `GameTools` as snake's, with a
    session whose scene has the runner, the chasers and their modes, the
    pellets left, the score, the lives and the step, and `replay`
    through `ticks`.
  - the targets snake lists, a frame the Playdate fits, the font copied
    as snake's is (with its `THIRD_PARTY_ASSETS.md` row), and a README
    in hello-grid's form with the lines of its own code.
- **Tests of the example** (`tests/it/`), as snake's:
  - a Harness playthrough of seed 1 through actions only: eat a pellet,
    eat a power pellet, eat a frightened chaser, lose a life, and start
    again from the game-over screen;
  - a turn held before a corner is taken at the corner, in frames of
    1/30 s as in frames of 1/60 s;
  - with no input for one second of play, every actor is on the same
    tile in frames of 1/30 s as in frames of 1/60 s;
  - replay: the playthrough's `TickRecord` through the contract's
    `replay`, with no difference, and a golden table in 2116's form
    checked in CI's `determinism` job;
  - a property test: random inputs for 3 000 steps never panic, no
    actor is ever inside a wall, and the pellets never grow;
  - `frame-png` pictures: the start, and the first frightened chasers;
  - `cargo shuyi check`, `check --target all` and `check-keys` pass.
- **CI and Pages:** after 2739, the `wasm` job and `pages.yml` loop over
  every example with a `web` section, so the example plays beside snake
  on Pages, wherever the site serves its examples on the day, with no
  workflow change. If they still name each example, add it as 2739's
  scope says.

**Out (do not do):**
- The eval task, `evals/tasks/maze-chase.ron`: 2572.
- The Bevy side and the comparison: 2562.
- Bots for the maze chase: `bots` stays unsupported, as snake's.
- Sound: the A/B (2562) measures the game without it. A follow-up
  ticket may add it once the example lands.
- Any engine API the example finds missing: write it as a ticket
  (`write-ticket`), open if this example needs it and parked if not
  (2026-10-30), and note the lines it would
  save in the completion notes.

## Implementation steps

1. Make the game with `cargo shuyi new --template arcade --name
   maze-chase` into `examples/maze-chase`, and note its lines before any
   change.
2. Draw `files/maze.map` and write `files/tuning.ron`.
3. Write the rules (`src/rules.rs`) and their unit tests, the chasers'
   targets one function each.
4. Write the screens, the tools and the README.
5. Write the Harness tests, the golden record and the pictures.
6. Run the `run-gates` skill.

## Acceptance criteria

- [ ] `examples/maze-chase` builds for the desktop and the web and plays
      on Pages.
- [ ] The maze, the names and the art are the example's own (the
      2026-10-26 rule); the PR says so.
- [ ] Every test under *Tests required* passes, the golden record in the
      `determinism` job among them.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The README states the example's lines over the template.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each chaser's target tile, the mode table, a held turn at a
  corner, the tunnel, scores, lives.
- Property: random inputs for 3 000 steps (above).
- Snapshot / integration: the Harness playthrough, the 1/30 s and
  1/60 s runs, the replay and its golden table, the `frame-png`
  pictures, `cargo shuyi check` and `check --target all`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
