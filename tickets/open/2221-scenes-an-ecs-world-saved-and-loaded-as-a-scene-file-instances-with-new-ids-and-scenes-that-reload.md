---
id: "2221"
title: "Scenes: an ECS world saved and loaded as a scene file keeping its ids, instances with new ids, and scene files that reload in a running game"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2107", "2220"]
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
- **`Scene`**, an `AssetKind` in `shuyi-ecs`, and its format
  `*.scene.ron`:
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
  `register::<T>("Pos")`); an unknown name is an error naming the file,
  the entity and the name.
- **Saving:** `Scene::from_world(&World, filter)` writes the entities
  and resources a filter picks, in id order, so the text is stable.
- **Loading keeps ids:** `scene.load_into(&mut World)` puts each entity
  at its id, for saves (through `Storage`, as saves are today in
  `shuyi_core::save`).
- **Instancing gives new ids:** `scene.instance(&mut World) ->
  InstanceId` spawns fresh entities. A component field of type `Entity`
  is remapped to the new ids (a `MapEntities` trait the derive
  implements for `Entity` fields), so a door still points at its
  partner. Instances may nest (a scene names another by path).
- **Reload:** a `Hot<Scene>` (2220) and `World::reload_instance`: in
  debug builds, a changed scene file despawns its instance's entities
  and spawns the new ones at the same ids, keeping components the
  scene doesn't name (a component added at run time stays).
- **Packs:** the pack format (ADR-0023) bakes scenes, and the equality
  test covers a scene fixture.
- The book (2802, if done): a chapter on scenes and saves.

**Out (do not do):**
- an editor for scenes (2507, parked);
- scenes for the arena of plain structs (it has its own dump; a
  follow-up if a game asks);
- Tiled or LDtk maps as scenes (1124, parked).

## Implementation steps

1. Read 2107's dump and load, 2220's `Hot`, and ADR-0023.
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
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each error; remapping; nested instances.
- Property: save, load, compare; instance twice, no shared ids.
- Fuzz-style: random bytes never panic the format (ADR-0007).

## Completion notes

*(Filled in by the session that completes the ticket.)*
