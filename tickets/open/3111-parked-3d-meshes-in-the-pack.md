---
id: "3111"
title: "Parked: 3D meshes in the pack: meshes, materials, skeletons and clips baked as postcard sections and proven equal to their glTF"
type: feature
milestone: "Parked: 3D"
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["3102"]
nick_input: none
completed:
---

# 3111 — 3D meshes in the pack

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3102 loads `Mesh`, `Material`, `Skeleton` and `Clip3` from glTF 2.0 (and
OBJ). Its Out list promised this follow-up: an optimized form of those
kinds, derived at bake time and tested equal to the glTF it came from
([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)
§4,
[ADR-0011](../../docs/adr/0011-3d-fixed-point-space-a-3d-view-in-the-frame-and-hd-2d-as-a-skin.md)
§4).

2204 made the pack
([ADR-0023](../../docs/adr/0023-the-pack-one-indexed-file-of-baked-assets-proven-equal-to-its-sources.md)):
`Bake` (`crates/assets/src/pack/bake.rs`) writes every `Bakeable` kind
(serde's `Serialize` and `Deserialize`, `PartialEq` and `Debug`) as a
postcard section, and `Bake::check` proves each baked asset equal to the
one read from its source. The engine's own kinds are baked that way, and
the pack's tests pin each kind's binary form with a snapshot.

glTF files are large and slow to parse (JSON plus buffers), so a game
that ships 3D models gains the most from the pack.

## Nick input

None.

## Scope

**In:**
- 3102's `Mesh`, `Material`, `Skeleton` and `Clip3` are `Bakeable`:
  they derive serde's `Serialize` and `Deserialize` (behind `shuyi-3d`'s
  serde dependency, `default-features = false`), `PartialEq` and
  `Debug`;
- 2204's `Bake` writes them as postcard sections, with no change to
  the pack's format;
- `Bake::check` proves each equal to the glTF it came from, on 3102's
  fixtures (the textured cube and the two-bone arm, `.gltf` and `.glb`);
- a snapshot pins each kind's binary form, as the pack's tests pin the
  engine's kinds;
- the load time of the fixtures from glTF and from the pack is measured
  and written in the completion notes and `docs/formats.md`'s pack
  section.

**Out (do not do):**
- a new pack format or version: the sections are postcard, as for
  every kind;
- compressing vertex data (quantized normals, index packing): a ticket
  if the measured sizes call for it;
- OBJ in the pack beyond what `Bake` does for any kind;
- drawing (3103, 3104).

## Implementation steps

1. Read ADR-0023, `crates/assets/src/pack/bake.rs` and its tests, and
   3102's kinds.
2. Derive the traits on the four kinds; register them in the 3D
   module's bake plan.
3. Add the equality check and the snapshots on 3102's fixtures.
4. Measure the load times (a test that prints them, or a bench), and
   write them down.
5. Run the gates.

## Acceptance criteria

- [ ] `shuyi bake` on a game with 3102's fixtures writes their four
      kinds into the pack, and `shuyi check --baked` passes.
- [ ] A changed mesh in the pack (one vertex moved) fails
      `Bake::check`, naming the file.
- [ ] Each kind's binary form is pinned by a snapshot.
- [ ] The load times from glTF and from the pack are in the
      completion notes and `docs/formats.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each kind round-trips through postcard equal to itself.
- Property: random meshes (positions in `Fx12`, UVs, weights) bake and
  read back equal.
- Snapshot / integration: each kind's binary form; `Bake::check` over
  the fixtures, passing, and failing on a changed vertex.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
