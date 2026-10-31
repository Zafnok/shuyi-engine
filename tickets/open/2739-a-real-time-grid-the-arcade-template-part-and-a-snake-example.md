---
id: "2739"
title: "A real-time grid: the arcade template part and a snake example on TickSim, built for the web and the Playdate"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2137", "2115", "2501", "2511"]
nick_input: none
completed:
---

# 2739 — A real-time grid: the arcade template part and a snake example

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Their first-hour paths are small real-time games:

- one of Defold's official tutorials is a snake game
  (defold.com/tutorials/snake/). It moves the snake a tile whenever the
  `dt` it adds up in `update` passes a period;
- Bevy ships breakout among its examples
  (github.com/bevyengine/bevy/blob/main/examples/showcase/breakout.rs);
- Godot's first 2D game is "Dodge the Creeps"
  (docs.godotengine.org/en/stable/getting_started/first_2d_game/index.html);
- Construct 3 starts games from its 8 Direction and Platform behaviors,
  and Unity from its 2D Platformer Microgame.

**The end goal** (Nick, 2026-10-19, `docs/decisions.md`): an agent asked
"make me a snake game with Shuyi Engine" makes it, with a web build, a
Playdate build and the other targets, in under an hour and in few
lines. Every new API is judged against it. On 2026-10-20 Nick put the
real-time work snake needs after the game's launch. On 2026-10-30 he
unparked the demo chain (snake, the maze chase and the A/B against
Bevy): "we need better demos. hello, grid is garbage it's too easy and
simple". This ticket is where snake gets built: the shortest path to
the end goal's first named game, and the first demo of that chain.

What Shuyi has today:

- two templates, `templates/grid` and `templates/blank`. The grid one's
  rules are `TurnRules` (`templates/grid/crates/rules/src/lib.rs`).
  Nothing starts a game that runs in real time;
- one example, `examples/hello-grid`, a turn game;
- `FrameInput::ticks()` (`crates/ui/src/input/frame.rs:114`): the fixed
  steps due this frame, counted by `TickClock`
  (`crates/platform/src/tick.rs`, 60 a second, at most 4 a frame).
  Outside tests nothing reads it;
- the contract's replay helper is for turn games only:
  `shuyi_cli::contract::replay::turns`
  (`crates/cli/src/contract/replay.rs:95`) takes `TurnRules` and
  commands, and `docs/contract.md` (*What a game gives*) tells a game's
  `GameTools::replay` to call it;
