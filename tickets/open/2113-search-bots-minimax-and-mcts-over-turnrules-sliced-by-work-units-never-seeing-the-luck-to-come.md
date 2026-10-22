---
id: "2113"
title: "Search bots: minimax and MCTS over any TurnRules game with sides, sliced by work units, never seeing the luck to come"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2112", "3009", "2125"]
nick_input: none
completed:
---

# 2113 — Search bots: minimax and MCTS

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Their AI tools are for real-time agents: Unreal's
Behavior Trees and StateTree, and Unity's Behavior package. Bevy and
Godot have none built in. Unity ML-Agents and Unreal's Learning Agents
train agents; they don't search. None ships a game-tree search for turn
games, so this is an edge.

Shuyi today:

- `crates/core/src/bots.rs`: `Sides` (whose move, which turn), the
  `Bot` trait, `play` and `RandomBot`. Its docs say a bot must never
  learn what the game hides from its players, the luck to come above
  all. Nothing enforces it: a bot that clones the state and tries
  commands sees the rolls kept in the state's `SimRng`.
- `examples/hello-grid/src/rules.rs`: `Chase`, the foe's mind, one
  greedy step by design. The hero is played by `RandomBot` in
  `tests/it/bots.rs` and in `bots` (`src/tools.rs`). Hello-grid's
  `Stage` keeps its luck in a `SimRng` (the foe may trip), so a naive
  search would know whether the foe trips.
- 2112: `shuyi_core::search` (`crates/core/src/search.rs`) and its
  `solve`, a breadth-first search that picks every side's commands. It
  proves a puzzle can be won. It can't play one side against another.
- 3009: `shuyi_core::think::Think` and `Progress`, a decision spread
  over frames in work units.
