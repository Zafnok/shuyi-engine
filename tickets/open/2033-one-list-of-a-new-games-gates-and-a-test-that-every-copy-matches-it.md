---
id: "2033"
title: "One list of a new game's gates: the template's AGENTS.md, and a test that every copy matches it"
type: infra
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2033 — One list of a new game's gates, and a test that every copy matches it

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). A game made with `shuyi new` runs seven gates: `cargo fmt
--all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo shuyi check`, `cargo shuyi check
--target all`, `cargo shuyi check-keys` and `cargo shuyi lint`. The list
is written by hand in six places:

- **The template's `AGENTS.md`** (`templates/shuyi-default/base/AGENTS.md`,
  *The gates*, lines 132–145): one code block, the seven lines
  (138–144). It says "run what CI runs (`.github/workflows/ci.yml`)".
- **The template's `ci.yml`**
  (`templates/shuyi-default/base/.github/workflows/ci.yml`, the `gates`
  job, lines 30–40): one step per gate.
- **The root `ci.yml`'s `templates` job** (`.github/workflows/ci.yml`,
  *Run its gates*, lines 179–189): the seven lines in one `run: |`
  block, then `cargo shuyi doctor`, the engine's own.
- **The `run-gates` skill** (`.claude/skills/run-gates/SKILL.md`, *A new
  game from each template*, lines 116–126): `shuyi new`, `cd
  ../demo-grid`, then the seven lines.
- **The `shuyi-gates` skill of the pack** (`skills/shuyi-gates/SKILL.md`):
  its description (line 3) names them in words ("fmt, clippy, the
  tests, check, check --target, check-keys and lint"); step 1 runs
  `cargo shuyi lint --json` (line 33), and step 2's block (lines 46–51)
  the other six.
- **Its worked example** (`skills/shuyi-gates/example.ron`, lines
  29–36): after the last `Edit`, `lint` and then the six, as `Shuyi`
  and `Cargo` steps.

The `blank` and `grid` games share the list: both get `AGENTS.md` and
`ci.yml` from the `base` part. No test compares any two of the six
places. A test in `crates/cli/src/template/tests.rs` checks only that
the grid game's `AGENTS.md` names `crates/rules/clippy.toml` and
`crates/rules/tests/layers.rs`.

Open tickets that change the list must find every copy by hand: 2543
drops `check-keys` from all six (until 2026-10-28 it named only three),
and 3050 adds `--locked` to `clippy` and `test` in the two `ci.yml`
files only.

## Nick input

None.

## Scope

**In:**
- **The source:** the template's `AGENTS.md`, *The gates*: the lines of
  the first code block after the `## The gates` heading. Nothing about
  it changes.
- **One test,** `every_copy_of_a_new_games_gates_is_the_templates_list`,
  in `crates/cli/src/template/tests.rs`. It builds the `blank` and
  `grid` games' plans (`built_in_plan`, read with `text_of`), and reads
  the repository's files from `concat!(env!("CARGO_MANIFEST_DIR"),
  "/../..")`, as `crates/cli/src/contract/codes/tests.rs` does. Before
  comparing, each command is trimmed and loses any `--locked`. It
  checks:
  1. each made game's `.github/workflows/ci.yml`: in the `gates` job,
     the commands of its `run:` lines, from the one that is `cargo fmt
     --all --check` on, are that game's own `AGENTS.md` list;
  2. the *shared list*: the commands of the grid game's list that the
     blank game's list also has, in order (today, all seven);
  3. the root `.github/workflows/ci.yml`, `templates` job: every `run:
     |` block with a line that is exactly `cargo shuyi check`. There must
     be at least one. Its lines from `cargo fmt --all --check` on start
     with the shared list. Lines after it (`cargo shuyi doctor`) are the
     job's own;
  4. `.claude/skills/run-gates/SKILL.md`: under `## A new game from each
     template`, the first code block's lines after its `cd ` line are
     the shared list;
  5. `skills/shuyi-gates/SKILL.md`: a code block under `## Steps` holds
     `cargo shuyi lint --json`, and step 2's block is the shared list
     without `cargo shuyi lint`;
  6. `skills/shuyi-gates/example.ron`: the `Cargo(args: [...])` and
     `Shuyi(args: [...])` steps after the last `Edit(`, as commands
     (`Cargo` is `cargo` and its args, `Shuyi` is `cargo shuyi` and its
     args, joined by spaces), are the shared list with `cargo shuyi
     lint` moved first.

  A failure names the file and gives both lists. It also says the
  template's `AGENTS.md` (`templates/shuyi-default/base/AGENTS.md`,
  *The gates*) is the list, and the copy changes to match it.
- **The readers are small private functions** in the tests module, line
  by line. No new dependency.
