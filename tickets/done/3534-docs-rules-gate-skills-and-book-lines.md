---
id: "3534"
title: "A docs gate over the changed paths: a contract, template or format change updates the book and the skill pack, or its ticket says why not"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
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
  done ticket's `## Completion notes

- **Rules as data:** `tools/docs-rules.ron`, 18 rules. The ticket's first
  ones: a schema (`crates/cli/schemas/**`) asks for the skill pack
  (`Skills:`), the book (`Book:`) and `docs/contract.md` (`Contract:`);
  the template (`templates/**`) for the skill pack and the book or
  `site/tutorial/` (`Book:`); a format for the skill pack, the book and
  `docs/formats.md` (`Formats:`); a problem code for the skill pack. The
  narrowest globs that catch a new `Format`: `docs/formats.md`, the
  standard registry `crates/assets/src/format.rs`, and a kind's
  `crates/assets/src/**/formats.rs` (2203, 2225 and 2234 each touched one).
  The other rows the paths can show, each with its own mark: the template
  format and composer (`Templates:`), the platform boundary (`Backends:`),
  crash reports (`Crash:`), the Playdate (`Playdate:`), the desktop
  packages (`Desktop:`), the web shell (`Web:`), the eval (`Evals:`), the
  site's build (`Site:`) and the sound parity gates (`Sound parity:`).
- **The check:** `cargo xtask docs-rules` (`crates/xtask/src/docs_rules.rs`),
  in CI's `tickets` job on every PR (`--base HEAD^1 --pr-branch --pr-title`)
  and in the local gates' *tickets' checks*, where it only warns and
  reads the ticket from `tickets/open/` too, as `pr-size` does. A
  ticket-only, bot, Sonar or game PR is reported, not failed; so is a PR
  naming no ticket. Every glob must match a tracked file, or it fails.
  A rename counts as both its paths (`--no-renames`). `--head <rev>`
  checks a commit without the working tree, which is how the history
  below was run.
- **Shared with `book_rule`:** the `api` job's message and the new
  check's are built by `docs_rules::ways` and judged by
  `docs_rules::waived` over `tickets::notes_lines` (now `pub(crate)`;
  `tickets::book_lines` went, its one caller gone). The book's message
  now reads "…changed, and nothing in site/content/learn/ or
  site/snippets/ did." where it read "and no file under … did".
- **Docs:** a row in the docs map for `tools/docs-rules.ron`, the
  *Pairs* row on the book and the skill pack's row name the check; the
  `work-ticket` skill's §5 names `Skills:` and the other marks; the
  `run-gates` skill lists the command; `tickets/TEMPLATE.md` has a
  `Skills:` checkbox; ADR-0149 records the rules and why waivers.
- **Deviation:** `tools/sound-parity/footprint.ron` got an edit taking the
  sound parity rule out of `tools/docs-rules.ron`: the give-up test fails
  on any file naming the sound replacement that giving up wouldn't
  clean.
- **Run over the last 60 merged PRs** (`ffda9362` back to `#503`), 13
  would have failed:
  - 2502 (#563): `contract-skills`, `template-skills`;
  - 2225 (#556): `template-skills`, `format-skills`, `playdate`;
  - 3419 (#542): `contract-skills`;
  - 2333 (#541): `template-skills` (its `Book:` line waived
    `template-book`);
  - 2234 (#537): `format-skills`, `playdate`;
  - 2345 (#536): `format-skills`, `playdate`;
  - 2397 (#528): `format-skills`;
  - 2203 (#529): `format-skills`;
  - 3411 (#530): `contract-skills`;
  - 2215 (#527): `template-skills`, `template-format`;
  - 2133 (#523): `template-skills`;
  - 3504 (#518): `template-skills`;
  - 3318 (#509): `format-skills`.

  None is fixed after the fact (the ticket's *Out*); 3428 catches the
  skill pack up on the formats.
- **Size:** 1,257 counted lines, past 1,000. It would have split cleanly
  after the ticket's first rules: the other rows' rules (data, no code)
  could land apart.
- No break: `xtask` is not an engine crate, and no public item changed.
- Book: none, the change is the repository's own tooling; no public item
  a game names changed.
- Nothing in the game must change: it runs neither `cargo xtask
  docs-rules` nor this repository's ticket lint.
