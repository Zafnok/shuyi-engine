# ADR-0167: The trim after `--fast` gates, once a day in the background

- **Status:** Accepted
- **Date:** 2026-11-28
- **Related tickets:** 3545 (this), 2076 (the trim), 3544 (fast gates
  locally)
- **Amends:** ADR-0121 §4 (the trim's run at the end of the gates) and
  ADR-0166 §4 (the trim after a full local run only)

## Context

`cargo xtask trim-targets` (ADR-0121 §4) deletes the incremental caches
of the other, idle worktrees. It ran by itself only at the end of a
passing full local `cargo xtask gates`. Since ADR-0166 the full run is
CI's and the local default before a push is `--fast`, so the trim seldom
ran, and idle worktrees kept their caches, about half of each `target/`.

Ticket 3545 asked a passing `--fast` to end with the trim too, and to
run it at most once a day if it added more than about two seconds. It
was measured on Nick's machine on 2026-11-28, with 110 worktrees, 98 of
them idle:

| Run | Time |
| --- | --- |
| `trim-targets`, freeing 24.4 GB | 58 s |
| `trim-targets` again, nothing to delete | 43 s |
| `trim-targets --dry-run` (it sizes each whole `target/`) | 5 min 48 s |

The cost with nothing to delete is the walk itself: each idle worktree's
fingerprints and profile folders, about 0.4 s a worktree. The pre-push
hook runs `--fast`, and a push must never wait on a minute's trim or
fail because of one. Once a day alone would still hold one push a day
for a minute.

## Decision

1. **After a passing full run** the trim runs as before: waited for, its
   line on stderr.
2. **After a passing `--fast` run** the trim is started, with its
   defaults, **in the background** and **at most once a day**. The gates
   print one line, that it started (and where its log is) or that it ran
   in the last day, and return at once.
   - The stamp is `shuyi-trim/stamp` under the repository's common
     `.git` folder (`git rev-parse --git-common-dir`), shared by every
     worktree; its modification time is the last start. A stamp in the
     future counts as due.
   - The trim runs from a copy of xtask in `shuyi-trim/`, so a build of
     xtask in this worktree can replace its program meanwhile (Windows
     won't while it runs). A copy still running holds its file, so a
     second start fails to copy and is said in a warning line.
   - It runs detached, holding no handle of this process: no stdio, its
     lines written by itself to `shuyi-trim/last.log` (`trim-targets
     --log <file>`), so neither git nor a harness reading the push's
     output through a pipe waits for it. On Windows a child inherits
     every inheritable handle of its parent whatever its own stdio (a
     first try with `DETACHED_PROCESS` held a piped shell for the whole
     trim), and xtask forbids the `unsafe` that would turn that off: a
     PowerShell that exits at once starts it with `Start-Process`, which
     goes through `ShellExecute` and passes no handle. Elsewhere it is a
     process group of its own. Ctrl+C in the terminal doesn't stop it.
3. As before: not with `CI` set, not with `SHUYI_NO_TRIM=1`, never with
   `--list` or after a failure, and it never fails the gates. What it
   deletes and its defaults don't change.

The choice is the pure `gates::trim_after` (`No`, `Now`, `Daily`) and
`trim::due`, each tested; the start itself is `trim::after_fast_gates`.

## Consequences

- Idle worktrees lose their caches again without anyone remembering,
  once a day, and a push costs nothing more than the start: about 1.1 s
  on Windows, PowerShell's own start (7.6 s for `--fast` against 6.5 s
  with `SHUYI_NO_TRIM=1`), once a day; 0 s on the other days.
- The trim's line no longer says what it freed after `--fast`: that is
  in `last.log`. A full run still prints it.
- A background trim may still be running when the next build starts in
  an idle worktree. As before, a worktree whose cargo holds its lock is
  left alone, and the next build there is Fresh; only its first edit
  pays.
- Deleting the stamp makes the next `--fast` start it again.

## Alternatives considered

- **Wait for it once a day** — one push a day held most of a minute,
  which ticket 3545 rules out for the hook.
- **Speed the walk up** — out of the ticket's scope (no change to what
  the trim does), and still seconds over a hundred worktrees.
- **Only after a full run, as before** — the full run is now rare, and
  the disk filled.
