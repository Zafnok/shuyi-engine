---
id: "2076"
title: "cargo xtask trim-targets deletes the incremental caches of idle worktrees, never a worktree, and the gates run it"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2076 — cargo xtask trim-targets deletes the incremental caches of idle worktrees, never a worktree, and the gates run it

## Context

Each worktree's `target/` is about 25 GB on Nick's machine and he runs
about 30 worktrees (Claude Code's under `.claude/worktrees/`, Codex's
under `C:\Users\Zafno\.codex\worktrees\`, a few under `D:\`), so the
disk fills each week. Nick does **not** want worktrees archived or
removed to get the space back.

A study on 2026-11-13 measured:

- Incremental output is 57% of a `target/` (about 15 GB of 26.3 GB in
  `target/debug/incremental`, `target/gates/games/debug/incremental`
  and `target/tutorial/debug/incremental`). Only workspace crates get
  any.
- Incremental stays on for the workspace: turning it off makes the
  edit loop about 2x slower (a test rebuild after a small edit 16–30 s
  on, 53–62 s off), about 7 minutes per median ticket.
- After deleting `incremental/`, cargo still reports the build Fresh
  and compiles 0 units. Only the first edit after it is slower (106 s
  against 50 s), then the cache is back.
- So a worktree no one has built in for days can drop its incremental
  output at little cost.
- In `deps/`, one crate under several hashes takes up to 3.6 GB, but
  most of those are live (features, triples, clippy). cargo never
  rewrites a Fresh dependency's files, so an **mtime** sweep would
  delete live files. Only last-access time (with a threshold of days,
  as NTFS updates it about hourly) can find the stale ones.
- 14 worktrees whose PRs are merged held 196 GB of `target/`
  (measured).

Related: 2074 (the one-off gate builds keep no incremental cache) and
2075 (compression). This ticket writes the ADR covering all three.

## Nick input

None.

## Scope

**In:**
- `cargo xtask trim-targets [--idle-days N] [--dry-run]
  [--stale-hashes] [--merged]`:
  - lists every worktree of this repository (`git worktree list
    --porcelain`, so Claude's, Codex's and others);
  - for each with a `target/` that is **idle** (no build in the last
    N days, default 3) and **not locked** (no cargo running there),
    deletes every `incremental` folder under its `target/`;
  - `--stale-hashes`: also deletes, in each `deps/`, the older hashes
    of a crate whose last access is older than N days. If NTFS
    last-access updates are off, it says so and skips this;
  - `--merged`: also deletes the whole `target/` of a worktree whose
    branch is merged into `origin/main` or whose PR is merged (`gh`),
    keeping the worktree;
  - prints what it deleted and the space freed.
- `cargo xtask gates`, locally (not when `CI` is set), runs
  `trim-targets` with its defaults at the end of a passing run. It
  never fails the gates. `SHUYI_NO_TRIM=1` turns it off.
- An ADR (the `write-adr` skill): what a worktree's build output keeps,
  compresses and trims, with the measured numbers. Kept: incremental on
  in the workspace. Off: in the one-off gate builds (2074). Compressed:
  every target (2075). Trimmed: idle worktrees (here). Rejected, with
  why:
  - one shared `CARGO_TARGET_DIR`, `build-dir` or a junction: a
    worktree builds and tests another's code, measured;
  - sccache as a disk fix: 0 GB saved, measured;
  - a plain hard-link copy: it corrupted 31 metadata files in the
    donor;
  - incremental off everywhere: about 7 minutes per ticket.

**Out (do not do):**
- Removing, archiving or pruning a worktree, or deleting anything
  outside `<worktree>/target/`.
- A Scheduled Task or any Windows setting.
- Touching the current worktree's target (the gates are running in it).

## Implementation steps

1. New `crates/xtask/src/trim.rs`, wired in `main.rs` and its usage
   text.
2. Worktrees: parse `git worktree list --porcelain` (skip `prunable`
   and missing folders). Skip the current one.
3. Idle: the newest mtime of the files directly inside each
   `target/**/.fingerprint/<unit>/` folder (cargo rewrites them on each
   build of the unit). Idle when it is older than N days.
4. Locked: skip a target when any `.cargo-lock` (or cargo 1.99's
   `.cargo-build-lock`) under it can't be locked
   (`std::fs::File::try_lock`, released at once). A build starting
   during the delete only fails that build; deletes are best-effort, a
   file in use is skipped and counted.
5. Delete only folders named `incremental` whose parent is a profile
   folder (`debug`, `release` or a custom profile's) under `target/`.
   Never follow a junction or symlink (check before descending).
6. `--stale-hashes`: group `deps/` files by crate stem and extension;
   in a group with more than one hash, delete files whose last-access
   time is older than N days, keeping the newest-accessed hash of each
   group. Before that, check last-access updates
   (`fsutil behavior query disablelastaccess`), and skip with a message
   if they are off.
7. `--merged`: a worktree's branch is merged if `git merge-base
   --is-ancestor <branch> origin/main`, or `gh pr list --state merged
   --head <branch>` finds one. Detached heads are skipped.
8. Call it from the end of `cargo xtask gates` (step 2's rules), with
   its output as one summary line.
9. The ADR, the run-gates skill (`pwsh -File
   scripts/sync-agent-skills.ps1`), and the docs map's place for
   xtask's commands.

## Acceptance criteria

- [ ] `--dry-run` lists each worktree with its target size, whether it
      is idle or locked, and what would be deleted; it deletes nothing.
- [ ] On an idle worktree, after `trim-targets`, `cargo test
      --workspace --no-run` there compiles 0 units (output in the
      notes).
- [ ] The current worktree, a worktree built in the last N days, and a
      worktree with a cargo build running are left untouched (tested
      with a held lock).
- [ ] Nothing outside `<worktree>/target/` is deleted, and no worktree
      is removed (`git worktree list` is the same before and after).
- [ ] `--merged` deletes the target of a merged branch's worktree and
      keeps the worktree.
- [ ] A passing local `cargo xtask gates` ends with the trim line;
      with `SHUYI_NO_TRIM=1` or `CI` set it doesn't run.
- [ ] The notes give the space freed on Nick's machine by the first
      run.
- [ ] The ADR is written and linked from `docs/adr/`'s index.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, with temp folders and `File::set_modified`: idle detection,
  `incremental` found at every depth (`debug`, `gates/games/debug`,
  `<triple>/debug`), a folder named `incremental` elsewhere left alone,
  a junction not followed (Windows).
- Unit: the porcelain parser (prunable, detached, bare).
- Unit: the stale-hash grouping keeps the newest-accessed hash.
- Unit: a held `.cargo-lock` makes the target skipped.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
