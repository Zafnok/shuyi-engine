---
id: "2839"
title: "The book's steps resolve the engine's features alike, so its Codespace builds the engine once, not three times"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2837"]
nick_input: none
completed:
---

# 2839 — The book's steps build the engine once

## Context

2837 made the book's Codespace (`.devcontainer/book/`, ADR-0126 §1): it
builds every step of `site/tutorial/` into one target folder, so that each
step after the first builds only its own crate. Measured on a 2-core
Codespace (2837's notes), that holds for steps sharing a lock, about 10 s
each, but the ten locks resolve three sets of features of the engine's
dependencies, and each set builds much of the engine again:

- step 1 (`01-new`, as `shuyi new` makes it) has no `serde` of its own;
  step 2 adds `serde = { version = "1", features = ["derive"] }`, its
  default features on: step 1 took 73 s after step 2;
- step 7 (`07-save`) adds `ron = "0.12"`, its default features on, where
  the engine's root takes `default-features = false`: step 7 took 239 s
  after step 6.

All ten took 786 s against step 2's 404 s alone, which is why 2837 builds
step 2 before the Codespace opens and the other nine behind it. With one
set of features, the ten would take about 404 + 9 × 10 s.

## Nick input

None.

## Scope

**In:**
- Make the steps' (and, if the cause is there, the template's) manifests
  ask for `serde` and `ron` with the features the engine's crates already
  turn on, native and `wasm32-unknown-unknown` alike, so the ten locks
  resolve one set; refresh the locks. The book's pages that show those
  lines (the `snippet` shortcode's `step`) follow, and say why in a
  newcomer's words if the line grows.
- A check that fails when two steps resolve different features of a
  crate the engine depends on (`cargo metadata` of each step, `resolve`'s
  features), in `cargo xtask check-devcontainers` or the tutorial's job.
- Measure the ten builds again in a Codespace of the branch, and if they
  take about step 2's time, build them all in the book's
  `onCreateCommand` and drop the `updateContentCommand` and `waitFor`
  (2837's notes say why they are there).

**Out (do not do):**
- changing what a step teaches;
- a prebuild (Nick's, 2837).

**Estimated size:** about 200 lines (manifests and locks 60, the check and
its tests 120, the container and docs 20).

## Implementation steps

1. `cargo tree -e features -i serde` and `-i ron` in steps 1, 2 and 7, for
   both targets, to find which feature differs.
2. Ask for it alike in the steps (or the template), refresh the locks
   (the run-gates skill, *The tutorial's steps*).
3. The check, then the measure, then the container.

## Acceptance criteria

- [ ] The ten steps resolve one set of features of the engine's
      dependencies, and a check fails when they don't.
- [ ] In a Codespace, each step after the first builds in about 10 s;
      the notes have the numbers.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the check, on two steps' metadata with a feature apart.
- By hand: the ten builds in a Codespace, timed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
