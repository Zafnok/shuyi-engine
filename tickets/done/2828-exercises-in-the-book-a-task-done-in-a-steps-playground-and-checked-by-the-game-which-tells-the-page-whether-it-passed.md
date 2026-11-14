---
id: "2828"
title: "Exercises in the book: a task done in a step's playground and checked by the game, which tells the page whether it passed and why"
type: feature
milestone: "Engine: public"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2820", "2827"]
nick_input: none
completed: 2026-11-13
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
- **From 2820 (done):** each project step's page has a playground of
  that step's data files beside its game, folded in a `<details>` under
  *Play this step* (`site/templates/chapter.html`, the playground itself
  in `playground-macros.html`), its files listed in the step's
  `extra.files`. The steps' games get the page's files with no code of
  their own: `shuyi_app::run_game` lays them (`lay_page`) and reports, so
  the step's `page_check` goes beside that, in `run_game`, or each step's
  `main.rs` stops calling it. `playground.js` keeps each page's files
  under keys of its own (`shuyi-playground-run:<page's path>:files/…`)
  and hands them to `shuyi-page-file:` as the page loads, before its
  frame, and empties them as the reader leaves: `shuyi-page-check` can
  follow the same pattern. A try's **Try this here** button
  (`try_it`, `tries()` in the script) makes edits in the editors and
  runs: an exercise's button can reuse it.
- **From 2827 (done):** a quick check is the body shortcode
  `site/templates/shortcodes/check.html`, answered by
  `site/static/book-check.js`, and written as text in the twins by
  `llms.rs`'s `checked` (`site/README.md`, *Quick checks*).
  - Every step and concept chapter ends with `## Check yourself`, its
    last `## ` section, holding two or three checks. `cargo xtask
    check-book` (`book.rs`'s `check_section`) fails a heading after it, so
    an exercise goes in that section after the checks, with no heading
    of its own.
  - `book.rs`'s `kind()` knows two body openers, `newcomer` and `check`,
    and takes every `{% end %}` as the close of one of them. An
    `exercise` body shortcode adds its opener there (an `Opened` variant),
    or its `{% end %}` fails as one with no mark open.
  - A check with `to` is an `aside.check.newcomer` with a
    `.newcomer-label`, so 2826's filter folds it and the bar counts it as
    a check ("6 notes and 1 check hidden", `counted` in
    `book-filter.js`). An exercise with `to` can do the same, and adds
    its own word there.
- **From 2823 (done):** [ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md) §8 says only where an exercise sits:
  in the step's `## Check yourself` section, after its quick checks. On
  2026-11-05 the exercises' design moved here (`docs/decisions.md`, *the
  first release waits for everything before going public*), to be made
  once the playground they are done in exists. An exercise takes an
  audience as a quick check does: ADR-0090 §5's `to`, optional.

The playground edits data files, not Rust (decision of 2026-10-28), so an
exercise's task is a change to a data file.

## Nick input

None.

## Scope

**In:**
- **The design, in an ADR** (the `write-adr` skill), first: where a
  check lives (outside the code a step's page asks the reader to type,
  so the reader's project stays as the page says), how the game reports
  it to the page, and how the exercise's shortcode sits with ADR-0090's
  `newcomer` marks and 2827's quick check. Where it changes what the
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

1. Read ADR-0090, 3069's, 2820's and 2827's completion notes,
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
   **From 2832 (done):** each editor paints its text in a `<pre>` under
   it (a `.ron` file colored); code that sets an editor's text calls
   `set(area, text)` in `playground.js`, never `area.value =` alone, or
   the painted copy goes stale.
6. Try three exercises in the built-in browser: done right, done wrong,
   and with a broken file.
7. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] The exercises' ADR is in `docs/adr/`.
- [x] Every project step with data files has one or two exercises.
- [x] *Check my work* on a correct edit says it passed; on a wrong one it
      says why not; on a broken file it shows the file's problems
      (3069) and no check result.
- [x] A message that isn't from the page's own frame and origin is
      ignored, as 2814's are.
- [x] Each check has a test that passes on the step's own files with the
      exercise's edit and fails without it.
- [x] On native, `page_check` is `None` and `report_check` does nothing.
- [x] The plugin's version test covers the new JS.
- [x] The book's chapter is updated (2802): this ticket is that change.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/page.rs`): `page_check` and `report_check` on
  native; `CheckReport`'s constructor.
- Unit, per step: each check, with and without its edit.
- `shuyi-cli`: the plugin version test.
- By hand: step 6, in the notes.

## Completion notes

**What was done**

