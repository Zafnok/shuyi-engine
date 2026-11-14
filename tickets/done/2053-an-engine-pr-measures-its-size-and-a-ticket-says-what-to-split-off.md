---
id: "2053"
title: An engine PR measures its size, and a ticket says what to split off
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
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

- [x] `cargo xtask pr-size --base origin/main` prints counted and
      excluded totals.
- [x] Run against #274's squash commit on `main` (`gh pr view 274 --json
      mergeCommit`, then `--base <that commit>^`), it reports over 1,500
      counted lines.
- [x] A PR over 1,500 lines whose ticket has no `Size:` line fails the
      CI step, and passes with one.
- [x] An open `effort: high` feature ticket with no estimate fails
      `ticket-lint`.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the line counter over a fixture diff with each excluded kind.
- Unit: the `Size:` and estimate rules in `ticket-lint`.

## Completion notes

What was done:

- `cargo xtask pr-size [--base <rev>] [--hook] [--ticket <file>]
  [--pr-branch <branch>] [--pr-title <title>]` (`crates/xtask/src/pr_size.rs`),
  ported from the game's: the lines added plus removed from the base's
  merge base to the working tree, untracked files included. It prints the
  counted total by kind (code, tests, docs, data), the left-out total by
  why (snapshots, golden tables, schemas, lock files, generated), the
  tickets' lines apart, and the five largest counted files. Past 1,000
  lines it warns; past 1,500 a `[NNNN]` PR needs a `Size:` line in its
  done ticket's notes with the measured number (within a tenth) and why
  it wasn't split. A ticket-only, bot or Sonar PR is only reported.
- CI: a `Measure the PR's size` step in the `tickets` job, after the
  ticket lint, on pull requests (`--base HEAD^1`, the PR's title and
  branch). It fails as above; a local run and the hook only warn.
- The hook: six `PreToolUse` entries in `.claude/settings.json`, in the
  game's shape (`git commit`, `git push`, `gh pr create`, under Bash and
  PowerShell). It measures the session's worktree, from the hook's `cwd`.
- `ticket-lint` (`crates/xtask/src/tickets/size.rs`): an open `feature`
  or `infra` ticket of effort `high` or above, not parked, has an
  `**Estimated size:** about N lines` line; any estimate over 1,000
  lines has a `**Split:**` line naming a point in lines; the PR's done
  ticket's `Size:` line, if any, has a number and a reason.
- `write-ticket` step 5 and `work-ticket` §3 say how to estimate, name
  the split, and write a `Size:` line; `tickets/TEMPLATE.md` has the two
  lines; `tickets/README.md`, *Pull requests*, and the docs map say what
  CI checks.

Acceptance, as run on this branch:

- `cargo xtask pr-size --base origin/main` printed the counted total
  (1,594 at the time) by kind, the left-out total and the tickets'.
- In a worktree at #274's squash commit (`bc31403`), `pr-size --base
  HEAD^` reported 3,278 counted lines (69 of snapshots and 6 of golden
  tables left out). `--base bc31403^` from this branch measures from the
  merge base with this branch's HEAD instead, so it counts all of `main`
  since then; the worktree run is the one the criterion means.
- The CI rule fails and then passes in
  `pr_size::tests::a_real_diff_is_measured_and_judged`: a throwaway
  repository on `t2999-x`, 1,505 counted lines, judged with
  `--pr-branch` and `--pr-title "[2999] X"`, fails with no done ticket,
  and passes once its notes have `Size: 1,560 counted lines, …`.
- `tickets::size::tests::a_high_feature_ticket_with_no_estimate_fails`,
  and the real tickets: before the list below, `ticket-lint` failed 94
  open tickets.

Deviations:

- The left-out paths are this repository's, matched by path in Rust
  rather than the game's `linguist-generated` pathspec, so each kind can
  be totalled apart. Golden tables are any non-Rust file under a
  `golden` folder (they live in `crates/*/tests/golden/` and
  `examples/*/tests/golden/`, not `crates/*/golden/`); the schemas are
  `crates/cli/schemas/` (no `*.schema.json` exists; that pattern is kept
  too); every `Cargo.lock`, the tutorial steps' and the research tools'
  included; generated files are those that say `Do not edit`: the agent
  skill loaders in `.agents/skills/`, each `font/atlas.ron` and each
  `app_info.rs` outside the engine's own `crates/`. `site/snippets/` is
  counted as docs, and `tickets/` apart, as the ticket says.
- `--pr-branch` and `--pr-title` were added beside `--ticket`: CI reads
  the PR's ticket from its title, as `ticket-lint` and `api` do.
- `pr-size` measures the checkout it is run in, not the one xtask was
  built from, so an older revision's worktree can be measured (the #274
  criterion).
- Back-fill: the 94 open tickets that lack an estimate are listed in
  `tools/unestimated-tickets.txt` and let off, without a warning each
  run; an id that no longer needs to be there warns. Estimating 94
  tickets well means reading each against the code, which is each one's
  session's work (`work-ticket` §3 says to estimate before the code and
  take the id off). A new ticket is never let off. Three of them
  (2397, 3032, 3079) have `parked` in their file names but not `Parked:`
  in their titles, so the lint treats them as open.
- A mark inside inline code or a fenced block is not an estimate, so a
  ticket can quote the rule.

Size: 1,598 counted lines, the lint rule and the CI check share the
`Size:` parser and the skills' one text, and splitting the estimate rule
off would have left that text half true for a PR.

Book: none, this is the repository's own tooling (`xtask`), with no
public item a game uses.

No break, and nothing a game sees: `xtask` is never shipped, and the
game's `pr-size` and its hook are its own.
