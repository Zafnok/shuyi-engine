---
id: "2003"
title: "Modules: the catalog by concern, module metadata, the shuyi crate with a feature per module, and shuyi modules and shuyi add"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["game#1131", "1129"]
nick_input: none
completed:
---

# 2003 — Modules: the catalog, the `shuyi` crate, `shuyi add`

## Context

Nick, 2026-10-16 (`docs/decisions.md`): "we should support plenty of
modules for our engine, so that we don't overload devs", and "a dev will
set a direction and import non-conflicting modules to help them out".
[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
says what a direction is (§2), what a module ships with (§4), and how a
game turns one on (§6). This ticket builds §4's metadata and §6, so that
the first module (physics, 1128, or any other) lands in a place that
already works.

The perspective markers (§3) come with the tickets that add
perspectives: 1120, 1122, 1126 and 1127. The guides (§5) are 2004.

It needs the engine's code here (game#1131) and the contract with
`shuyi new` (1129).

## Nick input

None.

## Scope

**In:**
- **The `shuyi` crate** (`crates/shuyi`):
  - it re-exports `shuyi-core`, `shuyi-assets`, `shuyi-ui` and
    `shuyi-app` under their short names (`shuyi::core`, …);
  - it has one cargo feature per module, all off by default. Until a
    module crate exists, it has no feature; add none in advance;
  - the example `hello-grid` (1118) depends on `shuyi` only.
- **Module metadata** (ADR-0009 §4): `[package.metadata.shuyi]` with
  `concern`, `answer`, `serves`, `alternatives` and `works_with` (the
  parked 3017 later adds `needs`, ADR-0012 §1). One module may answer
  several concerns, so `concern` and `answer` come as a list of pairs
  (the parked `shuyi-3d` answers three perspectives and *World
  drawing*). They are read
  into a `ModuleInfo` type in `shuyi-cli`. The concerns and their
  answers are the table of ADR-0009 §2, with the answers ADR-0011 §2
  adds (first person, third person, top-down 3D; the *World drawing*
  concern), kept as data in
  `docs/modules.ron`, which the catalog and the check read.
- **The catalog**, `docs/modules.md`, grouped by concern. It has a row
  per module, format plugin and platform backend: the name, the
  directions it serves, its status (planned, parked or ready) and its
  tickets. Start it with:
  - `shuyi-physics` (1128, parked);
  - `shuyi-net` (1140–1150, parked);
  - `shuyi-procanim` (2701–2704, parked);
  - `shuyi-3d`, `shuyi-app-3d` and `shuyi-diorama` (3101–3109, parked);
  - the format plugins of 2202–2204;
  - the backends `shuyi-app` and `shuyi-playdate` (3010), and
    `shuyi-gba` (3021) and `shuyi-nds` (3024), parked.
- **A check**, `cargo xtask check-modules`, run in CI and listed in the
  `run-gates` skill. It fails when:
  - a default crate (`shuyi-core`, `shuyi-assets`, `shuyi-ui`,
    `shuyi-app`, `shuyi-cli`) depends on a module, read from
    `cargo metadata`;
  - a crate under `crates/` outside the default five has no README, no
    metadata, or no catalog row;
  - a module's `concern` or `answer` isn't in `docs/modules.ron`;
  - two modules answer the same concern without naming each other in
    `alternatives`;
  - a feature of `shuyi` names no catalog row.
- **Two contract operations** (ADR-0005), as library functions with JSON
  output like the rest of 1129:
  - `shuyi modules`: the catalog by concern, plus which modules this
    game has on;
  - `shuyi add <module>`:
    - turns on the feature of `shuyi` in the game's `Cargo.toml`;
    - copies the module's skill into the game's skills folder, if the
      skill pack (1134) is there;
    - prints the first paragraph of the module's README;
    - if the game already has an alternative on, says so and links the
      README section on using both;
    - running it twice changes nothing the second time, and an unknown
      name lists the known ones.
- **`shuyi new`** templates depend on `shuyi` and turn on only the
  modules their direction needs (today: none).
- **CI** builds the default engine, each module alone, and every module
  together.

**Out (do not do):**
- writing any module (each has its own ticket);
- the perspective markers (1120, 1122, 1126, 1127);
- the guides (2004);
- `shuyi remove` (add it when someone asks);
- moving anything out of the default crates (ADR-0009 §1).

## Implementation steps

1. Add `crates/shuyi` and move `hello-grid` onto it.
2. Write `docs/modules.ron`, `ModuleInfo`, `docs/modules.md` and
   `cargo xtask check-modules`, with a fixture workspace for each
   failure.
3. Add `modules` and `add` to the contract, with integration tests on a
   game made by `shuyi new` in a temporary folder. Use two fixture
   modules that are alternatives to test the warning.
4. Add the CI jobs. Add `check-modules` to the `run-gates` skill.
5. Link `docs/modules.md` from the README.

## Acceptance criteria

- [ ] `hello-grid` builds and passes its tests depending on `shuyi`
      alone.
- [ ] `cargo xtask check-modules` passes on the repository, and fails
      on each fixture with a message naming the crate and the rule.
- [ ] `shuyi add` on a fresh game edits `Cargo.toml` once, and a second
      run reports that nothing changed.
- [ ] `shuyi add` of a module whose alternative is on prints the
      warning and the link.
- [ ] `shuyi modules --json` matches its schema (1129's schema test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the check, on fixture workspaces; `ModuleInfo` parsing.
- Integration: `shuyi modules` and `shuyi add` on a game from
  `shuyi new`.

## Completion notes
