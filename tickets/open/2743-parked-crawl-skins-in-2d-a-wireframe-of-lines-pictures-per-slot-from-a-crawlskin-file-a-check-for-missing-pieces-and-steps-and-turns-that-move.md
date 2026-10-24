---
id: "2743"
title: "Parked: Crawl skins in 2D: a wireframe of lines, pictures per slot from a crawlskin file, a check for missing pieces, and steps and turns that move"
type: feature
milestone: "Engine: dungeon crawlers"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2742", "2336", "2109"]
nick_input: none
completed:
---

# 2743 — Crawl skins in 2D: lines and pictures

> **Parked** (Nick, 2026-10-22: "if we can have tickets to add support for this type of game too"): like the other kinds of game, this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0033](../../docs/adr/0033-dungeon-crawlers-a-labyrinth-of-cells-and-walls-a-view-as-plain-data-skins-from-lines-to-3d-and-a-party-of-any-size.md)
§5. Two classic looks of the genre, both painting 2742's `CrawlView`:

- **lines**: Wizardry I's wireframe. No art, so a new game shows its
  labyrinth on the first run, in one bit and on the Playdate;
- **pictures**: a picture per wall position, as in Eye of the Beholder
  and the 2D Etrian Odyssey games.

It builds on 2742 (the view), 2336 (line shapes in the frame) and 2109
(tweens drawn from the time alone).

**Either order with 2340:** if 2340 is done, a step or turn is instant
when the player's reduced-motion setting is on. If not, 2340 adds that
here when it lands.

**Either order with 2744:** if 2744 is done, `draw` already pulls in
`shuyi-ui` with its `tools/layers.ron` line (keep both), and 2744's
travel sends its next command when a step's motion ends, not each
frame (a test: travel over three cells plays three whole steps). If
not, nothing to do (2744 paces travel by the motion).

**Either order with 2753:** if 2753 is done, `motion_ms` is already in
`crawl.ron`: use it, don't add a second. If not, add it as below, and
2753's camera glide reads it.

**Who uses it** (ADR-0033 §17's bar for engine work): crawler RPGs and labyrinth puzzle games on every 2D target, from the Playdate in one bit (lines) to a painted desktop game (pictures).

## Nick input

None.

## Scope

**In:**
- **The `draw` feature** of `shuyi-crawl`, which pulls in `shuyi-ui`.
  Its line in `tools/layers.ron` allows `shuyi-ui` behind the feature.
- **The viewport**: `CrawlViewport { rect: PxRect }`, and
  `perspective_rects(rect, max_depth) -> Vec<PxRect>`: the rectangle of
  the far face at each depth, in whole pixels, each a fixed fraction of
  the one before (a table of integer ratios in `crawl.ron`, defaulting
  to Wizardry's look). The side faces join the corners of two depths.
  Integers only.
- **`LinesSkin`**: paints a `CrawlView` into a viewport as 2336 shapes:
  - walls as outlined quads, doors as a smaller quad inside the wall,
    open doors as a gap, side walls as trapezoids, the floor and ceiling
    as no lines (Wizardry's look) or as a grid, by setting;
  - colours by palette role (wall, door, background), so the one-bit
    renderer maps them (ADR-0026);
  - things in a slot as a picture if the game gave one, else a small
    marked rectangle on the slot's floor.
- **`PictureSkin`**: paints from `*.crawlskin.ron`, asset kind
  `CrawlSkin`, format `crawlskin-ron`:
  - a table from `(depth, side, face, look)` to a picture id and a
    pixel offset, where `face` is `front`, `left`, `right`, `floor`,
    `ceiling`, `door-front`, `door-left`, `door-right`;
  - `mirror: true` lets a left piece serve as the right one, drawn
    with the sprite item's `flip_x`;
  - a background picture drawn first, and things as pictures per depth
    (smaller when farther, from the skin's per-depth scale list or one
    picture per depth);
  - the same fuzz-style test and golden row as the other formats.
- **The compass**: while the view's `compass` is set (2742), both
  skins draw the facing's letter (N, E, S or W) in a corner of the
  viewport, by palette role.
- **The check**: `CrawlSkin::missing(shape, looks) -> Vec<Piece>`
  lists every piece a view of that shape could ask for that the skin
  has no picture for, and a helper turns them into `ContentError`s
  naming the skin's file. A game's `GameTools::check` calls the
  helper, so `shuyi check` reports a missing piece; 2754's example
  does. `shuyi check` has no hook for modules' checks, and this ticket
  adds none.
- **Steps and turns that move**: a `CrawlMotion` the stage starts from
  the events of a command (`Moved` forward or back, `Turned` left or
  right):
  - pictures: a step zooms the view toward the next depth, a turn
    slides the old and new views sideways, over `motion_ms` (in
    `crawl.ron`, default 150; 0 is instant);
  - lines: the same, by drawing between the two depths' rectangles;
  - a tween from 2109, drawn from the time alone; the rules have
    already moved, so a command sent during the motion finishes it at
    once and starts the next;
  - a `Bump` flash for a refused step, from the refusal, not from an
    event. If 2340 is done, the bump also asks for a short `Ctx::shake`;
    if not, 2340 adds that shake when it lands.
- **Snapshots**: `frame-png` pictures of both skins at three places of
  a test map, and the frame's text snapshot.

**Out (do not do):**
- the 3D skin (2753);
- the automap and its widget (2744);
- the battle screen (2749);
- a raycasting skin (ADR-0033, *Alternatives*);
- art beyond a test skin made from shapes (2754 makes the example's).

## Implementation steps

1. The feature and its layers line; `perspective_rects` with tests.
2. `LinesSkin`, with `frame-png` snapshots.
3. The `crawlskin-ron` format, `PictureSkin`, the `missing` check.
4. `CrawlMotion` for both skins; the bump.
5. Snapshots and the check's tests.

## Acceptance criteria

- [ ] `perspective_rects` gives whole-pixel rectangles that shrink
      toward the centre, for viewports from 64×64 to 400×240 (a property
      test).
- [ ] `frame-png` snapshots of the lines skin and the picture skin at
      three places, the same on every run.
- [ ] `missing()` names each missing piece of a test skin, and a full
      skin has none.
- [ ] A step and a turn play over `motion_ms` and are instant at 0 (a
      test that steps the clock).
- [ ] The one-bit renderer draws the lines skin with each role distinct
      (ADR-0026's check).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `perspective_rects`; the skin table lookup with mirroring;
  `missing()`.
- Property: rectangles are nested and whole.
- Snapshot: `frame-png` and text for both skins, still and mid-motion.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
