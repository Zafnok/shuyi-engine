---
id: "2704"
title: "Parked: Example: a side-view cave with creatures that climb and reach, and the procedural animation module's README, skill and guide"
type: feature
milestone: "Engine: procedural animation"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2703", "1121", "1127", "1134", "2004"]
nick_input: sign-off
completed:
---

# 2704 — Example: a cave with creatures that climb and reach

> **Parked** (Nick, 2026-10-16: "this will be very late addition since it won't tie into visions of shuyi game"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 1119 and 1122. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0009 §4: a module is done when an example shows a direction it
serves. ADR-0003 §1: the example is also its integration test. This
ticket finishes `shuyi-procanim` (2701–2703) with a small side-view
game, then finishes the README, the skill, the guide and the catalog
row.

The example is a `SideView` stage (1127, ADR-0009 §3). It shows the
module in the direction it serves first (ADR-0010 §1).

Rain World inspired the module. The example is our own: its creatures,
names, art and sounds are made for it and owe nothing to Rain World.

## Nick input

**Sign-off:** run the example (natively, or in the browser through the
docs site) and say whether the creatures feel alive. Comments on how
they move become tuning in the creature files, or follow-up tickets.

## Scope

**In:**
- **`examples/cave-crawl`**: a `SideView` tick stage on a small tile
  cave (a Tiled or LDtk map through 1124, drawn by 2360, if both are
  done; otherwise our `.map`). It has:
  - **a crawler**: a long body with four legs and a tail. It climbs
    floors, walls and ceilings toward a lure that the player moves with
    the pointer or a stick (1121), through actions only (the
    `input-actions` skill);
  - **a hanger**: a creature pinned to the ceiling. Its tentacles reach
    for the crawler when it comes near, and hang when it leaves;
  - **a few rigid crates** if 1128 is done, to show alternatives side by
    side (ADR-0010 §1). If not, skip them, and say in the README that
    1128 adds them;
  - art made for the example (CC0, ours). Anything not ours goes in
    `THIRD_PARTY_ASSETS.md`.
- **Tests**:
  - a replay determinism test: the same input record twice gives the
    same simulation and `Rig` hashes;
  - `frame-png` pictures at three steps;
  - a Harness test that the crawler reaches a lure on the ceiling
    within a set number of steps.
- **The README** of `crates/procanim`, finished in ADR-0009 §4's order:
  - what it is for: creature-driven side views and cave crawlers,
    anything whose creatures should adapt to any terrain; top-down free
    with gravity at zero;
  - what it is not for: authored skeletal animation, rigid bodies,
    deciding where a creature goes (ADR-0010 §7), and which to use
    instead;
  - the cost: the WASM size it adds to `cave-crawl`, measured against
    the same example without it;
  - how to start, with good starting values for iterations and gait
    numbers;
  - how it sits beside drawn clips and `shuyi-physics`.
- **The skill** `shuyi-procanim` in the skill pack (1134, done):
  `skills/shuyi-procanim/` in the pack's form, with a worked example,
  `example.ron`, that `shuyi add procanim` and a creature file make run
  on a fresh game. CI's `skills` job runs it, and `check-skills`
  passes.
- **The guides** (2004): `side-view.md` and `top-down-free.md` list the
  module as an answer for bodies and motion.
- **The catalog**: set the row to ready.
- **The debug view (2319).** If 2319 is done, the cave example calls
  its `skeleton` and `footholds` helpers, and `frame-png --debug
  skeleton,footholds` on the cave is reviewed and its snapshot
  committed. If it isn't, 2319 does this.
- **Bodies against boxes (from 2701).** 2701's bodies collide with
  tiles only, and leave boxes here. If the cave needs a body to meet
  boxes (the crawler on a crate), add `Body::step`'s optional
  collision with the boxes of 1122's broad phase (ADR-0010 §2), with a
  property test like 2701's tile one. If not, the README says bodies
  collide with tiles only.

**Out (do not do):**
- gameplay beyond following the lure;
- sound;
- more creatures than the two.

## Implementation steps

1. Make the cave, the two creature files, and their art.
2. Write the stage: the lure, the crawler's AI (push its head chunk
   toward the lure along tile paths), and the hanger's reach.
3. Write the tests.
4. Measure the WASM size. Write the README, the skill and its worked
   example, and the guide lines. Set the catalog row.
5. Add the example to the website's Examples page (2801).

## Acceptance criteria

- [ ] The replay, `frame-png` and Harness tests pass.
- [ ] The README states the measured WASM size.
- [ ] `check-skills` passes on `shuyi-procanim`, and its worked example
      runs in CI.
- [ ] `cargo xtask check-modules` passes, with the row set to ready.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: replay, Harness and `frame-png`, as above.

## Completion notes
