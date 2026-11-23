---
id: "1119"
title: "Parked: Real time: 2137's example sim moves onto fixed point with inertia, and the stage-field skill teaches it"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["game#1131", "2116", "2137", "2139"]
nick_input: none
completed:
---

# 1119 — 2137's example sim on fixed point, and the stage-field skill

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). On 2026-10-30 Nick unparked the demo chain alone ("Just the demo chain"), and this ticket's tick part went to 2137; the rest stays parked. Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0003 §2–§4. Top-down free movement, side-scrolling and real-time
isometric games need a simulation that steps in time. To keep what
makes this codebase fast to work in (replays, headless tests, bots, and
later rollback networking, ADR-0006), the step is fixed (60 a second),
numbers are integer fixed-point, and a run is its seed plus its inputs.

**Split on 2026-10-30** (`docs/decisions.md`): `shuyi_core::tick`
(`TickSim`, `TickInput`, `TickRecord`, `replay`), `TickDriver` with its
input latch, and the example sim went to 2137, which snake (2739) and
the maze chase (2776) build on. Their grids step in tiles and need
neither of what is left here: positions in fixed point, and an arena of
entities. Free movement, collision (1122) and the platformer (1127) do,
so the parked tickets that wait for 1119 still wait for it, and for
2137 through it.

**Split again on 2026-11-23** (the blind-spot audit): the numbers and
the arena went to 2139 (parked), which gathers `Fx`, `FxVec2`,
`FxRect`, `Angle`'s `Fx` sine and cosine, square roots, lengths,
`Lerp` for `Fx` and `FxVec2`, and `Arena<T>` into one ticket. What is
left here is using them: 2137's example sim on `FxVec2`, and the
`shuyi-stage-field` skill.

It needs the engine's code in this repository (the game's ticket 1131).

## Nick input

None (technical). Scheduling: see above.

## Scope

**In:**
- **From 2139 (a blocker):** `Fx`, `FxVec2`, `FxRect`, `Angle`'s `Fx`
  sine and cosine, `Lerp` for both, and `Arena<T>` are 2139's. This
  ticket uses them.
- **2137's example sim moves onto `Fx`:** the dot's position and
  velocity become `FxVec2`, with inertia (it slows down when let go
  rather than stopping). Its golden table
  (`crates/core/tests/golden/tick.ron`) is regenerated in this PR, and
  the Completion notes say so (a `Breaking:` line if 2137's table was
  public by then, ADR-0038).
- **The skill pack. From 1134 (done):**
  `skills/shuyi-stage-field/SKILL.md` lists 1119 in its `waits_for`,
  and `check-skills` fails once this ticket is in `done/`: drop 1119
  from that list, and teach `Fx` and `Arena` (from 2139) in the skill
  (2137 taught fixed steps, `TickSim`, records, `replay` and
  `TickDriver`). If that empties the list, write the skill's worked
  example, `skills/shuyi-stage-field/example.ron`, which CI's `skills`
  job runs on a fresh `shuyi new` game.
- **Either order with 2604:** if 2604 is done, nothing to do (its
  `Crowd` keeps a `Vec<Bouncer>`, and its note says the arena is a `Vec`
  of slots that clones the same way); if not, nothing to do (2604 keeps
  `Crowd`'s bouncers in `Arena<Bouncer>` with `Fx` positions and
  measures the arena's save cost).

**Out (do not do):**
- `Fx`, `FxVec2`, `FxRect`, angles, square roots, `Lerp` for `Fx`,
  `Arena<T>` and their golden table `fx.ron` (2139);
- `shuyi_core::tick`, `TickDriver` and its latch, the debug view's and
  debug commands' tick parts, and the ECS note: 2137;
- collision (1122);
- world layers (1120);
- networking;
- thousands of units (the parked 2705 measures an arena against columns
  and an ECS for the real-time strategy module, ADR-0015 §3);
- timers: `shuyi_core::timer` (2115).

**Estimated size:** about 400 lines (code 120, tests 100, docs 180).

## Implementation steps

1. Move 2137's example sim onto `FxVec2`, with inertia, and regenerate
   its table.
2. The skill pack, and the book (2802), or a `Book:` line.

## Acceptance criteria

- [ ] The example record's hashes match the regenerated `tick.ron` in
      CI's `test` and `determinism` jobs (x64, ARM64, wasm32).
- [ ] The example dot slows down when let go rather than stopping (a
      unit test).
- [ ] The `shuyi-stage-field` skill no longer waits for 1119, and
      `check-skills` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the example dot's inertia.
- Golden: the example sim's table on `Fx`.

## Completion notes
