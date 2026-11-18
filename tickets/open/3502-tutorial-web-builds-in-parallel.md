---
id: "3502"
title: The tutorial's web builds boot in a job of their own, each step built from one folder so the engine builds once, and web-render takes under 8 minutes
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3502 — The tutorial's web builds boot in a job of their own, each step built from one folder so the engine builds once, and web-render takes under 8 minutes

## Context

The retro of PRs #379–#484 (2026-11-17) found CI's `web-render` job to
be its critical path. One step does most of that:

- `web-render` runs "Every tutorial step's web build boots"
  (`.github/workflows/ci.yml`, the loop over `target/exercises/*/`, added
  with the book's project, 2803 and 2828).
- That step runs `cargo shuyi web --release` on each of the ten steps,
  one after another, then boots each page with `tools/web-check/check.py`.

Measured on 8 green PR runs of 2026-11-17 (`gh run view <id> --json jobs`,
the steps' `startedAt`/`completedAt`; runs 44171658437, 44169652310,
44149670628, 44133210416, 44043765930, 44040072899, 44030872636,
44026989504):

| | min | max |
| --- | --- | --- |
| `web-render` | 15 m 45 s | 20 m 6 s |
| its tutorial loop | 583 s | 761 s |
| the next-longest jobs (`api`, `tutorial-tries`, `test (windows-latest)`, `bench`, `coverage`) | 6 min | 9.5 min |

The rest of `web-render` takes about 6.4 minutes: the setup is about 78 s,
then the examples, the packed export, the plan and the render card
(run 44171658437).

**Why the loop is slow.** In run 44171658437's log
(`gh api repos/Zafnok/shuyi-engine/actions/jobs/134352462296/logs`),
every step compiles the whole dependency tree for wasm32 again
(`serde_core`, `libm`, …, `macroquad`, every engine crate), although all
ten steps share `CARGO_TARGET_DIR=target/tutorial`. Step 3 took 38.9 s of
compiling and step 7 took 98 s; with the boot, each step takes about 41 s.

The cause:

- A release web build remaps the workspace's root to the package's name
  through `CARGO_ENCODED_RUSTFLAGS` (`crates/cli/src/web.rs`, the release
  branch around `paths::remaps`; `crates/cli/src/export/paths.rs`).
- Each step's copy is its own folder, `target/exercises/<step>/`, so each
  step has different flags.
- The flags are part of every unit's hash (as ticket 2079 found for
  `RUSTFLAGS`), so no unit of one step is reused by the next.

Steps 1 and 7 cost more again because their locks resolve other features
of `serde` and `ron`. That is 2839's subject.

**After this ticket**, the critical path is expected to be `bench`. It
waits for the whole `export` matrix (`needs: [changes, export]`, though it
reads only `package-web`) and then runs for about 6–7.5 minutes. In the
runs above it finished 12–15.6 minutes after the run started. A whole PR
run in about 12 minutes needs that chain shortened too. That is not this
ticket's job (**Out**).

## Nick input

None.

## Scope

**In:**
- **One folder per build.** CI builds every tutorial step's web build
  from the same folder, so the remap and the flags are the same for
  every step and the dependencies and engine crates build once per job.
  For each step:
  1. empty the folder, for example `target/exercises/web/` (three
     folders down, like a step, so its path dependencies still resolve);
  2. copy the step's copy into it;
  3. run `cargo update -p shuyi -p shuyi-cli -p shuyi-app && cargo shuyi
     web --release` as today;
  4. move its `dist/web` out, to `target/web/tutorial-<step>/`, and boot
     that with `check.py` as today.

  Measure the loop before the next item and put the per-step times in
  the notes.
- **A job of its own**, `web-tutorial`, which takes the loop out of
  `web-render` and runs beside it. Its steps:
  - checkout, `rustup show`, rust-cache, `scripts/setup-linux.sh`, the
    checker and Chromium, as `web-render` sets them up;
  - `cargo xtask exercises --copy`;
  - the loop.

  If the loop in one folder measured over 4 minutes, the job is a matrix
  of two legs (`shard: [1, 2]`). Each leg takes every other step folder
  (`ls -d target/exercises/*/ | sort | sed -n "${SHARD}~2p"`, or the
  same in another form), so a new step joins a leg without a list to
  keep. Each leg uses its own one folder.
  - `needs: changes`, `if: needs.changes.outputs.code == 'true'`, like
    `web-render`.
  - It is added to `ci-result`'s `needs`.
  - It uploads its consoles and `target/web-check/`, as `web-render`
    does, under the artifact name `web-tutorial` (`web-tutorial-<shard>`
    with legs).
