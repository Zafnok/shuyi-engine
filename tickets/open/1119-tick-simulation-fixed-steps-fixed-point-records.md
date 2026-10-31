---
id: "1119"
title: "Parked: Real time: fixed-point numbers and an entity arena for tick simulations"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["game#1131", "2116", "2137"]
nick_input: none
completed:
---

# 1119 — Fixed-point numbers and an entity arena

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

It needs the engine's code in this repository (the game's ticket 1131).

## Nick input

None (technical). Scheduling: see above.

## Scope

**In:**
- `shuyi_core::fx`: `Fx` (i32, 8 fraction bits), `FxVec2`, with
  arithmetic that rounds toward negative infinity, saturating where
  documented, `from_px`, `floor_px`, and sine and cosine in `Fx` from
  2335's `shuyi_core::angle` (its table shifted from 16 fraction bits
  to `Fx`'s 8, rounding toward negative infinity), not a table of its
  own.
  - **Either order with 2335:** if 2335 isn't done when this starts,
    write `shuyi_core::angle` as 2335's scope says (the `Angle` type and
    its table only), and 2335 builds on it.
  - **Either order with 2109:** if 2109 is done, implement its
    `shuyi_core::ease::Lerp` for `Fx` and `FxVec2` here, rounding as
    `Fx` does, with a test that `mix` hits both ends exactly; if not,
    2109 adds them.
  - **Either order with 2118:** if 2118 is done, add a one-line
    `shuyi_procgen::noise::to_fx`, a noise value as an `Fx` (`n >> 8`),
    with a test; if not, nothing to do (2118 adds it).
- `shuyi_core::arena`: `Arena<T>` with generational `Id<T>`, insert,
  remove, get, iteration in id order, serde, and a text dump.
- **2137's example sim moves onto `Fx`:** the dot's position and
  velocity become `FxVec2`, with inertia (it slows down when let go
  rather than stopping). Its golden table
  (`crates/core/tests/golden/tick.ron`) is regenerated in this PR, and
  the Completion notes say so (a `Breaking:` line if 2137's table was
  public by then, ADR-0038).
- **The skill pack. From 1134 (done):**
  `skills/shuyi-stage-field/SKILL.md` lists 1119 in its `waits_for`,
  and `check-skills` fails once this ticket is in `done/`: drop 1119
  from that list, and teach this ticket's work in the skill (`Fx` and
  `Arena`; 2137 taught fixed steps, `TickSim`, records, `replay` and
  `TickDriver`). If that empties the list, write the skill's worked
  example, `skills/shuyi-stage-field/example.ron`, which CI's `skills`
  job runs on a fresh `shuyi new` game.

**Out (do not do):**
- `shuyi_core::tick`, `TickDriver` and its latch, the debug view's and
  debug commands' tick parts, and the ECS note: 2137;
- collision (1122);
- world layers (1120);
- networking;
- thousands of units (the parked 2705 measures an arena against columns
  and an ECS for the real-time strategy module, ADR-0015 §3);
- timers: `shuyi_core::timer` (2115).

## Implementation steps

1. `Fx` with property tests: the algebra holds within documented
   rounding; a golden table of a fixed list of operations,
   `crates/core/tests/golden/fx.ron`, through 2116's
   `shuyi_assets::golden` (a dev-dependency on `shuyi-assets` with
   `default-features = false, features = ["golden"]`, so the no-std run
   stays without `std`). CI's `test` job checks it on x64 and ARM64,
   and its `determinism` job on wasm32, which runs `shuyi-core`'s
   tests.
2. `Arena` with property tests (ids never reused while live; order
   stable). If 2904 is done, read its ADR first. `Arena<T>` follows what
   it says the arena shares with `shuyi-ecs`: kits name neither store,
   and scenes (2221) get an arena side only if the ADR calls for it, as
   a follow-up ticket written here (the `write-ticket` skill), not in
   this PR.
3. Move 2137's example sim onto `FxVec2` and regenerate its table.
4. The skill pack, and the book (2802), or a `Book:` line.

## Acceptance criteria

- [ ] `Fx` results and the example record's hashes match their golden
      tables in CI's `test` and `determinism` jobs (x64, ARM64, wasm32).
- [ ] `Arena`'s property tests pass, and a snapshot of its text dump.
- [ ] The `shuyi-stage-field` skill no longer waits for 1119, and
      `check-skills` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Property: `Fx`'s algebra; `Arena`'s ids and order.
- Golden: `fx.ron`, and the example sim's table on `Fx`.
- Snapshot: the arena's text dump.

## Completion notes
