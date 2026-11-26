---
id: "2316"
title: "The debug view in real time: pause, step one frame, slow motion, and a readout of the top screen"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2315"]
nick_input: none
completed: 2026-11-26
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
  gets no pause, and nothing else changes. **From 2083 (done):**
  `UiAction` is `#[non_exhaustive]`, so a game's `match` on it (hello-grid's
  `stage.rs`) already has a `_` arm, and the new variants are additive.
  Like `Debug`, they act
  only with debug tools on, and the sample `ActionSet`
  (`keymap/sample.rs`, `sample.ron`) marks them not rebindable.
  Map them in the sample and in the toy game. They get their arms in
  `UiAction`'s own `ActionSet` impl (`keymap/action_set.rs`), neither
  required nor rebindable, with `ui` `Some(self)`, and stay out of `ALL`
  (ADR-0036 §3).
- **Either order with 2097:** if 2097 is done, `cargo xtask
  omitted-patterns` fails each engine match on `UiAction` outside
  `shuyi-assets` that doesn't name the two new variants: name them in
  each, beside `Debug`, doing what `Debug` does there; if not, nothing
  to do (2097 names them).
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

  Audio requests made by a stepped update play as usual. **From 2394
  (done):** pausing calls `AudioQueue::pause_sounds` under the queue's
  crate-private owner that no pop releases (`OUTSIDE_THE_STACK` in
  `crates/ui/src/audio/handles.rs`: `set_owner(OUTSIDE_THE_STACK)`, then
  `pause_sounds`, then the stack's `own_top` again), and unpausing calls
  `resume_sounds` the same way, so looped and other handled sounds hold
  while the game is paused. A step doesn't release them.

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
  **Either order with 2375** (the `log` channel): its log lines are
  `Label` gizmos from the top-left corner too. The readout keeps rows
  0 to n - 1 (n its lines, `paused` included), and the log's lines
  start on row n. If 2375 is done, `log_gizmos` draws from row 0: give
  it the first row (`first_row: usize`), have `Game` pass the
  readout's line count (0 with the `readout` channel off), and add a
  test with both channels on that finds no two labels on one row; if
  not, nothing to do (2375 starts its lines below the readout).
- **A tick game. From 2137 (done):** a step is one fixed simulation
  step (a frame with `FrameInput::ticks()` of 1, which a stage's
  `TickDriver` steps once), and the readout names the step number
  (`TickDriver::steps`).
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
- pausing the music, the voice or one-shot cues already playing (2394
  holds only sounds started with a handle);
- game speed and hit-stop for tick games (2123, parked with real time).
  This ticket's slow motion stays per frame and works for turn games
  too. It composes with 2123's speed (debug slow times game speed), and
  whichever of the two lands second tests the pair;
- debug commands, a console to type them and a `--command` step (2356).

## Implementation steps

1. Read 2315's ADR and code, `game.rs`, the `input-actions` skill, and
   `keymap/sample.rs`, and 2137's `TickDriver`. Check which of 2123,
   2394, 2361 and 2367 are done (the either-order clauses).
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

- [x] Harness: the toy's grid game, paused and stepped through a script
      of N frames, ends with the same state hash as the script played
      straight.
- [x] Harness: while paused, a Confirm press changes nothing, and
      nothing happens on the frame after resuming.
- [x] Harness: while paused, the Debug action opens the toy's debug
      screen (its gallery) and Cancel closes it, and the paused game's
      state hash is unchanged.
- [x] Harness: with `slow` = 4, 40 frames run 10 updates.
- [x] Harness: with `slow` = 4, a Confirm tapped on a frame that runs no
      update reaches the next update, once.
- [x] Harness: while paused, a module's
      `before_update` doesn't run, and a step runs it once.
- [x] The readout's lines are in the snapshot, after `paused`, when the
      `readout` channel is on.
- [x] With debug tools off, DebugPause and DebugStep do nothing.
- [x] `check-keys` passes; help text names the new actions through the
      keymap.
- [x] All gates in the `run-gates` skill pass.

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

Done as scoped, with ADR-0153 (extends ADR-0085; its status line and
the index name it).

- **Actions:** `UiAction::DebugPause` and `DebugStep`, with
  `UiAction::is_debug` for the three debug actions; neither required
  nor rebindable, out of `ALL`. The sample `Action` (and so the toy,
  which plays the sample's actions) gains `DebugPause` (`F3`) and
  `DebugStep` (`F4`), appended, keyboard only, not rebindable.
- **`DebugView`** gains `paused`, a pending step (`step`, `take_step`,
  `steps`), `slow` (`set_slow`, 1 to `SLOWEST` = 8, and
  `slow_frame_runs`) and `pause_line`; none of it is saved.
- **`Game`** paces each frame (`Pace`: run, held, step, skip). The two
  keys act when pressed, read from the frame's key presses through the
  keymap, before the input state counts the frame's time: that is what
  keeps a stepped run's hash equal to the straight run's (ADR-0153
  *Alternatives*). The actions never reach a screen. A debug tools'
  screen on top runs as usual on a paused game, while the clock and the
  modules stand. `Game::act` takes the two actions too. `STEP_DT` is
  public. The pause holds handled sounds under `OUTSIDE_THE_STACK`
  (re-exported crate-wide from `audio`), and their fades take only the
  time the game played.
- **The readout:** `Screen::debug_readout`, `ScreenStack::debug_readout`
  and the `readout` channel (`debug::gizmo::READOUT`), which the game
  offers itself every debug frame, drawn by `Game::readout`. The toy's
  `Field` and `DotStage` write one.
- **"Debug view" screen:** a slow-motion row after the channels
  (`ViewRow`), a status line under the title with the bound pause and
  step keys (the help text naming them through the keymap), and the
  empty-channels line under the menu.
- **Tests:** unit tests of the counter, the step taken once and the
  speeds; `tests/it/debug_pause.rs` (every Harness criterion, a
  proptest of scripts paused and stepped through, the tick stage, the
  sounds, `act`, and a snapshot of the paused field with the readout);
  the live-link test in `crates/platform/src/link/tests.rs`; the debug
  view's snapshots, updated.
- **Either-order parts:** 2097, 2104, 2123, 2361, 2367 and 2375 aren't
  done, so this PR did none of them. Their clauses now read as what
  holds after 2316, naming the code they use (2097, 2104, 2123, 2356,
  2361, 2367, 2375).

Deviations:

- The toy's grid field has no turn or side, so its readout is
  `cursor 2,3`, `piece 2,3` (with ` picked up` or ` walking`) and
  `clock N ms`.
- The pause keys pressed in slow motion's skipped frames act at once,
  not at its next update, and a pause drops what slow motion kept.
- A frame slow motion skips drops its fixed steps (otherwise a tick
  game would not slow down); 2123's clause keeps them once game speed
  lands.
- The sample keymap's golden value (`crates/assets/tests/golden/formats.ron`,
  `keymap: keymap`) and its pack snapshot moved, because the fixture
  gained two actions; no format and no encoding changed, so the pack's
  `VERSION` stays.
- Not mapped in `examples/hello-grid` or the template: the ticket names
  the sample and the toy only. No ticket: a game maps them when it
  wants a debug pause, as the book's new section says.

Size: 1,420 counted lines, under the cap. It would have split cleanly
between the pause (actions, `Game`, the screen's row) and the readout
(`Screen::debug_readout`, the channel, the toy's readouts).

Breaking: shuyi-ui: `DebugViewScreen::menu`: returns `&Menu<ViewRow>` (a channel's row is `ViewRow::Channel(id)`) instead of `&Menu<&'static str>`.

```yaml
crate: shuyi-ui
item: DebugViewScreen::menu
before: "screen.menu().items()[i].value  // &'static str, a channel id"
after: "screen.menu().items()[i].value  // ViewRow::Channel(id) or ViewRow::SlowMotion"
why: The screen's menu gained a slow-motion row.
```

Breaking: shuyi-assets: `keymap::sample::Action` (feature `sample-actions`, tests only): gains `DebugPause` and `DebugStep`, `ALL` has 21; the sample keymap's golden value moved with them.

```yaml
crate: shuyi-assets
item: keymap::sample::Action
before: "match action { …, Action::DialogueLog => … }"
after: "match action { …, Action::DialogueLog => …, Action::DebugPause | Action::DebugStep => … }"
why: The sample keymap maps the debug pause and step.
```

Behaviour: shuyi-ui: with debug tools on, the game offers the `readout` channel every frame, after the screens' channels: `DebugView::offered` and the "Debug view" screen list it.
Behaviour: shuyi-ui: the "Debug view" screen shows a status line under its title, a "Slow motion" row after the channels, the no-channel line under the menu, and its help says `change` where it said `on/off`.
Behaviour: shuyi-cli, shuyi-platform: `frame-png --debug all` and a script's `debug("all")` draw the readout too: the top screen's lines in the top-left corner, over the frame.
Behaviour: shuyi-ui: an action a game maps to `UiAction::DebugPause` or `DebugStep` never reaches a screen, with debug tools on or off.

Book: site/content/learn/book/frame-and-skins.md gains *Stopping on the frame that goes wrong*.

Nothing in the game must change: `gh search code --repo
Zafnok/visions-of-shuyi` for `DebugViewScreen`, `debug_readout`,
`OUTSIDE_THE_STACK` and `keymap::sample` finds no code, and for
`UiAction::Debug` only `crates/content/src/keymap.rs`, which maps its
actions onto `UiAction` and matches on none. A game without the two
actions gets no pause.
