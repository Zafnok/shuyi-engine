# ADR-0121: A worktree's build output: incremental kept in the workspace, off in one-off builds, compressed everywhere, trimmed when idle

- **Status:** Accepted
- **Date:** 2026-11-16
- **Related tickets:** 2076 (this), 2074 (the one-off builds without an
  incremental cache), 2075 (compression), 2092 (the stale hashes in
  `deps/`), 2078 (seeding a new worktree), 3403 (the same for a game)

## Context

Nick works with many git worktrees at once: Claude Code's under
`.claude/worktrees/`, Codex's under `~/.codex/worktrees/`, and a few of
his own. On 2026-11-16 `git worktree list` had 78. Each has its own
`target/`, about 25 GB on his machine, and the disk filled each week.
He does **not** want worktrees archived or removed to get the space
back.

A study on 2026-11-13 measured one worktree's `target/` (26.3 GB):

- The incremental output was 57% of it, about 15 GB, in
  `target/debug/incremental`, `target/gates/games/debug/incremental`
  and `target/tutorial/debug/incremental`. Only workspace crates get
  any.
- With incremental off, a test rebuild after a small edit took 53–62 s
  against 16–30 s with it on: about 7 minutes more per median ticket.
- After `incremental/` is deleted, cargo still reports the build Fresh
  and compiles 0 units; only the first edit is slower (106 s against
  50 s), and the cache is back after it.
- In `deps/`, one crate under several hashes takes up to 3.6 GB, but
  most hashes are live (features, triples, Clippy). cargo never
  rewrites a Fresh dependency's files, so their modification time says
  nothing; only the last access can find the stale ones.
- 14 worktrees whose PRs were merged held 196 GB of `target/`.

What 2074 and 2075 then measured:

- **2074:** the one-off gate builds without an incremental cache:
  `target/gates/games` 2.64 to 0.95 GB, `target/tutorial` 2.63 to
  0.97 GB, the skill games in `target/skill-examples-build` (2.35 GB,
  no cache), and a `target/` from 26.3 to 20.0 GB.
- **2075:** NTFS compression of `target/` stores a full
  `cargo test --workspace --no-run` (6.59 GB) in 3.44 GB, 1.9:1, no
  slower.

And this ticket, on a worktree of a clone built with `cargo test
--workspace --no-run` (5.6 GB, 2.6 GB of it in `debug/incremental`):
`cargo xtask trim-targets` deleted the 2.6 GB, and the same command
there then compiled 0 units, `Finished` in 0.60 s.

## Decision

What a worktree's build output keeps, compresses and trims:

1. **Kept: incremental on in the workspace.** `target/debug` and the
   other profiles of the workspace keep cargo's default: the edit loop
   is about twice as fast with it.
2. **Off: incremental in the one-off builds** (2074): the gates' demo
   games (`target/gates/games`), the tutorial tries
   (`target/tutorial`) and the skill examples' games
   (`target/skill-examples-build`) build with `CARGO_INCREMENTAL=0`.
   Their games are made fresh each run and never edited.
3. **Compressed: every `target/`** (2075): on Windows, the
   `post-checkout` hook marks a new worktree's `target/` compressed,
   and a full `cargo xtask gates` compresses one that lacks the
   attribute. Never on CI, never failing a checkout or a gate.
