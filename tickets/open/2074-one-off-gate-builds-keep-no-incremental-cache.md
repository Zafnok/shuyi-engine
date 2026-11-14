---
id: "2074"
title: "The gates' one-off games build without incremental caches, the skill examples in a target folder of their own"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2074 — The gates' one-off games build without incremental caches, the skill examples in a target folder of their own

## Context

Every worktree's `target/` is about 25 GB on Nick's machine, and he runs
about 30 worktrees, so the disk fills each week. A study on 2026-11-13
measured where it goes (worktree `release-game-tag-automation`, 26.3 GB):

- `target/gates/games` (the template demo games of the `templates`
  gate, `crates/xtask/src/gates.rs:259-282`): 2.64 GB, of which
  incremental 1.77 GB.
- `target/tutorial` (the tutorial tries' renderer,
  `crates/xtask/src/tries.rs:377`, its builds at `:487`): 2.63 GB, of
  which incremental 1.72 GB.
- The skill examples' games (`cargo xtask skill-examples`,
  `crates/xtask/src/main.rs:352`) build in the root `target/`. They are
  another workspace, so the engine's crates get other hashes there:
  about 0.9–1.2 GB of extra engine-crate incremental in `target/debug`
  (0.88 GB measured for one skill).

These games are made fresh for each gate run and never edited, so an
incremental cache only costs disk there. Measured with
`CARGO_INCREMENTAL=0` in a demo game's own target folder: 1.07 GB to
0.50 GB, cold build 152 s to 100 s, warm rerun 15 s to 22 s (noise
under load).

Incremental stays **on** for the workspace itself: Nick's edit loop
would be about 2x slower without it (measured: a test rebuild after a
small edit 16–30 s on, 53–62 s off), about 7 minutes per median ticket.

`CARGO_INCREMENTAL` is part of each unit's hash, so setting it for one
command in a target folder another command shares makes a second copy
of every workspace unit (measured 3.41 to 8.14 GB). That is why it goes
only on folders with one writer, and why the skill examples move to a
folder of their own.

Related: 2075 (compression) and 2076 (the idle sweep) shrink what
stays. 2076 writes the ADR for the whole policy.

## Nick input

None.

## Scope

**In:**
- `CARGO_INCREMENTAL=0` on each step of the `templates` gate that builds
  in `target/gates/games`.
- `CARGO_INCREMENTAL=0` on the tutorial renderer's builds, **only when
  xtask chose the target folder** (`target/tutorial`). A
  `CARGO_TARGET_DIR` from the environment (CI may set one) is left as
  it is.
- The skill examples: the maker (`cargo run -p shuyi-cli -- new`) stays
  in the root target; each game builds in `target/skill-examples-build`
  with `CARGO_INCREMENTAL=0`, when `CARGO_TARGET_DIR` is not set.
- The run-gates skill's list of target folders says this.

**Out (do not do):**
- Turning incremental off for the workspace (`[profile.dev]`) or in
  the templates' `Cargo.toml` (ADR-0049's compile budget: measured
  3.09 s against 2.9 s).
- `compile_times` (it must time what a real game does) and `api.rs`
  (CI only).
- Deleting these folders after a gate run (that costs 6–8 minutes on
  the next run). 2076 trims idle ones.

## Implementation steps

1. `crates/xtask/src/gates.rs:264-282`: add `CARGO_INCREMENTAL=0`
   beside `CARGO_TARGET_DIR={root}/target/gates/games` on each step, in
   the same `VAR=value` form the step parser already reads.
2. `crates/xtask/src/tries.rs`: give `Renderer` a flag for "xtask
   chose the folder" (set in `Renderer::new` when `CARGO_TARGET_DIR` is
   unset), and add `.env("CARGO_INCREMENTAL", "0")` at `:487` only
   then.
3. `crates/xtask/src/main.rs:346-356` and `crates/xtask/src/skills.rs`:
   split the one `script::System` runner into a maker and a game
   runner, as `compile_times` already does (`main.rs:378-381`). The
   game runner builds in `target/skill-examples-build` with
   `CARGO_INCREMENTAL=0`. Keep the rule in `skills.rs`' module doc that
   two games never share a crate's name, and update the doc's "builds
   in this repository's target folder".
4. Update `.claude/skills/run-gates/SKILL.md` (and run
   `pwsh -File scripts/sync-agent-skills.ps1`).

## Acceptance criteria

- [ ] After `cargo xtask gates` from a clean `target/`, these hold no
      incremental output: `target/gates/games/debug/incremental`,
      `target/tutorial/debug/incremental`,
      `target/skill-examples-build/debug/incremental`.
- [ ] The root `target/debug/incremental` gets no dir of a skill game's
      crates (none named after a skill, e.g. `shuyi_gates*`) and no
      second engine-crate dir from the skill games (compare the dir
      names before and after `cargo xtask skill-examples`).
- [ ] With `CARGO_TARGET_DIR` set, the tutorial renderer and the skill
      examples don't set `CARGO_INCREMENTAL`.
- [ ] The completion notes give each folder's size before and after one
      full gates run.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each `templates` gate step that names `target/gates/games`
  also carries `CARGO_INCREMENTAL=0`.
- Unit: the tutorial renderer sets `CARGO_INCREMENTAL` only when it
  chose the folder.
- Unit: the skill examples' game runner uses
  `target/skill-examples-build` and the maker the root target, and
  neither changes with `CARGO_TARGET_DIR` set.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