- **`shuyi-gates`' description** names no command list. It says "the
  checks a game on Shuyi Engine passes before every commit (`lint`
  first, then the rest its `AGENTS.md` lists)", and keeps its *Use* and
  *Not* sentences. The pack's rules still pass (`cargo shuyi
  check-skills skills`).
- **A comment** above *Run its gates* in the root `ci.yml`, and a
  sentence in `run-gates`' *A new game from each template*: the lines
  are the template's `AGENTS.md` gates, and a test in
  `crates/cli/src/template/tests.rs` keeps them equal.
- **Either order with 2543:** if 2543 is done, write the test on the
  lists as 2543 left them (no `cargo shuyi check-keys` in any of the
  six); if not, write the test on today's lists, and 2543 changes every
  copy the test reads, `shuyi-gates` and its example among them.
- **Either order with 3050:** if 3050 is done, the two `ci.yml` files run
  `clippy` and `test` with `--locked`, and the template's `ci.yml` has a
  Cargo.lock step before `cargo fmt`: check that the test passes, since
  it drops `--locked` and starts at `cargo fmt --all --check`; if not,
  nothing to do (the test is already written that way).
- **Either order with 2374:** if 2374 is done, the `templates` job has a
  step "An eight-way grid game passes its gates": rule 3 reads it,
  since it has a `cargo shuyi check` line. Make its lines match if they
  don't; if not, nothing to do (2374 writes the step to match).
- **Either order with 2527:** if 2527 is done, the soak (`cargo shuyi
  soak 20 --turns 100`) is a check of grid games only, in files only the
  grid stage writes: its own workflow
  (`stages/top-down-grid/.github/workflows/soak.yml`) and its
  `crates/rules/AGENTS.md`. None of the six rules reads those, the
  base's `ci.yml` and `AGENTS.md` gates don't hold it, and the root job
  runs it in its grid-only step, after "Run its gates". Check the test
  passes; if not, nothing to do (2527 adds the soak by these rules).
- **Either order with 2552:** if 2552 is done, its module's lines
  (`fmt`, `clippy`, `test` in the module's folder, `shuyi add`, `cargo
  build`) are in a `run:` block and a code block of their own: check
  that rule 3 doesn't read that block (it has no `cargo shuyi check`)
  and rule 4 reads only the game's block; if not, nothing to do (2552
  keeps its lines apart).
- **Either order with 2524:** if 2524 is done, its grader's `Gates`
  check (`crates/xtask/src/eval/`) is one more copy. Make it read its
  commands from `templates/shuyi-default/base/AGENTS.md`'s *The gates*
  block, minus the ones it leaves out today, never from the graded
  game's copy; if not, nothing to do (2524 reads the block).
- **For the game:** no API change (ADR-0038 §6: no `Breaking:` line).
  A test, a skill's description and two comments; no template file
  changes, so no made game changes.

**Out (do not do):**
- Changing which gates a game runs: 2543 drops `check-keys`, and 3050
  adds `--locked`.
- Writing the copies from the source (a generator, or an include): the
  copies are a workflow, Markdown and RON, read by people and agents as
  they are. The test is enough.
- The engine's own gates (the rest of the `run-gates` skill, and CI's
  other jobs), and the `hello-grid` steps of CI's examples job
  (`ci.yml` line 117).
- Other skills' examples that run some of the same commands
  (`shuyi-input`, `shuyi-screen`, `shuyi-ship`): they run what their
  skill teaches, not the gate list.
- An `xtask` check instead of the test: only `shuyi-cli` composes a
  template's plan, and its tests already read the repository's files.

## Implementation steps

1. Read `crates/cli/src/template/tests.rs` (`built_in_plan`, `text_of`,
   `the_engine_template_has_blank_and_grid_and_a_stage_for_grid`), and
   the six places in Context.
2. Write the readers as private functions in that tests module, each
   with a unit test on a short sample:
   - `agents_gates(md)`: the first code block after `## The gates`;
   - `workflow_gates(yml)`: the `gates` job's `run:` commands from
     `cargo fmt --all --check` on;
   - `job_gate_blocks(yml)`: the `run: |` blocks of the `templates` job
     with a `cargo shuyi check` line;
   - `example_gates(ron)`: the `Cargo` and `Shuyi` steps after the last
     `Edit(`, as commands;
   - `command(line)`: trimmed, with `--locked` dropped.
3. Write `every_copy_of_a_new_games_gates_is_the_templates_list` with
   rules 1–6 of Scope.
4. Settle the either-order clauses above with what is done.
5. Change `shuyi-gates`' description. Add the comment in the root
   `ci.yml` and the sentence in `run-gates`.
6. Run `cargo test -p shuyi-cli --lib template`. Then edit one copy by
   hand (not committed), for example `--workspace` gone from `run-gates`'
   `cargo test` line, and see the test fail naming that file. Undo it.
7. Run the gates.

## Acceptance criteria

- [ ] `cargo test -p shuyi-cli --lib template` runs
      `every_copy_of_a_new_games_gates_is_the_templates_list`, and it
      passes.
- [ ] Removing a line from any one of the six places, by hand and not
      committed, makes that test fail with a message naming the file.
- [ ] `grep -n "check --target, check-keys and lint" skills/shuyi-gates/SKILL.md`
      finds nothing.
- [ ] `git diff main --stat -- templates` is empty.
- [ ] `cargo shuyi check-skills skills --tickets tickets/open` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-cli`, `template::tests`): each reader of step 2 on its
  sample: a block after another heading isn't read; a `--locked` is
  dropped; a `run:` before `cargo fmt` is skipped; a block with no
  `cargo shuyi check` line is skipped.
- Unit: `every_copy_of_a_new_games_gates_is_the_templates_list` on the
  real files.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
