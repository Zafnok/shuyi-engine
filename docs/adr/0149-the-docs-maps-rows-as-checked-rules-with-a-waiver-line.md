# ADR-0149: The docs map's rows as checked rules, with a waiver line

- **Status:** Accepted
- **Date:** 2026-11-25
- **Related tickets:** 3534; 2815 (the book's rule on the public API);
  the docs audit of 2026-11-25 (2850, 2851, 3352, 3426–3428, 3533)
- **Extends:** ADR-0038's `api` job, whose `Book:` rule stays as it is

## Context

The docs map (`docs/README.md`) says, row by row, which change updates
which home, and the `document` skill tells a session to walk it. Until
now one row was checked: CI's `api` job fails a change to the Rust public
API that changes no file of the book and whose ticket has no `Book:` line
(ticket 2815). The docs audit of 2026-11-25 read 60 PRs and found the
skill pack, the book's project and the contract's prose behind. What
slipped through:

- a contract or template change is not a Rust API change, and the `api`
  job runs only when `crates/` changes;
- the skill pack had no check at all: `shuyi-import-art` missed six
  formats (2203, 2234) and `.sfx.ron` (2225);
- any file of the book passes the `api` job's rule.

## Decision

### 1. The rows a PR's paths can show are data

`tools/docs-rules.ron` holds one rule per row, or per home of a row: the
globs whose change triggers it, the globs of its homes, and its mark,
the start of a line in the ticket's `## Completion notes`. The first are
the ticket's: a schema of the contract asks for the skill pack (`Skills:`)
and the book (`Book:`); the template the same, the book's project
counting; a format (`docs/formats.md`, the registry in
`crates/assets/src/format.rs`, a `formats.rs` of a kind) and a problem
code the skill pack. Each other row the paths can show got one too: the
contract's prose, the template format, `docs/formats.md`, the platform
boundary, the crash reports, the Playdate, the desktop packages, the web
shell, the eval, the site's build and the sound parity gates.

A row the paths can't show (a rule a game keeps, which chapter) stays
the `document` skill's.

### 2. A waiver line, not a hard rule

A rule triggered with no home changed passes when the PR's done ticket
has a line starting with its mark: `Skills: none, <why>`. Many changes to
a schema or the template change nothing a skill teaches (a field's
description, a version bump, a test of the template); a hard rule would
push sessions into empty edits of the skill pack to pass, which is worse
drift than none. The line makes the choice visible in the one place the
game's pin review and the next session read, as `Book:` and `Breaking:`
lines do.

One line waives every rule of its mark: a template change that needs
neither the skill pack nor the book writes `Skills:` and `Book:` once
each, whichever rules asked.

### 3. The check, its strictness and who it skips

`cargo xtask docs-rules` reads the PR's changed paths (a rename is both
its paths) against its base. CI's `tickets` job runs it on every PR, with
the PR's title and branch: a missing line fails, and the failure names
the rule, the files that triggered it, the homes and the line to write.
A PR for no ticket's work (ticket-only, a bot's, a Sonar batch, a game
ticket's small change) is reported, not failed, as `pr-size` does. A
local run only warns, and reads the ticket from `tickets/open/` too,
since a session checks before it archives.

Every glob of the rules matches a tracked file, or the check fails, so a
folder that moves takes its rule with it rather than silently checking
nothing.

### 4. The `api` job's rule shares the words

The `api` job's `book_rule` stays for the Rust API: a listing diff, not a
path, says the API changed. Its message and the new check's are built by
the same code (`docs_rules::ways`) and judge the ticket's notes with the
same reader (`docs_rules::waived`, over `tickets::notes_lines`), so both
say the same thing in the same words.

## Consequences

- Run over the 60 PRs merged before it, the check would have failed 13
  of them (ticket 3534's completion notes list them), among them 2225,
  2203 and 2234 for the skill pack.
- A session that changes a schema, the template or a format writes a
  `Skills:` or `Book:` line more often. That line is the point: it says
  the home was looked at.
- A new home whose trigger the paths can show gets its rule in the same
  PR as its row of the docs map.
- Which chapter or which skill changed is not checked: a home changing is
  enough (2850 checks the book's project's first step).
