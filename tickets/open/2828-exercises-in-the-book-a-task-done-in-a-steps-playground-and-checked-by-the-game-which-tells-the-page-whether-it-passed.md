---
id: "2828"
title: "Exercises in the book: a task done in a step's playground and checked by the game, which tells the page whether it passed and why"
type: feature
milestone: "Engine: public"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2820", "2827"]
nick_input: none
completed:
---

# 2828 — Exercises in the book

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md), *one
book, read at the reader's level*): the book is "a unified learning
resource and highly interactive". 2827 asks a reader whether they
followed. An exercise asks them to do something: "add a second box and
its target to level 1", "make the walls blue", "make a level that can't
be won", done in the step's playground (2820), and the game says whether
it is done. Exercism's tasks and Rustlings' exercises work this way, each
checked by a test.

What there is once the blockers are done:

- 3069: a web build lays the files its page holds (`sessionStorage`,
  `shuyi-page-file:<path>`) over its own through
  `shuyi_app::page::page_layer`, and `page::report(text)` sends its
  problems to the parent frame as `{ kind: "shuyi-report", text }`.
- 2820: each project step's page has a playground of that step's data
  files beside its game, reusing 2814's `playground.js`.
- 2827: a quick check's shortcode, its script and its expansion in the
  twins.
- 2823's ADR names where an exercise sits in a page (beside the step's
  quick check), and nothing more: on 2026-11-05 the exercises' design
  moved here from 2823 (`docs/decisions.md`, *the first release waits
  for everything before going public*), to be made once the playground
  they are done in exists.

The playground edits data files, not Rust (decision of 2026-10-28), so an
exercise's task is a change to a data file.

## Nick input

None.

## Scope

**In:**
- **The design, in an ADR** (the `write-adr` skill), first: where a
  check lives (outside the code a step's page asks the reader to type,
  so the reader's project stays as the page says), how the game reports
  it to the page, and how the exercise's shortcode sits with 2823's
  audience marks and 2827's quick check. Where it changes what the
  points below say, it wins, and the completion notes say so.
- **The page asks for a check:** `playground.js` writes the exercise's
  name to `sessionStorage` under `shuyi-page-check` before it runs the
  game, and clears it after.
- **The engine:** in `shuyi_app::page`, `page_check() -> Option<String>`
  (the check the page asks for; `None` on native or when there is none)
  and `report_check(result: &CheckReport)`, which on `wasm32` sends
  `{ kind: "shuyi-check", name, passed, text }` to the parent frame, and
  does nothing elsewhere. `CheckReport` is `#[non_exhaustive]` with a
  constructor (ADR-0039). The JS side goes in 3069's `page-files.js`, with
  its version moved on and the version test kept equal.
- **The checks:** functions of plain Rust, each taking what the step's
  game loaded (its levels, its palette, its text) and returning a
  `CheckReport`, with a reason a reader can act on ("level 1 has two boxes
  but one target"). A check that needs play runs a list of actions through
  the step's rules, headless. They live where the ADR says, and the
  step's web build runs the one `page_check` names after its files load,
  then plays as usual.
- **The exercise**, a shortcode beside 2827's check: the task, a hint
  shown on request, the playground file it is done in, the check's name,
  and an audience. **Check my work** runs the playground with the check;
  the result shows under the task, passed or not and why.
- **Exercises written** for the project's steps that have data files:
  one or two each, each with its check and a test.
- **The twin:** the task and the hint as text, and that it is checked in
  the page's playground.

**Out (do not do):**
- exercises that edit Rust: those wait for the code pad (2916);
- exercises on the concept chapters (Moon Garden has no playground);
- a score, progress kept past the tab, or an account;
- changing what a step's page asks the reader to type.

## Implementation steps

1. Read 2823's ADR, 3069's, 2820's and 2827's completion notes,
   `crates/app/src/page.rs`, `crates/cli/web/page-files.js` and its
   version test, `site/static/playground.js`, and the steps' web starts.
2. Write the ADR, trying the check's path from the playground to the
   game and back on one step first.
3. Write `page_check`, `CheckReport` and `report_check`, and the JS, with
   their tests.
4. Write each step's checks with their tests, and run them from the
   step's web start.
5. Write the exercise shortcode, its script in `playground.js`, its
   styles and its twin, then the exercises.
6. Try three exercises in the built-in browser: done right, done wrong,
   and with a broken file.
7. Run the `document` skill, then the gates.

## Acceptance criteria

- [ ] The exercises' ADR is in `docs/adr/`.
- [ ] Every project step with data files has one or two exercises.
- [ ] *Check my work* on a correct edit says it passed; on a wrong one it
      says why not; on a broken file it shows the file's problems
      (3069) and no check result.
- [ ] A message that isn't from the page's own frame and origin is
      ignored, as 2814's are.
- [ ] Each check has a test that passes on the step's own files with the
      exercise's edit and fails without it.
- [ ] On native, `page_check` is `None` and `report_check` does nothing.
- [ ] The plugin's version test covers the new JS.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/page.rs`): `page_check` and `report_check` on
  native; `CheckReport`'s constructor.
- Unit, per step: each check, with and without its edit.
- `shuyi-cli`: the plugin version test.
- By hand: step 6, in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
