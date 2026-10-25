# ADR-0044: A planner's candidate order: ranked groups in the core, and the tactics order in the kit

- **Status:** Accepted
- **Date:** 2026-10-24
- **Related tickets:** 2132 (this), 2128 (the turn planner), 2131 (threat
  areas), game 0506 and 1180

## Context

- The turn planner ([ADR-0029](0029-the-turn-planner-a-beam-over-the-sides-own-commands-copies-kept-and-the-best-continuation-backed-up.md))
  and the rollout bot ([ADR-0027](0027-bots-that-plan-see-a-determinized-copy-integer-rollouts-shared-seeds-and-work-units.md))
  try only the first `candidates` commands of the game's `Order`. A
  tactics turn has hundreds of legal commands, and the cost of a decision
  grows with the cap.
- The game's persona bots (game 0506) first ran on the plain legal order
  capped at ten. It held one unit's attacks in every flavour and no
  unit's move: the planner never tried another unit, a step towards the
  goal or ending the phase, and stalled (game ADR-0095, *Alternatives
  considered*). The order that worked was tuned with the owner on a real
  battle, in eight steps (the game's `crates/bots/src/persona.rs`,
  `order`): about 174 lines, of which about 50 are the game's own (which
  command is a strike and its forecast, which is a heal, the enemy AI's
  pick).
- Nick asked (2026-10-22) that the generic parts of the game's
  playtesting move into the engine.
- 2131 gave the kit `Threats` (which tiles a piece can strike next turn)
  and `GoalDistance` (walking distance to the nearest goal): what the
  order's advances and holds need.

## Decision

1. **Core: ranked groups.** `shuyi_core::bots::order` holds
   `Ranked<C, K>` (a command, its `group: u8`, its `value: (i64, i64)`,
   its `key: Option<K>`) and `ranked_order(items, last, cap)`: groups
   ascending; in a group the best value first, ties in the game's order
   (a stable sort); in a group one per key first (a round-robin: all
   firsts, then all seconds; no key is never held back); at most `cap`
   commands (0 counts as 1), with `last` (the end of the turn) in place of
   the last of them. A deck-builder's or city builder's order is this
   alone. Integers and `Vec`s only: no hash maps, no floats, so the order
   is the same on every target.
2. **Tactics kit: `TacticsOrder`.** `shuyi_core::threat::TacticsOrder<H>`
   (`hooks`, `cap`) implements `Order<R>` for grid games. The game's
   `OrderHooks<R>` (`prepare`, `classify`, `pick`, `at`, `distance`,
   `safe`) holds its `Threats` and `GoalDistance`, built in `prepare`,
   and tags each legal command with a `Kind<P>`: `Strike`, `Extra`,
   `Step`, `End`, `Drop` or `Other`. The order is the game's eight steps:
   each piece's best strike on each target (one per piece, target and
   way, the best tile; the first per piece and target here), the game's
   pick, the extras, each piece's advance, each piece's hold, the strikes
   held back, the rest in the legal order, and the end within the cap.
   The module's docs list them and why.
3. **The game keeps its classifier.** What a strike, a heal or a pick is,
   and how much a strike is worth, stay the game's (`classify`, `pick`).
   Scoring is still the game's `Evaluate` (2130).
4. **`Kind::Extra` carries a `once`** (a piece, its target and its way)
   and a pair value: the game's heals are listed once per healer, target
   and action, from the healer's own tile when it can, which a value and
   a group alone can't say.

## Consequences

- A game's planner order is a classifier of about 50 lines; a capped
  list always holds every piece's say, a step towards the goal, a step to
  safety and the end of the turn.
- `TacticsOrder` lists at most `cap` commands: a game gives it the same
  cap as the planner's `candidates`, or the end can fall past the
  planner's own cut.
- `Kind` is `#[non_exhaustive]` (ADR-0039 §6): a new kind is not a break.
  Its variants are plain values; a new field on one is a break, with its
  `Breaking:` line.
- The order asks the hooks for a walking distance per step; the game's
  `GoalDistance` caches one field per movement kind, so a turn costs one
  flow field per kind.

## Alternatives considered

- **A section in ADR-0029.** ADR-0029 is accepted; this repository never
  rewrites an accepted ADR, and the order is its own decision on top.
- **`TacticsOrder` built on `ranked_order` alone.** Its step 6 lists the
  held-back strikes best first across every key, not in rounds, and its
  advances and holds are one per piece; mapping the game's `Extra`
  groups into `ranked_order`'s `u8` groups between the pick and the
  advances would cap the game's groups. They share the cap and the
  kept end (`keep_last`), and each lists its own steps.
- **The game's order moved as is.** It names the game's commands
  (`Act`, `EndPhase`), its forecast and its AI: game rules in the
  engine (`AGENTS.md` rule 4).
- **A key of `(piece, way)` for the strike dedupe**, as the ticket's
  sketch says. The game's `once_per_action` keys on the unit and the
  whole action, target included: a piece's sword on two targets is two
  ways to keep. The order keys on `(piece, target, way)`.