- [ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
  §1: "The time concern (turns or ticks) is not asked. It follows from
  the perspectives: a grid stage takes turns, every other stage takes
  ticks. A template can ask it when a game needs both." ADR-0009 §2
  lists Time as a concern of its own (turns; fixed ticks), but nothing
  serves a grid that steps in real time. Snake is one; so are a
  falling-block puzzle, a light-cycle game and a maze chase on a grid.

What is planned:

- 2137 (1119's tick part, split out on 2026-10-30): `TickSim`,
  `TickInput`, `TickRecord`, `replay` and the driver. Snake needs only
  that, not 1119's `Fx` or arena (still parked);
- 2115 (After MVP): `Countdown` and `Every` in `shuyi_core::timer`. It
  names this ticket: the snake moves every few steps with `Every`, never
  with `dt`, so a replay and the Playdate's 30 frames a second agree with
  the web's 60;
- 2501 (After MVP): templates as parts in `templates/shuyi-default`.
  2511 (MVP): engine defaults, a prelude and `run_game`, the short path
  a small game is written on;
- 3004 and 3010 (After MVP): `shuyi export` for the desktop and the web,
  and the Playdate backend with `shuyi export playdate`;
- 2524 (After MVP): the agent eval. It writes `evals/tasks/snake.ron`
  with `waits_for: ["2739"]` and no reference; this ticket gives it one;
- 1127 (parked): the platformer, the end goal's other named game.

Why it matters:

- **The game first:** Visions of Shuyi needs none of this, which is why
  it waited until the game's needs from the engine were met
  (2026-10-30).
- **Other 2D games:** a grid that steps in real time is a direction
  ADR-0009 §2 allows and nothing serves. ADR-0003 §1 claims a style only
  when its example passes CI.
- **The end goal:** the arcade part is what an agent starts a real-time
  grid game from, `examples/snake` is the proof and the eval's reference
  solution, and the skill teaches the tick path. No agent then fakes
  real time with `dt` in rules, or with a turn per `ticks()`.

Nothing comes from an existing snake game's code or art, and nothing
from Visions of Shuyi (`AGENTS.md` rule 4).

## Nick input

None.

## Scope

**In:**
- **An ADR** (the `write-adr` skill, the next free number): "A grid
  stage may run in real time".
  - It supersedes two sentences of ADR-0013 §1: that the time concern
    "is not asked", and that it "follows from the perspectives: a grid
    stage takes turns". Now a grid stage takes turns by default and
    ticks when the game asks for real time; every other stage takes
    ticks. §1's last sentence, "A template can ask it when a game needs
    both", is how the built-in template asks it (below). The rest of §1
    stands.
  - ADR-0013's `Status:` line gains "its §1 sentences on the time
    concern are superseded by ADR-NNNN". In `docs/adr/README.md`,
    ADR-0013's row says the same and the new ADR gets a row.
  - It says how a grid tick stage is built: rules as a `TickSim`
    (ADR-0003 §2) in the grid kit's tile coordinates
    (`shuyi_core::grid`), moves timed in steps with `Every` (2115), and
    drawn through `MapScene` (`crates/ui/src/grid/scene.rs`) and the grid
    skins, as a turn stage is.
  - Alternatives considered include `TurnRules` with one command per
    step of `ticks()`: a record would hold a command per step, bots and
    `legal_commands` would see time as a move, and agents would learn a
    path that 2137 makes legacy.
- **A tick replay in the contract:**
  `shuyi_cli::contract::replay::ticks(file, initial, record, outcome) ->
  Result<ReplayRun, String>` beside `turns`, over 2137's `TickSim` and
  `TickRecord`:
  - `initial` is the state the caller built from the record's seed;
    `outcome` is a function `Fn(&S) -> Option<String>`, because 2137's
    `TickSim` has no `outcome()` (the snake passes its own);
  - step 0, then one `ReplayStep` per step;
  - `command` is the step's input in words (the actions pressed and
    released, such as "pressed Up"), none for a step with no input.
    `hash` is `TickSim::hash` in sixteen hex digits, as `StateHash`
    prints; `events` as `turns` gives them; `refused` is always none;
  - the answer's schema doesn't change. If 2137 or another ticket has
    already added a tick replay, use it;
  - `docs/contract.md`'s `replay` row (*What a game gives*) names
    `replay::ticks` for a tick game beside `replay::turns`.
- **`examples/snake`**, a workspace member written on 2511's short path
  (`shuyi::prelude`, with its `run_game`), on the `shuyi` crate alone
  for the engine, as hello-grid is, drawn on the glyph console:
  - **the rules** (`src/rules.rs`, pure: no I/O, no clock, no float):
    `Snake`, a `TickSim`:
    - the board is a `.map` (`files/board.map`, read with
      `shuyi_assets::grid_map` as the grid template reads `stage.map`):
      floor, walls and a `Start` feature. It is at most 25 × 14 tiles,
      because a glyph tile is two cells across
      (`shuyi_ui::grid::glyph::TILE_W_CELLS`) and the frame below leaves
      one row for the score;
    - the snake starts 3 tiles long, heading right, and moves one tile
      each time its `Every` fires: every 8 steps at first, 7.5 tiles a
      second;
    - a turn comes from a direction action pressed in the step's
      `TickInput`. Up to two turns wait for the next moves, so two quick
      presses both count. A turn back into the neck is ignored;
    - food goes on a free tile drawn from the `SimRng` kept in the
      state: `roll_below` over the free tiles, or `pick` if 2114 is
      done. Eating grows the snake by one at its next move and scores a
      point. Every 5 points the period drops by one step, down to 4 (a
      new `Every` with the shorter period);
    - hitting a wall or its own body ends the run
      (`Outcome::Died { step, length }`); filling the board ends it as
      `Outcome::Filled`;
    - events: `Turned`, `Moved`, `Ate`, `Died`, `Filled`;
    - the numbers (start length, first and fastest period, points per
      speed-up) are a `Tuning` in `files/tuning.ron`, read with
      `shuyi_assets::ron::Ron<T>`;
    - `hash` is the state's `state_hash` (`shuyi_core::state_hash`).
  - **the screens:** a title, the stage, a pause menu on `Cancel`, and a
    game-over screen with the score that starts again on `Confirm`. The
    stage steps the rules once for each of `input.ticks()` through
    2137's driver, and builds a `MapScene` whose units are the head, the
    body and the food. A suspended game gets 0 ticks, so the snake
    waits. If 3042 is done, the example's `GameSettings::BACKGROUND` is
    `Background::KEEP_RUNNING.paused()`, and its system hook pushes the
    pause menu on `FocusLost`; if not, 3042 does it.
  - **the actions:** `Up`, `Down`, `Left`, `Right`, `Confirm` and
    `Cancel`, the grid template's six
    (`templates/grid/crates/game/src/action.rs`). `files/keymap.ron` has
    a table for the Playdate (the d-pad, A and B) and one for the Steam
    Deck, as hello-grid's does. `shuyi.ron` has a `check_keys` section
    as the grid template's has. Nothing names a key (the `input-actions`
    skill).
  - **a frame the Playdate fits:** `ConsoleSpec::text(50, 15)`, 400 ×
    240 at scale 1, holding the board and a score line. `shuyi.ron`
    lists `desktop`, `web`, `steam-deck` and `playdate`. The palette's
    roles stay apart in 1-bit: `files/palette@onebit.ron` gives each
    role its patterns (ADR-0026), and the tools' `one_bit` returns the
    palette and that map, as hello-grid's `src/tools.rs` does. Without
    it, `check --target playdate` says the patterns weren't checked.
  - **the tools** (`src/tools.rs`): a `GameTools` of its own that calls
    2511's `BasicTools` for what it doesn't change, and adds `check`,
    `session` (a scene with the body, the food, the score, the step and
    the outcome), `replay` through `ticks` above, and `controls`.
    `bots` stays unsupported (Out).
  - **sound:** if 2225 is done, an eat and a death sound from its
    presets. If not, none, the README says so, and 2225's PR adds them.
  - the font atlas copied from `examples/hello-grid/files/font`
    (Terminus, OFL-1.1, with its license file), and the copy added to
    `THIRD_PARTY_ASSETS.md` (to the row that lists the templates'
    copies, or a row of its own).
    **From 3039 (done):** `shuyi.ron` has the `notices` part naming
    `files/font/Terminus-LICENSE.txt`, as hello-grid's has, and Pages
    links the example's `NOTICES.txt`.
  - **a README** in hello-grid's form: how to run it, its targets, and
    the lines of its own code without tests and data (`cargo xtask
    template-lines` if 2512 is done; counted and said how if not).
    **Either order with 2514:** if 2514 is done, an animated picture of
    its first seconds made with `frame-png`; if not, 2514 adds it (its
    clause says so). **Either order with 2811:** if 2811 is done, that
    picture is the example's preview: `preview.png` in its folder, made
    by a `preview` part in its `shuyi.ron` (`cargo shuyi frame-png
    --preview`), checked by a test in `tests/it/` as hello-grid's is,
    and shown on its card on the Examples page. If not, nothing to do
    (2811 makes the picture its preview).
