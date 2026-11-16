---
id: "2076"
title: "cargo xtask trim-targets deletes the incremental caches of idle worktrees, never a worktree, and the gates run it"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-16
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

**From 2075 (done):** the ADR records compression as built there:
`scripts/git-hooks/post-checkout` marks a new worktree's `target/`
(Windows only, never failing the checkout), and a full `cargo xtask
gates` run compresses one that lacks the attribute (`--fast` and
`--list` skip it, not on CI). 2075's completion notes hold the measured
ratio.

**From 2078 (done):** `crates/cli/src/worktrees.rs` exists, with
`seed`; add `trim` beside it. `seed::LOCKS` lists the three lock files
cargo 1.99 keeps in a profile folder (`.cargo-lock`,
`.cargo-build-lock`, `.cargo-artifact-lock`): step 4 checks all three.
`cargo run` holds none of them while the program runs (measured), so
a worktree whose MCP server is running isn't locked. The ADR records
seeding as built there: a partly built target is kept and added to
(2078's completion notes), and its measured first build.

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
    last-access updates are off, it says so and skips this (split off
    to 2092 at the size valve below);
  - `--merged`: also deletes the whole `target/` of a worktree whose
    branch is merged into `origin/main` or whose PR is merged (`gh`),
    keeping the worktree;
  - prints what it deleted and the space freed.
- `cargo xtask gates`, locally (not when `CI` is set), runs
  `trim-targets` with its defaults at the end of a passing run. It
  never fails the gates. `SHUYI_NO_TRIM=1` turns it off.
- An ADR (the `write-adr` skill): what a worktree's build output keeps,
  compresses and trims, with the measured numbers. Kept: incremental on
  in the workspace. Off: in the one-off gate builds (2074; **from 2074
  (done):** `target/gates/games` 2.64 to 0.95 GB, `target/tutorial`
  2.63 to 0.97 GB, the skill games in `target/skill-examples-build`,
  their numbers in 2074's completion notes). Compressed:
  every target (2075). Trimmed: idle worktrees (here). Rejected, with
  why:
  - one shared `CARGO_TARGET_DIR`, `build-dir` or a junction: a
    worktree builds and tests another's code, measured;
  - sccache as a disk fix: 0 GB saved, measured;
  - a plain hard-link copy: it corrupted 31 metadata files in the
    donor;
  - incremental off everywhere: about 7 minutes per ticket.
- **For 3403:** 3403 gives a game the same command (`cargo shuyi
  trim-targets`). So the walk goes in `shuyi-cli`
  (`crates/cli/src/worktrees/trim.rs`, public as
  `shuyi_cli::worktrees::trim`), and xtask's command is a thin wrapper
  over it. Write it once there now; moving it later would count every
  line twice in `pr-size`. The places `incremental/` is looked for
  are a parameter: xtask passes the engine's (`debug`, `gates/games`,
  `tutorial`, `skill-examples*`), and 3403 passes a game's. If
  `check-layers` asks for it, the edge from xtask to `shuyi-cli` gets
  its line in `tools/layers.ron`.

**Out (do not do):**
- Removing, archiving or pruning a worktree, or deleting anything
  outside `<worktree>/target/`.
- A Scheduled Task or any Windows setting.
- Touching the current worktree's target (the gates are running in it).

**Estimated size:** about 1,300 lines (code 600, tests 400, docs 300).

**Split:** past 1,100 lines, split off `--stale-hashes` (step 6) into a
new ticket.

## Implementation steps

1. New `crates/cli/src/worktrees/trim.rs` (the **For 3403** item), and
   xtask's `trim-targets` wired in its `main.rs` and usage text over it.
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

- [x] `--dry-run` lists each worktree with its target size, whether it
      is idle or locked, and what would be deleted; it deletes nothing.
- [x] On an idle worktree, after `trim-targets`, `cargo test
      --workspace --no-run` there compiles 0 units (output in the
      notes).
- [x] The current worktree, a worktree built in the last N days, and a
      worktree with a cargo build running are left untouched (tested
      with a held lock).
- [x] Nothing outside `<worktree>/target/` is deleted, and no worktree
      is removed (`git worktree list` is the same before and after).
- [x] `--merged` deletes the target of a merged branch's worktree and
      keeps the worktree.
- [x] A passing local `cargo xtask gates` ends with the trim line;
      with `SHUYI_NO_TRIM=1` or `CI` set it doesn't run.
- [x] The notes give the space freed on Nick's machine by the first
      run.
- [x] The ADR is written and linked from `docs/adr/`'s index.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit, with temp folders and `File::set_modified`: idle detection,
  `incremental` found at every depth (`debug`, `gates/games/debug`,
  `<triple>/debug`), a folder named `incremental` elsewhere left alone,
  a junction not followed (Windows).
- Unit: the porcelain parser (prunable, detached, bare).
- Unit: the stale-hash grouping keeps the newest-accessed hash.
- Unit: a held `.cargo-lock` makes the target skipped.

## Completion notes

What was done:

- `crates/cli/src/worktrees.rs` and `worktrees/trim.rs`, public as
  `shuyi_cli::worktrees::trim`: `Options` (`#[non_exhaustive]`, built by
  `Options::new(roots)`: `idle_days`, `dry_run`, `merged`, `base`),
  `Report` (`summary()`), `run(repo, &options, out)` and
  `DEFAULT_IDLE_DAYS`. It parses `git worktree list --porcelain`, skips
  the current, bare, prunable and missing worktrees, a missing or
  linked `target/`, a held `.cargo-lock`, `.cargo-build-lock` or
  `.cargo-artifact-lock` (`File::try_lock`), and a worktree whose newest
  `.fingerprint/<unit>/` file is newer than the cutoff. A profile folder
  is one holding `.fingerprint/`, found to depth 5 without entering a
  link, `doc`, `src` or `.git`; its `incremental/` goes when it is
  `<root>/<profile>` or `<root>/<triple>/<profile>`.
- `crates/xtask/src/trim.rs` and `main.rs`: `cargo xtask trim-targets
  [--idle-days N] [--dry-run] [--merged]` with the engine's roots (`""`,
  `gates/games`, `tutorial`, `skill-examples*`), and `after_gates`,
  which `gates::run` calls after a passing full run.
