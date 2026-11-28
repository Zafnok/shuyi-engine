---
id: "3449"
title: "pr-size, its commit hook and the gates runner in shuyi-process, set up per repository"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3448"]
nick_input: none
completed:
---

# 3449 — pr-size, its commit hook and the gates runner in `shuyi-process`

## Context

This is the second half of 3448, found by the redundancy sweep of
2026-11-28. 3448 creates the light `shuyi-process` crate and asks Nick
whether games get its tools.

**What the game has.** Its `crates/xtask/src/pr_size.rs` (968 lines, 446
before its tests) is a fork of the engine's `crates/xtask/src/pr_size.rs`
(639). They share `numstat`, `untracked_lines`, `thousands`,
`changed_files`, `repo_of`, `hook_input`, `hook_json`, `hook_answer` and
`runs_a_pr_step`.

**What the game plans.** game#0145 ports the engine's gates runner
(`crates/xtask/src/gates.rs`, 1,720 lines, engine 2054), about 900
lines with its tests, and says so: "this ticket ports it rather than
designing one".

**What stays per repository.** The game's pr-size has rules the engine
lacks: the moved-marker warnings (`changed_lines`, `touches`,
`moved_warnings` and `moved_markers`, about 110 lines). Each repository
keeps its own list of gates, read from its own `ci.yml` and
`changes.yml`.

## Nick input

None. 3448 asks Nick whether games get these tools.

## Scope

**In:**
- **pr-size's counting, its kinds and its commit hook** move into
  `shuyi-process`. The generated and left-out rules, the thresholds and
  the exempt branches become fields of the `Config` from 3448.
- **The gates runner's core** moves too: running a list of steps,
  `--fast`, `--list` with reasons, and the summary. The list itself
  stays in each repository (the engine's `GATES` in
  `crates/xtask/src/gates.rs`).
- **The game's moved-marker rules** become a hook, or a config field if
  they turn out to be generic.
- **If Nick said yes in 3448:** `cargo shuyi pr-size` and `cargo shuyi
  gates` for a game, and the template's hook settings.
- **Estimated size:** about 1,300 lines (code 450, tests 550, docs 300).
  Mostly moved code.
- **Split:** past 1,500 lines, split the gates runner off into a new
  ticket. Keep pr-size here.

**Out (do not do):**
- Changing which gates the engine runs, or any threshold. `cargo xtask
  pr-size` and `cargo xtask gates` give the same output before and
  after.
- The game's adoption: game#1423.

## Implementation steps

1. Move pr-size's code out of `crates/xtask/src/pr_size.rs` with its
   tests. The xtask keeps the command and passes its config.
2. Read the game's `pr_size.rs` (public repository), and port its
   moved-marker cases as a hook with tests.
3. Split `gates.rs` into the runner (into the crate) and the engine's
   list (stays).
4. If Nick said yes, add the `cargo shuyi` commands, the template's
   hook lines and the skill pack's docs.

## Acceptance criteria

- [ ] `cargo xtask pr-size` gives the same counts on the last 20 merged
      PRs as before the move.
- [ ] `cargo xtask gates --fast --list` prints the same steps and
      reasons.
- [ ] A test with the game's config gives the moved-marker warning.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the moved tests, and the hook and config cases.
- Integration: the runner over a two-step fixture list, `--fast` and
  `--list`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
