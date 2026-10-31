---
id: "3009"
title: "Bots think in slices: spread a bot's search over frames by work units, so the frame rate holds and the decision is the same on every device"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: todo
blocked_by: ["game#1131", "3008", "2112"]
nick_input: none
completed:
---

# 3009 — Bots think in slices

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6. A bot decides in one call today (`Bot::choose`,
`crates/core/src/bots.rs`). The carve brought only `Bot`, `play` and
`RandomBot` into the engine (`docs/carve.md`, the game's 1116); the
game's enemy AI stayed in the game. The engine's first search is 2112's
level solver, and 2113's minimax and MCTS bots come after this ticket.
On a desktop one call is fine. On a 168 MHz Playdate, a turn with many
enemies could freeze the screen for seconds. The game's 1025 names "the
enemy AI's thinking time" as the first thing to measure.

Two rules hold together:

- **The frame rate holds.** The Playdate budget is 30 fps while a bot
  thinks.
- **The decision never depends on the device.** A bot that stopped
  "when time ran out" would choose differently on a slow device, and
  replays, lockstep and bug reports would break (ADR-0002 §2). So
  slicing counts work units, never time. A slow device reaches the same
  decision, only later.

**Its first real user is the game's enemy AI on the Playdate**
(game#1051, `github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1051-enemy-ai-plans-once-per-phase.md`;
noted 2026-10-21). On Nick's Playdate a 14-enemy phase thinks for
1.2 s in all (game#1026). game#1051 plans the phase once in the game
and doesn't wait for this ticket; when this lands, the game's PR that
moves the pin implements `Think` for its enemy AI (as Scope says). The
game's playtest bots don't need slicing: they run on a PC, on 2128's
turn planner (ADR-0029; they began on 2125's rollout bot). Both keep
their loop in a value (`RolloutSearch`, `PlanSearch`), so `impl Think`
for each is a few lines here.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::think`:**
  - `trait Think { type Decision; fn step(&mut self, units: u32) ->
    Progress<Self::Decision> }`, where `Progress` is `Working` or
    `Done(decision)`;
  - a "unit" is whatever the bot counts: a node visited or a move scored.
    The bot defines it, and it never depends on time.
  - `crates/core/src/bots/rollout.rs` already has a `pub(super) enum
    Progress<C> { Working, Chosen(C) }`, which the rollout bot and the
    turn planner step with. `think::Progress` replaces it: one type, not
    two of the same name.
  - **Either order with 2906** (asset memory research): if 2906 is
    done, read its ADR's point 4. If a load steps with `Think` or
    `Progress`, keep both fit for it (a unit could be a file decoded)
    and add what the ADR asks of them. If not, nothing to do: 2906
    counts units as this ticket plans.
- **The engine's search implements `Think`.** 2112's
  `shuyi_core::search::Solver` already runs in `step(units)` slices,
  with its state in the value: add `impl Think for Solver`, where a unit
  is one state expanded. Its total work is fixed by its `Limits`, never
  by how many slices it gets. A test proves that `step(1)` repeated and
  `step(u32::MAX)` give the same answer on every fixture. 2113's minimax
  and MCTS bots implement `Think` when they land (2113 waits for this
  ticket). The game's own enemy AI implements it in the game, in the PR
  that moves the pin. 2125's bot is done: add `impl Think for
  RolloutSearch<'_, R, O, P, E>` (`Decision = R::Command`; `step(units)`
  is its `advance(units)`) and a slicing test on its fixtures. `Rollout`
  itself has no `advance`: `Rollout::search(&rules)` starts a decision,
  and the search borrows its bot, so a decision sliced over frames keeps
  the bot and the search together (ADR-0027 §7 and Consequences).
  2128's planner is done too: the same for `PlanSearch<'_, R, O, E>`,
  started by `TurnPlan::search(&rules)` in
  `crates/core/src/bots/plan.rs`, with a slicing test on its fixtures.
- **A frame-time budget in `App`.** `shuyi-platform`'s `App` gives a
  thinking bot `units_per_frame` from the target profile (a new
  `budgets.think_units_per_frame`; calibrated by 3011 and 3013). The
  screen keeps animating meanwhile: the cursor blinks, and a "thinking"
  sign appears after 0.5 s. **Either order with 3013** (the Steam Deck
  measured): if 3013 is done, set `steam-deck`'s value from its report
  in `docs/targets/steam-deck.md`, the bench scenarios' frame times on
  the Deck against their x86 instruction counts (3008), applied to
  16.7 ms as 3011 does for `playdate`; if not, 3013 sets it.
  **Either order with 3022 and 3026 (parked):** if one is done, set
  `think_units_per_frame` in its profiles (3022's `gba`; 3026's `nds`,
  and `dsi` if measured) as it set `frame_cycles`: the search scenario
  below, counted in the emulator, scaled by the worst device ratio in
  its report (`docs/targets/gba.md`, `docs/targets/nds.md`), with 20 %
  headroom; if not, nothing to do (each sets it from its own numbers).
- **Headless and replays.** The Harness runs bots to completion in one
  call (`step(u32::MAX)`), so tests stay fast and records don't change.
- **A bench scenario** (3008): a search with the most units (2112's
  `Solver` on the grid template's stage, or a fixture as big), measured
  per frame against the `playdate` budget.

**Out (do not do):**
- threads (the Playdate has one core; the web has none we use);
- making a search or a bot smarter or cheaper (2113, and game tickets
  for the game's enemy AI);
- time-based limits of any kind;
- the real-time strategy AI (parked 2710, 2714, 2715, ADR-0015 §8). It
  will use `Think` for decisions that recur every few steps, so keep
  `Think` free of anything that assumes one decision per turn.

## Implementation steps

1. Add `Think` and `Progress` with docs and an example.
2. Implement `Think` for 2112's `Solver`. Its state already lives in
   the value, not on the call stack.
3. Add the `App` budget and the profile field (3001's format gets a
   version bump with a default).
4. Add the equivalence property test and the bench scenario.

## Acceptance criteria

- [ ] On every solver fixture (2112), slicing by 1, 7, 100 and unlimited
      units gives the same decision (property test with random slice
      sizes).
- [ ] The rollout bot (2125) and the turn planner (2128) give the same
      decision sliced by 1 unit as unsliced, on their fixtures.
- [ ] Replays recorded before the change replay the same.
- [ ] The search bench scenario stays under the `playdate`
      instruction budget per frame.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: slice-size independence.
- Integration: replay fixtures; the bench scenario.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