- 2125 (split out of this ticket on 2026-10-21): `Determinize` and
  `Evaluate` in `shuyi_core::bots::determinize`, and the rollout bot
  (flat Monte Carlo on determinized copies) in `shuyi_core::bots::rollout`,
  with hello-grid's `Determinize`, `score`, `distances_to_goal` and
  `Walk`. It waits for nothing, because the game's playtest bots
  (game#0506) run on a PC and need it now. This ticket builds on it:
  the trees here search the same copies and score them the same way.
- 2128: the turn planner (`shuyi_core::bots::plan`, ADR-0029), a beam
  over one side's own commands for its whole turn, on the same
  determinized copies. It plans one side and searches no replies;
  minimax and MCTS here do. The code it shares with the rollout bot
  (`draw_seeds`, `checked_copy`, `apply_candidate`, `pick`) is
  `pub(super)` in `bots/rollout.rs` (its `Progress` becomes 3009's
  `think::Progress`). This ticket's bots are a third
  user: move those items to a module of their own
  (`bots/shared.rs`), and use them from all three.

Nick's rule of 2026-10-18 (`docs/decisions.md`): the AI is the
engine's, fair and deterministic; a computer opponent sees only what its
side sees, unless its difficulty settings say otherwise; it thinks in
work units, never time. This ticket gives every turn game with sides a
computer opponent, or a playtest bot stronger than `RandomBot`, that
keeps that rule. The game writes only a score and a `determinize`
(2125's traits).

Why it matters. Visions of Shuyi doesn't ask for it: its enemy AI is
game code, and a wide tactics map branches too much for a game-tree
search (its playtest bots use 2128's turn planner instead). It serves turn games with two sides and few commands a turn:
board games, small skirmishes, hello-grid's hero, and the hero of a
card battle in a test (2737, parked until after the launch). Until the
launch, the end goal of 2026-10-19 is measured on turn-based games
(2026-10-20), and an agent asked for a board game should be able to
give it an opponent. A module is written when a kind of game needs it,
and shows one in an example (2026-10-16, ADR-0009 §7): here,
hello-grid.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::search` gains two submodules**, `no_std`, declared
  in 2112's `crates/core/src/search.rs`: `minimax.rs` and `mcts.rs` in
  `crates/core/src/search/`. Tests go in `search/minimax/tests.rs` and
  `search/mcts/tests.rs`, and the fixture games both use in
  `search/fixtures.rs` (`#[cfg(test)]`; reuse 2125's dice race and Nim
  from `bots/fixtures.rs` rather than writing them twice). It stays in
  `shuyi-core` beside the solver: it is generic, adds no dependency, and
  a game that never names it pays nothing (ADR-0009 §1).
  - **`Determinize` and `Evaluate` are 2125's**
    (`shuyi_core::bots::determinize`). **The bots take only games that
    implement `Determinize`**, so every game chooses what its bots may
    see. A leaf is scored by `Evaluate::score(root, leaf, side)`, the
    root being the state the decision began in.
  - **The bot's side** is the side to move when a decision starts.
  - **`Minimax<E>`**: alpha-beta, depth first, by iterative deepening.
    `MinimaxSettings { depth: u32, nodes: u32, samples: u32 }`, with
    `Serialize` and `Deserialize` so a difficulty is data. `depth` is
    counted in commands.
    - A state where the bot's side moves is a max node; any other
      side's is a min node (with more than two sides, every other side
      plays against it). A side that moves twice in a row works, as in
      a game where each of a side's pieces moves in turn.
    - A leaf is a finished state or `depth` reached, scored by
      `Evaluate::score` for the bot's side.
    - Each root command is searched with a full window, so its score is
      exact and scores can be summed across copies (below). Alpha-beta
      prunes below the root.
    - A copy's answer is the root scores of the deepest depth its
      `nodes` limit let it finish.
    - A unit of work is one node visited.
  - **`Mcts<E>`**: Monte Carlo tree search with an integer UCB.
    `MctsSettings { iterations: u32, playout: u32, exploration: i32, samples: u32 }`,
    with `Serialize` and `Deserialize`. Its tree is a `Vec` of nodes
    with child indices; each node keeps its visits and the sum of its
    scores for the bot's side.
    - Selection: an unvisited child first, in `legal_commands` order;
      expansion adds it as the one new node of the iteration. Else the
      child with the best
      `mean + ((exploration * isqrt(((ilog2(parent_visits) + 1) << 16) / visits)) >> 8)`,
      where `mean` is the child's sum over its visits (`i64` division),
      negated when the side to move at the parent isn't the bot's. All
      in `i64`, with `u64::isqrt` and `ilog2`: no floats (`AGENTS.md`
      rule 5). Ties go to the earlier child.
    - Playout: random legal commands from the bot's own `SimRng`, up to
      `playout` commands or the end, then `Evaluate::score` for the
      bot's side, added to every node on the path.
    - A copy's answer is the visits of each root child.
    - A unit of work is one iteration, whose playout `playout` bounds.
  - **Luck and hidden things, for both:**
    - A decision draws `samples` seeds from the bot's own `SimRng` when
      it starts, each from two `next_u32` draws (the first is the high
      half). A `samples` of 0 counts as 1.
    - It searches each `determinize(side, seed)` copy in turn, each
      with the whole `nodes` or `iterations` limit, and sums the copies'
      answers per root command in `i64`: root scores (minimax) or
      visits (MCTS).
    - The answer is the root command with the best sum. Ties go to the
      first in `legal_commands` order. If no minimax copy finished depth
      1, the answer is the first legal command.
    - The real state's `legal_commands` are the root's. A copy whose
      `legal_commands` differ (another list or another order) is a bug
      in the game's `determinize`, and the bot panics with a message
      naming `determinize`.
  - **Both are `Bot`s and `Think`s.** `Minimax::new(eval, settings,
    seed)` and `Mcts::new(eval, settings, seed)` implement `Bot<R>` for
    `R: Determinize` with `R::Command: Clone + PartialEq` and
    `E: Evaluate<R>`. `think(&mut self, rules: &R)` returns a search
    value (`MinimaxSearch` or `MctsSearch`) that implements 3009's
    `Think` with `Decision = R::Command`, and `choose` steps it with
    `u32::MAX`. The search keeps its stack and tree in the value, not on
    the call stack. Every draw from the bot's `SimRng` happens in a
    fixed order inside the search, so slice sizes never change them.
  - Re-exports in `crates/core/src/lib.rs`: `Minimax`,
    `MinimaxSettings`, `MinimaxSearch`, `Mcts`, `MctsSettings` and
    `MctsSearch`, and the crate docs' line on `search` names the bots.
- **An ADR** (the next free number in `docs/adr/`, with the `write-adr`
  skill), extending 2125's (copies from `Determinize`, integers, work
  units): the UCB in integers; slice independence through 3009's
  `Think`; ties by `legal_commands` order; when a game should pick
  minimax or MCTS over 2125's rollout bot (few commands a turn, replies
  that matter).
- **Hello-grid shows it** (`examples/hello-grid/`), with 2125's
  `Determinize`, `score` and `distances_to_goal`:
  - `tests/it/bots.rs`: a minimax hero against `Chase` wins more of
    seeds 0 to 19 than `RandomBot` does (both counts in the Completion
    notes), and the first command of a minimax hero and of an MCTS hero
    doesn't change when only the stage's seed does;
  - `README.md`: the sentence 2125 added after the tools block also
    says `tests/it/bots.rs` plays a minimax hero against the foe;
  - the stage screen's foe stays `Chase`, and `bots`' hero stays
    `RandomBot`, so records and snapshots don't change.
