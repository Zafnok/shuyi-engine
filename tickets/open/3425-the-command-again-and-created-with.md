---
id: "3425"
title: "The command that makes a game again, and created_with: shuyi new prints its one command and keeps the answers in shuyi.ron"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["3424"]
nick_input: none
completed:
---

# 3425 — The command that makes a game again, and created_with

## Context

Split from 3424 at its size valve (1,381 counted lines with this part
in). [ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§2: at the end the wizard prints the one command that makes the same
game with no questions, and writes the answers into the new game's
`shuyi.ron` under `created_with`.

**From 3424 (done):** `wizard::questions` lists every question, and
`shuyi new <dir> --questions [--json]` answers them
(`new.questions.json`). The code of this ticket was written and tested
with 3424, then taken out at the split: it is on the local branch
`t3425-the-command-again-and-created-with` (commit `6f244ae9`, "WIP:
3424 with the command again and created_with"), which holds all of 3424
too. Take from it `wizard::quote`, `wizard::command` and their tests,
`NewOutput::command`, `compose::created_with`, `project::CreatedWith`
and the tests named below, rather than writing them again; if the
branch is gone, write them as below.

## Nick input

None.

## Scope

**In:**
- **The command again.** `NewOutput::command`: `shuyi new <dir>
  --no-input --name <name>` with `--id` if given, each concern's answers
  in the catalog's order by its flag (an empty list as `--set <id>=`),
  `--targets`, `--examples` or `--no-examples`, `--set` for the
  template's other questions, then the engine (`--engine-rev`,
  `--engine-path`) and `--private-url`.
  - `wizard::command(&NewInput, name, &Answers, &Concerns, &Template)
    -> (String, bool)`: the line, and whether every word could be quoted
    alike.
  - `wizard::quote(word) -> Option<String>`: as it is when it holds only
    letters, digits and `_ . / : = , + -` (PowerShell passes `a,b` to a
    program as it is, checked), in single quotes otherwise, in double
    quotes when it holds a `'`; `None` for a `'` with one of `` $ ` ! "
    \ ``, which the command then quotes for a POSIX shell, with a limit
    saying so.
  - `new`'s text ends with it ("the same game again, with no
    question:").
  - `NewOutput` becomes `#[non_exhaustive]` as it gains the field
    (ADR-0039, kind 3): one `Breaking:` line for both.
- **`created_with`** in `shuyi.ron`: the composer merges `created_with:
  (template, version, answers)`, every answer with the defaults, into
  the game's `shuyi.ron` through `ron_merge` (`compose::created_with`,
  with a comment saying what it is), and `project::Config` gains
  `created_with: Option<CreatedWith>` (it denies unknown fields).
- Regenerate the schemas (`new.output.json` gains `command`) and raise
  `contract::VERSION` over `main`'s.
- **Docs:** `docs/templates.md`'s *The wizard's questions* gains the
  printed command and `created_with`, with an example; the book's quick
  start names the command; `docs/contract.md`'s `new` row; the
  `shuyi-new-game` skill says the answer's `command` makes the game
  again.

**Out (do not do):**
- the terminal's prompts (3423);
- the manifest's `targets` following the targets picked (2504).

**Estimated size:** about 500 lines (code 170, tests 300, docs 30).

## Implementation steps

1. Bring `wizard::quote`, `wizard::command` and their tests over from
   the branch, and `NewOutput::command` with `plan_with` (the plan with
   its catalog) in `new.rs`; print it.
2. Bring `compose::created_with`, its merge in `compose` and
   `project::CreatedWith`; fix `template::tests`' merge and examples
   tests; bless `plans.golden`.
3. Regenerate the schemas, raise the version, write the docs.

## Acceptance criteria

- [ ] Running the printed command with `--no-input` into another folder
      gives the same bytes, for a grid game and for a blank one with
      `--id`, `--no-examples`, `--targets` and a name with a `'`.
- [ ] A new game's `shuyi.ron` holds `created_with` with every answer,
      and loads.
- [ ] A word with a space, a `$`, an `@` or a `~` is quoted, and a
      comma list isn't.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `wizard::quote` and `wizard::command`'s flags, order and
  quoting; `created_with` merged into a fixture's `shuyi.ron`; a game
  without examples differs from one with them only in its skills and
  that answer.
- Integration: the printed command reproduces the game (split by a
  small POSIX word splitter, its folder changed); `Project::load` reads
  `created_with`.

## Completion notes
