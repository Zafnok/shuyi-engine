---
id: "1127"
title: "Parked: Side view: a platformer controller, one-way platforms, parallax backgrounds, and a platformer example"
type: feature
milestone: "Engine: more styles"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["1125"]
nick_input: none
completed:
---

# 1127 — Side view: platformer controller and example

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0060 says the army walks around freely (then 1119–1125 become game work). Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

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
  documents them.
- Parallax: several world layers (1120) with factors under ½, and a
  backdrop that repeats sideways.
- Example `examples/platformer/`: two short levels drawn in Tiled
  or LDtk (1124), an animated player (1123), coins, a door to the next
  level, and CC0 art.
- The `SideView` perspective marker (ADR-0009 §3). The platformer
  controller works with `SideView` only, and a `compile_fail` doc test
  shows that it can't be used in a `TopDown` stage.
- Guides (ADR-0009 §5): write `docs/guides/side-view.md` in full. If
  2004 isn't done, create the file and 2004 adds it to its list.
- The new-game wizard (ADR-0011 §3–§4): set the `side-view` answer in
  `docs/modules.ron` to ready, if 2502 has added it. If 2503 is done,
  add the part `stage-side-view` to the built-in template: a room with
  the platformer controller, generic over `SideView`, and its test. If
  2503 isn't done, 2503 adds it.
- Tests:
  - controller unit tests, including coyote time and the jump buffer at
    their exact step limits;
  - a Harness playthrough of level 1 from a recorded input;
  - replay hashes;
  - snapshots.

**Out (do not do):**
- slopes, unless the example needs them;
- enemies with AI beyond walking back and forth.

## Implementation steps

1. Write the controller and its tests.
2. Add parallax repeat.
3. Build the example, its tests, CI and Pages.

## Acceptance criteria

- [ ] The controller tests pass at their exact step limits.
- [ ] The recorded playthrough finishes level 1. The replay is
      deterministic.
- [ ] The web build is on Pages.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the controller.
- Integration: the playthrough and replay.

## Completion notes