4. **Trimmed: idle worktrees** (this ticket). `cargo xtask
   trim-targets [--idle-days N] [--dry-run] [--merged]`, over
   `shuyi_cli::worktrees::trim`:
   - It lists every worktree of the repository (`git worktree list
     --porcelain`, so Claude Code's, Codex's and any other), and leaves
     alone the one it runs in, a bare, prunable or missing one, one
     whose `target/` is missing, a link or a file, one where cargo holds
     a lock, and one built in the last N days (3 by default).
   - **Idle** is read from the newest modification of a file directly
     inside a `.fingerprint/<unit>/` folder of any profile folder under
     `target/`, which cargo rewrites each time it builds the unit.
   - **Locked** is a `.cargo-lock`, `.cargo-build-lock` or
     `.cargo-artifact-lock` in `target/` or a profile folder that
     `File::try_lock` can't take (released at once).
   - In each idle worktree it deletes the `incremental/` of every
     profile folder (a folder holding `.fingerprint/`) at
     `<root>/<profile>` or `<root>/<triple>/<profile>` under `target/`,
     for each root it is given. xtask gives the engine's: `target/`
     itself, `gates/games`, `tutorial` and `skill-examples*`; 3403
     gives a game's.
   - `--merged` deletes all of `target/`'s content (the folder stays,
     with its compression) in an idle worktree whose branch is an
     ancestor of `origin/main` or whose pull request is merged (`gh pr
     list --state merged --head`). A detached head and the base's own
     branch are never merged.
   - Never: a worktree removed, archived or pruned; anything outside
     `<worktree>/target/`; a link or junction followed; a folder holding
     `.git` entered (a worktree can sit inside another's `target/`).
     Deletes are best-effort: a file in use is left and counted.
   - `cargo xtask gates` runs it with its defaults at the end of a
     passing full run, locally: not with `CI` set, not with
     `SHUYI_NO_TRIM=1`, not with `--fast` or `--list`. Its output is
     one line on stderr, and it never fails the gates.

The walk is in `shuyi-cli`, not xtask, so a game's `cargo shuyi
trim-targets` (3403) calls the same code with its own roots.

## Consequences

- An idle worktree drops about half its `target/` with no rebuild: the
  next build is Fresh, and only its first edit pays (about a minute).
- The trim runs by itself after each passing local gates run, so the
  disk is freed without anyone remembering to. On 2026-11-16 it found
  nothing to delete: every worktree with an incremental cache had been
  built in the last 3 days, and the 11 idle ones (Codex's, which build
  without one) had none.
- The space freed is printed as the files' lengths. On a compressed
  `target/` the disk gets back about half of that.
- A worktree a session starts using again within seconds of the check
  can lose its cache mid-build: that build fails once, and the next one
  rebuilds. The lock check makes this rare.
- `--merged` deletes notes an agent kept in `target/` (Codex's
  worktrees hold some): `target/` is build output, and nothing else is
  promised to last there.
- xtask now depends on `shuyi-cli` (a workspace crate; xtask is no
  engine crate, so `tools/layers.ron` has no line for it).

## Alternatives considered

- **One shared `CARGO_TARGET_DIR`, a shared `build-dir`, or a junction
  to one `target/`.** Workspace crates get the same hash in every
  worktree, and cargo decides freshness by modification time: measured,
  in worktree a cargo said Fresh and a's test program ran worktree b's
  test. One build lock also serializes every worktree. 2078 seeds a
  new worktree from a warm one instead, its own crates built fresh.
- **sccache as a disk fix.** It saves compile time, not disk: each
  worktree still keeps its own outputs, 0 GB saved, measured.
- **A plain hard-link copy (`cp -al`) of a warm `target/`.** cargo
  rewrites some files in place (`.fingerprint/*`, `*.d`, build scripts'
  `output`): it corrupted 31 files in the donor, which then rebuilt.
  2078 links only the outputs rustc always writes anew.
- **Incremental off everywhere.** About 7 minutes more per ticket, for
  a cache an idle worktree can drop at little cost.
- **A sweep of `deps/` by modification time.** cargo never rewrites a
  Fresh dependency, so it would delete live files. The last access
  (with a threshold of days: NTFS updates it about hourly) can find the
  stale hashes; that is 2092's `--stale-hashes`.
- **A Scheduled Task, or removing worktrees whose PRs merged.** Nick
  doesn't want worktrees removed, and a Windows setting is outside the
  repository; the gates already run in every worktree, often.
