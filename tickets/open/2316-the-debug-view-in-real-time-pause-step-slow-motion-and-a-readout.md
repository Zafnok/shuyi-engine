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

- **Determinism** (`AGENTS.md` rule 5). A paused frame runs no update.
  A stepped frame runs one update with the normal frame time (1/60 s).
  So a run paused and stepped through ends in the same state, with the
  same hash, as the same inputs played straight. Slow motion runs one
  update every *n* frames, each with the normal frame time; it never
  stretches `dt`.
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
  gets no pause, and nothing else changes. `ActionSet::ui` returns an
  `Option`, so the new variants break no game. Like `Debug`, they act
  only with debug tools on, and the sample `ActionSet`
  (`keymap/sample.rs`, `sample.ron`) marks them not rebindable.
  Map them in the sample and in the toy game.
- **`DebugView` gains** `paused: bool`, `slow: u8` (1 = normal speed,
  2–8 = one update every *n* frames) and a pending step.
- **`Game::frame`:**
  - DebugPause toggles `paused`;
  - DebugStep, while paused, runs exactly one update of the stack with
    `dt` = 1/60 s;
  - while paused, the stack is drawn and the gizmos are built, but no
    screen's `update` runs, and the frame's input other than the debug
    actions is dropped, not queued;
  - with `slow` above 1, the stack updates on one frame in `slow`.

  Audio requests made by a stepped update play as usual.
- **The "Debug view" screen (2315)** gains a row for slow motion
  (1×, ½×, ¼×, ⅛×, from `slow`) and shows whether the game is paused.
- **The readout:** `Screen::debug_readout(&self, ctx: &Ctx<G>) ->
  Vec<String>`, defaulted to none. It is called on the top screen only,
  when the `readout` channel is on. Its lines are drawn as `Label`
  gizmos (role `Note`) in the top-left corner, one per line. While
  paused, a line `paused` (or `paused, step N` after N steps) comes
  first. Write a readout for the toy's grid game (turn, side, cursor).
- **A tick game.** If 1119 (tick simulation) is done, a step is one
  fixed simulation step, and the readout names the step number. If
  1119 isn't done, 1119 adds that, and this ticket steps frames.

**Out (do not do):**
- stepping backward or rewinding. The game's history (`shuyi_core::
  history`) and replays already cover going back; an editor may use
  them (2506);
- editing state from the readout (the editor's inspector, 2507);
- pausing music or sounds already playing.

## Implementation steps

1. Read 2315's ADR and code, `game.rs`, the `input-actions` skill, and
   `keymap/sample.rs`.
2. Add the two `UiAction`s, and map them in the sample keymap and the
   toy.
3. Add pause, step and slow motion to `Game::frame`, with Harness tests.
4. Add `Screen::debug_readout` and the `readout` channel, and the toy
   grid game's readout.
5. Add the slow-motion row and the paused line to the "Debug view"
   screen.
6. Amend 2315's ADR with a section on pausing, or write a short new ADR
   that links it.
7. Run the gates.

## Acceptance criteria

- [ ] Harness: the toy's grid game, paused and stepped through a script
      of N frames, ends with the same state hash as the script played
      straight.
- [ ] Harness: while paused, a Confirm press changes nothing, and
      nothing happens on the frame after resuming.
- [ ] Harness: with `slow` = 4, 40 frames run 10 updates.
- [ ] The readout's lines are in the snapshot, after `paused`, when the
      `readout` channel is on.
- [ ] With debug tools off, DebugPause and DebugStep do nothing.
- [ ] `check-keys` passes; help text names the new actions through the
      keymap.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the slow-motion counter, and the pending step being consumed
  once.
- Property: for any script and any pattern of pauses and steps that
  ends with every update run, the final hash equals the straight
  run's.
- Snapshot / integration: the toy grid game paused with the readout
  on; the "Debug view" screen with the slow-motion row.

## Completion notes

*(Filled in by the session that completes the ticket.)*
