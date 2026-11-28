---
id: "3117"
title: "Parked: More light in 3D views: spot lights, shadows from point and spot lights, cascaded sun shadows, light cookies, a skybox or procedural sky, and exponential, height and volume fog"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3103", "3104"]
nick_input: none
completed:
---

# 3117 — Lights, shadows, sky and fog in 3D

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3103's view has ambient, one directional and up to 8 point lights and
linear fog; 3104 paints one directional shadow map, and its **Out**
leaves "more than one shadow-casting light". The Bevy coverage audit of
2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found the 3D
games this leaves out: horror and stealth games lit by flashlights and
spot lights with shadows, outdoor and open-world games with a sky, sun
shadows over distance and atmospheric fog. Bevy has spot lights,
point and spot shadows, cascades, light textures, skyboxes, an
atmosphere and volumetric fog (`crates/bevy_light/src/spot_light.rs`,
`cascade.rs`; `crates/bevy_pbr/src/atmosphere`, `volumetric_fog`;
`examples/3d/spotlight.rs`, `light_textures.rs`, `skybox.rs`,
`atmosphere.rs`, `volumetric_fog.rs`). No ticket mentions a skybox,
cascades or volumetric fog.

Lights stay a look: rules never read shadow state. A stealth game's
"seen" test is a rule over Fx12 geometry through 3105.

## Nick input

None.

## Scope

**In:**
- **Lights:** `Spot { pos, dir, colour, range, inner, outer: Angle16,
  shadows }` and an optional cookie image on spot and directional
  lights; `casts_shadow` and `receives_shadow` on each mesh draw;
  `cascades: u8` with split distances in Fx12 on the directional light.
- **Shadows:** 3104 paints spot and point shadows (cube maps) and
  cascades; the raster reference paints them by testing against its
  own depth pass from the light, so frame-png shows them.
- **Sky:** `sky: Option<Sky>` on the view: `Cube { image }` (an asset
  kind from six images or one cross image through the image registry)
  or `Procedural { sun_dir, turbidity, colours }` in Fx12, drawn
  behind everything; a game's clock may drive the sun as data.
- **Fog:** `Exponential { density }`, `Height { base, falloff }` and
  `FogVolume { box, density }` beside linear fog.
- Limits per target in 3017's profile; lights over the limit are counted
  in `limits`, never dropped silently; `shuyi check --target` refuses
  an effect a target can't run.

**Out (do not do):**
- area lights, soft shadows (PCSS) and contact shadows;
- clustered shading for hundreds of lights (a follow-up when a measured
  scene needs it);
- baked light and probes (3118).

**Estimated size:** about 1,500 lines (code 750, tests 500, docs 250).

**Split:** past 1,000 lines, split off the sky and fog (third and
fourth bullets) into a new ticket.

## Implementation steps

1. Read 3103, 3104, 3017 and ADR-0011.
2. Add spot lights, cookies and the shadow flags to the view.
3. Paint spot, point and cascaded shadows on the GPU; write the raster
   reference.
4. Add the sky kinds and the fog kinds in both painters.
5. Add target limits; write the docs page: a flashlight in a corridor.

## Acceptance criteria

- [ ] A spot light's cone edge in the raster falls at the outer angle
  within one pixel (golden).
- [ ] A mesh with `casts_shadow: false` casts none in either painter.
- [ ] A lights-over-limit fixture lists the extra lights in `limits`.
- [ ] A cube sky made from a cross image equals its golden in the
  raster.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: light and fog parsing, the cube image reader.
- Snapshot / integration: raster goldens; render-job comparisons with
  the GPU painter.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
