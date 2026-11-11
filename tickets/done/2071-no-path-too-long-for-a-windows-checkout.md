---
id: "2071"
title: "No tracked path too long for a game's Windows checkout"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-10
---

# 2071 — No tracked path too long for a game's Windows checkout

## Context

game#1262
([ticket](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1262-drop-the-short-cargo-home-on-windows-ci.md)),
from the game's pin move to `ab593bb9`
([Zafnok/visions-of-shuyi#624](https://github.com/Zafnok/visions-of-shuyi/pull/624)).

**The job in one sentence:** a game that depends on the engine by git
builds on Windows without changing any setting.

A game depends on this repository by a git URL and a revision. Cargo
checks it out under
`C:\Users\<user>\.cargo\git\checkouts\shuyi-engine-<16 hex>\<7 hex>\`:
80 characters on a GitHub Windows runner (`runneradmin`), more for a
longer user name. Cargo's libgit2 refuses a path over Windows' 260
unless git's `core.longpaths` is set, which a fresh Windows machine
doesn't have. The ticket file
`tickets/done/2585-a-dialogue-preview-scenes-read-from-a-folder-…` was
196 characters, 276 on the runner, so the game's `test (windows-latest)`
failed at `ab593bb9`. The game works around it with `CARGO_HOME=C:\c` on
its Windows runners. Nick's machine has `core.longpaths` on, so only CI
and other people's machines show it.

No friction review: a bug in the repository's own file names.

**The limit, 150 characters** from the repository's root, leaves 109 for
the prefix: the runner's 80 and a user folder up to 49 characters
long (`C:\Users\` and a long `first.last.DOMAIN` included), under 259
(260 with the closing null).

## Nick input

None.

## Scope

**In:**
- Rename every tracked path over 150 characters to a short slug, keeping
  its number: 23 tickets, ADR-0058, ADR-0084, and three snapshots of the
  dialogue preview (their tests renamed with them). Fix every link to
  them.
- `cargo xtask check-paths`: fails on any tracked path over 150
  characters; a step in CI's `tickets` job.
- `cargo xtask ticket-lint` fails a ticket file name that makes a path
  over 150, so `write-ticket` catches it at once.
- The `write-ticket` and `write-adr` skills, `tickets/README.md` and
  `run-gates` name the limit and why.

**Out (do not do):**
- Removing the game's workaround: game#1262 does it when the game's pin
  passes this ticket. No public API changes; the game's next pin move
  follows this PR.

## Implementation steps

1. `git mv` each long file to `NNNN-<a few words>.md`, and replace the
   old names in every tracked text file.
2. `crates/xtask/src/paths.rs`: `MAX_PATH_LEN`, `run` over
   `git ls-files -z`, and `path_error` for one path; `check-paths` in
   `main.rs`.
3. `parse_ticket` in `tickets.rs` calls `path_error` on
   `tickets/<folder>/<file name>`.
4. The CI step, the skills and the README.

## Acceptance criteria

- [x] `git ls-files | awk 'length($0) > 150'` prints nothing.
- [x] `cargo xtask check-paths` passes, and fails a path of 151 characters
  (unit test).
- [x] `cargo xtask ticket-lint` fails a ticket file name that makes a path
  over 150 (unit test).
- [x] `cargo xtask check-links` passes after the renames.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `paths.rs`, a path at the limit passes and one over fails,
  characters are counted rather than bytes, and the repository itself
  passes; `tickets.rs`, a file name one over the limit is flagged and one
  at it isn't.

## Completion notes

Renamed 28 paths (23 tickets, ADR-0058, ADR-0084, three snapshots with
their tests). Only 8 files linked them by name. The longest tracked
path is now 150 characters. Nothing in the game repository linked to
the old names.

The limit lives in one constant, `xtask::paths::MAX_PATH_LEN`, which
both `check-paths` and the ticket lint use.

The game's pin move that passes this PR removes `CARGO_HOME=C:\c` from
its `ci.yml` and `release.yml` (game#1262).

Book: no change; the book doesn't describe the repository's own checks.
