---
id: "2128"
title: "A turn planner: a bot that plans its side's whole turn, several commands deep, on copies that never see the luck to come"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-21
---

# 2128 — A turn planner

## Context

A request from the game.

1. **The game ticket:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0506-persona-bots-casual-normal-hardcore.md`).
   It now waits for this ticket (Nick, 2026-10-21). It is not a blocker
   of this one.
2. **The job in one sentence:** a computer player whose side issues
   several commands a turn (each of its pieces acts, cards are played one
   after another) and that weighs short *sequences* of its own commands,
   so a first command that only pays off with a second ("weaken it, then
   finish it"; "step into danger so the others can strike safely") is
   chosen for what the sequence does, not judged alone.
3. **No friction review:** a missing feature, found by measurement. The
   game built its personas on 2125's rollout bot (flat Monte Carlo, one
   command at a time). On its first real battle (a rout of 14 enemies,
   most holding their ground) they played worse than a mindless baseline
   that always charges: in 24 tries the careful persona won 11, the
   strongest won 4 and ran into the 60-turn cap in 20, while the game's
   owner, an intermediate player, clears it with nobody falling. Traced
   tries showed why: every first step of a coordinated move scores as
   "a piece now stands in danger" and is refused, so near the end the
   bot ends its turn without acting, turn after turn. Scoring changes
   (a danger estimate, a reward for closing in) did not cure it, and
   playing the opponent's reply out with its real mind leaked what the
   owner wants hidden (whom the opponent will choose; game ADR-0095).
4. **The generic shape:** a bot over `TurnRules + Sides + Determinize`
   that plans a turn: a bounded search over sequences of its own side's
   commands until the side to move changes (or a depth cap), each
   command applied on several determinized copies (2125's
   `Determinize`), each partial turn scored by the game's `Evaluate`, the
   candidates of each step from the game's `Order`. It returns the first
   command of the best plan, and plans again for the next command. Its
   randomness, its integers and its work units are 2125's.
5. **An example from a game that isn't tactics:** a deck-builder's turn
   (play a card that draws, then the card it drew; spend energy on a
   setup card whose value shows only with the next one); Arimaa or any
   "several steps a turn" board game; a city builder where a road only
   pays off with the house built next to it. Each would write this same
   search over its own commands.

Builds on 2125 (`shuyi_core::bots::{determinize, rollout}`, ADR-0027).
Not 2113: minimax and MCTS search both sides' replies, which a wide map
with many pieces branches too much for (2113 says so itself); this
searches only the side's own turn and leaves the opponent to the
`Evaluate` (a game scores the danger it can see, as a player would).

## Nick input

None.

## Scope

**In:**
- `shuyi_core::bots::plan` (`crates/core/src/bots/plan.rs`): the
  `TurnPlan` bot and `PlanSettings`, re-exported beside `Rollout`.
- The search (below), deterministic and in integers, with the work-unit
  loop of 2125 (`PlanSearch::advance(units)`), so slicing never changes
  the command.
- Tests on toy games in `crates/core/src/bots/fixtures.rs` (a game whose
  side issues several commands a turn, with luck in the state).
- ADR-0027 extended, or a new ADR, for the planner's rules (what a copy
  sees, how luck is drawn, how a plan's value is backed up).
- `bots.rs` docs and the engine's bots docs page, if there is one.

**Out (do not do):**
- Searching the opponent's replies (2113).
- Slicing over frames in a game loop (3009); the work-unit loop is
  enough.
- Anything named after the game: no units, factions, danger zones or
  forecasts. The game's `Evaluate` is where those live.
- The game's use of it (game#0506 moves the pin after this merges).

## Implementation steps

1. **Read** `crates/core/src/bots/rollout.rs`, `determinize.rs`, their
   tests, and ADR-0027. Reuse `Order`, `Evaluate`, `Determinize`,
   `at_least_one`, `pick` (the base-2 softmax) and the seed drawing.
2. **`PlanSettings`** (serde, `Copy`, like `RolloutSettings`):
   - `width`: partial turns kept at each depth (a beam);
   - `candidates`: commands tried from each kept partial turn (the first
     of the game's `Order`);
   - `depth`: the most of its own commands a plan holds (the side's turn
     may end sooner);
   - `samples`: copies each partial turn is played on;
   - `temperature`: the first command's pick, as in `RolloutSettings`.
   0 counts as 1 for every count but `temperature`.
3. **The search** (a beam over the side's own commands, its value backed
   up by the best continuation):
   - At the decision's start draw `samples` seeds from the bot's own
     `SimRng` (as `Rollout::search`) and make one determinized copy of
     the root per seed. Check each copy's legal commands against the
     root's, as 2125 does.
   - A partial turn is a list of commands. Its copies are the root's
     copies with the list applied in order; a command a copy refuses (a
     piece fell there, say) ends that copy's list early, and it is
     scored where it stopped.
   - Its candidates are the game's `Order` of the legal commands of its
     **first** copy, capped at `candidates`. (Copies may differ after
     luck; the first is the one the bot "expects". Write this down in
     the ADR.)
   - Its score is the mean, over its copies, of the game's
     `Evaluate::score(root, copy, side)`.
   - Depth 1 is every candidate of the root. Each later depth extends
     the `width` best partial turns of the one before by each of their
     candidates; a partial turn whose copies all reached the other side's
     move (the turn ended) or `depth` is not extended.
   - A first command's value is the best score among all partial turns
     that start with it (its own included: stopping after it is a plan
     too). The pick is `pick(values, temperature, rng)` over the root's
     candidates, in their order.
   - Work units: one command applied to one copy. A decision does at
     most `samples × depth × (candidates + width × candidates × (depth -
     1))` units; document the bound.
4. **`TurnPlan<O, E>`**: `new(order, eval, settings, seed)`,
   `settings()`, `search(&rules) -> PlanSearch`, and `Bot<R>` (the loop
   `advance(u32::MAX)` until a command, as `Rollout`). It plans again at
   every command; it keeps nothing between decisions but its `SimRng`.
5. **Fixtures**: a toy game where a side issues up to N commands a turn
   and a command's worth shows only with a later one (for example two
   counters that score only when both are raised in the same turn, with
   a costly-looking first raise), with luck in the state (a raise may
   slip, from the state's `SimRng`), and its `Determinize`.
6. **Docs**: module docs in the style of `rollout.rs`; `bots.rs`'s list;
   ADR.

## Acceptance criteria

- [x] On the combo fixture, `TurnPlan` at temperature 0 with `depth ≥ 2`
      picks the first step of the combo, which a `Rollout` with the same
      `Order` and `Evaluate` (its playout ending the turn at once) and the
      same budget does not.
- [x] With `depth` 1, `width` 1, `candidates` N and `samples` S it picks
      what a `Rollout` with `candidates` N, `samples` S, horizon
      `Sides(1)` and a playout that ends the side's turn at once picks,
      for the same seed (a planner one command deep is the rollout bot).
- [x] It never peeks at the luck to come: on two states that differ only
      in their real luck, the same seed picks the same command.
- [x] Same seed and state, same command; slicing `advance` into units of
      1, 7 or `u32::MAX` gives the same command.
- [x] A refused command in one copy ends that copy's plan and is scored
      where it stopped (a fixture where a piece falls in some copies).
- [x] The work done never exceeds the documented bound.
- [x] `PlanSettings` round-trips through RON.
- [x] Docs: module docs, `bots.rs`, the ADR; no game names (the leak
      test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each acceptance criterion; `width`, `candidates`, `depth` and
  `samples` of 0 count as 1.
- Property: on random fixture states, the command picked is one of the
  root's legal commands, and the work bound holds.
- Integration: hello-grid isn't needed (its hero issues one command a
  turn); the fixtures carry it.

## Completion notes

**What was done.**

- `shuyi_core::bots::plan`: `TurnPlan<O, E>`, `PlanSettings` and
  `PlanSearch`, re-exported from `bots` and the crate root beside
  `Rollout`. A beam over the side's own commands, as the steps say:
  shared seeds and root copies, the candidates of each partial turn from
  the order of its first copy still playing, mean scores, and a first
  command worth its best partial turn (itself included). The pick is the
  rollout bot's softmax.
- `rollout.rs` shares its seed drawing, root check, candidate application,
  `at_least_one` and `pick` with `plan.rs` (`pub(super)`); the rollout
  bot's behaviour is unchanged (its tests are untouched and pass).
- A fixture, `Counters`: up to three commands a turn, a combo that pays
  only when both counters are raised in one turn, raises that slip from
  the state's luck, and a slip that knocks the other counter over (the
  refused command). With slips off it is deterministic, for the exact
  unit counts.
- [ADR-0029](../../docs/adr/0029-the-turn-planner-a-beam-over-the-sides-own-commands-copies-kept-and-the-best-continuation-backed-up.md)
  records the planner's rules and extends ADR-0027.
- Tests: every acceptance criterion; zero settings count as one; the
  beam keeps the best (the earlier among equals); each partial turn's
  order chooses its candidates; a won game ends the plan; the root
  checks panic as the rollout bot's do; a planner beats a random bot.
  Property tests: one command deep it is the rollout bot (same command,
  and the same next command); the luck to come never changes the
  command; slicing never does; the command is legal and the work within
  the bound.

**Deviations.**

- The work bound is tighter than the ticket's: each partial turn keeps
  its copies, so extending one applies one command per copy instead of
  replaying the whole list. A decision does at most `samples ×
  candidates × (1 + width × (depth − 1))` units, the ticket's
  `samples × depth × (candidates + width × candidates × (depth − 1))`
  divided by `depth`. The memory cost is in ADR-0029's consequences.
- "Its first copy" is read as its first copy *still playing*: a stopped
  first copy may already be on the other side's move, whose commands
  aren't the side's.
- Only partial turns with a copy still playing take a place in the beam;
  a finished one has nothing to extend, and its score already counts
  toward its first command's value.
- "The same budget" in the first criterion is read as the same
  candidates, samples and most units: the rollout bot there has
  `commands` 4, so its bound (3 × 8 × 5) equals the planner's (8 × 3 ×
  (1 + 2 × 2)).
- There is no automated "leak test" in the repository; the new code and
  docs were checked by hand for game names, and hold none.

**Follow-ups.** None.

**For Nick.** Nothing to decide. The game's personas can now plan a whole
turn (game#0506); the game ticket game#0506 moves the pin to this change
and builds its personas on `TurnPlan`. No API the game uses changed.
