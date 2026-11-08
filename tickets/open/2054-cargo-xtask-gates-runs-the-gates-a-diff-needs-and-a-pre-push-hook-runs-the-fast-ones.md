---
id: "2054"
title: "`cargo xtask gates` runs the gates a diff needs, and a pre-push hook runs the fast ones"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2054 — `cargo xtask gates` runs the gates a diff needs, and a pre-push hook runs the fast ones

## Context

`run-gates` is long: 20 sections, each with its own command. Agents pick
the gates they think apply, and the PR body says "all gates pass". The
retro of engine PRs #254–#313 (2026-11-08) found that the cheap gates
never failed on any of the 60 PRs: fmt, clippy, docs, deny, machete,
layers, modules, no-std, wasm, tutorial, templates and zizmor. The ones
that failed in CI were gates `run-gates` covers but the session left out:

- **#263:** `compile-times`, with 149 crates against a budget of 145.
- **#276:** the engine lint SH2101 on a lone `"x"`. That trap is in
  `run-gates`' "engine's own input and text rules" section.
- **#279:** the skill examples, after a template change. The PR said
  every local gate passed.
- **#273:** a typo pushed in a follow-up commit.
- **#284 and #301:** a clippy fix pushed minutes after opening, in
  `site/snippets` for #284.
- **#269:** the `site` job. `run-gates` never mentions
  `site/check-build.sh`, so this one is a real gap.

The game's retro found the same pattern: game PRs #514, #554 and #572
pushed without fmt, typos or clippy. Its twin is game#0145.

## Nick input

None.

## Scope

**In:**
- `cargo xtask gates [--base <rev>] [--fast] [--list]` decides from the
  diff which gates apply, using the same path rules as
  `.github/workflows/changes.yml` (read from that file, not copied).
  It runs them and prints a summary block with the `HEAD` commit, each
  gate run, its result, and each gate left to CI with the reason ("CI
  only: sound parity"). The block is pasted into the PR body.
- `--fast` runs only the gates that take seconds: `cargo fmt --all
  --check`, `typos`, `cargo xtask ticket-lint --pr-branch <current>`,
  and the engine lint (`cargo run -q -p shuyi-cli -- lint`) when `.rs`
  files changed.
- A tracked hooks folder, `scripts/git-hooks/pre-push`, which runs
  `cargo xtask gates --fast`. `scripts/setup-linux.sh` and a line in
  `AGENTS.md` set it up with `git config core.hooksPath
  scripts/git-hooks`. It works in every harness because git runs it,
  not the harness.
- `site/check-build.sh` becomes a gate in `run-gates` and in `gates`
  when `site/` changes.
- `run-gates` starts with `cargo xtask gates` and keeps its sections as
  the reference for each gate.

**Out (do not do):**
- Mutation testing locally (never on Nick's machine; 2055 covers what
  can happen before the push).
- The CI-only gates (sound parity, render, bench).
- The game's version (game#0145).

## Implementation steps

1. Read `.github/workflows/changes.yml` and write
   `crates/xtask/src/gates.rs`. It parses the filters (they are YAML
   path globs) and maps each filter to the commands `run-gates` gives
   for it.
2. A gate is a name, a filter, a command line and a `ci_only` flag, kept
   in one table in `gates.rs`. A test checks that every job in `ci.yml`
   that has a filter appears in the table.
3. Add `gates` to `crates/xtask/src/main.rs`. The summary goes to
   stdout, in a form a PR body can hold.
4. Write `scripts/git-hooks/pre-push`, a POSIX sh script, and run it
   from Git Bash on Windows. The `.ps1` sync tooling shows the pattern.
5. Update `run-gates`, `work-ticket` §4 and §6 (paste the summary
   block), and `AGENTS.md`'s environment section. Then run
   `pwsh -File scripts/sync-agent-skills.ps1`.

## Acceptance criteria

- [ ] On a diff touching only `docs/`, `cargo xtask gates --list` lists
      the docs and tickets gates and not clippy.
- [ ] On a diff touching `templates/`, it lists the templates job and
      the skill examples.
- [ ] On a diff touching `site/`, it runs `site/check-build.sh`.
- [ ] With the hook set up, pushing a commit with a typo or unformatted
      Rust is refused.
- [ ] A test fails when `ci.yml` gains a filtered job that `gates.rs`
      doesn't know.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: path-to-gate selection over fixture file lists.
- Unit: the `ci.yml` and `changes.yml` coverage test.
- Manual: the hook refusing a push, recorded in the completion notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
