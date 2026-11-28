---
id: "2793"
title: "Parked: Fighting games: moves and frame data as data, an input buffer that reads motion inputs, and a two-seat versus example with rollback-ready state"
type: feature
milestone: "Engine: action RPGs"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2756", "2123", "2406"]
nick_input: none
completed:
---

# 2793 — A fighting-game kit

> **Parked** (Nick, 2026-10-22: "I also want support for roguelikes, and Diablo-likes"): like the other kinds of game, and the real-time work it builds on (2756, 2123, 2406), this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

`shuyi-action` (2756, parked) gives real-time hits and hurts with
hitboxes and hurtboxes, and names beat-'em-ups among its users. The
Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found that one-on-one fighting games need more that no
ticket plans: each move's startup, active and recovery frames and its
boxes per frame, cancels between moves, and an input buffer that
recognises motions (a quarter circle, a charge, a double tap) by
priority. Bevy has no fighting example, but its fixed timestep and
input crates make one (`crates/bevy_time/src/fixed.rs`,
`examples/input/gamepad_input.rs`, `keyboard_input_events.rs`).

Shuyi's tick sims are deterministic, so a fighting game's state is
ready for rollback netplay (1147, parked) by construction.

## Nick input

None.

## Scope

**In:**
- **Moves as data:** a `.moves.ron` of moves per fighter: frames per
  phase, hitboxes and hurtboxes per frame (2756's boxes), damage, hit
  stun, block stun, pushback, and the moves it may cancel into; checked
  by `shuyi check` with the file and line.
- **An input buffer** over `TickInput`'s per-seat history: motion
  patterns on actions (never keys; directions relative to facing), a
  window in steps per pattern, priority when two match.
- **The fight step:** startup, active and recovery; cancels; hit-stop
  from 2123; pushback; all in integers, deterministic.
- **A versus example** in the gallery: two fighters on one stage,
  against a bot and two players on 2406's seats, with replay hashes as
  tests.

**Out (do not do):**
- rollback netplay itself (1147, parked);
- a training mode's frame-data display (a follow-up);
- 3D fighters.

**Estimated size:** about 1,500 lines (code 700, tests 550, docs 250).

**Split:** past 1,000 lines, split off the versus example into a new
ticket.

## Implementation steps

1. Read 2756's module, 2123's hit-stop, 2406's seats and `TickInput`.
2. Write the move format and its checks.
3. Write the input buffer and its pattern matcher.
4. Write the fight step.
5. Write the example and its bot.
6. Write the docs page.

## Acceptance criteria

- [ ] Down, down-forward, forward then punch within the window gives
  the quarter-circle move, facing either way.
- [ ] A move cancelled into another on its listed frame starts the
  second at once; on an unlisted frame it doesn't.
- [ ] A hit on frame 3 of a 3-frame startup move's active phase hits;
  one frame earlier misses.
- [ ] A recorded versus match replays to the same hash.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each pattern, phases, cancels.
- Property: any input sequence gives the same state on native and
  wasm32.
- Snapshot / integration: a replay hash; frame-png of the example.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
