---
id: "2836"
title: "cargo xtask exercises can check a step with the step before it's build, when their newest files share a timestamp"
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-15
---

# 2836 — `cargo xtask exercises` can check a step with the step before it's build

## Context

`cargo xtask exercises` (ticket 2828, [ADR-0117](../../docs/adr/0117-exercises-checks-spliced-into-a-copy-of-each-step-and-run-by-its-web-build.md))
copies each of the book's steps into `target/exercises/<step>`
(`crates/xtask/src/exercises.rs`, `copy`, with `tries::copy_tree`) and
builds every copy in one target folder, `target/exercises-build`.

Every step is the same package, `box-pusher` 0.1.0 at `crates/game` in
a workspace of its own, with the same manifest and, from step 07 on, the
same lock. Cargo hashes a path package by its path from the workspace's
root, so the copies of two steps share every unit's hash and output
files. Cargo tells them apart only by their fingerprints. The
fingerprint of a build script's run is the newest file in the package:
its time and its path (`Precalculated`, as
`target/exercises-build/debug/.fingerprint/box-pusher-*/run-build-script-build-script-build.json`
shows). On Windows `fs::copy` keeps a file's time, so a copy's newest
file is the step's.

Ticket 2087 rewrote the ten steps' `crates/game/Cargo.toml` with one
script, within one tick of Windows' clock. Steps 08 and 09 got the same
newest file (`Cargo.toml`) at the same time. In one run, 09's copy was
taken as fresh after 08's and checked with 08's build of the game,
which has no `tools` module:

```text
error: 09-test: `cargo clippy --locked -q --bin box-pusher --tests -- -D warnings` failed:
error[E0432]: unresolved import `game::tools`
```

Run alone (`cargo xtask exercises 09-test`), or after giving the
manifests distinct times, it passes. CI passed on `main` at the same
time. The same can happen on any Windows checkout whose steps' newest
files share a time, and to any later edit that writes the steps
together. `cargo xtask tutorial-tries` copies steps with the same
`copy_tree` and may share the fault.

## Nick input

None.

## Scope

**In:**
- `cargo xtask exercises` checks each step with that step's own build,
  whatever the times of its files. The fix in its order of preference:
  1. a copy whose package Cargo can't confuse with another step's, for
     example each step built with its own `--config` that changes the
     units' hash (`build.rustflags` stays the same: a change there
     rebuilds every dependency per step);
  2. or each copy's files given the time of the copy, newest last, so
     two steps' fingerprints always differ.

  Measure the run's time before and after on a warm target folder, and
  say it in the notes: the fix must not rebuild the engine for each step.
- The same check of `cargo xtask tutorial-tries`: fix it the same way
  if its steps share a target folder, or say in the notes why they
  can't collide.
- A test: two copies of a stand-in step whose newest files share a
  time, built in one target folder, each checked with its own code.

**Out (do not do):**
- Changing the steps or their locks.

## Implementation steps

1. Reproduce: in a copy of two steps with identical manifests, set the
   same time on both `crates/game/Cargo.toml`, build one, then check the
   other.
2. Fix `exercises.rs` (and `tries.rs` if it shares the fault) as Scope
   says.
3. The test, under `crates/xtask/src/exercises/` beside the others.

## Acceptance criteria

- [x] Steps 08 and 09 with the same time on their newest file pass
      `cargo xtask exercises` in one run.
- [x] A warm run of `cargo xtask exercises` takes no more than a tenth
      longer than before (the notes give both times).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the two-copies case above, in a temporary folder.

## Completion notes

Done with the fix Scope prefers first. `cargo xtask exercises` runs each
step's Clippy and tests with a `--config` of its own,
`profile.dev.package.box-pusher.codegen-units=<257 + the step's place>`
(`exercises::own_build`). A package's profile is part of its units' hash,
so each step's game gets units and output files of its own in
`target/exercises-build`, and no fingerprint can take one step's build
for another's. The engine's crates keep their profile and build once.
`cargo fmt` gets no config: it builds nothing. A debug build barely sees
the count (its default is 256).

- **Reproduced** in the new test: two stand-in steps (`box-pusher` at
  `crates/game`, a build script, a binary printing its step), every file
  of both at the same time, built in one target folder. With one config
  for both, the second printed the first's `08`; with `own_build`, each
  prints its own, and again after switching back.
- **Steps 08 and 09:** both `crates/game/Cargo.toml` set to the same time
  (2030-01-01, so it is each copy's newest file), then `cargo xtask
  exercises`: every step passed, three runs in a row.
- **Time, warm target folder, this machine:** before, 1 min 40 s and
  1 min 31 s (every run rebuilt each step's game, since the folder held
  only the last step's). After: the first run, filling each step's own
  units, 1 min 51 s; then 43 s and 37 s, about two and a half times
  faster. A cold run was 5 min 46 s before. The cost is disk: ten builds
  of the game's own crate in place of one, against one of the engine.
- **`cargo xtask tutorial-tries` can't collide:** its `Renderer` already
  sets every file of a copy to the current time each time the target
  folder switches to it (`switch_to`, `touch_tree`, ticket 2822), so a
  copy's newest file is always newer than any build before it, and its
  sources newer than every output. It builds in its own folder
  (`target/tutorial`), never in `target/exercises-build`. Left as it is.

No break: xtask only.

Book: none, the change is to `cargo xtask exercises` only, no public API
or anything a reader runs.
