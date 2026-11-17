# ADR-0123: Stale hashes in `deps/`, read by last access against the worktree's last build

- **Status:** Accepted
- **Date:** 2026-11-16
- **Related tickets:** 2092 (this), 2076, 3403
- **Extends:** ADR-0121 §4

## Context

ADR-0121 §4 trims an idle worktree's `incremental/` folders and, with
`--merged`, all of a merged one's `target/`. It left `deps/` to this
ticket: there, one crate sits under several hashes, up to 3.6 GB a
crate, but most hashes are live (features, triples, Clippy, the test
and check builds). cargo never rewrites a Fresh dependency's files, so
their modification time says nothing.

What was measured on Nick's machine on 2026-11-16:

- **What updates a file's last access on NTFS.** Reading it, and MSYS's
  `ls -l` and `stat` (Git Bash's), which open it. Not a directory
  listing, PowerShell's `Get-Item`, nor Rust's `fs::symlink_metadata`,
  which is all cargo's freshness check and this trim do. So a Fresh
  `cargo build` reads none of its outputs: a live hash's last access is
  the last build that compiled something on top of it, and an idle
  worktree's live hashes were all read last around its last build.
- **The ticket's rule as written, counted back from now,** deletes every
  hash of an idle worktree but the newest-read of each crate and kind:
  4–9 GB a worktree. In a worktree whose last builds were Clippy's and
  the tests', that deletes the test build's `.rmeta` of a crate whose
  Clippy `.rmeta` was read later, and cargo then rebuilds the unit for a
  missing output.
- **Counted back from the worktree's last build,** with the default 3
  days, the 15 idle worktrees had nothing stale: each had been built
  within a few hours, so every hash it held had been read in that
  time. `cargo test --workspace --no-run` in one of them compiled 0
  units afterwards (8.4 s). The main worktree, built over weeks, would
  give up 1,109 files (4.8 GB) by the same rule once idle: hashes left
  by dependency bumps and changed features.

## Decision

`cargo xtask trim-targets --stale-hashes` (and
`shuyi_cli::worktrees::trim::Options::stale_hashes`, which 3403's
`cargo shuyi trim-targets` sets the same way) also deletes, in an idle
worktree, stale hashes in the `deps/` of each profile folder under the
roots ADR-0121 §4 names:

1. **The hash** of a file name is the first `-` followed by exactly 16
   hex digits and then a `.` or the end: `libmy-crate-<hash>.rmeta` is
   (`libmy-crate`, `.rmeta`). Files are grouped by (crate, kind); a file
   without a hash, a folder and a link are never looked at.
2. **Stale** is a file in a group of more than one, last accessed more
   than `--idle-days` before the worktree's **last build** (ADR-0121's
   newest fingerprint; for a worktree never built, before now), and
   never the most recently accessed of its group.
3. **Last access must be updated.** On Windows `fsutil behavior query
   disablelastaccess` says so: its words (`Updates ENABLED` or
   `DISABLED`) are trusted, else the value's low bit (0 and 2 on, 1 and
   3 off, hex too). Off or unreadable, the run deletes nothing in
   `deps/` and says why in the summary line. Elsewhere it is taken as on
   (Linux's `relatime` updates it at least daily).
4. Each worktree's line counts the stale files and their size, and the
   summary line the run's total; a file in use is skipped and counted,
   as in the rest of the trim.
5. The gates' own trim keeps ADR-0121's defaults: no `--stale-hashes`.

## Consequences

- A worktree used over several days drops the hashes its later builds
  stopped reading (about 5 GB for the main worktree), with no rebuild
  of anything its last session built.
- A worktree used within one day loses nothing from `deps/`: its hashes
  are all live by this measure, though most are variants (Clippy, the
  gates' builds) a resumed session may not need. That space is left;
  `--merged` frees all of it once the branch is merged.
- Anything that opens the files (MSYS's `ls -l`, `du` or `find`, an
  antivirus or a backup) refreshes their last access, and the trim then
  finds less. It errs toward keeping files.
- A hash whose build ran more than `--idle-days` before the worktree's
  last build, and not since (a Clippy run at the start of a long
  ticket), is deleted, and that build pays its compile once.

## Alternatives considered

- **The cutoff counted back from now, as 2092 first wrote it.** It
  deletes live hashes of every idle worktree: their last access is the
  last build, older than any cutoff in days.
- **The modification time.** cargo never rewrites a Fresh dependency,
  so it would delete live files (ADR-0121).
- **Keeping only the hashes cargo's fingerprints name.** It would tie
  the trim to cargo's fingerprint format, which is internal and changes
  between releases; the last access needs none of cargo's internals.
- **Running it from the gates.** Out of 2092's scope, and on this
  machine it would free nothing today.
