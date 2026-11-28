---
id: "2858"
title: "The book's project steps written once: step 1 and a patch per step, the step folders generated and checked"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-28
---

# 2858 — The book's project steps written once: step 1 and a patch per step

## Context

Found by the redundancy sweep of 2026-11-28. The book's project (the box
pusher) is ten full copies of one growing game, `site/tutorial/01-new`
to `10-ship` ([ADR-0117](../../docs/adr/0117-exercises-checks-spliced-into-a-copy-of-each-step-and-run-by-its-web-build.md),
[ADR-0126](../../docs/adr/0126-the-code-pad-a-codespace-of-the-book-not-a-build-server.md)).
Each step is the one before it plus that chapter's change.

- **Measured** (`find <step> -type f ! -name Cargo.lock | xargs cat | wc -l`):
  1,370, 1,834, 1,917, 1,986, 2,101, 2,383, 2,468, 2,795, 3,179 and
  3,177 lines, about 23,200 hand-kept lines in all.
- **The real changes are small.** `diff -rNu` between consecutive steps
  (lock files left out) totals about 3,900 lines. Step 1 plus nine
  patches is about 5,300 lines.
- **The cost of the copies shows.** A change to how a game starts has to
  be made by hand in every step. 2850 exists because two template
  changes (2225, 3425) reached none of the ten copies and nothing failed.
- **The ten `Cargo.lock` files** hold 14,853 lines but only three
  distinct contents.

The step folders are read in many places:
- the book's `snippet()` and `try_it()` shortcodes (`crates/xtask/src/book.rs`,
  `tries.rs`, `site/tutorial/tries.toml`);
- the exercises' copies (`crates/xtask/src/exercises.rs`);
- CI's `tutorial` matrix (`.github/workflows/ci.yml`, about line 505,
  with `git diff --exit-code` after the build);
- `pages.yml`'s web builds;
- the Codespace (`crates/xtask/src/devcontainers.rs`);
- GitHub links from each page ("Its whole project").

So the folders stay where they are, checked in, and become generated
output of one source.

## Nick input

None.

## Scope

**In:**
- **The source.** `site/tutorial/01-new/` stays hand-written. Steps
  02–10 get one patch each, `site/tutorial/patches/NN-<name>.patch`, the
  unified diff from the step before (lock files left out).
- **`cargo xtask tutorial-steps`** has three modes:
  - `--write` rebuilds folders 02–10 by applying the patches in order
    to a copy of 01. It copies each step's `Cargo.lock` from the lock of
    its step's group.
  - `--check` fails when a folder differs from what `--write` would make,
    naming the first differing file.
  - `--record NN` rewrites patch NN, and patch NN+1 when there is one,
    from the folders as edited. You edit the generated folder, then
    record it.
- **The lock files.** Three hand-kept locks under
  `site/tutorial/locks/`, and a table in the tool saying which steps
  take which.
- **`pr-size` counts the generated folders apart** (`is_generated` in
  `crates/xtask/src/pr_size.rs`). It counts 02–10, but not 01, the
  patches or the locks.
- **CI's `tutorial` job** runs `--check` first.
- **The docs.** The docs map's row for the book's project, and the
  `document` skill's line for it, say how to change a step.
- **Either order with 2850:** if 2850 is done, nothing to change here,
  because its new-game files are already in every folder and `--record`
  keeps them. If not, 2850 makes its change once in `01-new` and runs
  `--write`, as its own Scope says.
- **An ADR amends ADR-0117 and ADR-0126:** the folders are generated,
  and the patches are the source.
- **Estimated size:** about 1,000 lines (code 450, tests 350, docs 200).
  On top of that come about 3,900 lines of patches, which are the
  folders' existing diffs recorded by the tool, not new writing.
- **Split:** past 1,500 counted lines of code, tests and docs, split
  `--record` and the docs off into a new ticket. Keep `--write` and
  `--check` here. The patches' own lines get a `Size:` line in the
  completion notes saying they are the recorded diffs.

**Out (do not do):**
- **Deleting the step folders and building them only in CI and the
  Codespace.** That breaks the book's GitHub links and the Codespace
  ("Its whole project"), and a reader would lose the folder. That is a
  product change for Nick, not this ticket's.
- Changing what any step contains: `--check` must pass on the folders
  exactly as they are before this ticket.
- The exercises' copies (`exercises.rs`): they read the folders as
  before.

## Implementation steps

1. Write `crates/xtask/src/tutorial_steps.rs`. It applies a patch with
   `git apply --directory` in a scratch copy, then compares or copies the
   tree. Use the workspace's walker (`shuyi_embed::walk`) and
   `remove_dir_all`, never a shell `rm`.
2. Record the nine patches from today's folders with `--record` (or
   `git diff --no-index`). Check that `--write` reproduces every folder
   byte for byte.
