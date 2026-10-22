---
id: "2132"
title: "A planner's candidate order from the game's kinds of command: each piece's best strike first, advances and holds by walking distance, and the end of the turn always within the cap"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2131"]
nick_input: none
completed:
---

# 2132 — A planner's candidate order

## Context

A request from the game.

1. **The game ticket:** game#0506, the persona playtest bots
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0506-persona-bots-casual-normal-hardcore.md`).
   The game's follow-up that moves onto this ticket is game#1180.
   Neither is a blocker of this one.
2. **The job in one sentence:** give 2128's turn planner the few
   commands worth trying first out of hundreds of legal ones, so that a
   cap of ten holds a strike by every piece that can strike, each
   piece's step towards the goal, each piece's step to safety and the
   end of the turn, and not one piece's attack in four flavours.
3. **No friction review triggered:** game#0506 measured 1.8×. Nick
   (2026-10-22) asked that the generic parts of the game's playtesting
   move into the engine; this is one ticket of that sweep (game
   `docs/engine/friction.md`, row "0506 (sweep)"). The game's `order`
   and its helpers (`crates/bots/src/persona.rs`: `order`, `advance`,
   `hold`, `attack_key`, `once_per_action`) are about 174 production
   lines, of which about 50 are the game's own (which command is a
   strike and its forecast, which is a heal, the enemy AI's own pick).
   Its order was found by tuning with the owner on a real battle (game
   ADR-0095, "Alternatives considered": the plain legal order capped at
   ten held one unit's attacks in every flavour and no unit's move).
   With this ticket the game keeps a classifier, about 50 lines.
4. **The generic shape**, in two layers:
   - **core** (`bots`): an order made of ranked groups. The game tags
     each legal command with a group and a value (or drops it); the
     order is the groups in a fixed sequence, each sorted by value, best
     first, with "one per key first" (a command whose key was already
     listed waits for a later round), and one command (the end of the
     turn) always kept as the last within the cap. A deck-builder would
     use it alone: damage plays best first, one per target first, then
     draws, then the rest, end turn always kept.
   - **tactics kit**: a `TacticsOrder` on top of it for grid games:
     strikes (each piece's best strike on each target first, every piece
     before any piece's second), the game's own extra picks, **advances**
     (per piece, its destination nearest the goals, walking, by 2131's
     `GoalDistance`, if nearer than where it stands), **holds** (per
     piece, its destination out of every threat by 2131's `Threats`,
     nearest the goals, if not its advance), the other ways of striking,
     the rest, the end of the turn.
5. **An example from a game that isn't tactics:** the core layer is a
   deck-builder's or a city builder's planner order (above). The
   tactics layer is for grid games (Advance Wars-, Into the Breach- or
   Fire Emblem-like): it belongs in the kit.

## Nick input

None.

## Scope

**In:**
- `shuyi_core::bots::order` (`crates/core/src/bots/order.rs`):
  `Ranked<C, K>` and `ranked_order`.
- `shuyi_core::threat::TacticsOrder` (or `crates/core/src/threat/order.rs`,
  next to 2131's module): `Kind<P>`, `OrderHooks`, `TacticsOrder<H>` implementing
  2125's `Order<R>`.
- Tests on a toy grid game in `crates/core/src/bots/fixtures.rs` (two
  pieces a side, a goal, a wall), and the core layer on the existing
  multi-command fixture.
- hello-grid's bot uses `TacticsOrder` if its stage has strikes and
  several pieces a side; if it has only one piece walking to a goal,
  leave hello-grid and say so in the Completion notes (the fixture is
  then the example).
- Module docs; ADR-0029 (the turn planner) gains a section on candidate
  orders, or a short new ADR if that reads better.

**Out (do not do):**
- What a strike, a heal or a pick is: the game's classifier.
- Scoring (2130, and the game's `Evaluate`).
- Threat areas and goal distances themselves (2131).
- The game's use of it (game#1180).

## Implementation steps

1. **Read** `crates/core/src/bots/rollout.rs` (`Order`), `plan.rs`
   (how the cap `candidates` cuts the order), 2131's module, and the
   game's `crates/bots/src/persona.rs` `order` and its doc comment (the
   eight steps): that is the behaviour to keep.
2. **Core layer**:
   ```rust
   pub struct Ranked<C, K> { pub command: C, pub group: u8, pub value: (i64, i64), pub key: Option<K> }
   pub fn ranked_order<C, K: Eq>(items: Vec<Ranked<C, K>>, last: Option<C>, cap: usize) -> Vec<C>
   ```
   Groups ascend; in a group, value descends (ties keep the game's
   order, a stable sort); within a group, the first item of each `key`
   comes before any second item of a key already listed (a round-robin
   by key: all firsts, then all seconds…); items with no key are never
   held back. `last`, if any, goes in place of the last of the first
   `cap` items (or at the end if there are fewer), so the cap never cuts
   it. `cap` 0 counts as 1.
3. **Tactics layer**:
   ```rust
   pub enum Kind<P> {
       Strike { piece: P, target: Option<P>, way: u32, value: (i64, i64) },
       Extra { group: u8, value: i64 }, // the game's own lists, after the pick
       Step { piece: P, dest: Pos }, // ends on dest, doing nothing else
       End,
       Drop,
       Other,
   }
   /// What the order asks the game, once per `order` call.
   pub trait OrderHooks<R: Sides> {
       type Piece: Eq + Copy;
       /// Called first: build this state's `Threats` and `GoalDistance` (2131).
       fn prepare(&mut self, rules: &R);
       fn classify(&mut self, rules: &R, command: &R::Command) -> Kind<Self::Piece>;
       /// The game's own pick (an AI's command), if it has one; it may
       /// also be a strike or a step, and is listed once.
       fn pick(&mut self, rules: &R) -> Option<R::Command>;
       /// Where `piece` stands now.
       fn at(&self, rules: &R, piece: Self::Piece) -> Option<Pos>;
       /// How far `piece` would walk from `from` to the nearest goal.
       fn distance(&mut self, rules: &R, piece: Self::Piece, from: Pos) -> Option<u32>;
       /// Whether no threat reaches `at`.
       fn safe(&self, rules: &R, at: Pos) -> bool;
   }
   pub struct TacticsOrder<H> { pub hooks: H, pub cap: usize }
   ```
   The game's hooks hold 2131's `Threats` and `GoalDistance`, built in
   `prepare`; the kit gives a ready `prepare` helper only if it falls
   out naturally. The order, matching the game's eight steps:
   1. each `Strike`'s first per `(piece, way)` (the same way from
      another tile is dropped: the best tile is kept), best value first,
      round-robin by `(piece, target)`;
   2. the hooks' `pick`, if it is one of the commands given, isn't
      `End` or dropped, and isn't already listed (it is then left out of
      the later steps);
   3. the `Extra`s by group, then value;
   4. per piece, its advance: the `Step` with the least goal distance,
      if less than the distance from where the piece stands;
   5. per piece, its hold: the `Step` whose `dest` is safe, with the
      least goal distance, if it isn't the piece's advance;
   6. the strikes held back in round 1 (each other way), best first;
   7. `Other` and the remaining `Step`s, in the legal order;
   8. `End`, kept within `cap` (step 2's `last`).
   `Drop` commands are left out.
4. **hello-grid** (only as Scope says): its bot uses `TacticsOrder`
   with hooks of a few lines; its README names it.
5. **Docs**: module docs with the eight steps and why (a capped list
   needs every piece's say before any piece's second; a planner whose
   pieces never step towards the goal or out of danger stalls: game
   ADR-0095).

## Acceptance criteria

- [ ] `ranked_order` with keys `a a b` in one group gives `a b a`; with
      cap 2 and a `last`, the second is the `last`.
- [ ] On the toy grid game, a `TacticsOrder` capped at 4 lists both
      pieces' best strikes before either piece's second way, then an
      advance, then the end of the turn.
- [ ] A piece's hold is a tile `Threats::safe` says is safe, nearest the
      goal by walking (around the wall), and is left out when it equals
      the advance.
- [ ] A piece with no step nearer the goal than where it stands has no
      advance.
- [ ] `End` is in the order for every cap from 1 to the number of
      commands.
- [ ] hello-grid plays with it and its tests pass, or the Completion
      notes say why its stage has nothing to order.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the criteria above.
- Property: every command listed is one of the commands given, none
  twice, none the classifier dropped, and `End` (when given) is within
  the cap.
- Determinism: the same order on native and wasm32 (no hash maps, no
  floats).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
