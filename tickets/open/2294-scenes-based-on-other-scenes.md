---
id: "2294"
title: "Scenes based on other scenes: a scene file that inherits a base and overrides fields by JSON Pointer, and instances that patch their prefab, resolved at load and at bake"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2221"]
nick_input: none
completed:
---

# 2294 — Scenes based on other scenes

## Context

2221 saves and loads an ECS world as a `*.scene.ron` file, keeping its
ids, spawning instances with new ids, and reloading when the file
changes. The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`,
*Bevy's game coverage*) found what level-designed games build on top of
that: prefab variants. A goblin archer is the goblin with a bow and 12
hit points; a room is the base room with one door moved. Bevy's BSN
scenes inherit other scenes and patch them
(`crates/bevy_scene/src/scene_patch.rs`, `resolved_scene.rs`;
`crates/bevy_bsn/src/parse.rs`; `examples/scene/bsn.rs`).

Here there is no macro language. Overrides are addressed by
[ADR-0084](../../docs/adr/0084-one-description-of-a-games-types.md)'s JSON Pointers into the scene's data, with
the same meaning as 3417's patch operation, and resolved
deterministically, so the baked pack is checked equal to the resolved
text (ADR-0007).

## Nick input

None.

## Scope

**In:**
- **A base:** a top-level `base: "room.scene.ron"` and a list of
  overrides per entity, `set: { "/door/Position/x": 12 }`, plus `add`
  and `remove` of whole components and entities.
- **Instances:** an entry `(instance: "goblin.scene.ron", set: {
  "/0/Stats/hp": 12 })` spawns a prefab with its overrides applied.
- **Resolution:** bases resolved depth first, overrides applied in file
  order, at load and at bake; the result is a plain scene, the same as
  writing it out by hand. `shuyi scene resolve <file>` prints it.
- **Checks:** `shuyi check` reports a pointer that names nothing, a
  value of the wrong type, and a cycle of bases, each with the file,
  the line and the entity.
- **Reload:** an edited base re-resolves every scene and instance built
  on it (2220's reload).
- **Either order with 3417:** if 3417 is done, apply overrides through
  its patch operation's code; if not, write the pointer application
  here, and 3417 reuses it.
- New types follow ADR-0039; errors are `ContentError`s with a line.

**Out (do not do):**
- parameterised templates (a prefab with named parameters): a follow-up
  if a game asks;
- prefabs in the arena of plain structs (1119, parked);
- an editor for scenes (2506, parked).

**Estimated size:** about 1,100 lines (code 500, tests 450, docs 150).

**Split:** past 900 lines, split off instances (the second bullet) into
a new ticket.

## Implementation steps

1. Read 2221's scene format and loader, ADR-0084, and 3417.
2. Add `base`, `set`, `add` and `remove` to the format and its schema.
3. Write resolution with the cycle check.
4. Add instances.
5. Add the checks, `shuyi scene resolve`, and the bake's equality test.
6. Re-resolve on reload.
7. Write the docs page: a goblin and its archer variant.

## Acceptance criteria

- [ ] A scene with a base and one `set` resolves to the base with that
  one value changed, equal to a hand-written fixture.
- [ ] A pointer that names nothing fails `shuyi check` with the file,
  line and entity.
- [ ] Two scenes that are each other's base fail with a cycle error.
- [ ] Editing a base's value changes every instance on the next reload.
- [ ] The baked pack's scene equals the resolved text.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each override kind, the cycle check.
- Property: resolving then writing then resolving again changes
  nothing.
- Snapshot / integration: the bake's equality; reload of a base.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
