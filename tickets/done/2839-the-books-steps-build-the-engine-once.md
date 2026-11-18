---
id: "2839"
title: "The book's steps resolve the engine's features alike, so its Codespace builds the engine once, not three times"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2837"]
nick_input: none
completed: 2026-11-17
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
- a prebuild (Nick's, 2837);
- how CI builds the steps' web builds (3502).

**From 3502 (open):** 3502 isn't done, so CI's `web-tutorial` job
doesn't exist yet and nothing is measured there; 3502 measures with this
ticket's locks when it lands.

**Estimated size:** about 200 lines (manifests and locks 60, the check and
its tests 120, the container and docs 20).

## Implementation steps

1. `cargo tree -e features -i serde` and `-i ron` in steps 1, 2 and 7, for
   both targets, to find which feature differs.
2. Ask for it alike in the steps (or the template), refresh the locks
   (the run-gates skill, *The tutorial's steps*).
3. The check, then the measure, then the container.

## Acceptance criteria

- [x] The ten steps resolve one set of features of the engine's
      dependencies, and a check fails when they don't.
- [x] In a Codespace, each step after the first builds in about 10 s;
      the notes have the numbers.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the check, on two steps' metadata with a feature apart.
- By hand: the ten builds in a Codespace, timed.

## Completion notes

**What was done:**

- **The cause** (`cargo tree -e features` of each step, Linux and
  `wasm32-unknown-unknown`): the only difference was the name of a
  `default` feature. Steps 2–10 asked for `serde` with its default
  features, and steps 7–10 for `ron` with theirs; each `default` adds
  nothing the engine doesn't already turn on (`std`), but the feature
  list is part of every crate's build hash, so `serde` (and everything
  above it, the engine included) built again for step 2's set, and `ron`
  for step 7's.
- **The steps' manifests** (02–10) now ask
  `serde = { version = "1", default-features = false, features = ["derive", "std"] }`
  and `ron = { version = "0.12", default-features = false, features = ["std"] }`:
  what the engine's crates turn on. Every crate the ten steps build, the
  engine's own included, now resolves one set of features on Linux, the
  web and Windows. The locks didn't change (features aren't in a lock).
  No template change: `shuyi new` makes step 1, which has neither line.
- **The book:** step 2's page gives the new `serde` line, with a
  newcomer box on `default-features = false` (asks for exactly what the
  engine uses, so serde is built once); step 7's gives the `ron` line and
  says why in a sentence.
- **The check** (`crates/xtask/src/devcontainers/features.rs`, run by
  `cargo xtask check-devcontainers`, so in CI's `tickets` job and the
  `tickets' checks` gate): `cargo tree --locked --edges normal,build
  --target <t> --format '{p}|{f}'` in each step for both targets, the
  step's own crates dropped, and an error naming the two steps, the crate
  and both feature sets when a crate differs. A crate one step adds is
  fine. Six unit tests on two steps' trees (a feature apart, alike, an
  added crate, a step's own crate, the host's and the target's sets).
  It takes about 20 s locally. Checked by hand: step 7's old `ron` line
  makes it fail on both targets.
- **The container:** the `onCreateCommand` builds all ten steps, then
  serves step 2; the `updateContentCommand` and `waitFor` are gone.

**Measured** on a 2-core Codespace (`basicLinux32gb`, US East), made
from the branch at `156c8cd3` (on `main` at `241d2607`; the `main` merged
after brings tickets 2840 and 2841 only, nothing the container, the
steps or the check read), with no prebuild:

| What | Time (2837's) |
| ---- | ---- |
| Step 1 from an empty target folder | 357 s (step 2: 404 s) |
| Then each of steps 2–10 | 7–11 s (step 1 73 s, step 7 239 s) |
| All ten (`build.sh`) | 440 s (786 s) |
| The target folder | 1.7 GB (2.7 GB) |
| Created to *Available*, every step built and step 2 served | 8 min 58 s (7 min 36 s for step 2 alone; 12 min 43 s for all ten) |

Locally (24 cores, Windows), into one target folder after step 2: steps
1, 7 and 10 each compiled only their own crate (10–14 s).

**Deviations:**

- The check reads `cargo tree`, not `cargo metadata`'s `resolve`: the
  latter merges every target's and dev-dependency's features, and showed
  step 1 and step 2 alike where the build differed.
- It runs in `run_features`, beside `devcontainers::run`, so the fixture
  trees of 2837's tests need no cargo project.

**Codespace made:** `symmetrical-orbit-7v959gg9r5w2pr9q`, deleted; the
scratch branch `scratch-2839-one-build` it was made from, deleted.

No ticket: `templates/shuyi-default/stages/top-down-grid/Cargo.toml.tmpl`
asks for `serde` and `ron` with their default features too, but in a
game's own crate, with no shared target folder to rebuild; it costs a
game nothing it would build once anyway.

**Handoffs:** 3502 now says the ten steps resolve one set of features,
so its steps 1 and 7 should build as fast as the others. 2838 and 2840 now say the ten steps build before the
Codespace opens, in one command.

Book: step 2's and step 7's pages give the new lines and say why.

Game: nothing to adopt, the change is the book's steps, a dev container
and an xtask check; no crate changed.

No break. No `Behaviour:` line: no crate changed.
