---
id: "3121"
title: "Parked: 3D output quality: anti-aliasing, HDR colour with tonemapping and exposure, and more named effects (ambient occlusion, screen-space reflections, motion blur, outlines)"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3103", "3104", "3227"]
nick_input: none
completed:
---

# 3121 — 3D output quality

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3104 paints ADR-0011 §7's five named effects, and its **Out** leaves
"screen-space reflections, ambient occlusion and other effects not in
ADR-0011 §7". No ticket covers 3D anti-aliasing or tonemapping. The
Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game
coverage*) found that modern-looking 3D games need them: clean edges,
HDR light with exposure, ambient occlusion, reflections, motion blur,
and outlines (which also give toon looks). Bevy has all of them
(`crates/bevy_anti_alias`, `crates/bevy_core_pipeline` tonemapping,
`crates/bevy_post_process` auto exposure and motion blur,
`crates/bevy_pbr/src/ssao`, `ssr`; `examples/3d/anti_aliasing.rs`,
`tonemapping.rs`, `ssao.rs`, `ssr.rs`, `motion_blur.rs`). Growing the
effect list amends ADR-0011 §7, so this ticket writes an ADR.

## Nick input

None.

## Scope

**In:**
- **Anti-aliasing:** a view gains `antialias: None | Msaa(4) | Fxaa`.
- **HDR and tonemapping:** `tonemap: { curve: Linear | Aces | AgX,
  exposure: Fx12 }`, in the colour space 3227 decides.
- **Effects:** ADR-0011 §7's list grows by `AmbientOcclusion`,
  `Reflections`, `MotionBlur` (from the previous step's camera, view
  only), `ChromaticAberration` and `Outline { colour, width }` (a depth
  and normal edge pass).
- Each is plain data painted by 3104 in a fixed order, with a CPU
  reference where cheap and listed in `limits` otherwise; target
  profiles (3017) name what runs.
- An ADR amending ADR-0011 §7 (the `write-adr` skill).

**Out (do not do):**
- 2D anti-aliasing (3236's research, question 8);
- auto exposure from the picture (exposure is data);
- depth of field and bloom (already in ADR-0011 §7).

**Estimated size:** about 1,400 lines (code 700, tests 450, docs 250).

**Split:** past 1,000 lines, split off the effects (the third bullet)
into a new ticket.

## Implementation steps

1. Read 3103, 3104, ADR-0011 and 3227's ADR.
2. Write the ADR.
3. Add anti-aliasing; then tonemapping and exposure.
4. Add each effect in order, with its reference or its `limits` entry.
5. Write the docs page.

## Acceptance criteria

- [ ] With `Fxaa` on, a fixture's edges match their golden within the
  measured tolerance.
- [ ] An exposure of 2 doubles linear values before the curve in the
  raster reference.
- [ ] An outline appears at depth edges and not on flat faces.
- [ ] A target profile without `Reflections` lists it in `limits`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the curves, effect ordering.
- Snapshot / integration: raster goldens; render-job comparisons.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