- **Tests of the example** (`tests/it/`):
  - a Harness playthrough of seed 1 through actions only
    (`Harness::act` and `Harness::wait`): from the title, steer to the
    first food, eat it, see the snake grow, hit a wall, and start again
    from the game-over screen;
  - with no input for one second of play, the snake moves the same
    number of tiles in frames of 1/30 s (`Harness::frame`) as in frames
    of 1/60 s;
  - replay: the playthrough's `TickRecord`, written to a scratch folder,
    goes through the contract's `replay` with the example's tools, as
    hello-grid's `tests/it/contract.rs` does. It lists step 0 and one
    step per tick, and the two runs have no difference. 2116 is done
    before this ticket, so the record is also
    checked in with a golden table of the example's own, in the form
    2116 gives Hello, Grid's: the record in
    `examples/snake/tests/records/seed-1.ron`, the table
    `tests/golden/snake.ron` (`step NNN` rows and `outcome`) checked by
    `tests/golden.rs`, the window behind a default `window` feature, and
    a `cargo test --target wasm32-wasip1 -p snake --no-default-features
    --test golden` step in CI's `determinism` job, as 2116's line in the
    `run-gates` skill says a new example does;
  - a property test (`proptest`, a workspace dependency): random inputs
    for 3 000 steps never panic, the body never overlaps itself while
    alive, and its length never passes the board's free tiles;
  - `frame-png` pictures: the start, and the snake after its first food;
  - `cargo shuyi check`, `check --target all` and `check-keys` pass in
    the example's folder.
