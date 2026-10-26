---
id: "2316"
title: "The debug view in real time: pause, step one frame, slow motion, and a readout of the top screen"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2315"]
nick_input: none
completed:
---

# 2316 — The debug view in real time: pause, step, slow motion, readout

## Context

Nick, 2026-10-19 (`docs/decisions.md`) asked to see layouts, collision
bounds and skeletons "in realtime". 2315 draws them over the running
game. To *read* them, a developer has to stop the game on the frame
where something goes wrong, then move it on one frame at a time. Godot's
debugger and Unreal's "pause and step" both do this.

This ticket adds that to the debug view. It also adds a readout: a few
lines of text the top screen hands in about its own state, drawn in a
corner (a `Note` channel). Examples: "turn 3, side 1, cursor 4,7" for a
grid game, or "on ground, vx 1.5" for a platformer.

Rules it keeps:

- **Determinism** (`AGENTS.md` rule 5). A paused frame runs no update
  of the game's screens. A stepped frame runs one update with the
  normal frame time (1/60 s). So a run paused and stepped through ends
  in the same state, with the same hash, as the same inputs played
  straight. Slow motion runs one update every *n* frames, each with the
  normal frame time; it never stretches `dt`.
- **No key is hard-coded** (rule 8, the `input-actions` skill). Pause
  and Step are new `UiAction`s that a game maps its own actions onto.
  Their keys come from its keymap file, and the engine's sample keymap
  gives them keys like Debug's.

## Nick input

None.

## Scope

