---
id: "3264"
title: "A view drawn into a picture: a nested frame a skin paints through its own camera, placed scaled into a rectangle of the frame, for minimaps, monitors and replays"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3264 — A view drawn into a picture

## Context

Bevy's cameras can render into an image that is then drawn elsewhere
(`RenderTarget::Image` in `crates/bevy_camera/src/camera.rs`;
`examples/3d/render_to_texture.rs`). The Bevy coverage audit of
2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found the games
that need it: a minimap that shows the live world itself, security
camera monitors in a puzzle game, portals and mirrors, and a replay
window beside the play. No ticket plans it: split screen (2385 and
2386, parked) puts views side by side over the whole window, and
2716's minimap draws coarse grids.

Shuyi already has what this needs. A skin paints a `GlyphBuffer`; the
grid kit's painters take a camera (`crates/ui/src/grid/camera.rs`); and
2399 lets a skin hand the frame a picture it painted at run time. What
is missing is one item that holds a whole nested frame and is drawn,
scaled, into a rectangle, the same way by every renderer. Rule 9
holds: the rules never see it; a view is a skin's choice.

## Nick input

None.

## Scope

**In:**
- `Item::View` in `crates/ui/src/frame/`: a nested `GlyphBuffer` (its
  own size in console pixels), a destination rectangle, an integer
  scale (a whole ratio up or down, nearest), a clip, and a layer.
  `GlyphBuffer::add_view(sub, dest, scale)` builds it; a skin paints the
  nested buffer with any painter and camera it likes (for example the
  grid kit's painter with a second `GridCamera`).
- **Every renderer:** the raster paints the nested frame into a scratch
  picture in integers and blits it; the one-bit renderer the same; in
  `shuyi-app` it is drawn through 2399's painted pictures, re-uploaded
  only when the nested frame changes (its hash).
- **Snapshot text** lists the view and its items indented under it, so
  a test of what happened reads it as it reads the frame.
- **Hits:** hit areas recorded inside the nested frame are mapped to
  the outer frame through the scale and offset, so a click on the
  minimap reports the nested widget and a point in its pixels.
- **Limits:** a view inside a view is refused (one level), and a
  budget of views per frame is in the target profile, counted in
  `limits`.
- New types follow ADR-0039.

**Out (do not do):**
- world layers and their cameras (1120, parked): when 1120 is unparked
  its layers are painted into a nested buffer like any other painter;
- split screen (2385, 2386, parked);
- 3D views drawn into images: 3120 (parked);
- filtered (smooth) scaling: 3236's smooth tier.

**Estimated size:** about 1,100 lines (code 500, tests 450, docs 150).

**Split:** past 900 lines, split off hits through a view into a new
ticket.

## Implementation steps

1. Read `glyph_buffer.rs`, `snapshot.rs`, `hit.rs`, the raster,
   the one-bit renderer, `crates/app/src/render.rs` and
   `crates/app/src/painted.rs` (2399).
2. Add the item and its builder; refuse a nested view.
3. Paint it in each renderer; cache the upload by the nested hash.
4. Add snapshot text and hit mapping.
5. Add the per-target budget.
6. Write the docs page: a live minimap of a grid map beside the map.

## Acceptance criteria

- [ ] A view of a 64 by 64 frame at scale one half fills 32 by 32 pixels
  of the outer frame, equal to the golden picture.
- [ ] A view whose nested frame didn't change is not re-uploaded by
  `shuyi-app` (a counter in the test backend).
- [ ] A click on the minimap's pixel (10, 10) at scale 2 reports the
  nested hit at (5, 5).
- [ ] A view inside a view is refused with a frame problem.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: scaling, the nesting refusal, hit mapping.
- Property: a view at scale 1 equals painting the nested items offset.
- Snapshot / integration: goldens in each renderer; the capture (2018).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