3. Move the locks to `site/tutorial/locks/`, using the three distinct
   contents. `--write` copies them in.
4. Add `--check` to CI's `tutorial` job, before the build, and to
   `cargo xtask gates --list` as a CI gate.
5. Extend `is_generated` in `pr_size.rs` to `site/tutorial/0[2-9]-*/`
   and `site/tutorial/10-*/`, and add a test.
6. Write the ADR (the `write-adr` skill), the docs map row and the
   `document` skill line.

## Acceptance criteria

- [x] `cargo xtask tutorial-steps --check` passes on the folders as they
      are on `main` before this ticket.
- [x] An edit to a generated folder that is not recorded fails `--check`
      and names the file.
- [x] Changing a line in `01-new` and running `--write` changes it in
      every later step that has the line.
- [x] `--record 03` after an edit in `03-walk` gives patches 03 and 04
      that reproduce 03 and 04.
- [x] `cargo xtask pr-size` counts a change to `site/tutorial/05-undo/`
      as generated, and one to `01-new` or a patch as hand-written.
- [x] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: patch application into a scratch tree, the tree comparison, the
  lock table, and `is_generated`.
- Integration: `--write` then `--check` on a copy of the real steps.

## Completion notes

**Done.** `cargo xtask tutorial-steps` (`crates/xtask/src/tutorial_steps.rs`,
ADR-0168) makes the book's project's steps 02–10 from `site/tutorial/01-new/`
and nine patches, `site/tutorial-steps/patches/<step>.patch`, each step's
`Cargo.lock` from one of three locks in `site/tutorial-steps/locks/`
(`LOCK_GROUPS`: 01; 02–06; 07–10). `--write` makes the folders, `--check`
names the first file of a folder that differs, `--record NN` writes the
patches of step NN and NN+1 from the folders as edited, and `--locks` keeps
the locks after a refresh. The patches were recorded by the tool from the
folders on `main` (5fed216f), and `--write` reproduces every folder byte for
byte: `git status` is clean after it. `git apply` runs with no repository
found around its scratch folder and with the settings that change a patch
pinned (ADR-0168 §5).

- The `tutorial steps` gate runs `--check` with `--fast` when a step, its
  source or the tool changed; in CI, the `tickets` job runs it.
- `pr-size` counts `site/tutorial/NN-*/` past `01` as generated
  (`is_made_step`, with a test); `01-new` and the patches are counted, the
  locks are lock files.
- The `tutorial locks` gate also runs for `site/tutorial-steps/`, and its
  hint ends with `--locks`. `changes.yml` counts `site/tutorial-steps/` as
  code.
- Docs: the `run-gates` skill's *The tutorial's steps* says how to change a
  step; the docs map's row for the book's project, the `document` skill's
  line for a template change, and `site/README.md`'s folders table name the
  source and the command. ADR-0168 amends ADR-0117 and ADR-0126.
- 2850 is open: its clause now says it makes its change once in `01-new` and
  runs `--write`.

**Deviations.**

- **The source is in `site/tutorial-steps/`, not `site/tutorial/patches/`
  and `site/tutorial/locks/`.** The exercises (`exercises::steps`), the
  `tutorial locks` gate's test and hint loop, and the CLI's embedded corpus
  (`crates/cli/build.rs`) each take every folder of `site/tutorial/` for a
  step; two folders more there would have broken them or embedded the
  patches in the CLI (ADR-0168 §8).
- **The locks are `locks/<first step>/Cargo.lock`**, so `pr-size` reads them
  as lock files by name, with no new rule. `01-new`'s lock is made from its
  group's lock too, so `--check` covers all ten locks.
- **`--check` runs in CI's `tickets` job, not first in the `tutorial`
  matrix.** In the matrix, each of the ten jobs would build xtask (and
  `shuyi-cli` with it) on a cache keyed to its step. The `tickets` job
  builds xtask already and runs on every pull request, so a stale step
  fails as early.
- **`--locks` is new.** A dependency refresh (`cargo metadata` in each step)
  rewrites the folders' locks, which `--check` would then refuse. `--locks`
  keeps them in `site/tutorial-steps/locks/`, and fails when two steps of a
  group disagree.
- **`--write` stops at a patch that no longer applies** after writing the
  steps before it, and names the patch and the `--record` that fixes it.

Size: 5,143 counted lines, of which 3,905 are the nine patches, the
folders' existing diffs recorded by the tool, not new writing; the code,
tests and docs are 1,238, under the ticket's 1,500 split point.

Book: none, no step's content changes and no public item changed; the book's
pages read the folders as before.

Skills: none, no schema, template, format or problem code changed.

No break, and no `Behaviour:` line: the change is this repository's own
tooling, which the game doesn't run.
