# ADR-0027: Bots that plan see a determinized copy: integer rollouts, shared seeds and work units

- **Status:** Accepted
- **Date:** 2026-10-21
- **Related tickets:** 2125 (this), 2113 (minimax and MCTS), 2126 (the playtest runner), 2127 (autobalance), 3009 (thinking in slices), game 0506

## Context

- `shuyi_core::bots` has `Sides`, `Bot`, `play` and `RandomBot`. A bot
  that plans plays moves out on copies of the game. Cloning the state
  hands it the luck to come: the rolls wait in the state's `SimRng`.
  `Bot`'s docs forbid that, and nothing enforced it.
- Nick's rule of 2026-10-18 (`docs/decisions.md`): a computer opponent
  sees only what its side sees, unless its difficulty settings say
  otherwise. It thinks in work units, never time
  ([ADR-0008](0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
  §6).
- The core has no floats in a simulation
  ([ADR-0002](0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md)).
  A bot's pick changes what happens next in a replay, so it is
  simulation.
- The game's persona bots (game 0506) need a bot that tries a few good
  moves on redrawn luck and plays each out a little way. A wide tactics
  map branches too much for a tree of every side's replies (2113).

## Decision

1. **A bot that plans sees a copy from `Determinize`, never the state.**
   `shuyi_core::bots::determinize::Determinize: Sides + Clone` has
   `determinize(&self, side, seed) -> Self`: the state as `side` knows
   it, with what it can't see redrawn from `seed` (a new
   `SimRng::new(seed)`, hands dealt again, unseen pieces removed). The
   planning bots (`Rollout` here, 2113's later) take only games that
   implement it. This is how the 2026-10-18 rule is kept: each game
   writes what its side may see, once, and a difficulty that sees more
   says so in that function. A game with nothing hidden returns
   `self.clone()` and says so.
2. **The copy keeps the root's commands.** It lists the same
   `legal_commands` in the same order and accepts them. `Rollout` checks
   the list and the candidate's application on every copy and panics
   with a message naming `determinize`: a bug in the game, found by its
   first test.
3. **Scores are whole points.** `Evaluate<R>::score(start, end, side) ->
   i32`, higher is better. A closure is one.
4. **The rollout bot is flat Monte Carlo** (`bots::rollout`).
   - Candidates: the game's `Order` of `legal_commands`, capped at
     `candidates`.
   - Each candidate is played on `samples` copies by the game's
     `Playout` player (every `Bot` is one) up to a `Horizon` (`Sides` or
     `Turns`), or `commands` commands, or the end.
   - Means are `i64` sums `div_euclid` the samples.
   - Its settings, `RolloutSettings`, are serde data, so a persona's are
     a file.
5. **The softmax is base 2, the temperature in points.** At temperature
   `t`, a candidate `t` points below the best is half as likely: weight
   `exp2_q16(-(best - mean) * 65536 / t)`, in 16.16 fixed point. The whole
   part is a shift and the fraction a 16-entry table read straight
   between entries. One `roll_below` draw over the sum of the weights
   picks. Temperature 0 takes the best mean, the earlier among equals.
6. **One set of seeds per decision, shared by every candidate.** Each
   candidate is judged on the same redrawn luck, so the comparison is
   fairer at the same cost. A seed is two `next_u32` draws, high half
   first.
7. **Work units, and draws only at a decision's ends.** A unit is one
   command applied to a copy, the candidate's own included. A decision
   does at most `candidates × samples × (commands + 1)`. Its loop state
   is a value, `RolloutSearch`, with `advance(units) -> Option<Command>`;
   `choose` advances until it is done. The bot's `SimRng` is drawn when
   the search starts (the seeds) and when it ends (the pick), so how the
   work is sliced never changes the command. This extends ADR-0008 §6:
   3009's `Think` will be implemented for `RolloutSearch` there.
8. **Built on by later work.** 2113's minimax and MCTS search copies from
   `Determinize` and score with `Evaluate`. 2126's playtest runner plays
   personas made of `Rollout` bots. Determinize is where a game's types
   meet "what a side sees"; when 2907 describes a game's views, a view
   is what `determinize` keeps.

## Consequences

- A game opts in by writing `determinize`, an order, a playout player
  and a score; a game that never names the rollout bot pays nothing
  (ADR-0009 §1).
- A bot that clones the state itself is still possible for a `Bot` the
  game writes by hand; only the engine's planning bots are bound. The
  rule is kept where the engine can keep it.
- The check of the root's commands costs one `legal_commands` call per
  copy. It is not counted as a unit; a game whose list is costly pays
  it.
- `RolloutSearch` borrows its bot for the decision. A frame-sliced
  decision (3009) keeps the bot and the search together.
- `Bot` gains a blanket impl for `FnMut(&R) -> R::Command`, so a
  function of the state is a bot and a playout player. A type of a game
  that is both a closure and a hand-written `Bot` would now clash; none
  does.

## Alternatives considered

- **Let bots clone the state and trust them.** It was the rule before,
  in a comment, and nothing enforced it. A game's planning bot would
  see the rolls to come by accident.
- **A fresh set of seeds per candidate.** Same cost, noisier: two
  candidates differ by their luck as much as by their merit.
- **A float softmax (`e^(x/t)`).** Floats are out of the simulation
  (ADR-0002), and a base-2 integer softmax gives a temperature that
  reads plainly: `t` points below is half as likely.
- **Draw each playout's seed when it starts.** Same numbers, but a
  draw in the middle ties the bot's `SimRng` to how far the work got;
  drawing all at the start keeps slicing out of it with no care.
- **A time limit.** Time changes from run to run and machine to machine
  (ADR-0008 §6); work units don't.
