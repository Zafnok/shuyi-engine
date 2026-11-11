---
id: "2053"
title: An engine PR measures its size, and a ticket says what to split off
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2053 — An engine PR measures its size, and a ticket says what to split off

## Context

`write-ticket` asks for one PR of about 1,500 changed lines or fewer,
excluding fixtures and snapshots. Nothing measures that. In the retro of
engine PRs #254–#313 (2026-11-08), 15 of 60 PRs were over 1,500 lines,
and #284, #274, #257 and #277 were over 3,000 with no reason given.

- **Tickets with an estimate ran 2–2.5× over:** 2109, 2350 (600
  estimated, about 1,470 actual), 3046 (650 against about 1,200) and
  3090 (650 against about 1,700).
- **Tickets without one had scopes of several parts** that could each
  have stood alone. 2399 (#274, about 3,000 lines) had three pieces:
  types and raster, one-bit dither, and the `shuyi-app` cache. 3008
  (#277, about 2,450) had scenarios and the report, Callgrind and its
  job, and the targets doc. 2315 (#284) and 2137 (#257) were similar.
- **Size and missed mutants go together:** 13 of the 15 PRs over 1,500
  lines missed mutants in CI, against 8 of the 45 smaller ones.
- **What worked:** a ticket that wrote a size valve ("past 650 lines,
  split off …") split exactly as written. 3042 → 3093, 2226 → 2253 and
  3046 → 3094 were each merged within hours.

The game has had this since game ADR-0101: `cargo xtask pr-size`, a
PreToolUse hook and a CI step, warning about 3,000 and estimating about
2,000. Only 1 of its last 60 PRs was over 3,000.

## Nick input

None.

## Scope

**In:**
- A port of the game's `pr-size` to `crates/xtask`:
  - It counts code, tests, docs and hand-written data.
  - It leaves out snapshots (`*.snap`), golden tables, schema files,
    lock files and generated files, and lists those as a separate total.
  - It has a `--hook` mode for `.claude/settings.json`.
  - It runs as a CI step on PRs.
- CI fails a PR over 1,500 counted lines unless its done ticket's
  completion notes have a `Size:` line with the measured number and why
  it wasn't split. It warns over 1,000.
- `ticket-lint` requires an `**Estimated size:**` line in an open
  `feature` or `infra` ticket of effort `high` or above. When that
  estimate is over 1,000 lines, the ticket must also say what to split
  off and at what point.
- `write-ticket` step 5 and `work-ticket` §3 say how to write both.

**Out (do not do):**
- Splitting open tickets that are already large (each one's session does
  that).
- The game's tool (it exists).
- Counting mutants per size (2055 reads the counts).

## Implementation steps

1. Read the game's `crates/xtask/src/pr_size.rs` and game ADR-0101.
   Copy the counting rules and adapt the excluded paths to this
   repository: `**/snapshots/**`, `crates/*/golden/**`, `**/*.schema.json`,
   `Cargo.lock`, `site/snippets/**` (counted as docs), and
   `tickets/**` (counted apart).
2. Add `pr-size [--base <rev>] [--hook] [--ticket <file>]` to
   `crates/xtask/src/main.rs`.
3. Add the CI step to the `tickets` job, after the ticket lint.
4. Add the hook entry to `.claude/settings.json` in the shape the game
   uses.
5. Add the `ticket-lint` rule for the estimate and the split point.
   Back-fill the open `effort: high` tickets that lack an estimate in
   the same PR, or list them in a warning-only first step.
6. Update `write-ticket` and `work-ticket`, then run
   `pwsh -File scripts/sync-agent-skills.ps1`.

## Acceptance criteria

- [ ] `cargo xtask pr-size --base origin/main` prints counted and
      excluded totals.
- [ ] Run against #274's squash commit on `main` (`gh pr view 274 --json
      mergeCommit`, then `--base <that commit>^`), it reports over 1,500
      counted lines.
- [ ] A PR over 1,500 lines whose ticket has no `Size:` line fails the
      CI step, and passes with one.
- [ ] An open `effort: high` feature ticket with no estimate fails
      `ticket-lint`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the line counter over a fixture diff with each excluded kind.
- Unit: the `Size:` and estimate rules in `ticket-lint`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
