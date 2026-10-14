---
id: "2002"
title: "Gates for this repository before the code arrives: a ticket lint that knows game# ids, typos, and Markdown links, in CI"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2002 — Gates for this repository before the code arrives

## Context

Until the code arrives (game ticket 1131), this repository holds only
Markdown: plans, ADRs, tickets and skills. Ticket 2001 set it up with no
CI. The game repository checks its tickets with `cargo xtask ticket-lint`
(`crates/xtask/src/tickets.rs` there) and its text with `typos`. Those
checks have caught real mistakes, such as a `blocked_by` id that doesn't
exist.

This repository has two differences:

- `blocked_by` may name game tickets as `game#NNNN` (`tickets/README.md`);
- `Parked:` tickets have their own rules.

When 1131 brings the code, it brings the full code gates (fmt, clippy,
tests, mutation shards, `cargo deny`, the web build) and adds them to the
CI this ticket starts.

## Nick input

None.

## Scope

**In:**
- A workspace with one crate, `xtask` (repository tooling, never
  shipped), and `cargo xtask ticket-lint`, ported from the game's
  `tickets.rs` with its tests.
- **Changes from the game's lint:**
  1. `blocked_by` entries are this repository's ids, or `game#NNNN`. A
     `game#` id is checked against the game repository's ticket folders
     through `gh api repos/Zafnok/visions-of-shuyi/contents/tickets/open`
     and `…/done`, or against a local checkout given by
     `--game-repo <path>`. With neither available, it warns and skips.
  2. A ticket whose title starts with `Parked:` must have `status:
     blocked` and a `> **Parked**` note right under its heading.
  3. The valid `type`s are `feature`, `infra`, `decision`, `bug` and
     `research`.
  4. The `--pr-branch t<NNNN>-<slug>` check works as in the game.
- `typos` (with a `_typos.toml` that allows the engine's words) and a
  Markdown link check, written as a small `xtask` command that resolves
  relative links (no network).
- `.github/workflows/ci.yml` runs the three on every PR and on `main`.
- The `run-gates` skill lists them.

**Out (do not do):**
- the code gates (they come with 1131);
- branch protection (Nick's GitHub setting; say so in the PR if he wants
  it).

## Implementation steps

1. Set up the workspace (`Cargo.toml`, `rust-toolchain.toml` matching the
   game's, `xtask` with its `[alias]` in `.cargo/config.toml`).
2. Port the lint and its tests. Add the three changes, each with
   passing and failing tests.
3. Add the link check, the typos config, and CI.
4. Run everything on the repository and fix what it finds.

## Acceptance criteria

- [ ] `cargo xtask ticket-lint` passes on the repository, and its tests
      cover each changed rule.
- [ ] `cargo xtask check-links` and `typos` pass.
- [ ] CI runs all three on a PR.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each lint rule, passing and failing.

## Completion notes