- **The design:** [ADR-0115](../../docs/adr/0115-exercises-checks-spliced-into-a-copy-of-each-step-and-run-by-its-web-build.md),
  extending ADR-0090 §8. The checks live in `site/exercises/`, never in a
  step's project: one file a step (`<step>.rs`), the binary every step runs
  with them (`main.rs`) and shared helpers (`solve.rs`, a breadth-first
  search through the step's rules; `color.rs`; `edits.rs`, for the tests).
  `cargo xtask exercises` copies each step to `target/exercises/<step>/`
  with them compiled in (the step's manifest and lock unchanged), checks
  each page's exercises against its step's `CHECKS`, and runs `cargo fmt
  --check`, Clippy and the tests in each copy; `--copy` only makes the
  copies, which `pages.yml` now builds each step's `play/` from and CI's
  `web-render` job boots. CI has a new `exercises` job, and `cargo xtask
  gates` an `exercises` gate.
- **The engine:** `shuyi_app::page::{page_check, CheckReport,
  report_check}`, and `shuyi_app::run_game_checked(game, checks)` with
  the type `PageCheck`; `run_game` is `run_game_checked` with no checks.
  The JS is two more functions of `page-files.js` (`shuyi_page_check`,
  `shuyi_page_report_check`), its version and `PLUGIN_VERSION` moved to 2,
  and `shuyi-cli`'s test lists them.
- **The page:** the `exercise` body shortcode
  (`site/templates/shortcodes/exercise.html`), its styles, and in
  `playground.js` its two buttons, **Open `files/<file>`** and **Check my
  work**, which writes `shuyi-page-check` and runs; the answer is taken only
  from the playground's frame at the page's origin. `book.rs` reads it
  (`Opened::Exercise`, `read_exercise`, where it sits, one or two on a step
  with a playground), `llms.rs` writes it in the twins, the reader's filter
  counts it as an exercise, and `check-build.sh` checks a page with one
  loads `playground.js`.
- **The exercises:** 13 over the ten steps, each with its check and tests
  (the check passes on the step's files with the exercise's edit and fails
  without): step 1 `confirm-key`, `quit-word`; 2 `second-box`,
  `blue-walls`; 3 `wall-right` (plays a step through the rules); 4
  `eight-moves` (searches every way of playing); 5 `undo-two-keys`; 6
  `three-boxes` (searched); 7 `yard-first`; 8 `yellow-player`; 9
  `cant-win` (searched); 10 `made-by`.

**Deviations**

- A check is `fn(&BasicData) -> Result<String, String>` (why it passes, or
  why not), named in its step's `CHECKS`, and the engine builds the
  `CheckReport` with the name: a check doesn't repeat its own name. The
  ticket's `page_check` goes in `run_game_checked`, a new public function,
  because a step's `main.rs` stays as the page says and the checks can't be
  in `run_game` (ADR-0115 §3).
- The site's `play/` of each step is now built from its copy (ADR-0115
  §2), in `pages.yml` and in CI's boot check of the steps.
- Each exercise also has an **Open `files/<file>`** button: the exercises
  sit at the page's end and the playground at its top.
- Formatting of `site/exercises/` is checked by `cargo fmt --check` in
  each copy, not `rustfmt` on the folder, which can't follow the files'
  `#[path]` modules there.

**By hand (step 6)**, in the built-in browser, on a local `zola build` of
the site with step 2's copy built by `cargo shuyi web` as its `play/`, at
this branch's tip on `main` at d34b6245 (no newer `main`):

- *Check my work* with the files unchanged: "Not yet. Level 1 still has
  one box: add a second to its `features`, and an `o` for it to go on."
- With a second box and a target: "Done. Level 1 has two boxes and a
  target for each.", Problems "Loaded with your files.".
- With a box and no target for it: Problems "files/levels/1.map: 2 boxes
  and 1 targets", and the exercise "Not checked. Your files have problems,
  …" with no check result.
- The walls exercise, by real clicks: **Open** unfolded the playground on
  `palette.ron` with its editor focused; blue walls gave "Done. Both of
  the walls' colors are blue."
- A `shuyi-check` and a `shuyi-report` posted by the page itself, not its
  frame, were ignored: the line showed the game's own answer.
- The key `shuyi-page-check` was empty after each answer.

No ticket: nothing follows; 2916 (Rust exercises) and 2830 (a second
project) now say what this leaves them.

Behaviour: shuyi-app: on the web, a game run with `run_game` whose page
writes `sessionStorage`'s `shuyi-page-check` sends the page a
`shuyi-check` message saying it has no check of that name; a page that
writes none sees nothing new.

No break: additions only (`run_game_checked`, `PageCheck`,
`page::{page_check, CheckReport, report_check}`). The game runs on the long
form, not `run_game`, and uses nothing of the page plugin (`gh search code
--repo Zafnok/visions-of-shuyi 'run_game'`, `'page_layer'`,
`'shuyi-report'`): nothing in the game changes.