**In:**
- **`UiAction::DebugPause` and `UiAction::DebugStep`** in
  `shuyi_assets::keymap::action_set`. A game that doesn't map them
  gets no pause, and nothing else changes. `UiAction` isn't
  `#[non_exhaustive]`, so a game that matches it exhaustively
  (hello-grid's `stage.rs` does) adds arms (2013 left it exhaustive); the
  PR says so for the game's pin bump. Like `Debug`, they act
  only with debug tools on, and the sample `ActionSet`
  (`keymap/sample.rs`, `sample.ron`) marks them not rebindable.
  Map them in the sample and in the toy game. They get their arms in
  `UiAction`'s own `ActionSet` impl (`keymap/action_set.rs`), neither
  required nor rebindable, with `ui` `Some(self)`, and stay out of `ALL`
  (ADR-0036 §3).
- **`DebugView` gains** `paused: bool`, `slow: u8` (1 = normal speed,
  2–8 = one update every *n* frames) and a pending step.
- **`Game::frame`:**
  - DebugPause toggles `paused`;
  - DebugStep, while paused, runs exactly one update of the stack with
    `dt` = 1/60 s;
  - while paused, the stack is drawn and the gizmos are built, but no
    game screen's `update` runs, and the frame's input other than the
    debug actions is dropped, not queued. The debug tools' own screens
    (those the game names in `GameHooks::debug_screens`, such as its
    debug menu and 2315's "Debug view" screen) still update while on
    top, with the frame's input, so they work on a paused game, as
    2356's console will;
  - **either order with 2367** (overlays that let the screen below
    run): if 2367 is done, a Harness test shows the pause stops a
    `Passes::Everything` overlay's timer and the screen under it alike.
    If not, 2367 adds that test;
  - with `slow` above 1, the stack updates on one frame in `slow`. The
    frames between keep their input events, in order, and the next
    update applies them all: a press made in slow motion is never lost
    (pause drops input; slow motion doesn't), and key repeats keep the
    slowed pace.

  Audio requests made by a stepped update play as usual. **Either order
  with 2344:** if 2344 is done, pausing calls `AudioQueue::pause_sounds`
  under the queue's crate-private owner that no pop releases, and
  unpausing calls `resume_sounds`, so looped and other handled sounds
  hold while the game is paused. A step doesn't release them. If 2344
  isn't done, 2344 adds this wiring.

  **Modules (2358, ADR-0040):** `Game::step` runs the modules'
  `before_update` hooks just before the stack's update, and pausing
  holds them as it holds that update. None run while paused. One runs with `dt` = 1/60 s per step, and one on a single
  frame of `slow` in slow motion. Their `draw_over` still draws every
  frame.
- **The "Debug view" screen (2315)** gains a row for slow motion
  (1×, ½×, ¼×, ⅛×, from `slow`) and shows whether the game is paused.
- **The readout:** `Screen::debug_readout(&self, ctx: &Ctx<G>) ->
  Vec<String>`, defaulted to none. It is called on the top screen only,
  when the `readout` channel is on. It stays the top screen's when 2367
  (parked) lets screens below an overlay run. Its lines are drawn as
  `Label` gizmos (role `Note`) in the top-left corner, one per line.
  While paused, a line `paused` (or `paused, step N` after N steps)
  comes first. Write a readout for the toy's grid game (turn, side,
  cursor).
  **Either order with 2104** (animation state machines, parked): if
  2104 is done, add the helper it left out: a function beside its
  runner that a screen's `debug_readout` calls to show an `AnimState`
  as `state name, clip, frame`, with a unit test. If not, nothing to
  do (2104 writes it).
- **A tick game.** If 1119 (tick simulation) is done, a step is one
  fixed simulation step, and the readout names the step number. If
  1119 isn't done, 1119 adds that, and this ticket steps frames.
  **Either order with 2123** (game speed, parked): if 2123 is done, do
  the four points of its *Debug pause and slow motion* part, with its
  tests of the pair. `Game::clock_speed` returns `STOPPED` while
  paused, and otherwise the game speed slowed by `slow`
  (`GameSpeed::slowed`). A frame that slow motion skips keeps its ticks
  for the next update. DebugStep hands the stepped update exactly one
  tick, so it runs one simulation step at any `Ctx::game_speed`, 0
  included. The readout gains a line `speed 0.50x` (from `GameSpeed`'s
  `Display`) after `paused` when the speed isn't normal. If 2123 isn't
  done, 2123 does all four.
  **Either order with 2361** (smoothed world layers, parked): if 2361
  is done, a Harness test on its smoothed scene shows that 30 paused
  frames of 1/144 s with `BetweenSteps::Clock` give the same snapshot,
  and so do the frames between two slow-motion updates. If not, nothing
  to do (2361 tests it under this ticket's pause).
- **Over the live link** (**from 2521 (done)**, ADR-0047): a client
  pauses and steps a running build by sending the game's actions for
  `DebugPause` and `DebugStep` as `act` steps. No operation of its own
  is needed. Add a test that plays the toy grid game over
  `shuyi_platform::link::Server` (as `crates/platform/src/link/tests.rs`
  drives it: `Server::input`, `App::frame_after`, `Server::after_frame`),
  pauses and steps it that way, unpauses, and ends with the same screens
  and frame text as a run of the same updates with no pause.

**Out (do not do):**
- stepping backward or rewinding: 2357 goes back by replaying the
  session record (2518) headless to an earlier frame, and lands paused
  through this ticket's `DebugView::paused`. Going back in a turn
  game's rules stays `shuyi_core::history`'s;
- editing state from the readout (the editor's inspector, 2507);
- pausing the music, the voice or one-shot cues already playing (2344
  holds only sounds started with a handle);
- game speed and hit-stop for tick games (2123, parked with real time).
  This ticket's slow motion stays per frame and works for turn games
  too. It composes with 2123's speed (debug slow times game speed), and
  whichever of the two lands second tests the pair;
- debug commands, a console to type them and a `--command` step (2356).

## Implementation steps

1. Read 2315's ADR and code, `game.rs`, the `input-actions` skill, and
   `keymap/sample.rs`. Check which of 1119, 2123, 2344, 2361 and 2367
   are done (the either-order clauses).
2. Add the two `UiAction`s, and map them in the sample keymap and the
   toy.
3. Add pause, step and slow motion to `Game::frame`, with Harness tests:
   the debug screens updating while paused, and slow motion keeping its
   frames' input.
4. Add `Screen::debug_readout` and the `readout` channel, and the toy
   grid game's readout.
5. Add the slow-motion row and the paused line to the "Debug view"
   screen.
6. Do the either-order parts whose other ticket is done.
7. Amend 2315's ADR with a section on pausing, or write a short new ADR
   that links it.
8. Run the gates.

## Acceptance criteria

- [ ] Harness: the toy's grid game, paused and stepped through a script
      of N frames, ends with the same state hash as the script played
      straight.
- [ ] Harness: while paused, a Confirm press changes nothing, and
      nothing happens on the frame after resuming.
- [ ] Harness: while paused, the Debug action opens the toy's debug
      screen (its gallery) and Cancel closes it, and the paused game's
      state hash is unchanged.
- [ ] Harness: with `slow` = 4, 40 frames run 10 updates.
- [ ] Harness: with `slow` = 4, a Confirm tapped on a frame that runs no
      update reaches the next update, once.
- [ ] Harness: while paused, a module's
      `before_update` doesn't run, and a step runs it once.
- [ ] The readout's lines are in the snapshot, after `paused`, when the
      `readout` channel is on.
- [ ] With debug tools off, DebugPause and DebugStep do nothing.
- [ ] `check-keys` passes; help text names the new actions through the
      keymap.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the slow-motion counter, the pending step being consumed once,
  and a skipped frame's input events applied once at the next update.
- Property: for any script and any pattern of pauses and steps that
  ends with every update run, the final hash equals the straight
  run's.
- Snapshot / integration: the toy grid game paused with the readout
  on; the "Debug view" screen with the slow-motion row; the
  either-order tests above when their tickets are done.

## Completion notes

*(Filled in by the session that completes the ticket.)*
