# ADR-0168: The book's project's steps: the first written by hand, each later one made from a patch, the folders checked in and checked

- **Status:** Accepted
- **Date:** 2026-11-28
- **Related tickets:** 2858; 2850 (a template change made once); 2803,
  2062 (the steps as whole projects, each with its lock)
- **Amends:** ADR-0117 (the step folders the exercises copy are made, not
  written) and ADR-0126 (the step folders the Codespace opens are made,
  not written)

## Context

The book's project, the box pusher, is ten whole game projects,
`site/tutorial/01-new/` to `10-ship/`, each the one before it plus one
chapter's change. ADR-0117 copies each folder to compile its exercises'
checks in, and ADR-0126 opens the folders in the book's Codespace. Both
take the folders as written by hand.

The redundancy sweep of 2026-11-28 measured them: about 23,200 hand-kept
lines in all, where the changes between consecutive steps total about
3,900. A change to how a game starts had to be made in each copy by hand;
2850 exists because two template changes reached none of the ten and
nothing failed. The ten `Cargo.lock` files hold three distinct contents.

The folders are read in many places: the book's `snippet()` and `try_it()`
shortcodes, the exercises' copies, CI's `tutorial` matrix, `pages.yml`'s
web builds, the Codespace, the CLI's embedded docs corpus, and GitHub
links from each page ("Its whole project"). So they must stay where they
are, whole.

## Decision

1. **The source.** `site/tutorial/01-new/` is written by hand. Each later
   step has one patch, `site/tutorial-steps/patches/<step>.patch`: the
   unified diff from the step before (`git diff --no-index --no-prefix`,
   paths `<old step>/<file>` and `<step>/<file>`, applied with `-p1`),
   lock files left out, with no `index` line for a text file.
2. **The locks.** Each step's `Cargo.lock` is one of three kept in
   `site/tutorial-steps/locks/<first step of the group>/Cargo.lock`;
   `LOCK_GROUPS` in `crates/xtask/src/tutorial_steps.rs` says which steps
   take which. `01-new`'s lock is made too: it is a lock like the others.
3. **The folders stay checked in** under `site/tutorial/`, as before, and
   every reader reads them as before. They are output.
4. **`cargo xtask tutorial-steps`**:
   - `--write` makes folders 02 to 10: a copy of `01-new` in
     `target/tutorial-steps/`, each patch applied in turn with `git apply`,
     the tree mirrored into the step's folder (a build's `target/` and
     `dist/` left alone), and each step's lock put in;
   - `--check` fails on a folder that differs from what `--write` makes,
     naming its first differing file and the fix;
   - `--record NN` writes patch NN, from step NN−1 as `--write` makes it
     to folder NN as edited, and patch NN+1, from folder NN to folder
     NN+1: a step's edit is made in its folder, then recorded, and the
     next step stays as it was;
   - `--locks` keeps each group's lock from its steps' folders after a
     refresh (`cargo metadata` in each), refusing when the steps of a
     group disagree.
5. **`git` runs isolated**: no repository is found around the scratch
   folder (`GIT_CEILING_DIRECTORIES`, and `GIT_DIR` and its kin removed),
   since a `git apply` inside a repository reads the patch's paths from
   its top and silently leaves out those outside the current folder; and
   the settings that change a patch's text or how one applies
   (`core.autocrlf`, `diff.noprefix`, `diff.suppressBlankEmpty`, the
   algorithm, the context) are pinned on the command line.
6. **Where it is checked.** The local gate `tutorial steps` runs `--check`
   with `--fast` when a step, its source or the tool changed. In CI it runs
   in the `tickets` job, which builds xtask already, on every pull request.
7. **`pr-size`** counts folders 02 to 10 as generated; `01-new` and the
   patches are counted, the locks are locks.
8. **The source lives beside the steps, not among them**
   (`site/tutorial-steps/`, not `site/tutorial/patches/`): the exercises,
   the lock gate, the dev container check and the CLI's embedded corpus
   each take every folder of `site/tutorial/` for a step.

## Consequences

- A change every step shares is made once, in `01-new`, and reaches every
  later step that has the changed lines with `--write`. A change one step
  makes is one hunk of one patch.
- A step edited by hand and not recorded fails `--check`, naming the file;
  so does a step's lock that isn't its group's.
- The ten folders still hold about 23,200 lines in the repository, and a
  change to an early step shows in every later folder's diff; `pr-size`
  leaves those out.
- A patch that no longer applies (an edit in an earlier step that touches
  a later patch's context) fails `--write` and `--check`, naming it: the
  later step is edited in its folder and recorded.
- A dependency refresh now ends with `--locks`. A step whose dependencies
  change moves to a group of its own in `LOCK_GROUPS`, with its lock.
- `--check` needs `git` on the machine, which every gate already does.

## Alternatives considered

- **Delete the folders and build them only in CI and the Codespace.**
  Breaks the book's GitHub links and leaves a reader no folder to open;
  a product change, out of the ticket's scope.
- **Our own patch applier in Rust.** No dependency on `git`'s behavior,
  but a second unified-diff implementation to keep, for no gain: `git` is
  on every machine that builds the repository.
- **Run `--check` in each of the `tutorial` matrix's ten jobs.** Each
  would build xtask (and `shuyi-cli` with it) on a cache keyed to its
  step, ten times; the `tickets` job builds it once already.
- **`site/tutorial/patches/` and `site/tutorial/locks/`**, as the ticket
  first named them. Every reader that lists the steps' folders would take
  them for steps (§8).
- **One lock for all steps.** The steps' dependencies differ (02 adds
  `serde` to the game, 07 `ron`), and a lock whose game depends on a crate
  the step's manifest doesn't name fails `cargo metadata --locked` there.
