---
id: "3548"
title: "The run-gates skill points at cargo xtask gates --list instead of restating each CI job's commands"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3548 — The run-gates skill points at `cargo xtask gates --list`

## Context

The redundancy sweep of 2026-11-28 found that the `run-gates` skill
(`.claude/skills/run-gates/SKILL.md`, 759 lines) restates, in its
per-job sections (about lines 168–724), command blocks that
`crates/xtask/src/gates.rs` already holds. `cargo xtask gates --list`
already prints each gate, and each CI-only gate with its reason
(`gates.rs`, about lines 501 and 806). When a gate's command changes,
the skill's copy is a second place to change, and it goes stale.

Most of those sections hold procedure that `gates.rs` does not:
- how to bless golden tables, and the wasm32 runner;
- the API break lines;
- the templates;
- the game at the next pin;
- what to do when CI fails but local passes.

Only the command blocks that copy a `gates.rs` step are duplicates:
about 150 lines.

## Nick input

None.

## Scope

**In:**
- **Trim the duplicates.** In the sections for the code gates, the
  website and the `tickets` job, replace each command block that copies
  a step of `GATES` with one line pointing at `cargo xtask gates --list`
  and the step's name.
- **Keep every procedure, reason and trigger** ("when tutorial-tries
  runs").
- **Run the sync** (`pwsh -File scripts/sync-agent-skills.ps1`) so the
  loaders stay current.

**Out (do not do):**
- **Changing any gate.**
- **The shipped skill `skills/shuyi-gates/`**, which games read. A game
  has no `cargo xtask gates`.
- **Writing the skill's sections from `gates.rs`.** That is a generated
  block, 3513's job.
- **Either order with 3513:** if 3513 is done, trim only what its
  generated block does not already replace. If not, nothing to do
  (3513 replaces the trimmed blocks with its generated one).

## Implementation steps

1. List the command blocks in the skill. Mark each that equals a
   `GATES` step's command in `gates.rs`.
2. Replace the marked blocks. Keep the prose around them.
3. Run the sync script with `-Check`.

## Acceptance criteria

- [ ] Every command left in the skill is either not a `GATES` step or
      part of a procedure. The completion notes list the blocks kept.
- [ ] `pwsh -File scripts/sync-agent-skills.ps1 -Check` passes.
- [ ] `cargo xtask docs-rules` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the docs and link checks: the change is to a skill's
  text.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
