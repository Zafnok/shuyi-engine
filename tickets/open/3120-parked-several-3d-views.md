---
id: "3120"
title: "Parked: Several 3D views in a frame: viewports for split screen, a view drawn into an image a material or a sprite shows, and a first-person view model drawn over the world"
type: feature
milestone: "Parked: 3D"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3103", "3104"]
nick_input: none
completed:
---

# 3120 — Several 3D views

> **Parked** (Nick, 2026-10-17: "I want to expand on this and add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after the 2D work already planned (`docs/decisions.md`, 2026-10-17). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

3103's frame has one camera per 3D view, with no viewport and no image
target. 2320 notes that a second view of the same stage needs a ticket
of its own. The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`,
*Bevy's game coverage*) found the 3D games this leaves out: split-screen
racing and party games, in-game screens, mirrors and 3D portraits, and
first-person shooters whose weapon is drawn over the world. Bevy has
viewports, image targets, render layers and view models
(`crates/bevy_camera/src/camera.rs`, `visibility/render_layers.rs`;
`examples/3d/split_screen.rs`, `render_to_texture.rs`, `mirror.rs`;
`examples/camera/first_person_view_model.rs`). 3264 does the 2D side
(a nested frame drawn into a picture). Changing the view's fields
amends ADR-0011 §5, so this ticket writes an ADR.

## Nick input

None.

## Scope

**In:**
- **Viewports:** a 3D view gains `viewport: Option<Rect>` in frame
  pixels, matching 1120's world-layer viewports and 2406's seats.
- **Image targets:** `target: Screen | Image(name)`; a named view image
  may be named by a 3D material or a 2D sprite in the same frame. Views
  draw first, in a fixed order printed in the snapshot text; a view
  that samples its own image is an error.
- **View models:** `overlay: Vec<MeshDraw>` with its own field of view
  and a cleared depth, for a first-person weapon.
- Both the raster and 3104 paint them.
- An ADR amending ADR-0011 §5 (the `write-adr` skill).

**Out (do not do):**
- split-screen seats and their HUDs (2386, parked);
- 2D views in pictures (3264);
- recursive mirrors (one level).

**Estimated size:** about 1,100 lines (code 550, tests 350, docs 200).

**Split:** past 900 lines, split off image targets into a new ticket.

## Implementation steps

1. Read 3103, 3104, ADR-0011, 1120 and 3264.
2. Write the ADR.
3. Add viewports; then image targets with the order and the cycle
   check; then the overlay.
4. Paint them in both painters.
5. Write the docs page: a security monitor and a weapon view model.

## Acceptance criteria

- [ ] Two viewports side by side draw two cameras' pictures, equal to
  their goldens.
- [ ] A screen in the world shows a second camera's image the same
  frame.
- [ ] A view that samples its own image fails with a frame problem.
- [ ] A view model never clips into a wall it touches (depth cleared).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the view order and the cycle check.
- Snapshot / integration: raster goldens; render-job comparisons.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