- ADR-0121; the run-gates skill's trim paragraph (loaders checked with
  `sync-agent-skills.ps1 -Check`); `AGENTS.md`'s *Environment*; a docs
  map row for the build output and xtask's commands.
- Tests: 11 in `shuyi-cli` (the porcelain parser; `incremental` at
  `debug`, `release`, `gates/games/debug`, `x86_64-pc-windows-gnu/debug`,
  `tutorial/debug` and `skill-examples-build/debug`; one under
  `debug/deps`, `debug/build/…/out`, `doc`, a folder without
  `.fingerprint/` and a profile under no root left alone; a junction
  (Windows) or symlink never followed; a worktree inside a `target/`
  never deleted; the last build; each lock file held and released; the
  sizes and the summary; and a whole run on a temporary repository with
  four worktrees: current, idle, built a day ago, locked, and merged),
  2 in xtask (the flags, and when the gates trim).

Deviations:

- **Split:** `--stale-hashes` (step 6, its test) went to 2092 at the
  size valve: the PR measured 1,371 counted lines before the ADR with
  it, past the valve's 1,100.
- The gates trim after a passing **full** run only: `--fast` (the
  pre-push hook) and `--list` skip it, as they skip 2075's compression.
  It runs before the summary block, on stderr, so the block stays the
  PR's.
- Two safeguards the steps didn't name, found on this machine: a
  worktree sits inside another's `target/` (`…/target/api-base`), so no
  folder holding `.git` is ever entered; and the base's own branch
  (`main`, the main checkout) is never "merged", though it is an
  ancestor of `origin/main`.
- `--merged` deletes what is inside `target/` and keeps the folder
  itself, so its NTFS compression stays (2075). It applies to idle,
  unlocked worktrees only, like the rest.
- The space is counted as the files' lengths: std has no compressed
  size, and on a compressed `target/` the disk gets back about half.

Evidence (Windows 11, at `bc181523`, this branch; main's tip was
`4eae83e6`):

- **Zero units after a trim.** A `git clone --shared` of this branch
  with one worktree, `idle`, where `cargo test --workspace --no-run
  --locked` built (2 min 58 s; `target/` 5.6 GB, 2.6 GB of it
  `debug/incremental`). From the clone: `cargo xtask trim-targets
  --idle-days 0 --dry-run` printed `target 5.6 GB, idle (last build 0
  days ago), would delete debug\incremental (2.6 GB)` and deleted
  nothing; then without `--dry-run`: `deleted debug\incremental
  (2.6 GB)`, `trimmed 1 idle worktree(s), freed 2.6 GB, left 1 alone`.
  `git worktree list` was the same before and after. Then in `idle`,
  `cargo test --workspace --no-run --locked`: no `Compiling` line,
  `` Finished `test` profile [unoptimized + debuginfo] target(s) in
  0.60s ``.
- **Nick's machine, dry run** (75 worktrees, before the first run):
  10 idle, none with an incremental cache (Codex's worktrees build
  without one), so `would free 0 MB`; 46 built in the last 3 days, 3
  locked by a running build, 15 with no `target/`, and this one.
- **First run on Nick's machine:** the end of the passing `cargo xtask
  gates` at `71da4c27`: `== trim-targets: freed 0 MB in 12 idle
  worktree(s), left 63 alone`. **0 MB freed**: today every worktree
  with an incremental cache was built in the last 3 days. The space
  comes as worktrees go idle; the evidence clone above freed 2.6 GB of
  a 5.6 GB `target/`, the study's 15 GB of 26.3 GB.
- The current worktree, a recent one, a locked one and the merged one
  are the unit test `a_run_trims_idle_worktrees_only_and_keeps_every_worktree`
  (a held `.cargo-lock`; `git worktree list --porcelain` compared before
  and after).

Notes for Nick:

- Nothing to do: each passing `cargo xtask gates` now frees the
  incremental caches of worktrees no one built in 3 days, and keeps
  every worktree. `cargo xtask trim-targets --merged` frees all the
  build output of merged worktrees' idle `target/`s when you want more
  room; `--dry-run` shows what it would do first.

Book: none, the new `shuyi_cli::worktrees` module is the repository's
own tooling that 3403 will make a game command; the book is for people
making games and no game command changed.

No break (additions only). No `Behaviour:` line: no existing command,
format or output changed. Game: nothing to adopt, a game gets the trim
with 3403.
