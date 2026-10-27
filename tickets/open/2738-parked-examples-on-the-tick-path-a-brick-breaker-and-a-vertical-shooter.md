---
id: "2738"
title: "Parked: Examples on the tick path: a brick-breaker and a vertical shooter"
type: feature
milestone: "Engine: examples"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119", "1122", "2739", "2511"]
nick_input: none
completed:
---

# 2738 — Examples on the tick path: a brick-breaker and a vertical shooter

> **Parked** (Nick, 2026-10-20: real-time work waits "until visions of shuyi is complete since it's the first game I want to be launched with the engine", and a gallery of small games is one of the three kinds of game he added that wait for the launch): this builds on the real-time work (1119, 1122, 2739), and Visions of Shuyi, a turn game, needs neither example. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Small real-time games are how they show their engines:

- Bevy keeps a brick-breaker among its examples
  (https://github.com/bevyengine/bevy/blob/main/examples/showcase/breakout.rs);
- Godot's 2D demo projects include `bullet_shower`, a field of bullets
  to dodge, and `dodge_the_creeps`, the game of its first 2D tutorial;
- Construct 3 opens on an Example Browser of tagged projects.

What Shuyi has today:

- no simulation that steps in time. `FrameInput::ticks()`
  (`crates/ui/src/input/frame.rs:114`) counts the fixed steps due, from
  `TickClock` (`crates/platform/src/tick.rs`), but outside tests nothing
  reads it;
- no fixed-point numbers, entity arena or collision. `shuyi-core` has
  the grid, turn rules, grid paths, `SimRng` and state hashes
  (`crates/core/src/lib.rs`);
- the frame already draws what these games need, in
  `crates/ui/src/frame/glyph_buffer.rs`: `Overlay`, a solid rectangle in
  console pixels, and `Sprite`, part of an image in console pixels,
  with `Paint::Solid` for a hit flash. `cargo shuyi glyph-tiles` makes a
  tileset from the font and the palette, as it makes hello-grid's
  `files/tiles.png`;
- `UiAction` (`crates/assets/src/keymap/action_set.rs`) has the six
  actions these games need: `Up`, `Down`, `Left`, `Right`, `Confirm`
  and `Cancel`.

What it builds on:

- 1119 (parked): `Fx`, `FxVec2`, `Arena<T>`, `TickSim`, `TickInput`,
  `TickRecord`, `replay` and the driver;
- 1122 (parked): `TileCollision`, `Aabb`, the broad phase,
  `Mover::move_x` and `move_y` (one pixel at a time, returning what
  they hit), `Trigger`, and the top-down controller with its numbers as
  data;
- 2739 (parked): snake, the first real-time example, which Nick's end
  goal names. It adds `shuyi_cli::contract::replay::ticks`, which
  replays a `TickRecord` through the contract, and it builds on 2115's
  `Every`. These examples follow its form;
- 2511 (MVP): the short path, with `BasicGame`, `run_game`,
  `BasicTools` and the prelude.

Why it matters:

- **The game first:** Visions of Shuyi is turn-based and needs none of
  this, so it waits for the launch and for the real-time work.
- **Other 2D games:** Nick added a gallery of small games on
  2026-10-20. A brick-breaker is the smallest test of 1122's movers
  against tiles and boxes: a ball that must never pass through a brick.
  A vertical shooter is the smallest test of 1119's arenas with many
  things at once: shots, foes and their shots made and removed every
  step, and replayed exactly.
- **The end goal** (2026-10-19): "make me a snake game" comes first
  (2739). These are the next prompts an agent gets, and each example is
  an eval task's reference solution (2524).

**One example per PR.** An example in hello-grid's form is more than
600 changed lines (hello-grid's code and tests are about 4,600). So
this ticket's PR builds the brick-breaker, and writes the shooter,
specified below, as a ticket of its own.

The first is called a brick-breaker (`examples/bricks`), not by the
name of the original game, as 2803 calls its game a box-pushing
puzzle. Nothing comes from an existing game of either kind (no names,
levels, enemy patterns or art), and nothing from Visions of Shuyi
(`AGENTS.md` rule 4).

## Nick input

None. When it unparks is his call (above).

## Scope

**In:**
- **`examples/bricks`, a workspace member** in 2739's form, written on
  2511's short path (its prelude and `run_game`), on the `shuyi` crate
  alone:
  - **the rules** (`src/rules.rs`, pure: no I/O, no clock, no float):
    `Bricks`, a `TickSim`:
    - the field is a `.map` (`files/field.map`, read with
      `shuyi_assets::grid_map` as hello-grid reads `stage.map`). Its
      legend: `#` a wall, `.` empty, `1` a brick that takes one hit,
      `2` a brick that takes two. Walls on three sides, open at the
      bottom. 1122's `TileCollision` is built from it;
    - a hit on a two-hit brick turns its tile into a one-hit brick, and
      a brick's last hit makes its tile empty (through `TileCollision`'s
      setter if 1122 has one, rebuilt from the grid if not);
    - the paddle is an `Aabb` moved by held `Left` and `Right`. Its
      mover stops at the walls and at the ball;
    - the ball waits on the paddle until `Confirm`, then leaves at the
      launch speed. Each step it moves with `Mover::move_x`, then
      `move_y`, at an `FxVec2` speed. A hit on x reverses its x speed, a
      hit on y its y speed. Off the paddle, its x speed comes from how
      far from the paddle's centre it hit, in steps, so the player aims;
    - every 8 brick hits the ball speeds up by a step, to a cap;
    - a broken brick scores the points of its kind;
    - a `Trigger` below the paddle takes a life and puts the ball back
      on the paddle. Three lives. The example's own `Outcome` is
      `Won { score, step }` when no brick is left, and `Lost { score }`
      when the lives are gone;
    - events: `Bounced`, `Hit`, `Broke`, `LifeLost`, `Won` and `Lost`;
    - the numbers (the tile size in console pixels, the paddle's speed,
      the launch speed, the aim steps, the speed-up and its cap, the
      points per brick kind) are a `Tuning` in `files/tuning.ron`, read
      with `shuyi_assets::ron::Ron<T>`;
    - no luck: a run is its inputs alone. `hash` is the state's
      `state_hash`.
  - **the screens:** a title, the stage, a pause menu on `Cancel`, and
    an end screen with the score that starts again on `Confirm`. The
    stage steps the rules once for each of `input.ticks()` through
    1119's driver, and keeps the run's `TickRecord`. A suspended game
    gets 0 ticks, so the ball waits. If 3042 is done, losing focus opens
    the pause menu (its system hook on `FocusLost`); if not, 3042 adds
    that.
  - **the drawing:** `Overlay`s on the glyph console. The bricks take a
    palette role per row (`files/palette.ron`), then the paddle and the
    ball. The score and the lives are text. **Either order with 2336:**
    if 2336 is done, the ball is a filled circle; if not, it is a
    square `Overlay`, and 2336 makes it a circle. If 2340 is done, a
    lost life shakes the frame through `Ctx::shake`, so the player's
    reduced-motion setting holds; if not, 2340 adds that shake when it
    lands.
  - **the actions:** `UiAction`'s six, with the default layouts and a
    `steam-deck` and a `playdate` table in `files/keymap.ron`. Nothing
    names a key (the `input-actions` skill). `shuyi.ron` has a
    `check_keys` section, as `templates/blank/shuyi.ron` has.
  - **a frame the Playdate fits:** `ConsoleSpec::text(50, 15)`, 400 ×
    240 at scale 1, holding the field and a score line. `shuyi.ron`
    lists `desktop`, `web`, `steam-deck` and `playdate`. The palette's
    roles stay apart in 1-bit (3006's check).
  - **the tools** (`src/tools.rs`): a `GameTools` that calls
    `BasicTools`' methods for the rest, with `session` (a `Headless`
    with a scene, `Headless::with_scene`, holding `score`, `lives`,
    `bricks_left`, `step` and `outcome`) and `replay` through
    `contract::replay::ticks`. `bots` stays unsupported (Out).
  - **sound:** if 2225 is done, a bounce, a break and a lost-life sound
    from its presets. If not, none, the README says so, and 2225's PR
    adds them (its item on the gallery examples says so).
  - the font atlas copied from `examples/hello-grid/files/font`
    (Terminus, OFL-1.1, with its license file), and a row for the copy
    in `THIRD_PARTY_ASSETS.md`.
  - **a README** in hello-grid's form: how to run it, its targets, and
    the lines of its own code without tests and data (`cargo xtask
    template-lines` if 2512 is done; counted, and said how, if not).
    **Either order with 2514:** if 2514 is done, an animated picture of
    its first seconds made with `frame-png`; if not, 2514 adds it (its
    clause says so). **Either order with 2811:** if 2811 is done, that
    picture is the example's preview: `preview.png` in its folder, made
    by a `preview` part in its `shuyi.ron` (`cargo shuyi frame-png
    --preview`), checked by a test in `tests/it/` as hello-grid's is,
    and shown on its card on the Examples page. If not, nothing to do
    (2811 makes the picture its preview). The engine's `README.md`
    names it beside hello-grid.
- **Tests of the example** (`tests/it/`):
  - a Harness playthrough through actions only (`Harness::act`, with
    `Harness::wait` between). From the title: start, launch, and the
    ball cracks the test field's brick, comes down past the paddle and
    takes a life; launch again, break the brick, and reach the end
    screen with `Won`. The test field, `tests/it/one_brick.map`, has one
    two-hit brick in the line the ball leaves on, high enough that the
    ball comes down beside the unmoved paddle. The Harness reads it in
    place of `files/field.map` from a bundle whose first layer is a
    `Layer::table` (`crates/assets/src/bundle.rs`);
  - with `Right` held for one second, the paddle is in the same place at
    30 frames a second as at 60 (`Harness::frame` with a `dt` of 1/30
    and of 1/60, holding what the keymap binds to `Right`);
  - replay: the playthrough's `TickRecord` gives the same hash list
    twice. 2116 is done before this ticket (1119 needs it first), so
    the record is also checked in with a golden table of the example's
    own, in the form 2116 gives Hello, Grid's: the record in
    `examples/bricks/tests/records/playthrough.ron`, the table
    `tests/golden/bricks.ron` (`step NNN` rows and `outcome`) checked by
    `tests/golden.rs`, the window behind a default `window` feature, and
    a `cargo test --target wasm32-wasip1 -p bricks --no-default-features
    --test golden` step in CI's `determinism` job, as 2116's line in the
    `run-gates` skill says a new example does. Not in hello-grid's
    `tests/records/`: its `golden.rs` replays a `StageRecord`;
  - a property test: random inputs for 5 000 steps never panic, and the
    ball is never inside a brick, a wall or the paddle;
  - `frame-png` pictures: the field at its start and mid-play;
  - `cargo shuyi check`, `check --target all` and `check-keys` pass.
- **CI and Pages,** either order with 2735, 2736, 2737 and 2740 (2739
  lands first, and may already have made the loop):
  - if the `wasm` job of `.github/workflows/ci.yml` and
    `.github/workflows/pages.yml` loop over every `examples/*/shuyi.ron`
    that has a `web` section, the bricks are picked up with no workflow
    change;
  - if they still name each example by hand (today
    `examples/hello-grid` alone), change both to that loop: `cargo shuyi
    check --target all` and `cargo shuyi web --release` in each folder,
    an uploaded artifact per example, and in `pages.yml` each
    `dist/web` copied to a folder named after the example, with a line
    on the index page from its `web.title`. The `run-gates` skill's
    web-build step loops the same way;
  - `pages.yml` already loops (2801, done): each example gets its card,
    `site/content/examples/<name>.md`, plays at `/examples/<name>/`, and
    `/bricks/` redirects there (`aliases` in the card's front matter).
    If 2019 is done, its headless browser job opens it.
- **The eval:** if 2524 is done, add `evals/tasks/bricks.ron`: template
  `blank`, targets `desktop` and `web`, reference
  `Project("examples/bricks")`, and the checks `Check`, `Targets`,
  `Web`, `Responds` and `Scene` (on the facts its prompt asks the game
  to expose: `/lives` and `/score`). No `Replay` check: it asks `bots`,
  which this example doesn't offer. `cargo xtask eval-reference bricks`
  passes. If 2524 isn't done, add the task to 2524's first tasks in its
  ticket, in this PR.
- **The shooter's ticket** (the `write-ticket` skill, the next free
  `27xx` id): `examples/shooter`, a vertical shoot-'em-up in the
  bricks' form, with the scope below and what the bricks taught. It has
  this ticket's milestone and `blocked_by: ["2738"]`, and is parked only
  if Nick unparked the bricks alone. In the same PR, the mentions of
  2738's shooter in 2122, 2124 and 2335, and the line on the gallery in
  `docs/ROADMAP.md`, name the new ticket. Its scope:
  - the ship moves with 1122's top-down controller, its numbers in
    `files/ship.ron`, clamped to the field. `Confirm` held fires a shot
    each time an `Every` (2115) fires;
  - shots, foes and foes' shots live in 1119's `Arena`s. A thing past
    the field's edge is removed;
  - **waves as data,** `files/waves.ron`: each wave's start step, its
    foe kind, how many foes and how many steps apart, and its path as
    points, each with the steps a foe takes to reach it from the one
    before. Between two points a foe moves in a straight line, at
    `a + (b - a) * t / steps` in `Fx`. The foe kinds are in the same
    file: health, points, and how often it fires (0 = never). Foes'
    shots fly straight down. If 2122 is done, a path is a `Curve` and
    foes fly it with `CurveFollower`. If not, foes fly straight lines
    between the points, and 2122 leaves them so;
  - hits are overlaps of `Aabb`s through 1122's broad phase, or 2124's
    `overlap_rect` if 2124 is done. A hit ship loses a life and can't
    be hit for the next 120 steps. Three lives. `Won` when the last wave
    is cleared, `Lost` when the lives are gone;
  - drawn with `Sprite`s from a tileset made by `cargo shuyi
    glyph-tiles` (the ship, two foe kinds, the shots), a hit flash with
    `Paint::Solid`, and stars placed from the step number, in the view
    only. **Either order with 2335:** if 2335 is done, foes turn to
    face where they fly, to the nearest of 8 directions from the signs
    and sizes of their speed; if not, they keep one picture, and 2335
    adds nothing here (its Out says so).
    The tileset's glyphs are Terminus's, so its row in
    `THIRD_PARTY_ASSETS.md` names `files/tiles.png` too, as
    hello-grid's does;
  - tests: a Harness playthrough that fires straight ahead from the
    start until the first wave is cleared; 30 and 60 frames a second;
    replay; a property test that random inputs for 5 000 steps never
    panic and the arenas never hold more than a bound the README
    states; `frame-png` pictures; and units for a wave starting at its
    step with its spacing, a foe along its path, a shot hitting a foe, a
    foe's shot hitting the ship and the 120 safe steps, things removed
    past the edge, both outcomes, and a broken `waves.ron`;
  - the rest as the bricks have it: the actions, the Playdate frame,
    the tools, sound, CI and Pages, the README and an eval task;
  - its Out: power-ups, bosses, aimed shots and a scrolling field.

**Out (do not do):**
- building `examples/shooter`: its own ticket, written in this PR
  (above);
- power-ups, more than one field, and bricks that move: follow-ups if a
  game asks;
- snake and the real-time grid template part (2739); the platformer
  (1127);
- a scrolling field and parallax (1120);
- game speed and hit-stop (2123), and drawing between steps (2361);
- the mouse or an analog paddle (1121);
- a bot trait for tick games, and `shuyi bots` on these examples
  (2739's Out);
- high-score tables (3054) and two players (2406);
- a `kind` answer in `docs/modules.ron` (2003), a guide (2004) or a
  template part (2501): the examples prove the default engine, not a
  module (ADR-0009 §7);
- anything from an existing game of either kind or from Visions of
  Shuyi;
- new engine features. A gap the example shows becomes a ticket (the
  `write-ticket` skill), named in the Completion notes.

## Implementation steps

1. Read 2739's `examples/snake` (its tools, its stage and `tests/it/`)
   and `contract::replay::ticks`; 1119's `fx`, `arena` and `tick`
   modules and its driver; 1122's `collide` module and top-down
   controller; 2511's `BasicGame`, `BasicTools` and `run_game`; and
   `Overlay` and `Sprite` in `crates/ui/src/frame/glyph_buffer.rs`.
   Check which either-order tickets are done.
2. Write `examples/bricks`' `Cargo.toml`, `shuyi.ron` and `files/` (the
   field, the tuning, the keymap, the palette and the font copy), and
   the `THIRD_PARTY_ASSETS.md` row.
3. Write the rules with their unit tests.
4. Write the screens, the drawing and the tools.
5. Write the Harness tests, the property test and the `frame-png`
   pictures.
6. Write the CI and Pages loop if 2739 didn't, the README, the engine's
   README line and the eval task.
7. Write the shooter's ticket, and update 2122, 2124, 2335 and
   `docs/ROADMAP.md` to name it. Run the gates.

## Acceptance criteria

- [ ] The Harness playthrough launches, loses a life, and clears the
      one-brick field, through actions only.
- [ ] `Right` held for one second puts the paddle in the same place at
      30 and at 60 frames a second.
- [ ] The playthrough's record replays to the same hash list twice, and
      `cargo shuyi replay` on it lists one step per tick.
- [ ] The bricks' golden table passes natively and on wasm32 in CI's
      `determinism` job.
- [ ] The property test passes.
- [ ] `cargo shuyi check`, `check --target all` (`desktop`, `web`,
      `steam-deck` and `playdate`) and `check-keys` pass on the example,
      and its `frame-png` pictures are checked in.
- [ ] CI builds the example for the web, and Pages publishes it.
- [ ] The example's README states its own lines, and the engine's
      README names it.
- [ ] The eval has a bricks task with the example as its reference, or
      2524's scope has it.
- [ ] The shooter's ticket is in `tickets/open/`, and 2122, 2124,
      2335 and `docs/ROADMAP.md` name it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a bounce off a wall, off a one-hit and a two-hit brick, and off
  the paddle at its centre and at its edge; the paddle stopped by a
  wall; the speed-up and its cap; the trigger taking a life and putting
  the ball back on the paddle; both outcomes; a broken `tuning.ron`, and
  a field with an unknown tile, each with its file and line.
- Property: random inputs for 5 000 steps, as above.
- Integration: the playthrough; 30 and 60 frames a second; replay;
  `check --target all`.
- Golden: the playthrough's record, natively and on wasm32 (2116).
- Snapshot: the `frame-png` pictures.

## Completion notes

*(Filled in by the session that completes the ticket.)*
