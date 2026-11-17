---
id: "3405"
title: "The shuyi-worktrees skill: when a game seeds a new worktree and trims its idle ones, with its worked example"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["3403"]
nick_input: none
completed: 2026-11-16
---

# 3405 — The shuyi-worktrees skill: when a game seeds a new worktree and trims its idle ones, with its worked example

## Context

Split off 3403 at its **Split:** point (past 1,000 lines: the skill and
its worked example). 3403 gave a game the two commands, `cargo shuyi
trim-targets` and `cargo shuyi seed-target --into <worktree>`
(`crates/cli/src/worktrees/command.rs`), with `--json`, their problem
codes (`SH0007`, `SH0008`, `SH1010`, `SH1011`, `SH3016`, `SH3017`) and
the rows in the template's `AGENTS.md`. This ticket writes the skill
that says when to run them, the way 3403's **In** planned it.

**From 3403 (done):** a draft of the whole skill was written and its
worked example passed `cargo xtask skill-examples shuyi-worktrees` at
the first commit of 3403's PR, `96ecf2ef`; it was taken out in the next
commit, by the split. Start from that commit's `skills/shuyi-worktrees/`
and its `Git` step in `crates/xtask/src/script.rs` and
`crates/xtask/src/script/tests.rs` (`git show 96ecf2ef:<path>`; the
commit stays reachable from 3403's pull request). Check it against
`main` before reusing it.

Both commands act on the game's own git repository and refuse a game in
none (`SH1010`). A game the example runner makes (`shuyi new` in the
engine's `target/skill-examples/`) is in none of its own, so the example
needs the game put in git first, and the runner has no step that runs
`git`. The draft added one: `Git(args: [...], ok: true)`, run in the
game's folder like `Cargo`.

## Nick input

None.

## Scope

**In:**
- The runner's `Git` step (`crates/xtask/src/script.rs`): `git <args>`
  in the game's folder, which must exit 0 when `ok` and not otherwise,
  as `Cargo` does; in the module's doc list and `skills/README.md`'s
  description of `example.ron`.
- `skills/shuyi-worktrees/SKILL.md` (frontmatter as the pack's others;
  `cargo shuyi check-skills skills` passes):
  - when to seed: in a new worktree, before its first build;
  - the command from the new worktree, in Bash and PowerShell: it runs
    in the main checkout, the first line of `git worktree list
    --porcelain`, with `--into` the new worktree;
  - when to trim: after the gates pass, and when the disk is low;
  - what it never does: remove, prune or archive a worktree;
  - that the same works under Codex's and Claude Code's worktree
    folders, since both are `git worktree`s;
  - each problem code of the two commands, and what to do.
- `skills/shuyi-worktrees/example.ron`, on a game fresh from `shuyi new`
  put in git (`Git(args: ["init", "-q"])`):
  - `trim-targets --dry-run --json`: one worktree, the current one, so
    nothing to trim (`/result/idle` 0, `/result/kept` 1);
  - `seed-target --into crates`, a folder that isn't a worktree:
    refused with `SH0007`;
  - `seed-target --into .`: refused with `SH0008`.
  A real seed needs a second worktree with a build, which the runner
  doesn't make: the skill says so.
- `shuyi new` copies the skill like the rest of the pack (it is not a
  module's skill). The skill-pack list in `skills/README.md` names it.
- The template's `AGENTS.md`: its *Git's hooks and worktrees* section
  names the skill, and the `seed-target` row of its tools table points
  to it. The book's quick start (its worktrees paragraph, after `shuyi
  repos`) names it too.

**Out (do not do):**
- The commands themselves (3403), and the walks (2076, 2078).
- A Claude Code or Codex hook that seeds on its own: `AGENTS.md` says a
  shared instruction never relies on one harness's hooks.
- A runner step that makes a second worktree: the seed's real run is
  3403's manual check.

**Estimated size:** about 450 lines (code 60, tests 40, the skill and
its example 200, docs 30, doubled where the estimate is code).

## Implementation steps

1. The `Git` step in `crates/xtask/src/script.rs` (`Step`, `what`,
   `run_step`), with a test in `crates/xtask/src/script/tests.rs` (a
   `Fake` runner: the program is `git`, the folder the game's, the exit
   against `ok`, and the step read from RON).
2. `skills/shuyi-worktrees/SKILL.md` and `example.ron` (**In**).
3. `skills/README.md`, the template's `AGENTS.md` and the book's quick
   start (**In**).
4. `cargo shuyi check-skills skills --tickets tickets/open` and `cargo
   xtask skill-examples shuyi-worktrees`.

## Acceptance criteria

- [x] `shuyi-worktrees` passes `check-skills`, and its worked example
      passes `cargo xtask skill-examples`.
- [x] A new game's `.claude/skills/` has `shuyi-worktrees`.
- [x] A `Git` step that exits otherwise than its `ok` fails its verdict,
      naming the exit code.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `Git` step (step 1).
- Worked example: `example.ron`, run in CI by `skill-examples`.

## Completion notes

- The runner's `Git` step (`crates/xtask/src/script.rs`): `git <args>`
  in the game's folder, checked against `ok` like `Cargo`; in the
  module's doc list and `skills/README.md`. Its test runs a `Fake`
  runner and reads the step from RON.
- `skills/shuyi-worktrees/` (`SKILL.md` and `example.ron`), the skill
  pack's list in `skills/README.md`, the template's `AGENTS.md` (its
  worktrees section and the `seed-target` row) and the book's quick
  start.
- Taken from 3403's first commit, `96ecf2ef`, as the ticket said; it
  matched `main` (the files outside the draft hadn't moved since), and
  the flags and problem codes the skill names were checked against
  `crates/cli/src/worktrees/command.rs`.
- Checked: `cargo shuyi check-skills skills --tickets tickets/open`
  (18 skills OK), `cargo xtask skill-examples shuyi-worktrees` (4 steps
  ok), and the example's new game has `.claude/skills/shuyi-worktrees`.
- `crates/cli/src/template/plans.golden` re-blessed: only the
  template's `AGENTS.md` hashes moved.
- Deviations: none.

No break: the `Git` step is the xtask runner's, not a published crate's.

Book: the quick start's worktrees paragraph names the skill.
