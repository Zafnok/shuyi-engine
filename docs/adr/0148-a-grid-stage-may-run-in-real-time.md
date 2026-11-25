# ADR-0148: A grid stage may run in real time

- **Status:** Accepted
- **Date:** 2026-11-24
- **Related tickets:** 2739 (`examples/snake` and `replay::ticks`), 2137
  (`TickSim` and the driver), 2115 (`Every`), 2501 (templates made of
  parts), 2004 (the top-down grid guide), and the follow-up that adds the
  template's `time` question and its `stage-arcade-grid` part
- **Supersedes:** two sentences of ADR-0013 §1 on the time concern (below)
- **Extends:** ADR-0003 §2 (turn rules and tick rules), ADR-0009 §2 (the
  Time concern)

## Context

ADR-0009 §2 lists Time as a concern of its own: turns, or fixed ticks.
ADR-0013 §1 then decided it is never asked: "The time concern (turns or
ticks) is not asked. It follows from the perspectives: a grid stage takes
turns, every other stage takes ticks. A template can ask it when a game
needs both."

That left no way to make a grid game that steps in real time, though
many small games are one: a snake, a falling-block puzzle, a light-cycle
game, a maze chase on a grid. The end goal (Nick, 2026-10-19,
`docs/decisions.md`) names snake as the first game an agent should make
in under an hour, and one of Defold's own tutorials is a snake that moves
a tile whenever the `dt` it adds up passes a period. On 2026-10-30 Nick
unparked the demo chain that starts with it.

The pieces exist: 2137 gave rules that step in time (`TickSim`,
`TickInput`, `TickRecord`, `replay`) and the driver a stage steps them
with (`shuyi_ui::tick::TickDriver`), 2115 gave timers counted in steps
(`Every`, `Countdown`), and the grid kit draws any map of tiles and units
(`MapScene`, the glyph and sprite skins), whatever moves them.

## Decision

### 1. A grid stage takes turns by default, and ticks when the game asks

This supersedes ADR-0013 §1's sentences that the time concern "is not
asked" and that it "follows from the perspectives: a grid stage takes
turns". Now:

- a grid stage takes turns by default, and ticks when the game asks for
  real time;
- every other stage takes ticks, as before;
- the engine's own template asks it as its own question (§1's last
  sentence, "A template can ask it when a game needs both", stands): a
  question `time` ("On a grid: turns, or real time?") whose answers are
  `turns` (the default) and `ticks`. The follow-up ticket that adds the
  template's part writes it.

The rest of ADR-0013 §1 stands.

### 2. How a grid tick stage is built

- **The rules are a `TickSim`** (ADR-0003 §2) in the grid kit's tile
  coordinates (`shuyi_core::grid`: `Pos`, `Dir`, `Grid`), pure as turn
  rules are: no I/O, no clock, no float, luck from a `SimRng` kept in the
  state, and `hash` the state's `state_hash`.
- **Moves are timed in steps** with `shuyi_core::timer::Every` (or
  `Countdown`), advanced by 1 in each step. A move never reads a frame's
  `dt`, so the game steps the same at 30 and at 60 frames a second, and a
  record replays the same on every machine.
- **A turn of the player comes from the step's `TickInput`**: a direction
  pressed in it, read by the rules. A stage reads its moves from its
  steps, never from `FrameInput::actions`; menus and the pause key may
  read the frame.
- **The stage steps the rules through `TickDriver`**, once for each of
  `input.ticks()`, records a `TickRecord`, and hands each step's events
  on once (sounds come from them).
- **It is drawn as a turn stage is**: a `MapScene` of its tiles and units
  painted by the grid skins, built from the state each frame. Tests read
  the state or the scene.
- **The contract's `replay` runs its record** through
  `shuyi_cli::contract::replay::ticks(file, initial, record, outcome)`,
  beside `turns`: step 0, then one `ReplayStep` per step, each with the
  step's input in words (`pressed Up`), the state's hash, its events, and
  never a refusal. The answer's schema doesn't change.

`examples/snake` is the reference of this path.

## Consequences

- A real-time grid game is a direction the engine serves, with an example
  that passes CI (ADR-0003 §1 claims a style only then).
- The same grid kit and skins draw both kinds of grid stage, so moving a
  game from turns to ticks changes its rules and its stage's update, not
  its look. The guide says what changes.
- A tick game has no bots or `playtest` yet: `legal_commands` and the bot
  trait are turn-game things. A follow-up ticket adds a bot for tick
  games if a game asks.
- A replay of a tick game lists one step per tick, a few hundred a minute
  of play where a turn game has a few dozen moves: its records are bigger
  and its golden tables longer.

## Alternatives considered

- **`TurnRules` with one command per step of `ticks()`** — a record would
  hold a command per step, bots and `legal_commands` would see the
  passing of time as a move, and agents would learn a path that 2137's
  tick rules make legacy. Rejected.
- **Moves timed by the frame's `dt`, added up as Defold's tutorial does**
  — the game would step differently at 30 and 60 frames a second, and a
  record of inputs couldn't replay it. Rejected: rules never read time.
- **A new perspective for a real-time grid** — a perspective says how the
  world is seen, not how time passes in it; the grid kit, its maps and
  its skins are the same. Rejected for a question of time.
