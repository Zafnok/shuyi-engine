---
id: "2153"
title: "Relations in shuyi-ecs: parent and child entities, a game's own relations, despawn down the tree, and disabled, required and immutable components"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2107"]
nick_input: none
completed:
---

# 2153 — Relations and entity extras in shuyi-ecs

## Context

Nick asked for everything on Bevy's homepage, with an optional ECS
(`docs/decisions.md`, 2026-10-20). 2904 researches the ECS, 2107 builds
`shuyi-ecs`, 2108 runs it in a game and 2221 saves it as a scene. 2107's
**Out** says "relations (parent and child entities), observers, and
events beyond `Commands`. No ticket plans them yet". The Bevy coverage
audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found
that this is what keeps a whole family of games off the ECS: card games
whose piles are parent entities, roguelikes with containers in the
world, squads and their members, colony games where a building owns its
workers.

Bevy has `ChildOf` and `Children` and lets a game declare its own
relations, keeping the reverse side in sync and despawning a subtree
(`crates/bevy_ecs/src/relationship/`, `crates/bevy_ecs/src/hierarchy.rs`;
`examples/ecs/relationships.rs`, `hierarchy.rs`). It also has a
`Disabled` marker that queries skip by default
(`crates/bevy_ecs/src/entity_disabling.rs`), required components
inserted with their defaults, immutable components that can only be
replaced, and `iter_combinations` for every pair of matches
(`examples/ecs/iter_combinations.rs`, `entity_disabling.rs`,
`immutable_components.rs`).

Here every one of these keeps 2107's rules: no floats, iteration in a
fixed order, and everything in the world's hash and its text dump.
2904's ADR wins where it differs from this ticket's shape.

## Nick input

None.

## Scope

**In:**
- **Relations**, in `shuyi-ecs`:
  - a `#[relation]` derive on a component holding one `Entity` (the
    forward side). The world keeps the reverse list itself, in
    insertion order, so iteration and the hash are the same on every
    run;
  - `ChildOf(Entity)` and `Children` as the engine's one built-in
    relation; a game declares its own the same way;
  - ordered children: insert at an index, move within the list;
  - `Commands::despawn_tree(entity)`, applied in schedule order, and a
    relation's `on_despawn` policy (`Despawn`, `Detach`), declared in
    the derive;
  - traversal: `ancestors`, `descendants` (depth first, child order),
    `root_of`;
  - a relation to a despawned entity is an error the world reports at
    the command, never a dangling id.
- **Entity extras:**
  - a `Disabled` marker: queries skip it unless they ask for it
    (`With<Disabled>`, `Allow<Disabled>`); a disabled entity keeps its
    id, its save and its place in the hash (object pools, units held
    off the board);
  - required components declared in the derive
    (`#[require(Health, Facing)]`), inserted with `Default` in
    declaration order when missing;
  - immutable components (`#[component(immutable)]`): no `&mut`
    access, only replacement through `Commands`;
  - `Query::iter_combinations::<K>()` for K up to 4, in entity-id order.
- **Saved and dumped:** the text dump and the hash include relations
  and the disabled marker. 2221's scene writes only the forward side
  and rebuilds the reverse side on load. **Either order with 2221:** if
  2221 is done, add the forward-side rule to its scene format and a
  round-trip test; if not, nothing to do (2221 writes relations this
  way, as its scope will say when it reads this ticket).
- New public types follow ADR-0039: `RelationError` is
  `#[non_exhaustive]`.

**Out (do not do):**
- parent and child transforms: 2157 (parked) gives the fixed-point
  pose table, and 2904's ADR says whether an ECS uses it;
- observers and hooks on these relations, and events that travel up a
  tree: 2154;
- many-to-many relations and relations carrying data beyond the target:
  a follow-up if a game asks;
- relations in the arena of plain structs (1119, parked).

**Estimated size:** about 1,400 lines (code 700, tests 550, docs 150).

**Split:** past 1,000 lines, split off the entity extras (disabled,
required, immutable, combinations) into a new ticket.

## Implementation steps

1. Read 2904's ADR, 2107's code (`crates/ecs/`) and its derive crate.
2. Add the relation storage: forward component plus a reverse
   `Vec<Entity>` per target, kept in insertion order, updated by the
   world when the forward component is inserted, replaced or removed.
3. Add the derive attribute and `ChildOf`/`Children`.
4. Add `despawn_tree` and the `on_despawn` policy; check that the order
   of despawns is the same in serial and parallel runs (2108).
5. Add traversal helpers and the error for a missing target.
6. Add `Disabled` and the default query filter, then required and
   immutable components, then `iter_combinations`.
7. Extend the text dump and the hash, and 2221's format as the
   either-order clause above says.
8. Write the module's docs page and the book's ECS chapter section.

## Acceptance criteria

- [ ] Inserting `ChildOf(p)` on three entities gives `Children` of `p`
  in that order; removing the middle one keeps the others' order.
- [ ] `despawn_tree` removes every descendant; with `Detach` the
  children stay and lose the relation.
- [ ] A relation to a despawned entity fails the command with
  `RelationError::MissingTarget`.
- [ ] A disabled entity is skipped by a plain query, found by
  `Allow<Disabled>`, and still in the hash and the dump.
- [ ] A required component is inserted with its default; an immutable
  one has no `&mut` access (a compile-fail test).
- [ ] `iter_combinations::<2>()` over 5 entities gives the 10 pairs in
  id order.
- [ ] The hash of a world built in two different insertion orders of
  unrelated entities, with the same relations, is checked by a fixture.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each relation operation, each policy, each extra.
- Property: any sequence of inserts, removes and despawns keeps every
  reverse list equal to the forward components' targets, in order.
- Snapshot / integration: the text dump of a small tree.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
