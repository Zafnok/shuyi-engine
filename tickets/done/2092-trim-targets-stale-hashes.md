---
id: "2092"
title: "cargo xtask trim-targets --stale-hashes deletes the older hashes of a crate in deps/ that no build read in days"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2076"]
nick_input: none
completed: 2026-11-16
---

# 2092 — cargo xtask trim-targets --stale-hashes deletes the older hashes of a crate in deps/ that no build read in days

## Context

2076 wrote `cargo xtask trim-targets` over `shuyi_cli::worktrees::trim`
(`crates/cli/src/worktrees/trim.rs`, ADR-0121): in each idle, unlocked
worktree it deletes the profile folders' `incremental/`, and with
`--merged` all of a merged worktree's `target/`. Its `--stale-hashes`
was split off here, at 2076's size valve.

The study of 2026-11-13 (ADR-0121, *Context*): in `deps/`, one crate
under several hashes takes up to 3.6 GB, but most hashes are live
(features, triples, Clippy). cargo never rewrites a Fresh dependency's
files, so a sweep by modification time would delete live files. Only
the last access, with a threshold of days (NTFS updates it about
hourly), can find the stale ones.

## Nick input

None.

## Scope

**In:**
- `cargo xtask trim-targets --stale-hashes`: also deletes, in each
  `deps/` of a profile folder under the roots of an idle worktree, the
  older hashes of a crate whose last access is older than
  `--idle-days`, keeping the newest-accessed hash of each group.
- When NTFS last-access updates are off, it says so (one note in the
  summary line) and skips this.
- `Options::stale_hashes` in `shuyi_cli::worktrees::trim`, so 3403's
  `cargo shuyi trim-targets --stale-hashes` gets it too.
- The usage text, ADR-0121's point 4 (an `Extends:` ADR or a new
  point: ADR-0121 is accepted), and the run-gates skill's trim
  paragraph.

**Out (do not do):**
- Running it from the gates: they keep the defaults (incremental only).
- Any file outside `deps/`, or a worktree that isn't idle.

**Estimated size:** about 400 lines (code 120, tests 160, docs 120).

## Implementation steps

1. A `deps/` file's crate and kind: `libfoo-<16 hex digits>.rlib` is
   (`libfoo`, `.rlib`); the hash is the first `-` followed by 16 hex
   digits and then a `.` or the end (`my-crate-…` has a dash before
   it). Group the files by (crate, kind); in a group of more than one,
   delete those last accessed before the cutoff, never the most
   recently accessed one.
2. Last-access updates: `fsutil behavior query disablelastaccess` on
   Windows. Its line reads `DisableLastAccess = 2  (System Managed,
   Last Access Time Updates ENABLED)`: trust the words, else the
   value's low bit (0 and 2 on, 1 and 3 off). Elsewhere, assume on
   (Linux's `relatime` updates it at least daily).
3. Count what is deleted in the worktree's line and the run's total;
   a file in use is skipped and counted, like the rest of the trim.

## Acceptance criteria

- [x] `--dry-run --stale-hashes` lists, per idle worktree, how many
      stale hash files and how much space it would delete.
- [x] After a real run on an idle worktree, `cargo test --workspace
      --no-run` there compiles 0 units (output in the notes).
- [x] With last-access updates off (a parse of fsutil's "DISABLED"
      line), the run says it skipped and deletes nothing in `deps/`.
- [x] The notes give the space freed on Nick's machine.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the hash split (a dash in the crate's name, a 15- and a
  17-digit hash, a non-hex digit, a non-ASCII name).
- Unit, with `File::set_times`: the grouping keeps the newest-accessed
  hash, and keeps a hash read after the cutoff; a crate under one hash
  is never stale; `.rlib` and `.rmeta` are groups of their own.
- Unit: fsutil's answers (on, off, by words and by value, and an
  error).

## Completion notes

Done:

- `shuyi_cli::worktrees::trim` gained `Options::stale_hashes` and
  `Report::stale`; the hash split, the grouping and fsutil's answer are
  in `trim/stale.rs`. `cargo xtask trim-targets --stale-hashes` sets the
  flag, and its usage text says so.
- Each idle worktree's line says `would delete N stale hash file(s) in
  deps/ (size)` (or `deleted`), and the summary line `N stale hash
  file(s) among it`. With last-access updates off (fsutil's `DISABLED`,
  or an odd value) or fsutil unreadable, the summary says
  `…: stale hashes not looked for` and nothing in `deps/` is deleted
  (unit test of the parse, and of the run on a machine where the check
  fails).
- ADR-0123 extends ADR-0121 §4 (its status line and the index say so);
  the run-gates skill's trim paragraph names the flag.

Deviation: **the cutoff counts back from the worktree's last build, not
from now.** Measured: a Fresh cargo build only reads its outputs'
metadata, which doesn't update last access (neither does this trim; Git
Bash's `ls -l` and `stat` do). An idle worktree's live hashes were all
last read around its last build, so a cutoff from now would delete every
hash but the newest-read of each crate and kind (4–9 GB a worktree),
live ones among them: a test build's `.rmeta` beside a Clippy `.rmeta`
read later, rebuilt for a missing output. ADR-0123 records both.

On Nick's machine (2026-11-16, fsutil: `DisableLastAccess = 2`, on):

- `--dry-run --stale-hashes`: 15 idle worktrees, 0 stale hash files, 0
  MB. Each idle worktree had been built within a few hours, so all its
  hashes were read then. With `--idle-days 0` (dry, for scale only) it
  would list 101,552 files, 426 GB with the incremental caches of 60
  worktrees: most of it live variants.
- A real `cargo xtask trim-targets --stale-hashes`: `freed 0 MB in 15
  idle worktree(s), left 62 alone`. Then, in
  `book-contents-sidebar-5545c7` (idle 3 days),
  `cargo test --workspace --no-run` compiled 0 units:
  ``Finished `test` profile [unoptimized + debuginfo] target(s) in 8.43s``.
- Space freed: 0 today. The main worktree, built over weeks and not
  idle, would give up 1,109 files (4.8 GB) by the same rule once idle,
  counted with PowerShell against its last build.

No ticket: the gates keep the defaults (the ticket's Out), and with
nothing stale on this machine today there is nothing to add to them.

Book: none, `trim-targets` is a tool over a repository's worktrees
(the engine's xtask, and a game's with 3403), not something a game is
built with; the book has no chapter on it.

No break: `Options` and `Report` are `#[non_exhaustive]`, and gain a
field each. No game ticket is named, so no `Game:` line.

Nick input: none.
