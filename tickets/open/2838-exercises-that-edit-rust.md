---
id: "2838"
title: "Exercises that edit Rust: a command that runs one exercise's check on the step as the reader left it, and the first such exercise"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2837"]
nick_input: none
completed:
---

# 2838 — Exercises that edit Rust

## Context

The book's exercises (2828, [ADR-0117](../../docs/adr/0117-exercises-checks-spliced-into-a-copy-of-each-step-and-run-by-its-web-build.md))
are done in a step's playground, on data files, and checked by the step's
web build when the page asks. An exercise that edits Rust needs the
reader's Rust compiled with the check: 2916 chose a Codespace of the book's
project for that (2837), and a command there that runs the check
([ADR-0126](../../docs/adr/0126-the-code-pad-a-codespace-of-the-book-not-a-build-server.md) §2).

What there is:

- `site/exercises/<step>.rs`: each step's checks,
  `fn(&BasicData) -> Result<String, String>`, listed in `pub const
  CHECKS` with their names, and their tests (each passes on the step's
  files with its exercise's edit, laid over them by `edits.rs`, and fails
  without). `site/exercises/main.rs` is the binary every copy runs
  (`run_game_checked`). Step 3's `wall-right` already plays a command
  through the step's rules.
- `cargo xtask exercises [--copy] [<step>...]`
  (`crates/xtask/src/exercises.rs`): copies each step to
  `target/exercises/<step>/` with the checks spliced in, then Clippy and
  the tests in each copy; it also checks each page's exercises against
  its step's `CHECKS` (`page_problems`).
- The `exercise` shortcode (`site/templates/shortcodes/exercise.html`),
  read by `crates/xtask/src/book.rs` (`read_exercise`: `check`, `file`
  one of the page's `extra.files`, `to`), written in the twins by
  `crates/xtask/src/llms.rs`, run by `site/static/playground.js`'s
  **Check my work**.
- **From 2837 (done):** `.devcontainer/book/devcontainer.json` opens a
  Codespace at `site/tutorial/`, linked from each step's page, which names
  the step's folder and the two commands, `cargo shuyi web` and `bash
  ../serve.sh` (`site/templates/chapter.html`; the twin, `codespace` in
  `crates/xtask/src/llms.rs`). Every step's web build is built into one
  target folder, `CARGO_TARGET_DIR=/home/vscode/book-target`, all ten
  before the Codespace opens (or from a prebuild): a command run there
  builds into it too.
- **From 2839 (done):** the ten steps resolve one set of features of
  every crate they build (`cargo xtask check-devcontainers` fails
  otherwise), so a step after the first builds only its own crate, about
  10 s on 2 cores. A step an exercise copies or changes keeps asking for
  its crates with the engine's features.

## Nick input

None.

## Scope

**In:**
- **The command:** `cargo xtask exercise <step> <check>`, from the
  repository's root, in a Codespace or a clone: makes the step's copy as
  `exercises --copy` does (from the step's folder as it is), builds in it
  a headless program that reads the step's data as its web build does
  (its embedded files, through the step's `game()`'s data) and runs the
  check named from `CHECKS`, and prints `Done.` or `Not yet.` with the
  check's text; exit code 0 only on a pass. A compile error is the
  compiler's own output, then `Not checked: the step doesn't build.`; a
  data problem is the problems, then `Not checked`. A name not in `CHECKS`
  lists the names there.
- **The shortcode:** `code="<path>"` in place of `file`, a path under the
  step's project (`crates/game/src/rules.rs`) that exists; `book.rs`
  checks one of the two is there and that `code` exists. The page shows
  the task, *Do it in `<path>`, in the book's Codespace*, the hint, a
  link to the Codespace, and the command to run, with no **Check my work**
  button. The twin says the same as text.
- **The first exercise:** one, on a step whose rules the reader changes
  (step 3 or 4: say "let the player push two boxes in a row", or "a box
  can't be pushed onto a target from the left"), with its check and its
  tests as ADR-0117 asks (passes with its edit, fails without; the edit
  laid over the step's Rust by `edits.rs`).
- Docs: `site/README.md`, *Exercises*; ADR-0117 is extended by ADR-0126,
  so nothing there changes.

**Out (do not do):**
- running a Rust exercise's check from the page (no build server,
  ADR-0126 §3);
- more than one Rust exercise; the book's later steps can take more in
  their own tickets.

**Estimated size:** about 900 lines (xtask 350, its tests 250, shortcode
and book.rs 150, the exercise and its check 150).

## Implementation steps

1. Read ADR-0117, ADR-0126 §2, `exercises.rs`, `book.rs`'s exercise
   reading, `site/exercises/main.rs` and step 3's checks.
2. The headless program: where it lives in the copy (an example of the
   step's crate, written by the xtask, so the step's own folder gains
   nothing), and how it gets the step's `BasicData` without a window.
3. The command, its output and its exit codes, with tests on a fixture
   step (pass, fail, broken Rust, broken data, unknown name).
4. The shortcode's `code`, `book.rs` and `llms.rs`.
5. The exercise, its check and its tests; run the command in a Codespace
   of the branch made from its step's link (2837), done right and done
   wrong, and note both.

## Acceptance criteria

- [ ] `cargo xtask exercise <step> <check>` prints the check's answer on
      the step as it is, with exit code 0 only on a pass, and the cases of
      step 3 each print what the scope says.
- [ ] An exercise with `code` shows its Codespace and command on its page
      and in its twin, and `check-book` fails one whose `code` doesn't
      exist or that has both `file` and `code`.
- [ ] The first Rust exercise's check passes with its edit and fails
      without, in `cargo xtask exercises`.
- [ ] Run by hand in a Codespace, in the notes.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (xtask): the command's cases on a fixture step; `book.rs` on
  `code`.
- The exercise's check, with and without its edit.
- By hand: step 5.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
