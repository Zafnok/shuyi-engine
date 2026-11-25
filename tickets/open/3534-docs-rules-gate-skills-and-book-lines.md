---
id: "3534"
title: "A docs gate over the changed paths: a contract, template or format change updates the book and the skill pack, or its ticket says why not"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3534 — A docs gate over the changed paths: a contract, template or format change updates the book and the skill pack, or its ticket says why not

## Context

The docs audit of 2026-11-25 (tickets 2850, 2851, 3352, 3426–3428 and
3533) read the last 60 PRs against the docs map's homes and found drift
in the book's project, the glossary, the skill pack, the MCP docs and the
crate docs. Each fix is ticketed. This ticket is about why it happened,
so it doesn't happen again.

The docs map (`docs/README.md`) says, row by row, which change updates
which home, and the `document` skill tells a session to walk it. Only
one row is enforced: CI's `api` job (`book_rule` in
`crates/xtask/src/api.rs`, ticket 2815) fails a change to the **Rust
public API** that changes no file under `site/content/learn/` or
`site/snippets/` and has no `Book:` line in its ticket's completion
notes. What slipped past it:

- **Contract and template changes aren't Rust API changes.** 3424 and
  3425 changed `shuyi new`'s answer and the template; 2225 changed the
  template's files. The `api` job runs only when `crates/` changes
  (`ci.yml`, `needs.changes.outputs.crates`), and a schema or template
  change may change no listed item.
- **The skill pack has no gate at all.** The map's skill-pack row says
  it changes with "an operation, a field, a problem code, a template, a
  format, a rule a game keeps". `shuyi-import-art` missed six formats
  (2203, 2234) and `.sfx.ron` (2225), and no check asked.
- **Any book file passes.** 2225 changed chapter 3 and so passed, while
  its project stayed behind (2850 adds a check for that case).

## Nick input

None.

## Scope

**In:**

- **Rules as data**, `tools/docs-rules.ron`: each rule names the paths
  whose change triggers it (globs), the homes one of which must change
  in the same PR (globs), and the mark of the waiver line in the PR's
  done ticket's `## Completion notes` (`Book:`, `Skills:`). The first
  rules:
  - `crates/cli/schemas/**` (an operation or a field of the contract)
    → `skills/**`, else `Skills:`; and the book's folders, else
    `Book:`;
  - `templates/**` (what a new game starts with) → `skills/**`, else
    `Skills:`; and the book's folders or `site/tutorial/**`, else
    `Book:`;
  - `docs/formats.md` and `crates/assets/src/**` files that register a
    format (the session finds the narrowest glob that catches a new
    `Format`) → `skills/**`, else `Skills:`;
  - `crates/cli/errors/**` (a problem code) → `skills/**`, else
    `Skills:`.

  Read the map's rows and add a rule for each other "update it on a
  change that …" the paths can show. A row they can't show (a "rule a
  game keeps") stays the `document` skill's.
- **The check**, `cargo xtask docs-rules`, run by CI's `tickets` job on
  every PR (it runs whatever the PR touches, unlike `api`): for a
  `[NNNN] …` PR, each rule whose trigger changed and none of whose homes
  changed needs its mark in ticket NNNN's notes (`notes_lines` in
  `crates/xtask/src/tickets.rs` reads them, as `book_lines` does). The
  failure names the rule, the changed files that triggered it, the
  homes, and the line to write (`Skills: none, <why>`). Ticket-only,
  bot and Sonar PRs are reported, not failed, as `pr-size` does.
- `book_rule` stays in the `api` job for the Rust API. Its message and
  the new check's say the same thing in the same words, and share the
  code that reads the marks.
- **Docs:** the docs map gets a row for `tools/docs-rules.ron`, and its
  *Pairs* row on the book names the new check beside `api`; the
  `work-ticket` skill's §5 names `Skills:` beside `Book:`;
  `tickets/TEMPLATE.md` gets a `Skills:` checkbox beside the book's; an
  ADR (the `write-adr` skill): the docs map's rows as checked rules, and
  why waivers and not a hard rule.
- **Tests:** each rule on a fake diff and ticket: triggered and a home
  changed; triggered with a waiver; triggered with neither (fails with
  the message); a ticket-only PR (reported); the RON file parses and
  every glob matches at least one tracked file (so a moved folder fails
  the check, not silently).

**Out (do not do):**

- checking *which* chapter or skill changed: a home changing is enough
  (2850's check covers the project's first step);
- the glossary (2852) and the CLI flags' parity with MCP (3429);
- reworking `book_rule`'s trigger (the Rust API listing).

**Estimated size:** about 900 lines (code 400, tests 350, docs 150).

## Implementation steps

1. Read `crates/xtask/src/api.rs` (`book_rule`, `BOOK_DIRS`),
   `crates/xtask/src/tickets.rs` (`Pr`, `PrKind`, `notes_lines`), how the
   `tickets` job gets the changed files (`ci.yml`), and `docs/README.md`.
2. Write `tools/docs-rules.ron` and its reader.
3. Write the check and its tests; wire it into `cargo xtask gates`'
   ticket checks if they run on a PR's diff locally, and into the
   `tickets` job.
4. Run it over the last 60 merged PRs (each one's diff and done ticket)
   and put in the completion notes which would have failed. That should
   include 2225, 2203 and 2234. Those PRs are not fixed after the fact.
5. The docs, the skill, the template and the ADR.

## Acceptance criteria

- [ ] A PR that changes a schema, the template, a format or a problem
      code, with no skill changed and no `Skills:` line, fails the
      `tickets` job with a message naming the line to write.
- [ ] The same for the book and `Book:` on a schema or template change.
- [ ] A rule whose glob matches no tracked file fails the check.
- [ ] Run over the last 60 PRs, the check flags 2225 for the skill
      pack (shown in the completion notes).
- [ ] The docs map, the `work-ticket` skill, the ticket template and an
      ADR say it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each case above on fake diffs and tickets; the RON file's
  parse and globs.

## Completion notes

*(Filled in by the session that completes the ticket.)*
