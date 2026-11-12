---
id: "2054"
title: "`cargo xtask gates` runs the gates a diff needs, and a pre-push hook runs the fast ones"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
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

- [x] On a diff touching only `docs/`, `cargo xtask gates --list` lists
      the docs and tickets gates and not clippy.
- [x] On a diff touching `templates/`, it lists the templates job and
      the skill examples.
- [x] On a diff touching `site/`, it runs `site/check-build.sh`.
- [x] With the hook set up, pushing a commit with a typo or unformatted
      Rust is refused.
- [x] A test fails when `ci.yml` gains a filtered job that `gates.rs`
      doesn't know.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: path-to-gate selection over fixture file lists.
- Unit: the `ci.yml` and `changes.yml` coverage test.
- Manual: the hook refusing a push, recorded in the completion notes.

## Completion notes

What was done:

- `cargo xtask gates [--base <rev>] [--fast] [--list]`
  (`crates/xtask/src/gates.rs`). It reads the `classify` step of
  `.github/workflows/changes.yml` at run time and evaluates its `case`
  arms over the diff (committed from the merge base with `origin/main`,
  plus anything uncommitted), so the `code`, `crates` and `sound` outputs
  are CI's own. One table holds every gate: its name, the `ci.yml` job it
  mirrors, the job's filter, an optional local narrowing, whether
  `--fast` runs it, its commands (the `run-gates` skill's), or why it is
  CI's alone. It runs them fast to slow, stops at the first failure, sends
  the gates' output to stderr, and prints the summary block (HEAD, each
  gate and its CI job, its result, each CI-only gate and why) on stdout.
- A test parses `ci.yml` and fails on a job the table doesn't name, a
  gate whose filter differs from its job's, or a gate naming a job
  `ci.yml` lacks. Every job counts, not only the filtered ones; the
  checks of other jobs (`changes`, `api-label`, `ci-result`) are listed
  as no gate.
- `scripts/git-hooks/pre-push` (POSIX sh) runs `cargo xtask gates --fast`:
  fmt, typos, the ticket lint (`--pr-branch` only on a `tNNNN-` branch,
  as other branches name no ticket), and the engine lint when a `.rs`
  file changed. `setup-linux.sh` sets `core.hooksPath` (not on CI, and
  only in a checkout that has the folder); `AGENTS.md`'s environment
  section says to set it.
- The site's checks are a gate (`site` job): `site-data`, `zola check`,
  `check-book` and `site/check-build.sh`, and `run-gates` has a section
  for them.
- `run-gates` starts with `cargo xtask gates`; `work-ticket` §4 and §6
  say to keep the summary block for the PR's body.

Deviations:

- The ticket called the filters YAML path globs: they are shell `case`
  patterns in the classifier's script, so `gates.rs` parses the YAML for
  the script, then the `case` arms, with the shell's `*` (which matches
  `/`).
- The `site` gate runs locally when `site/`, `crates/cli/errors/`,
  `examples/hello-grid/` or `tickets/` changes, not only `site/`: those
  are what its checks read (the problem codes, the playground's files,
  the feature cards' tickets). CI runs it on every PR.
- `compile-times` has a new `--count-only`: it makes the `blank` game
  and holds its crate count to the budget, without the timings, whose
  budget is a CI runner's. That is the gate's local form (what #263
  failed on).
- The `tutorial` job (ten copies of the tutorial's game, each with
  `cargo update` on its lock) is CI-only, with its reason in the
  summary, as are `determinism` (wasmtime), `export`, `render`,
  `web-render`, `bench`, `coverage`, `zizmor`, `api`, the sound jobs and
  `playtest`. `modules` runs `run-gates`' three commands, not CI's build
  of each feature alone.
- A test holds the template's `setup-linux.sh` equal to the engine's,
  so the template's copy has the hooks clause too. In a new game it does
  nothing until the game adds a `scripts/git-hooks` folder of its own;
  then it sets it up (game#0145 may use that).
- `tools/sound-parity/footprint.ron` covers the three sound gates and
  the `-p shuyi-sound` of the no-std step, so giving up takes them out
  with the jobs.

Manual check of the hook (to a throwaway bare repository, with `git -c
core.hooksPath=scripts/git-hooks push`): a commit adding a Markdown file
with two typos was refused by the `typos` gate, and a commit with an
unformatted Rust line was refused by the `fmt` gate. The bare repository
had no ref after either.

Book: none, this is the repository's own tooling (`cargo xtask`, a git
hook and the skills); no public API or game-facing behaviour changed.

No break.

Game: nothing to adopt, the game's own gates command and hook are its twin
ticket game#0145; a new game's `setup-linux.sh` sets git's hooks only once
the game adds a `scripts/git-hooks` folder.

No ticket: this ticket leaves no work behind.
