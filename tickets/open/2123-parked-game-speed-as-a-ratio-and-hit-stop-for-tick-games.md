---
id: "2123"
title: "Parked: Game speed as a ratio for tick games, hit-stop as simulation state, and the view and UI on real time"
type: feature
milestone: "Engine: real time"
model: implementation
effort: high
status: blocked
blocked_by: ["1119"]
nick_input: none
completed:
---

# 2123 — Game speed and hit-stop for tick games

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches (`docs/decisions.md`, 2026-10-20), unless the game needs it first: its ticket 0060 says the army walks around freely, which unparks 1119, and the game asks through its `engine-change` skill for fast-forward, slow motion or hit-stop on that free-moving stage. Then this comes right after 1119. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and with Defold and Construct 3. Each of them lets a
game run its world slower, faster or not at all, while its menus keep
real time:

- Bevy has `Time<Virtual>` with a relative speed and a pause, drives
  `Time<Fixed>` from it, and keeps `Time<Real>` beside it
  (docs.rs/bevy/latest/bevy/time/struct.Virtual.html);
- Godot has `Engine.time_scale`, and the scene tree's pause with
  process modes for nodes that keep running;
- Unity has `Time.timeScale` and `Time.unscaledDeltaTime`
  (docs.unity3d.com/ScriptReference/Time-timeScale.html);
- Unreal has global time dilation and per-actor `CustomTimeDilation`;
- Defold's collection proxies take `set_time_step`; Construct 3 has a
  time scale for the game and per object.

Games use it for hit-stop (the world freezes for a few steps when a
blow lands), slow motion, fast-forward, and an active pause: the world
stops, but the stage's own cursor and HUD still answer the player.

What Shuyi has today:

- `crates/platform/src/tick.rs`: `TickClock` turns a frame's time into
  fixed steps, 60 a second (`STEPS_PER_SECOND`), at most `MAX_STEPS`
  (4) a frame. It keeps the time not yet stepped in `owed`, in
  sixtieths of a nanosecond, so a step is `SECOND` of them. It has no
  speed.
- `crates/platform/src/app.rs`: `Clock::at` and `Clock::after` call
  `TickClock::advance`, and `step` calls `game.set_ticks(elapsed.ticks)`
  (`crates/ui/src/game.rs:241`). The top screen reads
  `FrameInput::ticks()` (`crates/ui/src/input/frame.rs:114`), whose doc
  comment says "at most 4 a frame". ADR-0024 §4 makes this one
  accumulator for the whole game, not one per screen.
- `crates/ui/src/screen.rs`: `ScreenStack` updates only the top screen.
  So a pause menu pushed over a tick stage already stops its world, with
  no speed at all. This ticket doesn't redo that.
- `crates/ui/src/hurry.rs`: `HurryClock` (2312, done) runs a timed
  screen faster while a key is held. It works in real seconds (`f32`)
  for UI, not for a simulation.
- `Ctx::clock_s` (`crates/ui/src/ctx.rs`) adds up real seconds.

What is planned:

- 1119 (parked): `TickSim`, `TickRecord`, `replay`, and `TickDriver`,
  which steps the simulation `FrameInput::ticks()` times and keeps no
  clock of its own. The same comparison gives 1119 the input latch
  (its Scope, *Input reaches the steps*): a frame that runs no step
  keeps its presses for the next step, whatever the reason, a slowed or
  stopped game speed included. It ships in the same PR as `TickDriver`.
- 2316 (After MVP): debug pause, step and slow motion in
  `Ctx::debug_view` (2315). Its slow motion updates one frame in *n*,
  so it works for turn games too. Its mechanism stays; this ticket makes
  the two compose for tick stages.
- 2706 (parked): `CommandDriver` steps a command sim
  `FrameInput::ticks()` times, as `TickDriver` does.
- 2361 (parked): `TickClock::overstep`, how far a frame is into the
  next step, for drawing between steps.
