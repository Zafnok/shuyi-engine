---
id: "3541"
title: "shuyi-embed's walk takes a file that vanishes mid-walk as gone, and enters no skipped folder, so a build beside it can't fail shuyi-cli's"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-26
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

- [x] A file that vanishes between `read_dir` and `metadata` is left
      out of the walk, not an error; any other read error still is.
- [x] A folder `**/target/**` matches is not entered, and its files are
      neither embedded nor watched.
- [x] `crates/cli`'s embedded tables hold the same files as before (a
      test or a before/after listing in the notes).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/embed`): the vanished entry; the skipped folder not
  entered; a partly skipped folder still entered.

## Completion notes

- `walk` (`crates/embed/src/lib.rs`) reads each entry through `kind_of`,
  and each folder's listing through `gone_is_none`: a `NotFound` is the
  entry left out (a folder gone before its `read_dir` is neither walked
  nor in `Walk::folders`, so cargo isn't told to watch a missing path).
  Any other error is still `Error::Read`.
- `walk` doesn't enter a folder a skip pattern matches whole
  (`skips_folder`: the pattern is `**`, or ends `/**` after parts that
  match the folder, as `**/target/**` does `target` and `04-push/target`).
  A pattern that skips only some of a folder's files (`**/*.png`,
  `art/*`) still enters it.
- **The decision on `Walk::skipped`:** a skipped folder lists nothing
  there, not `target/`. `Layer::leaving_out` binary-searches exact file
  paths (`crates/assets/src/bundle.rs`, `left_out`), so it doesn't read a
  folder entry that way; `skipped!`'s only use is the embed fixture
  (`crates/embed/fixture`), whose patterns skip files. `crates/cli`
  never includes a `skipped!` list. The game (`gh search code --repo
  Zafnok/visions-of-shuyi 'skipped!'`, `'shuyi_embed'`) reads only
  `walk.files` (`crates/content/src/layer_files.rs`); its `leaving_out`
  list is its own `playdate_left_out.rs`, not a walk's.
- Docs: the crate's top docs, `walk`, `Walk`, `Table::skip`,
  `Written::watched` and `skipped!` say both. ADR-0138 is accepted and
  left as it is: its §1 says what the table and list are, which holds.
- Tests (`crates/embed/src/tests.rs`): the vanished entry
  (`kind_of` on a path that doesn't exist), another error still failing
  (a NUL in the path; `gone_is_none` on a permission error), the skipped
  folder not entered, listed or watched (by `walk` and `write_to`), a
  partly skipped folder still entered, and which patterns skip a folder.
- **`crates/cli`'s tables, before and after** (each `shuyi_embed_*.rs`
  of its `OUT_DIR`, built on this worktree): all seven tables are
  byte-identical. The `examples` and `tutorial` `_skipped` lists lost the
  files of folders skipped whole (`.cargo/`, `font/`, `examples/`,
  `.sentry-native/`); nothing reads them.
- Deviations: none.

Behaviour: shuyi-embed: `walk` leaves out a file or folder deleted while it walks, where it failed; a folder a skip pattern matches whole (`dir/**`) isn't entered, so its files are no longer in `Walk::skipped`, `skipped!` or `Written::watched`, and a change in it no longer reruns the build script.
