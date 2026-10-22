# ADR-0029: The turn planner: a beam over the side's own commands, copies kept, and the best continuation backed up

- **Status:** Accepted
- **Date:** 2026-10-21
- **Related tickets:** 2128 (this), 2125 (the rollout bot), 2113 (minimax and MCTS), 3009 (thinking in slices), game 0506

## Context

- [ADR-0027](0027-bots-that-plan-see-a-determinized-copy-integer-rollouts-shared-seeds-and-work-units.md)
  gave the engine `Determinize`, `Evaluate`, `Order` and the rollout bot:
  flat Monte Carlo that judges its next command alone, played out by a
  simple player.
- In a game whose side issues several commands a turn (each piece acts,
  cards are played one after another), a first command often pays off
  only with a second: weaken it, then finish it; a card that draws, then
  the card it drew. Judged alone, that first command looks bad.
- The game's persona bots (game 0506) were built on the rollout bot. On
  their first real battle they played worse than a bot that always
  charges: every first step of a coordinated move scored as "a piece now
  stands in danger" and was refused. Score changes didn't cure it, and
  playing the opponent's reply with its real mind leaked what the game
  hides (whom the opponent will choose).
- Minimax and MCTS (2113) search every side's replies, which a wide map
  with many pieces branches too much for.

## Decision

1. **`shuyi_core::bots::plan::TurnPlan<O, E>`** plans its side's own
   turn and leaves the other sides to the game's `Evaluate` (the danger
   it can see, as a player would). It is built from an `Order`, an
   `Evaluate`, `PlanSettings` and a seed, implements `Bot`, and
   re-exports beside `Rollout`. It returns the first command of the best
   plan and plans again at every command; it keeps nothing between
   decisions but its `SimRng`.
2. **`PlanSettings`** is serde data, `Copy`: `width` (partial turns kept
   at each depth), `candidates` (commands tried from each), `depth` (the
   most commands a plan holds), `samples` (copies each partial turn is
   played on) and `temperature` (as `RolloutSettings`). A count of 0
   counts as 1.
3. **What a copy sees.** At a decision's start, `samples` seeds are
   drawn from the bot's `SimRng` exactly as the rollout bot draws them,
   and one copy of the root is made per seed with `determinize`, checked
   to list the root's legal commands (ADR-0027 §2). A partial turn is a
   list of the side's commands; its copies are the root's copies with the
   list applied in order. The same copies serve every partial turn, so
   every plan is judged on the same luck.
4. **A copy stops** when it refuses a command (a piece fell there, say),
   when the side to move changes (the turn ended), or when the game is
   over. A stopped copy is scored where it stopped and takes no more
   commands; at depth 1 a refusal is a bug in `determinize` and panics,
   as in the rollout bot.
5. **Candidates.** The root's are the game's `Order` of the root's legal
   commands, capped at `candidates`. A deeper partial turn's are the
   `Order` of the legal commands of its **first copy still playing**,
   capped the same way. Copies may differ after luck; the first is the
   one the bot "expects", and a command another copy refuses stops that
   copy (§4). The order is asked about copies, never about the real
   state past the root.
6. **The beam.** A partial turn's score is the mean, over its copies, of
   `Evaluate::score(root, copy, side)`: an `i64` sum `div_euclid` the
   samples. Depth 1 is every root candidate. Each later depth extends
   the `width` best partial turns of the one before (the earlier among
   equals) by each of their candidates. Only partial turns with a copy
   still playing, and shorter than `depth`, are kept to be extended.
7. **Backing up.** A first command's value is the best score among all
   partial turns that start with it, its own included: stopping after it
   is a plan too. The pick is ADR-0027 §5's softmax over the root's
   candidates, in their order. One command deep (`depth` 1, `width` 1)
   the planner is the rollout bot whose playout ends the side's turn at
   once, at horizon `Sides(1)`: the same seeds, means and pick, for the
   same seed. A test holds them equal.
8. **Copies are kept, not replayed.** Each partial turn keeps its
   copies, and an extension clones them and applies one command to each
   copy still playing. A unit is one command applied to one copy (ADR-0027
   §7), so a decision does at most `samples × candidates × (1 + width ×
   (depth − 1))` units. Memory holds at most `width × candidates ×
   samples` states, plus the parents' copies.
9. **Work units and slicing as ADR-0027 §7.** The loop state is a value,
   `PlanSearch`, with `advance(units) -> Option<Command>`. The bot's
   `SimRng` is drawn at the search's start (the seeds) and its end (the
   pick), and the game's order and score are called in the same sequence
   however the work is sliced, so slicing never changes the command.
   Work that applies no command (scoring, cloning a stopped copy,
   choosing the beam) is not a unit and runs even when the units are
   spent. Each step of the loop returns the search's next phase, and the
   work left is held in stacks the loop takes from, never in counts: a
   step that did nothing, or a count that stopped rising, could otherwise
   spin the search forever without spending a unit (a mutant that only a
   timeout would catch). `RolloutSearch` holds its playouts to start the
   same way. Both searches' loops return a `Progress` (working, or the
   command chosen), which has no default, and `choose` runs that loop
   once on `u64::MAX` units instead of calling `advance` until it
   answers; a search that stopped moving fails at once.

## Consequences

- A game that already has a rollout bot gets a turn planner from the
  same `Order` and `Evaluate`, without a playout player.
- The planner never sees the other sides' replies. A game that wants
  its bot to fear a counter-attack says so in `Evaluate`, from what its
  side can see. That keeps the opponent's real mind (and what it would
  choose) out of the copies.
- Choosing candidates from the first copy is a bias: when that copy's
  luck went another way than most, the plan is extended by commands
  that suit it. More samples don't remove it; it is the price of one
  order call per partial turn.
- Keeping copies trades memory for work: a game with a large state pays
  `width × candidates × samples` clones at each depth.
- `rollout.rs`'s seed drawing, root check, candidate application,
  `at_least_one` and `pick` are shared with `plan.rs` (`pub(super)`), so
  the two bots can't drift apart.

## Alternatives considered

- **Replay each partial turn from the root copies.** Less memory, but a
  partial turn of `d` commands costs `d` units per copy: the ticket's
  first bound, `samples × depth × (candidates + width × candidates ×
  (depth − 1))`, is that. Keeping copies costs a clone instead, and the
  bound drops by the factor `depth`.
- **Candidates from every copy (the union, or the commands all copies
  accept).** Order would be asked once per copy, and the union could
  hold commands most copies refuse. The first copy is simpler and the
  refusals of the others are already handled (§4).
- **Back up the mean of the continuations, or only the deepest one.**
  The side chooses its next command after seeing the luck, so the best
  continuation is what it can reach; a mean would punish a first command
  for its bad follow-ups. Counting the partial turn itself keeps a
  command that is good alone from losing to its own extensions.
- **Search the other sides' replies too (2113).** It branches too much
  on a wide map, and playing the opponent with its real mind leaked what
  the game hides.
- **Full tree search of the side's turn.** It grows as `candidates ^
  depth`; the beam keeps it linear in `depth`.