- Things keyed on a step number: clips (1123, 2231), particles (2103),
  tweens in a simulation (2109), timers in steps (2115), the camera
  (1120, 2362).

Nothing gives a tick game a speed. Today a game would run its sim
`ticks() * 2` times in its own screen, or skip steps. A `TickSim::step`
takes no `dt`, so nobody can break determinism by stretching one; but
the parts that are easy to get wrong are left to each game:

- a part speed kept in integers (a half, a quarter), so the steps don't
  drift;
- the cap, so 4x on a 60 Hz display really is 4x and a slow device
  doesn't spiral;
- presses on the frames that run no step at a slow speed;
- proof that replays don't depend on the speed history;
- hit-stop that a rollback or lockstep session (ADR-0006) can replay.
  A driver that skips steps when a hit lands is wrong there, because
  every peer must run the same steps. So hit-stop belongs in the
  simulation's state.

Why it matters:

- **The game first.** Visions of Shuyi is turn-based. Its hurry and
  fast-forward needs are met by `HurryClock` (2312), so it needs this
  only if its 0060 adds a free-moving stage.
- **Other 2D games.** A platformer (1127) or an action game uses
  hit-stop. Tower defense, farming and city games use 2x and 4x.
  Real-time strategy games often have a speed setting in single player;
  the parked RTS tickets (2705–2720) have none.
- **The end goal** (2026-10-19: a game "like snake or mario" in under
  an hour), which waits for the launch with the rest of real time: an
  agent sets one field and gets the subtle parts right, and replays
  stay exact.

## Nick input

None.

## Scope