- `crates/xtask/src/gates.rs`: a CI-only gate for the new job ("needs
  headless Chromium and Playwright"), since a test fails on a `ci.yml` job
  with no gate (`every_ci_job_is_a_gate_with_its_filter`).
- ADR-0053 §5 says "One CI job, `web-render`". Amend it (the `write-adr`
  skill): the tutorial's web builds boot in `web-tutorial`, and why
  (this ticket's numbers).
- Docs that name the loop in `web-render`:
  - the `run-gates` skill, *The web build in a browser*;
  - `crates/cli/web/README.md`, *Checked in CI*;
  - `site/README.md` (its two mentions of `web-render` booting the steps);
  - ADR-0117's line saying `web-render` boots the copies (an amendment
    note, as for ADR-0053).

**Out (do not do):**
- The `bench` chain (`bench` waiting for the whole `export` matrix). It
  is the next critical path, and it needs its own ticket and measurement.
- Making the ten steps resolve one set of features. That is 2839 (see
  below).
- Changing `shuyi web`'s remapping (the `trim-paths` Cargo lacks on the
  pinned Rust; `crates/cli/src/export/paths.rs` says why it is rustc's
  `--remap-path-prefix`). A game builds from one folder, so only this
  loop pays for it.
- `pages.yml`'s build of the steps (it runs on a push to `main`, not on a
  PR).
- Any other step of `web-render`.

**Either order with 2839:** if 2839 is done, the ten steps resolve one
set of features, so measure with it: steps 1 and 7 should take about as
long as the others after the first; if not, nothing to do (2839 measures
this job again when it lands, and this ticket's bounds hold without it).

**Estimated size:** about 250 lines (the workflow 90, gates.rs 10, the
ADR amendments and docs 150).

## Implementation steps

1. In `web-render`'s loop (`ci.yml`), build each step from one folder as
   Scope says. Push, and read the per-step times from the job's log. A
   step after the first should no longer compile `serde_core` or
   `macroquad`. If it still does, find which flag or feature differs
   (`CARGO_LOG=cargo::core::compiler::fingerprint=info` on two steps)
   before going on.
2. Move the loop and the copy step into a new job, `web-tutorial`, with
   the setup steps it needs. Use two legs if step 1 measured over
   4 minutes. Add it to `ci-result`'s `needs`.
3. Add its CI-only gate to `gates.rs` and run `cargo test -p xtask
   gates`.
4. Amend ADR-0053 §5 and ADR-0117, and update the docs listed in Scope.
5. Measure: the job times over 10 PR runs after the change, or 10 runs
   of one commit (`calibrate.yml` offers only reusable jobs, so use the
   PR's own pushes or re-runs). Put each job's median, and the run's new
   critical path with its time, in the notes.

## Acceptance criteria

- [ ] In the new job's log, the second and later steps of a leg compile
      no dependency from crates.io, only the step's own crates (and, until
      2839 lands, what a change of features rebuilds).
- [ ] `web-render`'s median over 10 PR runs is 8 minutes or less.
- [ ] Each leg of `web-tutorial` has a median of 8 minutes or less over
      the same runs.
- [ ] Every tutorial step's web build is still built with `--release`,
      from the copy with its exercises' checks, and booted by `check.py`.
      A step that fails to build or boot still fails the job.
- [ ] `ci-result` needs `web-tutorial`, and `cargo xtask gates --list`
      lists it as CI-only.
- [ ] The notes give the measured medians and the run's new critical
      path with its time (`bench` is expected).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `gates.rs`' `every_ci_job_is_a_gate_with_its_filter` passes with
  the new job.
- CI: a scratch commit that breaks one step's web build (for example a
  `compile_error!` in one step's `crates/game/src/main.rs` under
  `site/tutorial/`) fails `web-tutorial`, naming the
  step. Drop the commit, and name its run in the notes.
- Measured: the 10-run medians above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
