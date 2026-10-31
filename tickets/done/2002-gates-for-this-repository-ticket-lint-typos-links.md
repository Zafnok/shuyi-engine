---
id: "2002"
title: "Gates for this repository before the code arrives: a ticket lint that knows game# ids, typos, and Markdown links, in CI"
type: infra
milestone: MVP
model: implementation
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-19
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

1131 came first (2026-10-19): it brought the code with the full code
gates (fmt, clippy, tests, mutation shards, `cargo deny`, the web build)
and started `.github/workflows/ci.yml` with them. This ticket adds its
three checks to that CI, and its `xtask` crate to the workspace that
came with the code.

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
- `.github/workflows/ci.yml` runs the three on every PR and on `main`,
  beside the code gates; `typos` and the ticket lint always run, like the
  game's `tickets` job, not only when code changes.
- The `run-gates` skill lists them.

**Out (do not do):**
- the code gates (1131 added them);
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

- [x] `cargo xtask ticket-lint` passes on the repository, and its tests
      cover each changed rule.
- [x] `cargo xtask check-links` and `typos` pass.
- [x] CI runs all three on a PR.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each lint rule, passing and failing.

## Completion notes

- `crates/xtask` (repository tooling, `publish = false` like the rest of
  the workspace) with `cargo xtask ticket-lint` and `cargo xtask
  check-links`; the alias is in `.cargo/config.toml`. The workspace and
  `rust-toolchain.toml` already came with the code (1131), so step 1 only
  added the crate and the alias.
- The lint is the game's `tickets.rs` with its tests, plus:
  1. `game#NNNN` in `blocked_by`, checked against the game's
     `tickets/open` and `tickets/done` with `gh api` or `--game-repo
     <path>`; a malformed `game#` id is an error, an unreachable game a
     warning. The game is only read when some ticket names one of its ids.
     A plain id is never looked up in the game.
  2. `Parked:` tickets must be `status: blocked` with a `> **Parked**`
     note as the first thing under the `# ` heading.
  3. The types are this repository's five; `nick_input` lost the game's
     `answer-first`, which `tickets/README.md` doesn't list.
  4. `--pr-branch t<NNNN>-<slug>` as in the game. It also reads CRLF
     checkouts.
- `check-links` parses Markdown with `pulldown-cmark` (MIT), so links in
  code aren't links. It checks relative links and images, `/`-rooted
  ones from the repository root, with exact letter case as on GitHub, and
  skips web links and anchors. It skips `.git`, `target`, `dist`,
  `node_modules` and the app's `.claude/worktrees`.
- New dependencies, both permissive: `serde_norway` (MIT OR Apache-2.0,
  as in the game) and `pulldown-cmark` without default features (MIT).
- `_typos.toml` was already there from 1131 and needed no new words.
- CI: a `tickets` job that always runs (not gated on `changes`), in
  `ci-result`'s needs. CI's token can't read the private game repository,
  so there the `game#` ids are warned about and skipped; the `run-gates`
  skill says to check them locally (where `gh` is signed in) when adding
  one.
- Mutation testing: `tests/cli.rs` runs the binary for its commands,
  options and exit codes. The `gh api` call itself (`list_on_github`) is
  excluded in `.cargo/mutants.toml`, as `RealWasmOpt` is: no test machine
  is sure to have `gh` signed in to the private game. What it does with
  `gh`'s answer is `gh_listing`, tested on its own.
- The repository had no lint errors, broken links or typos to fix.
- Branch protection is Nick's setting: if he wants it, `ci-result` is the
  one check to require (it now includes `tickets`).
