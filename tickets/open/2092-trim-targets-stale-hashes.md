---
id: "2092"
title: "cargo xtask trim-targets --stale-hashes deletes the older hashes of a crate in deps/ that no build read in days"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2076"]
nick_input: none
completed:
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

- [ ] `--dry-run --stale-hashes` lists, per idle worktree, how many
      stale hash files and how much space it would delete.
- [ ] After a real run on an idle worktree, `cargo test --workspace
      --no-run` there compiles 0 units (output in the notes).
- [ ] With last-access updates off (a parse of fsutil's "DISABLED"
      line), the run says it skipped and deletes nothing in `deps/`.
- [ ] The notes give the space freed on Nick's machine.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the hash split (a dash in the crate's name, a 15- and a
  17-digit hash, a non-hex digit, a non-ASCII name).
- Unit, with `File::set_times`: the grouping keeps the newest-accessed
  hash, and keeps a hash read after the cutoff; a crate under one hash
  is never stale; `.rlib` and `.rmeta` are groups of their own.
- Unit: fsutil's answers (on, off, by words and by value, and an
  error).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