**In:**
- **An ADR** (`write-adr` skill), extending ADR-0003 §2 and ADR-0024 §4.
  The status lines of ADR-0003 and ADR-0024, and their rows in
  `docs/adr/README.md`, gain "extended by ADR-NNNN". It says:
  - game speed changes how many fixed steps a real second brings, never
    a step's length. A step is still 1/60 s of game time, and no `dt`
    reaches a simulation;
  - the speed is applied by `App`'s `TickClock`, so there is still one
    accumulator for the whole game (ADR-0024 §4);
  - **the cap:** a frame runs at most `MAX_STEPS` times the speed
    rounded up to a whole number, and never fewer than `MAX_STEPS`.
    That is 4 at normal speed and below, 8 above 1x up to 2x, 12 above
    2x up to 3x, and 16 above 3x up to 4x. So a late frame catches up at
    least four frames' worth at that speed, and 4x on a 60 Hz display
    is really 4x;
  - records hold the steps that ran, so a replay never depends on the
    speed history;
  - the speed is local to one machine. A networked session (1147's
    rollback, 2720's lockstep) sets the pace of its own steps, and its
    ticket says what a local speed does under it. A speed every peer
    shares is a session setting, not this ticket's;
  - **hit-stop is simulation state** (`Freeze`, below), counted in steps
    and part of the hash, so it replays and rolls back. Setting the
    speed to 0 for some frames is not hit-stop: its length would depend
    on the frame rate, and no session could share it;
  - **two clocks:**
    - *game time* is steps. Clips keyed on `(clip, start_step)` (1123,
      2231), particles (2103), tweens in a simulation (2109), timers in
      steps (2115) and the camera stepped once per step (1120, 2362)
      follow the speed. They stop at speed 0, and they hold during
      hit-stop when keyed on `Freeze::world_step`. The camera is view
      state, stepped on every step, frozen ones too: through a hit-stop
      it settles on a target that doesn't move and 1120's shake keeps
      dying down. A stage that wants it held too skips its step while
      the sim's `Freeze` is frozen;
    - *real time* is `FrameInput::dt`, `Ctx::clock_s`, `HurryClock`
      (2312) and `Ctx::music_clock`. Menus, the text box, a stage's
      HUD, screen transitions (2339), frame shake and flash (2340) and
      the debug view read it, and never slow down;
  - turn games read only real time, and the speed doesn't touch them;
  - a pause menu stops the world with no speed, because only the top
    screen updates (`crates/ui/src/screen.rs`).
- **`GameSpeed`** in `shuyi_ui::tick` (the module 1119 creates):
  - a `u16` in parts of 256, in a private field: 256 is normal, 0 stops the
    world, 1024 (4x) is the most;
  - constants `STOPPED`, `QUARTER`, `HALF`, `NORMAL`, `DOUBLE`, `MAX`;
  - `GameSpeed::from_256(n: u16)`, clamped to `MAX`;
    `GameSpeed::ratio(num: u32, den: u32) -> Option<GameSpeed>`, worked
    out in `u64`, rounded down to parts of 256 and clamped to `MAX`, `None`
    when `den` is 0; `in_256(self) -> u16`;
  - derives `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `Hash`,
    `Serialize`, `Deserialize`; `Default` is `NORMAL`. It is written as
    a plain number: `#[serde(from = "u16", into = "u16")]`, with
    `From<u16> for GameSpeed` through `from_256` (so reading clamps
    to `MAX` too) and `From<GameSpeed> for u16`;
  - `Display` in hundredths, rounded down, in integers, as dumps and the
    readout print it: `0.25x`, `0.50x`, `1.00x`, `4.00x`.
- **`Ctx::game_speed: GameSpeed`** (`crates/ui/src/ctx.rs`), a public
  field like `Ctx::debug_tools`, `NORMAL` in `Ctx::new`. `Ctx` already
  has private fields, so a new public field breaks no game. A screen
  sets it in its update, and it applies from the next frame. The doc
  comment says:
  - a stage that changes it puts it back when it leaves;
  - presses made while it is `STOPPED` wait in 1119's latch for the
    next step, so a stage whose HUD used them clears the latch (as it
    does when covered) before it sets the speed back.
- **`Game::clock_speed(&self) -> GameSpeed`** (`crates/ui/src/game.rs`):
  the speed `App` hands the clock. Today it is `ctx.game_speed`; the
  2316 part below folds in the debug pause and slow motion here, so
  `shuyi-platform` never reads the debug view.
- **`TickClock`** (`crates/platform/src/tick.rs`):
  - `advance(&mut self, elapsed: Duration, speed: GameSpeed) -> u32`.
    `owed` gains the elapsed nanoseconds times `STEPS_PER_SECOND` times
    the speed in parts of 256, with saturating multiplies as today. A step is
    `STEP` (`SECOND * 256`, a new private constant) of these units, and
    `owed`'s doc comment says what its unit is now. At `NORMAL` it
    steps exactly as it does today, and every existing test in
    `tick.rs` (`Duration::MAX` included) still passes with `NORMAL`.
    `App` is the only caller today; the PR says that the public
    signature changed;
  - the cap above, as `pub fn max_steps(speed: GameSpeed) -> u32`. A
    frame with more due drops the rest, as today;
  - at `STOPPED`, nothing is added and the part step is kept. A change
    of speed keeps the part step too. `reset` still forgets it;
  - the module's doc comment and `MAX_STEPS`'s say how the cap grows
    with the speed.
- **The other doc comments that say "at most 4 a frame":**
  `FrameInput::ticks` (`crates/ui/src/input/frame.rs`) and
  `Game::set_ticks` say the steps come at the game's speed, capped by
  `max_steps`.
- **`App`** (`crates/platform/src/app.rs`): `frame` and `frame_after`
  read `Game::clock_speed` from the running game (`App::game`) after
  the system's events and before they advance the clock, and hand it to
  `Clock::at` and `Clock::after`. While loading or failed, the speed is
  `NORMAL`. `App` also counts the steps it has handed out
  (`App::ticks_given(&self) -> u64`), for the Harness.
  `docs/platform-backends.md`, *What one frame does*, item 2, says the
  steps come at the game's speed, with the cap above.
- **The Harness** (`crates/platform/src/harness.rs`):
  - `Harness::game_speed(&mut self, speed: GameSpeed) -> &mut Self`
    sets `Ctx::game_speed`;
  - 1119's `harness.steps(n)` runs exactly `n` simulation steps at any
    speed above 0. It runs frames until `App::ticks_given` has grown by
    `n`, each short enough to bring at most one step: `FRAME_DT` divided
    by the speed rounded up to a whole number (1/60 s at `NORMAL` and
    below, 1/120 s at `DOUBLE`, 1/240 s at `MAX`). At `STOPPED` it
    panics with a message naming the speed. Like `wait`, it panics after
    `MAX_FRAMES` frames, naming the speed, when the speed is too slow to
    finish.
- **`Freeze`**, the hit-stop helper, in `shuyi_core::tick` (1119's
  module), `no_std`, no floats, kept small:
  - `Freeze` with private fields `world_step: u64` and `left: u16`,
    `Freeze::new()` and `Default`;
  - `hold(&mut self, steps: u16)`: the world stops for the next `steps`
    steps. A hold during a hold keeps the longer one;
  - `step(&mut self) -> bool`, called once at the top of
    `TickSim::step`: `true` when the world moves this step (and
    `world_step` counts it), `false` while frozen (and `left` counts
    down);
  - `world_step(&self) -> u64` and `is_frozen(&self) -> bool`;
  - it derives `Clone`, `Copy`, `Debug`, `Default`, `PartialEq`, `Eq`,
    `Hash`, `Serialize`, `Deserialize`, so it sits in a sim's state and
    hash;
  - the doc comment says that presses still reach `step` while frozen,
    so a sim can keep a press for after the freeze, and that a sim keys
    clips and particles on `world_step` to hold them still.
- **1119's example sim** (the dot) uses it: when the dot reaches the
  edge of its area (give it one if 1119's dot has none) it freezes for
  6 steps. A direction pressed during the freeze is kept and used when
  the freeze ends.
- **Presses at slow speeds.** At `QUARTER` three frames in four run no
  step. 1119's latch keeps those presses for the next step; this ticket
  tests it under speed and builds no latch of its own. If 1119 is done
  without the latch, stop: write a ticket for it (`write-ticket` skill),
  add it to `blocked_by`, and say so in the Completion notes.
- **Command sims.** 2706's `CommandDriver` reads `FrameInput::ticks()`
  as `TickDriver` does, so it gets the speed with nothing more.
  **Either order with 2706:** if 2706 is done, add one test that `HALF`
  runs half its steps; if not, nothing to do (2706 adds it).
- **Lockstep.** 2720's `CommandLockstep` gate sets the pace of a
  networked command sim, and a peer's speed only makes it wait.
  **Either order with 2720:** if 2720 is done, add its test over
  `MemoryNetwork`: a peer at `DOUBLE` and one at `NORMAL` end with the
  same hash list, at the `NORMAL` peer's pace; if not, nothing to do
  (2720 adds it).
- **Hit-stop from a hit. Either order with 2756:** if 2756 is done, add
  `ActionMath::hit_stop(attacker, target, strike) -> u16` (default 0)
  to `shuyi-action`. Each `Hit` carries it, and a sim that keeps a
  `Freeze` holds it for that many steps (a hold during a hold keeps the
  longer one), with a test. If not, nothing to do (2756 adds it).
- **Debug pause and slow motion. Either order with 2316.** 2316's slow
  motion updates the stack on one frame in `slow`, and each update
  takes only its own frame's ticks. Under a game speed that alone
  doesn't multiply: at `HALF` the steps fall on every other frame, and
  slow motion could skip all of them. So if 2316 is done:
  1. `Game::clock_speed` returns `STOPPED` while `Ctx::debug_view` is
     paused, and otherwise the game speed divided by its `slow`
     (`GameSpeed::slowed(self, n: u8) -> GameSpeed`, rounded down to
     parts of 256; `n` of 0 or 1 changes nothing);
  2. a frame that 2316's slow motion skips keeps its ticks for the next
     update instead of dropping them: `Game::set_ticks` adds to what a
     skipped frame kept, and an update that runs takes them all;
  3. DebugStep still runs exactly one simulation step at any game speed:
     its update gets one tick, whatever is kept;
  4. the readout gains a line naming the game speed when it isn't
     `NORMAL` (`speed 0.50x`, from `Display`), after 2316's `paused`
     line.

  If 2316 isn't done, make its *Either order with 2123* clause (at the
  end of its **A tick game** bullet; add one there if it has none) hold
  all four points in this PR, as what 2316 does when it lands after
  this ticket.
- **Drawing between steps. Either order with 2361.** 2361's
  `TickClock::overstep` reads `owed`, whose unit this ticket changes.
  If 2361 is done:
  - `overstep` divides `owed` by `STEP` instead of `SECOND`, so slow
    motion is drawn smoothly. At `STOPPED` nothing is added, so the
    overstep holds by itself;
  - its rule that a frame which drops time forgets the part step uses
    `max_steps(speed)`, not `MAX_STEPS`;
  - hit-stop needs nothing: frozen steps build the same layers, so the
    blend holds still (2361, *Holding still*);
  - add the tests in the acceptance criteria below.

  If 2361 isn't done, its *Either order with 2123* clause already says
  it divides by `STEP` and adds these tests. Nothing is added to it.

**Out (do not do):**
- pausing the world under a menu: the screen stack already does it;
- a driver-level hold or hit-stop counted in frames;
- per-entity time dilation: a sim scales its own speeds;
- the input latch (1119's, *Input reaches the steps*): this ticket only
  tests it under speed;
- slowing, pitching or pausing sounds and music with the speed: a
  ticket when a game asks, on the playback rate the parked mixer (3030)
  brings. Sounds paused under a pause menu are 2394's;
- scaling `FrameInput::dt` for turn games: a turn stage that wants a
  battle-speed setting multiplies its own `dt`, as `HurryClock` (2312)
  does;
- a player option row for game speed (a game builds one with 2314's
  option rows);
- what a local speed does in a networked session (1147, 2720), beyond
  the test for 2720 above, and a speed every peer shares;
- frame shake and flash during a hit-stop (2340);
- overlay screens that let the screen below keep running (2367);
- the camera's own behaviour (1120, 2362): it is stepped once per
  simulation step, so it follows the speed already, and the ADR above
  says what it does in a hit-stop;
- drawing between steps (2361), beyond the either-order part above.

## Implementation steps

1. Read ADR-0003 §2, ADR-0024 §4 and §5, `crates/platform/src/tick.rs`,
   `Clock` and `step` in `crates/platform/src/app.rs`, `set_ticks` and
   `frame` in `crates/ui/src/game.rs`, 1119's `shuyi_core::tick`,
   `shuyi_ui::tick`, its latch, `harness.steps` and its example sim, and
   2316, 2361, 2706, 2720 and 2756 if they are done.
2. Write the ADR, the status lines of ADR-0003 and ADR-0024, and their
   rows in `docs/adr/README.md`.
3. Add `GameSpeed` with its unit tests.
4. Change `TickClock::advance` and add `STEP` and `max_steps`. Keep
   every existing test in `tick.rs` passing with `NORMAL`, then add the
   new tests. Update the doc comments of `FrameInput::ticks` and
   `Game::set_ticks`.
5. Add `Ctx::game_speed` and `Game::clock_speed`, read it in
   `App::frame` and `App::frame_after`, add `App::ticks_given`, and
   update `docs/platform-backends.md`.
6. Add `Harness::game_speed` and make `harness.steps(n)` count steps.
7. Add `Freeze` with its unit tests, and use it in 1119's example sim.
8. Write the property tests and the Harness tests below.
9. Do the either-order parts for 2316, 2361, 2706, 2720 and 2756. If
   the PR passes about 600 changed lines (snapshots aside), move the
   2316 part to a follow-up ticket (`write-ticket` skill), parked like
   this one and blocked by it, and say so in the Completion notes. The
   2361 part stays, since it keeps 2361's overstep right.
10. Run the gates.

## Acceptance criteria

- [ ] At `NORMAL`, `TickClock` gives the same steps as before for any
      list of frame times from 0 to 10 s, the cap included (property
      test against the formula as it was when this ticket started, kept
      in the test module).
- [ ] At `HALF`, 120 frames of 1/60 s run 60 steps, one every other
      frame. At `QUARTER`, frames of 16, 17 and 17 ms run exactly 15
      steps a second.
- [ ] At `MAX`, 60 frames of 1/60 s run 240 steps, and frames of 16, 17
      and 17 ms run exactly 240 steps a second. A 10-second frame at
      `MAX` runs 16 steps and drops the rest.
- [ ] `max_steps` is 4 at `STOPPED`, `QUARTER`, `HALF` and `NORMAL`, 8
      at 1.5x and `DOUBLE`, and 16 at `MAX`.
- [ ] At `STOPPED`, frames run no step and keep the part step: 10 ms at
      `NORMAL`, any number of frames at `STOPPED`, then 7 ms at
      `NORMAL` runs one step.
- [ ] For random frame scripts and random speed histories (0 to 4x,
      changing on any frame), the hash list of the live run equals
      `replay` over the `TickRecord` the run made (property test with
      the Harness and 1119's example sim). If 1119's `TickDriver`
      doesn't record its run, the test builds the record from the
      `TickInput`s the driver handed the sim.
- [ ] While the example's stage stays on top, its `TickRecord` has one
      step for each step `App` handed out, and frames at `STOPPED` add
      nothing to it.
- [ ] At `QUARTER`, every press reaches exactly one step. At `STOPPED`,
      a press made then reaches the first step after the speed goes back
      to `NORMAL`, once (Harness).
- [ ] An overlay screen pushed over the example's stage at `DOUBLE`
      runs no simulation step while it is on top (Harness).
- [ ] The dot's position is the same for the 6 steps after it reaches
      the edge, `world_step` doesn't move during them, and a direction
      pressed during the freeze is used when it ends.
- [ ] `harness.steps(10)` runs exactly 10 steps at `HALF`, `NORMAL` and
      `MAX`. At `STOPPED` it panics with a message naming the speed.
- [ ] `FrameInput::dt` and `Ctx::clock_s` are the same at every speed
      (Harness).
- [ ] `GameSpeed` round-trips through RON as a plain number, and `2000`
      reads as `MAX`.
- [ ] If 2316 is done: with `slow` = 4 at `HALF`, 800 frames of 1/60 s
      run 100 steps, give or take one (the part step, and the ticks a
      skipped frame holds for the next update), and every tick `App`
      handed out runs by the next update; while paused at `DOUBLE`,
      frames run no step and one DebugStep runs exactly one; the readout
      shows `speed 2.00x` after `paused`. If 2316 isn't done: its
      *Either order with 2123* clause holds all four points.
- [ ] If 2361 is done: from an empty clock at `QUARTER`, frames of
      1/60 s give an overstep of 250, 500, 750 and 0 (with a step) in
      turn; at `STOPPED`, 30 frames of 1/144 s with
      `BetweenSteps::Clock` give the same overstep and the same snapshot
      of 2361's smoothed scene each time.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `GameSpeed::ratio` rounding, clamping and `den` 0;
  `from_256` and reading clamping to `MAX`; `Display`; `max_steps`
  at each constant and at 1.5x; `TickClock` at each speed, a speed
  change keeping the part step, and `reset`; `Freeze::hold` keeping the
  longer hold, `step` counting down and counting `world_step`;
  `GameSpeed::slowed` if 2316 is done.
- Property: `NORMAL` equals the old `TickClock`; live hashes equal
  `replay` under any speed history; no press is lost or doubled at any
  speed.
- Snapshot / integration: the Harness tests above with 1119's example
  sim; a snapshot of the example's text dump across a hit-stop; the
  2316 and 2361 tests above when those tickets are done.

## Completion notes

*(Filled in by the session that completes the ticket.)*
