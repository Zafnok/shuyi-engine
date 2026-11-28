---
id: "2794"
title: "Parked: Idle games: a deterministic large-number type, the time away handed to the rules as one command, and an incremental example with a prestige reset"
type: feature
milestone: "Engine: examples"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2794 — Idle games

> **Parked** (Nick, 2026-10-20, choosing a gallery of small games as a kind of game the engine serves; Visions of Shuyi is "the first game I want to be launched with the engine"): like real-time strategy, visual novels and detective games, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Idle, incremental and clicker games count past `i64` (1e300 and more),
compound generators, reset with a prestige, and give the player what
was made while the game was closed. The Bevy coverage audit of
2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found no ticket
for them. Bevy has no idle example; its real clock and low-power loop
make one (`crates/bevy_time/src/real.rs`, `examples/window/low_power.rs`).

Two rules shape it here: the core reads no clock (ADR-0002), and
numbers in a simulation are integers. So the time away is read by the
backend and handed in as a command, and the big number is an integer
mantissa and exponent.

## Nick input

None.

## Scope

**In:**
- **`BigNum`** in `shuyi-core`: an integer mantissa and an integer
  exponent, normalised, with add, subtract, multiply, divide, compare,
  integer powers and serde; no floats. Formatting (short scales and
  scientific) is the text layer's, through the language pack.
- **Time away:** when a save loads, the backend computes the whole
  seconds since it was written and the game gets one command,
  `AwayFor(seconds)`, capped by game data. A replay with the same
  command gives the same hash; the core never reads the clock.
- **An example** in the gallery on the turn path (one command per
  second of play from 2115's timers): generators, upgrades, a prestige
  reset kept in the save, painted by a skin; a bot playtest reports the
  time to each milestone.
- `BigNum` follows ADR-0039.

**Out (do not do):**
- cloud saves and checks against clock cheating (a save's clock is the
  player's own);
- notifications while the game is closed;
- an idle template part.

**Estimated size:** about 1,200 lines (code 550, tests 450, docs 200).

**Split:** past 1,000 lines, split off the example into a new ticket.

## Implementation steps

1. Read `crates/core/src/save.rs`, 2115's timers and the language pack's
   number formatting.
2. Write `BigNum` with its tests.
3. Add the time-away command at load in the backend.
4. Write the example and its bot.
5. Write the docs page.

## Acceptance criteria

- [ ] `BigNum` arithmetic agrees with a big-integer reference within its
  mantissa's precision (property test), and equals on wasm32.
- [ ] A save loaded 3,600 seconds later gives `AwayFor(3600)`, capped
  at the game's limit.
- [ ] A record of the example replays to the same hash.
- [ ] 1.5e303 formats as the language pack says in English.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each operation's edge cases (zero, negatives, overflow of the
  exponent).
- Property: against a reference; normalisation.
- Snapshot / integration: the example's replay hash and playtest.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
