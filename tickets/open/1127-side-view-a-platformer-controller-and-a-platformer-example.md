---
id: "1127"
title: "Parked: Side view: a platformer controller, one-way platforms, parallax backgrounds, and a platformer example"
type: feature
milestone: "Engine: more styles"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["1120", "1122", "1123", "1124", "2115", "2232", "2338", "2360", "2362"]
nick_input: none
completed:
---

# 1127 — Side view: platformer controller and example

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

This covers ADR-0003 §6 (`SideView`) and ADR-0004 §4. Platformers live
or die by how the controller feels:

- gravity;
- variable jump height;
- coyote time (a jump still works a few steps after leaving a ledge);
- a jump buffer (a press just before landing still jumps);
- one-way platforms.

All of it goes in `Fx` on the pixel-stepping movers from 1122, with
every number in data.

## Nick input

None.

## Scope

**In:**
- `shuyi_core::controllers::platformer`: the rules above, plus
  wall-slide as an option. The numbers live in RON, and the example
  documents them. Coyote time and the jump buffer are
  `shuyi_core::timer::Countdown` values counted in steps (2115), and
  their lengths in the RON may be given in milliseconds and turned into
  steps with `steps_from_ms` at load.
- Parallax: several world layers (1120) with factors under ½, and a
  backdrop that repeats sideways, laid out by 2338's `tiled_sprites`
  (its tiles mapped into world items, the origin moving with the
  layer's camera), not a repeat of its own.
- Example `examples/platformer/`: an animated player (1123), coins, a
  door to the next level, CC0 art, and two short levels made in Tiled
  or LDtk (1124) and drawn by 2360's tile-layer items. The player
  start, the coins and the door are the example's own `Thing` enum,
  read with `shuyi_assets::tilemap::things::read` (2232), and its
  tools' `check` calls `things::check_under`. If its levels are
  Tiled's, its project's types are written by `shuyi map-types
  --tiled`; if LDtk's, CI runs `shuyi map-types --ldtk` on them. If
  2232 split `map-types` off and that follow-up isn't done, the Tiled
  types are written by hand, and the LDtk check waits for the
  follow-up.
- **The camera:** a `camera.ron` (2362's `CameraDef`) with look-ahead
  along the player's velocity and `vertical: OnGround`, plus rooms for
  level 2 from the map's `rooms` object layer (`Rooms::from_map`) with
  a slide between them. If 2503 is done, the `stage-side-view` part
  (below) ships the same file.
- The `SideView` perspective marker (ADR-0009 §3). The platformer
  controller works with `SideView` only, and a `compile_fail` doc test
  shows that it can't be used in a `TopDown` stage.
  **Either order with 1126:** if 1126 is done, the marker's projection
  is its `shuyi_ui::world::Projection::SideView { tile }` (ADR-0003 §6).
  If not, add `SideView { tile }` (`x·w, y·h`, depth by layer order)
  beside `TopDown` in the projection 1120's markers read, with its
  tests, and 1126 moves it into `shuyi_ui::world::Projection`.
- Guides (ADR-0009 §5): write `docs/guides/side-view.md` in full. If
  2004 isn't done, create the file and 2004 adds it to its list.
- The new-game wizard (ADR-0013 §3–§4): set the `side-view` answer in
  `docs/modules.ron` to ready (2003 wrote it parked). If 2503 is done,
  add the part `stage-side-view` to the built-in template: a room with
  the platformer controller, generic over `SideView`, and its test. If
  2503 isn't done, 2503 adds it.
- **Either order with 2775 (parked):** if 2775 is done, the `when` of
  its `local-players-split` and `local-players-shared` parts also holds
  on a `stage-side-view` stage, and a test makes a two-player side-view
  game and runs its two-pad Harness test. If not, nothing to do (2775
  does it).
- Tests:
  - controller unit tests, including coyote time and the jump buffer at
    their exact step limits;
  - a Harness playthrough of level 1 from a recorded input;
  - a recorded run through level 2's rooms, whose `Camera::dump()`
    lines show the camera in the player's room;
  - replay hashes;
  - snapshots.
- **Ground, wall and ledge checks. Either order with 2124:** if 2124
  is done, the controller's ground and wall checks call
  `Mover::probe_y` and `probe_x` (a dry run of `move_y` and `move_x` on
  a copy, so a check and a move always agree), and an enemy that walks
  back and forth, if a level has one, turns where
  `CollisionQuery::cast` of a short segment down ahead of its feet
  finds no ground. If 2124 isn't done, the checks move a copy of the
  mover by one pixel, and 2124 changes them to the probes with the
  tests unchanged.
- **The debug view (2317).** If 2317 is done, the platformer example
  calls its collision helpers, and draws the controller's ground and
  wall probes as `Note` lines. If it isn't, 2317 does this.
- **The agent eval.** If 2524 is done, `examples/platformer` becomes
  the reference of `evals/tasks/platformer.ron`: set
  `reference: Project("examples/platformer")`, empty its `waits_for`,
  make the example's scene data answer its `Scene` checks (the level
  and the coins its prompt names), and `cargo xtask eval-reference
  platformer` passes in CI. If 2524 isn't done, nothing here: 2524
  gives the task this reference (its scope says so).

**Out (do not do):**
- slopes, unless the example needs them;
- enemies with AI beyond walking back and forth;
- moving platforms that carry the player (a moving solid that pushes
  and carries 1122's movers, following a 2122 curve): a ticket of its
  own when a level needs one.

## Implementation steps

1. Write the controller and its tests.
2. Add parallax repeat with 2338's `tiled_sprites`.
3. Build the example, its tests, CI and Pages.

## Acceptance criteria

- [ ] The controller tests pass at their exact step limits.
- [ ] The recorded playthrough finishes level 1. The replay is
      deterministic.
- [ ] The web build is on Pages.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the controller.
- Integration: the playthrough and replay; the camera stays in the
  player's room through the recorded run of level 2 (checked on
  `Camera::dump()` lines).

## Completion notes
