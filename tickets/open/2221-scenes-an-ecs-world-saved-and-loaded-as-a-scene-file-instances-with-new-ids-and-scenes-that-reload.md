---
id: "2221"
title: "Scenes: an ECS world saved and loaded as a scene file keeping its ids, instances with new ids, and scene files that reload in a running game"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2107", "2220", "2907"]
nick_input: none
completed:
---

# 2221 — Scenes

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage. Bevy's scenes: "create, save, and load ECS worlds";
loading keeps entity ids, "useful for save games"; instancing makes
linked copies with new ids (prefabs); changes to scene files apply to
running apps.

2107's `World` already dumps to RON and loads back. This ticket makes
that an asset kind with a format, as ADR-0007 asks of every kind:
standard text first, an optimized form derived and tested equal.

## Nick input

None.

## Scope

**In:**
- **`Scene`**, an `AssetKind`, and its format `*.scene.ron`:
  ```ron
  Scene(
    entities: [
      (id: 1, components: { "Pos": (x: 3, y: 4), "Door": (to: 2) }),
      (id: 2, components: { "Pos": (x: 9, y: 4) }),
    ],
    resources: { "Weather": Rain },
  )
  ```
  Components by registered name (a `SceneRegistry` the game fills with
  `register::<T>("Pos")`, built on 2907's ADR: a component's name, shape
  and reading from text come from that one description of a game's
  types, not from a model of the registry's own); an unknown name is an
  error naming the file, the entity and the name.

  **The file names nothing of the ECS**, so a later scene of 1119's
  arena reads the same file and adds no second format. An entry is an
  `id` (a `u64`, as in the example) and its parts by registered name.
  An arena thing would be one part, named by its type. Archetypes and
  storage stay out of the file, and so does a generation as a field of
  its own: the `u64` is the whole id, so loading keeps it. 2904's ADR
  decides where the kind, the format and the remap trait live: in
  `shuyi-ecs`, or in `shuyi-assets` with only the `World` side in
  `shuyi-ecs`. It also decides whether they are written over a small
  store trait.
- **Saving:** `Scene::from_world(&World, filter)` writes the entities
  and resources a filter picks, in id order, so the text is stable.
- **Loading keeps ids:** `scene.load_into(&mut World)` puts each entity
  at its id, for saves (through `Storage`, as saves are today in
  `shuyi_core::save`).
- **Instancing gives new ids:** `scene.instance(&mut World) ->
  InstanceId` spawns fresh entities. A component field of type `Entity`
  is remapped to the new ids (a `MapEntities` trait the derive
  implements for `Entity` fields; it maps through a table of the file's
  `u64` ids, old to new, so the same table can serve an arena scene
  later), so a door still points at its partner. Instances may nest (a
  scene names another by path).
- **Reload:** a `Hot<Scene>` (2220) and `World::reload_instance`: in
  debug builds, a changed scene file despawns its instance's entities
  and spawns the new ones at the same ids, keeping components the
  scene doesn't name (a component added at run time stays).
- **Packs:** the pack format (ADR-0023) bakes scenes, and the equality
  test covers a scene fixture.
- **Either order with 2802 (the book):** if 2802 is done, its chapter
  7 (saves and settings) gets a section on scenes: a world saved to a
  `*.scene.ron` file, loaded back with its ids for a save, and
  instanced with new ids, with its snippets in `site/snippets/`; if
  not, 2802 writes that section (its chapter 7 says so).

**Out (do not do):**
- an editor for scenes: the editor research (2506, parked) decides
  whether one of its panels edits `*.scene.ron` files, and writes its
  ticket;
- scenes for the arena of plain structs. That is a follow-up after 1119
  (parked until Visions of Shuyi launches), and only if 2904's ADR calls
  for it. It reads this same file through this same remap. Until then
  the arena keeps its serde dump;
- Tiled or LDtk map objects read as a game's own types (2232, parked).
  Spawning them into an ECS world through `SceneRegistry` and
  `MapEntities` is a follow-up of 2232, written when a game on the ECS
  asks.

## Implementation steps

1. Read 2904's ADR (where the scene file and its remap live, and whether
   the arena gets a side later), 2107's dump and load, 2220's `Hot`,
   ADR-0023 and 2907's ADR.
2. Write `SceneRegistry`, the format and its errors.
3. Write saving, loading with ids, and instancing with remapping.
4. Add reload and the pack test.
5. Run the gates.

## Acceptance criteria

- [ ] A world saved to a scene and loaded into an empty world is equal
      (hash), with the same ids (property test).
- [ ] Two instances of a scene with a door pair point each door at its
      own partner.
- [ ] Editing a scene file in a running test (2220's headless script)
      changes the instance and keeps a run-time component.
- [ ] A broken scene file's error names the file, entity and component.
- [ ] The pack's equality test passes with a scene.
- [ ] The scene file and its remap live where 2904's ADR says, and the
      module docs show the file's shape with no field only an ECS has.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each error; remapping; nested instances.
- Property: save, load, compare; instance twice, no shared ids.
- Fuzz-style: random bytes never panic the format (ADR-0007).

## Completion notes

*(Filled in by the session that completes the ticket.)*
