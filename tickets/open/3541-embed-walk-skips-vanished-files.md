---
id: "3541"
title: "shuyi-embed's walk takes a file that vanishes mid-walk as gone, and enters no skipped folder, so a build beside it can't fail shuyi-cli's"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3541 — shuyi-embed's walk takes a file that vanishes mid-walk as gone, and enters no skipped folder

## Context

3412 (#606) made `shuyi-cli`'s build script (`crates/cli/build.rs`)
embed `site/tutorial/` as the `tutorial` table, skipping `**/target/**`
(`NOT_TEXT`). CI's `tutorial` job builds each step in its own
`site/tutorial/<step>/target/`, and the step's crates build `shuyi-cli`
there too. Both builds run at once, and `shuyi_embed::walk`
(`crates/embed/src/lib.rs`, about line 111) still enters every skipped
folder and stats each file in it (`fs::metadata`, about line 138). It
also lists the skipped files, which `write_to` watches with
`rerun-if-changed`. A file that cargo deletes between `read_dir` and
`metadata` then fails the whole build. That happened on PR #612's
`tutorial (04-push)` job on 2026-11-26 (run 45885756624, job
140135972217):

```text
shuyi-embed: table `tutorial`: …/site/tutorial/04-push/target/debug/build/box-pusher/…/out/build_script_build.….rcgu.o: No such file or directory (os error 2)
```

The other steps' jobs, and other PRs' runs, passed: it is a race, and it
can hit any PR.

## Nick input

None.

## Scope

**In:**
- `walk`: an entry that is gone by the time it is read (`NotFound` from
  `fs::metadata`, or from `fs::read_dir` on a folder found a moment
  before) is left out, as if the listing hadn't held it. Any other error
  still fails.
- `walk`: a folder a skip pattern matches whole (`**/target/**` matches
  `target/x` for every `x`) isn't entered. Its path is listed once in
  `Walk::skipped` as the folder (`target/`) if `skipped!` and
  `Layer::leaving_out` read it that way, else nothing changes for them.
  Decide which from their uses (`grep -rn "skipped!" crates`), and say
  so in the completion notes. Each file in it is no longer watched by
  `rerun-if-changed`, so a build there no longer reruns the build
  script.
- The crate's docs say both.

**Out (do not do):**
- Changing `crates/cli/build.rs`'s tables or patterns.
- Changing CI's `tutorial` job (running the steps one at a time, or
  building `shuyi-cli` elsewhere).

**Estimated size:** about 200 lines (code 60, tests 120, docs 20).

## Implementation steps

1. Read `walk`, `Table::write_to` and `matches` in
   `crates/embed/src/lib.rs`, and the uses of `Walk::skipped` and
   `skipped!` across the workspace.
2. Tests first, in the crate's tests: a walk over a folder whose file
   is deleted between listing and reading is not an error. To get there
   without timing, pull the per-entry read into a function the test can
   hand a path that doesn't exist. A skipped folder's files are not
   walked, and a pattern that matches only some of a folder's files
   still enters it.
3. The two changes; the docs.

## Acceptance criteria

- [ ] A file that vanishes between `read_dir` and `metadata` is left
      out of the walk, not an error; any other read error still is.
- [ ] A folder `**/target/**` matches is not entered, and its files are
      neither embedded nor watched.
- [ ] `crates/cli`'s embedded tables hold the same files as before (a
      test or a before/after listing in the notes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/embed`): the vanished entry; the skipped folder not
  entered; a partly skipped folder still entered.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