- **CI and Pages,** either order with the other examples (2735, 2736,
  2737, 2738 and 2740; 2738 comes after this one):
  - if the `wasm` job of `.github/workflows/ci.yml` and
    `.github/workflows/pages.yml` already loop over every
    `examples/*/shuyi.ron` that has a `web` section, snake is picked up
    with no workflow change and plays at `/snake/`;
  - if they still name each example by hand (today
    `examples/hello-grid` alone), change both to that loop:
    `cargo shuyi check --target all` and `cargo shuyi web --release` in
    each folder, an uploaded artifact per example, and in `pages.yml`
    each `dist/web` copied to a folder named after the example, with a
    line on the index page from its `web.title`. The `run-gates` skill's
    web-build step loops the same way;
  - `pages.yml` already loops (2801, done): snake plays at
    `/examples/snake/`, gets its card (`site/content/examples/snake.md`)
    in the Examples section, and `/snake/` redirects there (`aliases` in
    the card's front matter);
  - 2019's `web-render` job (done) boots it too: it loops over the
    examples. **From 3004 (done):** CI's `export` job exports
    `examples/hello-grid` alone, and `tools/export-check/`'s file lists
    name its program; this ticket adds snake to the job (its export and
    check steps, with the lists taking the program's name), so `shuyi
    export` builds it for each of the job's targets. **From 3010
    (done):** `shuyi export playdate` is there, and builds a game that
    does what *What a game adds* in `docs/targets/playdate.md` says (a
    `playdate` feature and `shuyi_playdate::main!`, as Hello, Grid's
    `src/playdate.rs`). It runs on a developer's machine, never in CI
    (the SDK's license), so the `export` job's targets leave it out.
    The example does that, `shuyi export playdate` builds it, and the PR
    says whether it runs in the Playdate Simulator (`--run`, if the SDK
    is installed where the PR is made).
- **The template part `stage-arcade-grid`** in `templates/shuyi-default/`
  (2501):
  - the built-in template gets its own question `time` ("On a grid:
    turns, or real time?"), with the answers `turns` (the default) and
    `ticks`. The part has `when: All([Picked("perspective",
    "top-down-grid"), Picked("time", "ticks")])` and `conflicts:
    ["stage-top-down-grid"]`, and `stage-top-down-grid`'s `when` gains
    `Picked("time", "turns")`;
  - its game is the grid part's small game in real time: the same map,
    pieces and goal, as a `TickSim`. The hero steps a tile every 8 steps
    while a direction is held, the wanderer every 16 steps. An agent
    reading the two parts side by side sees what changes between turns
    and ticks. It is not snake, so the eval's snake task (below) stays a
    task. If 3042 is done, its `GameSettings::BACKGROUND` is
    `Background::KEEP_RUNNING.paused()`; if not, 3042 does it;
  - `--template arcade` is a short name for those answers, as `grid`
    and `blank` are, and `--set time=ticks` picks it. If 2502 is done,
    its prompts show the question;
  - its test: with `Right` held for one second, the hero moves the same
    number of tiles in frames of 1/30 s (`Harness::frame`) as in frames
    of 1/60 s;
  - the matrix of the `templates` job in `.github/workflows/ci.yml`
    gains `arcade`.
- **The guide:** a section *In real time* in
  `docs/guides/top-down-grid.md` (2004): which part starts it, `TickSim`
  and `Every` against `TurnRules`, what not to use (`dt` in rules, a turn
  per tick), and `examples/snake` to read first. It replaces the planned
  *In real time* section 2004 writes there. If 2004 isn't done, create
  the file with this section, and 2004 writes the rest.
- **The skill** `shuyi-stage-arcade`, in ADR-0005 §2's form: a grid
  stage that steps in real time, with a worked example of contract
  operations that makes a game with `--template arcade`, runs it, and
  replays a record. 1134 leaves this skill to this ticket. **From 1134
  (done):** it goes in `skills/shuyi-stage-arcade/`, passes
  `check-skills`, and CI's `skills` job runs its worked example
  (`example.ron`, with `template: "arcade"`).
- **The eval:** 2524 writes `evals/tasks/snake.ron` with
  `waits_for: ["2739"]` and no reference. If 2524 is done:
  - set its `reference` to `Project("examples/snake")`, its `template`
    to `arcade`, and empty its `waits_for`;
  - make its `Scene` checks show that the snake grows when it eats and
    dies on a wall, through scene data its prompt names;
  - leave `Replay` out of its checks: a tick game has no `bots` here
    (Out), so the grader gets no record. This ticket's replay test
    covers the example;
  - **From 3010 (done):** 3010 left `evals/tasks` alone, so add
    `playdate` to its targets and drop its `unmeasured` Playdate entry,
    if its reference passes `cargo shuyi check --target playdate`;
  - the `eval` job's `cargo xtask eval-reference --all` grades it, and
    `cargo xtask eval-reference snake` passes.

  If 2524 isn't done, write the task file yourself with that reference,
  in the fields 2524's scope lists (2524 keeps it).
- **The engine's README:** its *Game styles* paragraph claims a grid in
  real time, linking the example (ADR-0003 §1).
- **What changes for the game:** nothing. `ticks` is new, and `turns`
  keeps its signature. The PR says so.

**Out (do not do):**
- free movement, collision, cameras and animation clips (1120–1123), and
  the platformer (1127);
- a bot trait for tick games, and `shuyi bots` on this example: a
  follow-up ticket if a game asks;
- two players on one keyboard (2406, parked), high-score tables (3054,
  parked), screen transitions (2339);
- game speed and hit-stop (2123, parked), and drawing between steps
  (2361, parked);
- the brick-breaker and the vertical shooter (2738, parked);
- an early snake on `TurnRules` with a command per `ticks()` before 2137
  (the ADR says why);
- new engine features beyond `ticks`. A gap the example shows becomes a
  ticket (the `write-ticket` skill);
- if the PR passes about 600 changed lines without fixtures and
  snapshots, move the template part, its CI entry, the guide section and
  the skill to a follow-up ticket (the
  `write-ticket` skill), and say so in the PR. The same PR changes
  2004's and 3042's mentions of 2739 for those pieces to the new
  ticket's id.
  The eval's snake task then keeps the `template` 2524 gave it (or
  `blank`, if this ticket writes the file), and the follow-up sets it
  to `arcade`.

## Implementation steps

1. Read ADR-0003 §1 and §2, ADR-0009 §2, ADR-0013 §1 and §3, 2137's
   `tick` module and driver, 2115's `Every`, 2511's short path, 2501's
   `template.ron`, and `examples/hello-grid` (its README, `shuyi.ron`,
   `src/tools.rs` and `tests/it/`). Check which of the either-order
   tickets are done: 2004, 2019, 2114, 2225, 2502, 2512, 2514,
   2524, 2801, 3004, 3010 and 3042.
2. Write the ADR, ADR-0013's `Status:` line and both index rows.
3. Write `contract::replay::ticks` and its tests, and the
   `docs/contract.md` row.
4. Write the snake's rules and their tests, then the screens, the files
   and the tools.
5. Write the Harness tests, the pictures, and the CI and Pages steps.
6. Write the template part, its question, its test and the matrix
   entry.
7. Write the guide section, the skill, the eval task and the README
   line. Run the gates.

## Acceptance criteria

- [ ] The ADR is written, ADR-0013's `Status:` line says which of its
      sentences it supersedes, and `docs/adr/README.md` has both rows.
- [ ] The contract's `replay` on the playthrough's record lists step 0
      and one step per tick, and finds no difference between its two
      runs.
- [ ] The snake's golden table passes natively and on wasm32 in CI's
      `determinism` job.
- [ ] The Harness playthrough eats the first food, grows, dies on a wall
      and starts again, through actions only.
- [ ] With no input, the snake moves the same number of tiles in a
      second at 30 and at 60 frames a second.
- [ ] The property test passes.
- [ ] `cargo shuyi check`, `check --target all` (`desktop`, `web`,
      `steam-deck` and `playdate`) and `check-keys` pass on the example,
      and its `frame-png` pictures are checked in.
- [ ] Pages publishes the web build at `/examples/snake/`, and `/snake/`
      redirects there.
- [ ] `shuyi new` with the `arcade` short name makes a game whose gates
      pass in the `templates` job, and its test passes at 30 and 60
      frames a second.
- [ ] The example's README states its own lines, and the engine's README
      claims the style.
- [ ] The skill is in `skills/shuyi-stage-arcade/`, passing
      `check-skills`, CI's `skills` job runs its worked example, and
      the eval's snake task has `examples/snake` as its reference.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a move; a turn, two queued turns, a turn back ignored; eating
  and growing at the next move; the speed-up and its floor; death on a
  wall and on the body; food never placed on the body (with
  `ScriptedRng`); `ticks` replaying a three-step record.
- Property: random inputs keep the body's rules; a replay twice gives
  the same hashes.
- Integration: the playthrough; the contract's `replay` on its record;
  30 and 60 frames a second, for the example and the template part's
  game; `check --target all`.
- Golden: the playthrough's record, natively and on wasm32 (2116).
- Snapshot: the two `frame-png` pictures.

## Completion notes

*(Filled in by the session that completes the ticket.)*
