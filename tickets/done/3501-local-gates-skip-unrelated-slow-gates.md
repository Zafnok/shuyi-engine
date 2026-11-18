---
id: "3501"
title: Local gates run the slow game-building gates only when the diff touches what they build, and a website-only change runs only the site's gates
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-17
---

# 3501 — Local gates run the slow game-building gates only when the diff touches what they build, and a website-only change runs only the site's gates

## Context

`cargo xtask gates` (2054) picks its gates from the diff using CI's own
classifier (`.github/workflows/changes.yml`). Most gates then follow their
`ci.yml` job's filter. Only `site` and `tutorial locks` (2073) add a
narrower local rule (`Local::Under`, `Local::When` in
`crates/xtask/src/gates.rs`). The five gates that build whole games run on
the coarse `code` output (`gates.rs`, the `templates (grid)`,
`templates (blank)`, `skill examples`, `tutorial-tries` and `exercises`
entries). They are the slowest gates by far. And
`changes.yml`'s last arm, `*) … code=true`, makes every non-Markdown path
under `site/` code.

The retro of PRs #379–#484 (2026-11-17, problem 4, reached by 9 reviewers
on their own) measured this across 70 PR bodies with timed gates: the
median local `cargo xtask gates` run took about 32 minutes. Checked
against the PR bodies (`gh pr view N --json files,body`):

- **#420** changed 7 lines of one test file, `crates/cli/tests/it/binary.rs`,
  plus its ticket. Its gates took about 3,900 s. Of that, skill examples
  took 957 s, tutorial-tries 900 s, exercises 298 s and the two templates
  329 s. None of them builds that file.
- **#415** changed one number in `tools/compile-budget.ron`, plus its
  ticket. Every gate ran, including both templates, skill examples and
  tutorial-tries. Only `compile-times` and `test` read that file
  (`crates/xtask/src/compile_times/tests.rs` reads it).
- **#405** changed only `site/templates/`, `site/static/`,
  `site/check-build.sh`, Markdown and its ticket. It ran the whole Rust
  set: clippy, test, no-std, wasm, templates, skill examples and
  tutorial-tries, about 25 minutes of gates that read none of those
  files. In CI the same diff was `code` and ran every heavy job.
- **#475** (a shell retry and docs) took about 57 minutes. It touched
  `templates/`, so its runs were warranted. It shows the cost when the
  gates do apply.

That is roughly 9 machine-hours per 10 PRs on Nick's machine. It is also
what fills the disk that the worktree tooling then has to trim
([ADR-0121](../../docs/adr/0121-a-worktrees-build-output-kept-compressed-and-trimmed.md)).
CI then runs the same jobs again.

What each of those gates actually reads (checked in the code):

- **Every game they build** is made by `shuyi-cli` or depends on the
  engine's crates. Their inputs are:
  - the crates' non-test code under `crates/`;
  - `templates/` (`include_dir!` in `crates/cli/src/template.rs`);
  - `skills/` (`crates/cli/src/skills.rs`);
  - `targets/` (`crates/cli/src/contract/target.rs`, `crates/app/src/facts.rs`);
  - `docs/modules.ron` (`crates/cli/src/catalog.rs`);
  - `LICENSE-MIT` and `LICENSE-APACHE` (`crates/cli/src/notices.rs`);
  - the root `Cargo.toml` (`crates/playdate/src/lib.rs` too), `Cargo.lock`,
    `rust-toolchain.toml` and `.cargo/`.

  Test-only files don't change any of those games: `crates/*/tests/`,
  `crates/*/benches/`, a `tests.rs` or a `tests/` folder under `src/`,
  and `*.snap`.
- **`skill examples`**: those inputs, plus its runner,
  `crates/xtask/src/skills.rs` and `crates/xtask/src/script.rs` (and their
  folders).
