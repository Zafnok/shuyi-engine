---
id: "2417"
title: "Hit areas in any shape: a sprite's opaque pixels or a polygon as a hit, so the pointer picks cut-out pictures and irregular regions, with the point inside the hit"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2417 — Hit areas in any shape

## Context

Painters record hit areas in the frame as they paint (ADR-0134,
`crates/ui/src/frame/hit.rs`), and `GlyphBuffer::hit_at` answers what
is under the pointer. Every hit is a rectangle. The Bevy coverage audit
of 2026-11-28 (`docs/ROADMAP.md`, *Bevy's game coverage*) found the
games this leaves out: board games with irregular territories (a
Risk-like map), hidden-object and point-and-click scenes, and card or
piece games with cut-out sprites, where a click on a transparent corner
must fall through to what lies behind. Bevy picks sprites by an alpha
threshold and reports the point hit
(`crates/bevy_sprite/src/picking_backend.rs`;
`examples/picking/sprite_picking.rs`, `custom_hit_data.rs`). Polygon
hotspots exist only inside the parked detective ticket's own module
(2728).

## Nick input

None.

## Scope

**In:**
- A hit gains a shape: `Rect` (today's), `Mask` or `Polygon`.
  - `Mask`: a sprite's one-bit opacity mask, built when the picture
    loads from its alpha with an integer threshold (default 128, set in
    the image's `.meta`), kept beside the atlas (2222), and shared by
    every hit on that sprite;
  - `Polygon`: up to 16 integer points, tested by crossing count in
    `i64`.
- `GlyphBuffer::hit_sprite(sprite, widget, item)` records a mask hit as
  the skin paints a sprite (honouring its flip), so the shape stays the
  look's own and views stay plain data.
- `hit_at` tests the rectangle first, then the mask bit or the polygon;
  a miss falls through to the hit beneath.
- A hit reports the point inside it, in the hit's own pixels.
- New types follow ADR-0039; today's `Hit` constructors keep working.

**Out (do not do):**
- hits on rotated sprites: 2335 rotates sprites, and its rotated hits
  are a follow-up of it;
- 2728's hotspots (parked) move onto this when it is unparked;
- picking in 3D (3122, parked).

## Implementation steps

1. Read ADR-0134, `hit.rs`, `glyph_buffer.rs`, 2222's atlas and the
   `.meta` import settings.
2. Build masks at load; store them beside the atlas pages.
3. Add the shapes and `hit_sprite`; update `hit_at`.
4. Report the point inside the hit.
5. Write the docs page: a map of irregular regions.

## Acceptance criteria

- [ ] A click on a transparent corner of a round sprite misses it and
  hits the widget behind.
- [ ] A click inside a concave polygon's notch misses it.
- [ ] A flipped sprite's mask is flipped.
- [ ] The point reported for a click at the sprite's top-left pixel is
  (0, 0).
- [ ] Every existing hit test passes unchanged.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: mask building, the crossing test, fall-through.
- Property: the polygon test against a brute-force pixel fill of the
  polygon by 2336's rule, or by a plain reference in the tests.
- Snapshot / integration: a Harness run clicking a map of regions.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
