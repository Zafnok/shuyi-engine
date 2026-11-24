# ADR-0145: The tactics order: one strike per piece first, and steps kept out of the rest

- **Status:** Accepted
- **Extends:** [ADR-0044](0044-a-planners-candidate-order-ranked-groups-in-the-core-and-the-tactics-order-in-the-kit.md) §2
- **Date:** 2026-11-24
- **Related tickets:** 2148 (this), 2132 (the tactics order), game 0517,
  0523 and 1180

## Context

- `TacticsOrder` (ADR-0044 §2, 2132) was built from the game's persona
  order as it stood on 2026-10-24. The game's `order`
  (`crates/bots/src/persona.rs`) has gained steps since, and game#1180
  moves its persona bots onto the engine's order with its seeded command
  traces unchanged.
- **game 0523:** a unit with several targets filled the places tried
  with blows, one per target, and a sturdier unit's advance was never
  weighed. The game now lists each unit's best strike only, over every
  target and way; each unit's best strike on each of its other targets
  comes after the advances and holds, and its other ways after those.
  `TacticsOrder`'s step 1 lists the first per `(piece, target)`, and its
  step 6 mixes the other targets and the other ways by value.
- **game 0517:** once no foe of the player is left, a unit that wanders
  wastes the cap. The game drops the rest's unit actions, but still
  weighs each unit's advance, its hold and the AI's pick. A `Kind::Drop`
  is never tried, so a game can't say that with today's kinds.
- The game's other later steps fit today's hooks: escape and seize, and
  a cover in a survive, are `Kind::Extra` groups; a hold in a survive is
  the `distance` hook answering from where the piece stands.

## Decision

1. **`TacticsOrder::targets_after_steps: bool`**, false in `new` (the
   order of ADR-0044 is unchanged), set by `with_targets_after_steps`.
   When it is true:
   - step 1 lists each piece's best strike only, over every target and
     way;
   - step 6 lists first each `(piece, target)`'s best strike not listed
     yet, best first, then the other ways, best first.

   The dedupe per `(piece, target, way)` is unchanged. A setting and not
   a second order: the steps are the same steps, and a game that wants
   ADR-0044's order keeps it with no change.
2. **`Kind::Spare { step: Option<(P, Pos)> }`**: a command tried only as
   the hooks' `pick` or, with a step, as that piece's advance or hold
   (steps 4 and 5, exactly as a `Kind::Step` to `dest`). It is never in
   step 7 (the rest). A `Spare` without a step is tried only as the
   pick. `Kind` is `#[non_exhaustive]`, so the variant is additive.
3. The module docs of `threat/order.rs` keep the steps' list, with the
   setting and the kind.

## Consequences

- The game's order is `TacticsOrder` with the setting on, its escapes,
  seizes and covers as `Extra` groups, and its unit actions tagged
  `Spare` once no foe is left (a wait with its step, the others
  without).
- Another game says "only towards the goal now" the same way: a puzzle
  roguelike whose floor is cleared tags its steps `Spare`, and only the
  step towards the stairs is tried.
- `TacticsOrder` gains a public field: additive, as it is
  `#[non_exhaustive]` and built with `new`.
- No golden value moves: with the setting off and no `Spare`, every
  order is what it was.

## Alternatives considered

- **Make one strike per piece the only order.** It changes every game's
  order and the engine's pinned order test; a game whose pieces have one
  target each sees no difference, and one that wants each target first
  loses it.
- **A filter hook on the rest** (`OrderHooks::keep(command) -> bool`).
  A new trait method is a break for every game's hooks, and the game
  already tags every command once (`classify`): a kind says it there.
- **Tag the wandering commands `Drop` and add the advances as
  `Extra`s.** The game would compute each piece's advance and hold
  itself, which is what `TacticsOrder` is for, and the pick would be
  lost when it is one of them.
- **`Spare` as a flag on `Kind::Step`.** A new field on a variant is a
  break (ADR-0044, *Consequences*), and a spare that isn't a step
  (a unit's other action) has no `dest`.