- **`tutorial-tries`**: those inputs, plus the paths in
  `crates/xtask/src/tries.rs`' constants:
  - `site/tutorial/` (the steps and `tries.toml`);
  - `site/static/tutorial-tries/` (the pictures);
  - `site/content/learn/book/` (the pages);
  - `site/snippets/` (the book's game);

  plus `crates/xtask/src/tries.rs` and `tries/`. It also copies the root
  `Cargo.lock`.
- **`exercises`**: those inputs, plus `site/tutorial/`, `site/exercises/`
  and `site/content/learn/book/box-pusher/`, plus
  `crates/xtask/src/exercises.rs` and `exercises/`, and what it calls:
  `book.rs` (and `llms.rs` through it) and `tries.rs`' `copy_tree`.
- **`templates (grid)` and `templates (blank)`**: those inputs. Their
  steps are inline in `gates.rs`, so no runner file is needed.

On `site/`, what a code job or a crate's test reads:

- `site/tutorial/`, `site/exercises/` and `site/snippets/` (a workspace
  member, `Cargo.toml` `members`);
- `site/static/tutorial-tries/`;
- `site/config.toml`, which `crates/xtask/src/llms/tests.rs` reads.

The rest is read only by CI's `site` job, which runs on every PR
(no `if:`): `site/templates/`, `site/static/` (all but the tries'
pictures), `site/syntaxes/`, `site/data/`, `site/tests/`, `site/*.sh`
and non-Markdown files under `site/content/`. CodeQL scans `rust` and
`actions` (`codeql.yml`), and Sonar scans `crates` and `examples`
(`sonar-project.properties`). So neither loses anything when those paths
stop being code. Markdown under `site/` is already docs-only (the `*.md`
arm).

2020 plans an `assets` output in its own `case` block of the same
classifier. `classify` reads every block, so the two don't touch.

## Nick input

None.

## Scope

**In:**
- **Local rules** in `crates/xtask/src/gates.rs`, as
  `Local::When(<text>, <fn>)` (`Local::Under` matches prefixes only, and
  the test-file exclusions need more):
  - one shared predicate, `builds_a_game(path)`, for the inputs listed in
    Context, test-only files left out;
  - `templates (grid)` and `templates (blank)`: `builds_a_game`;
  - `skill examples`: that, or its runner files;
  - `tutorial-tries`: that, or its `site/` paths, or its runner files;
  - `exercises`: that, or its `site/` paths, or its runner files and
    what they call.

  Each rule's text says what it looks for, as the `tutorial locks` rule's
  does.
- **The site split in `changes.yml`.** The classifier's code `case` block
  gains a site arm, so a website-only path is neither code nor docs:
  - the code arm also names `site/tutorial/*`, `site/exercises/*`,
    `site/snippets/*`, `site/static/tutorial-tries/*` and
    `site/config.toml`;
  - after the docs arm (`*.md|docs/*|…`), a new arm
    `site/*) echo "site: $path" ;;` that sets no output;
  - the comment at the top of `changes.yml` lists the new class and why:
    `site/…` paths that a code job reads stay code, and the rest only
    the `site` job reads.

  **No new output.** CI's `site` job has no `if:` and keeps running on
  every PR. An output that no job reads would be dead. (The retro's
  wording said "a `site` output"; this is the same split without one.)
- **CI's behaviour.** Only the site split changes it. A PR whose code
  paths are only those website files no longer runs the `code` jobs in
  CI (#405's diff: `code=false`), and `site` still runs. **CI does not
  narrow anything else.** `skills`, `tutorial-tries`, `exercises` and
  `templates` keep running on every code PR in CI. They are the safety net
  for anything the local rules miss, and the local narrowing exists to
  save Nick's machine, not the runners. No gate is weakened: every check
  still runs on every PR whose files it reads, in CI at least.
- **The summary block** (`summary` in `gates.rs`, which goes into the PR's
  body) puts the CI-only gates on one line after the table, not one row
  each. For example: `CI only (\`cargo xtask gates --list\` gives each
  reason): tutorial, determinism, export, render, web-render, bench,
  coverage, zizmor, api, sound parity, sound give-up, playtest.` `--list`
  keeps one row each, with the reason. "Not needed" rows stay in the
  table, each with its reason: they show why a narrowed gate didn't run.
- The `run-gates` skill: each of the four gates' sections says when the
  local run picks it, and the summary's CI-only line replaces the rows.
  It also says where the site split lives (the `changes.yml` comment).
  `cargo xtask gates`' module doc in `gates.rs` names the new rules.

**Out (do not do):**
- Running independent heavy gates at the same time. The gates run fast
  to slow and stop at the first failure. Each cargo build already uses
  every core. Skill examples and the templates both `cargo run -p
  shuyi-cli` in `target/` and wait on its lock. Nick's machine often
  runs several worktrees' gates at once, and their disk is what ADR-0121
  already trims. Running two at once might save some minutes or cost
  some, and that hasn't been measured. Decide it, if at all, once this
  ticket's narrowing shows in the summaries' times.
- Narrowing `clippy`, `test`, `docs`, `no-std`, `modules`, `wasm` or
  `compile-times`. The workspace's tests read files all over the
  repository (`tools/`, `docs/`, `site/config.toml`), and these gates
  take minutes, not a quarter of an hour.
- Running only the changed skills' examples (the `run-gates` skill's
  advice, by hand). A gate's steps are fixed strings.
- Making CI run `tutorial-tries` or `exercises` on a diff that only
  changes a book page. A `.md` under `site/content/learn/book/` is
  docs-only to CI today, so neither job runs on it, and that is unchanged
  here. A local rule can't fix it either, because `plan` checks CI's
  filter first.
- Changing the `site` gate's local rule or CI's `site` job.

**Estimated size:** about 800 lines (code 250, tests 400, docs and the
classifier 150).

## Implementation steps

1. In `gates.rs`, add `builds_a_game(path: &str) -> bool`:
   - a path under `crates/` and not under `crates/xtask/`, unless it is
     a test-only file (a `/tests/` or `/benches/` segment, a file named
     `tests.rs`, or one ending in `.snap`);
   - or a path under `templates/`, `skills/`, `targets/` or `.cargo/`;
   - or one of `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
     `docs/modules.ron`, `LICENSE-MIT`, `LICENSE-APACHE`.
2. Add `touches_skill_examples`, `touches_tutorial_tries` and
   `touches_exercises`. Each is `builds_a_game` or the gate's own paths
   from Context. Give each a doc comment naming the constants it mirrors
   (`tries::PICTURES`, `exercises::EXERCISES`, …). Make those constants
   `pub(crate)` where needed, so the paths are written once.
3. Set the four gates' `.local(Local::When(…))` (the two templates share
   one). The texts read like the existing `tutorial locks` one: "nothing
   a new game builds changed (crates/ outside tests and xtask,
   templates/, skills/, targets/, the root's Cargo files, …)".
4. In `.github/workflows/changes.yml`, extend the code arm and add the
   `site/*` arm after the docs arm, as Scope says. Update the comment.
5. In `summary`, collect the `Plan::CiOnly` gates into one line after the
   table. `run` with `--list` keeps a row each. Thread the `list` flag
   through `Header` if that's the simplest way.
6. Update the `run-gates` skill: the four gates' sections, the summary
   line and the website section. Check whether `work-ticket` §6's
   description of the block still holds; it should, since the block still
   names each gate that ran.
7. Run `cargo xtask gates --list` on a scratch branch for each evidence
   diff (#420's, #415's and #405's file lists), and paste the plans into
   the completion notes.

## Acceptance criteria

- [x] For #420's files (`crates/cli/tests/it/binary.rs`, `docs/ROADMAP.md`,
      `tickets/done/2063-….md`, `tickets/open/2063-….md`), the plan runs
      clippy and test. It runs neither template, nor skill examples,
      tutorial-tries or exercises.
- [x] For #415's files (`tools/compile-budget.ron`, `tickets/done/2080-….md`),
      the plan runs `compile-times (crates)` and `test`, and none of the
      five game-building gates.
- [x] For #405's files (the list in Context), the classifier says
      `code=false`. The plan runs `site` and the tickets' checks, and no
      Rust gate: no fmt, clippy, test, docs, no-std, modules, wasm,
      templates, skill examples or tutorial-tries.
- [x] For `crates/app/src/lib.rs` alone, the plan runs both templates,
      skill examples, tutorial-tries and exercises.
- [x] A runner file runs exactly the game-building gates that use it:
      `crates/xtask/src/skills.rs` runs skill examples only,
      `crates/xtask/src/tries.rs` runs tutorial-tries and exercises
      (`copy_tree`), `crates/xtask/src/exercises.rs` runs exercises only,
      and `crates/xtask/src/gates.rs` runs none of the five.
- [x] `site/static/tutorial-tries/<any>.png` runs tutorial-tries.
      `site/exercises/<any>` runs exercises. `skills/shuyi-gates/example.ron`
      and `templates/<any>` run all five.
- [x] `site/config.toml`, `site/snippets/src/lib.rs`,
      `site/tutorial/03-walk/crates/game/src/main.rs` and
      `site/exercises/<any>` classify as code. `site/templates/base.html`,
      `site/static/style.css` and `site/check-build.sh` don't.
- [x] The summary of a run puts every CI-only gate on one line after the
      table. `--list` still gives each one's reason.
- [x] CI's `ci.yml` is unchanged. Its jobs run on every PR whose
      `code` output is true, as before.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`gates.rs` tests, through the existing `running` helper, which
  reads the real `changes.yml`): one test per acceptance diff above, with
  #420's, #415's and #405's file lists written out as the PRs had them.
- Unit: `builds_a_game` on each kind of path in step 1, test-only files
  included (`crates/assets/tests/golden/formats.ron`,
  `crates/xtask/src/api/tests.rs`, `crates/ui/src/x/tests/y.rs`, a
  `.snap`).
- Unit: the paths `tries.rs` and `exercises.rs` name in their constants
  each make their gate run. Reading the constants keeps the rule from
  drifting from the code.
- Unit: `classify` on the site paths of the acceptance list.
- Unit: the summary's CI-only line (adapt
  `the_summary_names_head_each_result_and_the_ci_only_gates`).
- The existing `every_ci_job_is_a_gate_with_its_filter` still passes: the
  filters are unchanged, and only local rules are added.

## Completion notes

What was done:

- `gates.rs` gains `builds_a_game(path)`: the crates' code outside xtask
  and the tests (a `tests` or `benches` folder, a `tests.rs`, a `.snap`),
  `templates/`, `skills/`, `targets/`, `.cargo/`, and the root's
  `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `docs/modules.ron`
  and the two licenses. `touches_skill_examples`, `touches_tutorial_tries`
  and `touches_exercises` add each gate's own paths, read from
  `tries`' and `exercises`' constants (now `pub(crate)`: `tries::STEPS`,
  `PICTURES`, `PAGES`, `SNIPPETS`; `exercises::STEPS`, `EXERCISES`,
  `PAGES`), and its runner modules in xtask (`skills`, `script`; `tries`;
  `exercises`, `book`, `llms`, `tries`), file or folder.
- The two templates, skill examples, tutorial-tries and exercises take
  those as `Local::When` rules. A `When` rule's text is now the whole
  reason in the summary (the `tutorial locks` text starts "nothing
  changed:" itself), so the new texts read "not needed: nothing changed
  that a new game builds from (…)". The list of inputs is written once,
  in a `game_inputs!` macro the texts `concat!`.
- `changes.yml`'s code arm also names `site/tutorial/*`,
  `site/exercises/*`, `site/snippets/*`, `site/static/tutorial-tries/*`
  and `site/config.toml`; a new `site/*` arm after the docs arm sets no
  output. The comment at the top says which `site/` paths stay code and
  why the rest sets nothing. No new output; `ci.yml` is unchanged.
- The summary puts the CI-only gates on one line after the table;
  `--list` keeps a row each with its reason (`Header` gains `list`, and
  the block's first line names `--list` when it was given).
- The `run-gates` skill says when the local run picks each of the five,
  what a game is built from, the summary's CI-only line, and where the
  site split lives. The `work-ticket` skill's §6 still holds: the block
  names each gate that ran and which CI alone runs. `gates.rs`' module
  doc names the new rules.

The plans for the evidence diffs (`cargo xtask gates --list --base
<this branch>` on a scratch commit touching each PR's files), CI-only
line left out:

- #420 (`crates/cli/tests/it/binary.rs`, `docs/ROADMAP.md`, the 2063
  ticket in `open/` and `done/`): fmt, typos, ticket-lint, engine lint,
  skill loaders, tickets' checks, layers, site (for `tickets/`), clippy,
  docs, deny, machete, test, no-std, modules, wasm and compile-times
  run; tutorial locks, both templates, skill examples, tutorial-tries
  and exercises are "not needed".
- #415 (`tools/compile-budget.ron`, the 2080 ticket): the same plan as
  #420: compile-times and test run, none of the five game-building
  gates.
- #405 (the site's templates, statics, `check-build.sh`, Markdown and
  the 2834 ticket): `code` is false; typos, ticket-lint, skill loaders,
  tickets' checks and site run, and every Rust gate is "not needed:
  CI's `code` filter is false for this diff".

Deviations: none from the steps. The ticket was written today, so §0's
age check found nothing to re-check.

What CI does differently: a pull request whose only non-docs paths are website-only
files under `site/` (not the tutorial, the exercises, the snippets, the
tries' pictures or `site/config.toml`) no longer runs the `code` jobs;
the `site` job runs as before. No game sees it: the game runs no engine
workflow and no xtask.

No break: only xtask (unpublished) and the workflows change.

Book: none, no public API or behaviour a game sees changed; the change
is the engine's own gates.

Nick input: none.