- **Either order with 1134:** if `skills/shuyi-balance-bots/SKILL.md`
  exists, add a short section on the search bots and `Determinize`.
- **The float ban:** `search` passes 2015's float ban in `shuyi-core`
  (`deny(clippy::float_arithmetic)` outside tests).

**Out (do not do):**
- the level solver (2112);
- `Determinize`, `Evaluate` and the rollout bot (2125, done first);
- the turn planner (2128, done), beyond moving the shared code;
- expectimax with chance nodes. A game would have to list each roll's
  outcomes; a follow-up when a game asks;
- transposition tables, move ordering beyond `legal_commands`, opening
  books;
- behaviour trees, utility AI, fog of war, and anything real-time
  (parked 2710, 2713, 2714, 2715; ADR-0015 §8);
- learned agents;
- the soak bot (2527), which plays random commands on purpose;
- one description of a game's types and how a view differs from the
  state (2907). How `Determinize` fits it is 2125's ADR's job;
- the game's own enemy AI, and moving it onto these bots (game
  tickets);
- a new example game. The gallery of small games (2736, 2737) is parked
  until after the launch; when 2737 is unparked, it implements
  `Determinize` (the draw pile's order redrawn) and tests an `Mcts`
  hero with it; its foes keep following the intents they show;
- threads (3009's Out holds).

## Implementation steps

1. Read 2112's `search` module, 3009's `think` module and its slicing
   test, `bots.rs`, 2125's `bots/determinize.rs`, `bots/rollout.rs`
   and ADR, 2128's `bots/plan.rs` and ADR-0029, and ADR-0008 §6. If 2013's ADR on
   `#[non_exhaustive]` and constructors is in `docs/adr/`, follow it
   for `MinimaxSettings` and `MctsSettings`.
2. Write the fixtures in `search/fixtures.rs`, reusing 2125's Nim
   and dice race (move them where both test modules reach them):
   - tic-tac-toe;
   - a game where one side makes two picks in a row before the other
     side's one, whose best line the test works out by hand.
3. Write `Minimax` as a resumable search: a `Vec` of frames in the
   search value, and an outer loop over copies and depths.
4. Write `Mcts`: its tree is a `Vec` of nodes with child indices, and
   the loop over copies is the same as minimax's.
5. Write the tests below, then the ADR.
6. Add hello-grid's tests and the README lines (its `Determinize`,
   `score` and `distances_to_goal` are 2125's). Keep the bot test's settings small
   (depth 2 and 2 samples, for example) so the 20 plays take a few
   seconds in a debug build.
7. Do the either-order parts that apply, and run the gates.

Size: if the diff runs well past 600 changed lines (fixture games
aside), move `Mcts` and its tests to a follow-up ticket, and keep
`Minimax` and hello-grid's part here. 2737's
clause with 2113 then names that ticket for its `Mcts` hero.

## Acceptance criteria

- [ ] At full depth, minimax picks the winning take in Nim from every
      pile of 1 to 20 that isn't a multiple of 4, and never loses
      tic-tac-toe in 50 plays against `RandomBot`, 25 as each side.
- [ ] MCTS finds the winning take in Nim from every pile of 1 to 7 that
      isn't a multiple of 4, with the iterations the test sets (said in
      the PR).
- [ ] For both bots, on every fixture, slicing by 1, 7, 100 and
      `u32::MAX` units gives the same command (property test with
      random slice sizes).
- [ ] For both bots, on the dice race and on hello-grid, two states that
      differ only in their `SimRng` get the same command (property test
      over pairs of seeds).
- [ ] The same bot seed and the same state give the same command twice.
- [ ] A `determinize` that changes the root's commands makes the bot
      panic with a message naming `determinize`.
- [ ] Hello-grid's minimax hero wins more of seeds 0 to 19 against
      `Chase` than the random hero does.
- [ ] Hello-grid's existing records and snapshots pass unchanged.
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf` (CI's
      `no-std` job).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: Nim and tic-tac-toe positions with a known best move; ties by
  `legal_commands` order; the depth and node limits; the answer when
  the node limit stops iterative deepening, and when it stops depth 1;
  the integer UCB on known counts; a side that moves twice in a row;
  `samples` of 0 and of several copies; the panic on a bad
  `determinize`.
- Property: slice independence; luck independence; the same answer
  twice.
- Integration: hello-grid's `tests/it/bots.rs`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
