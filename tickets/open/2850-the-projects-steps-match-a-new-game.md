---
id: "2850"
title: "The book's project starts from the game shuyi new makes today, and a check keeps step 1 equal to it"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2850 — The book's project starts from the game `shuyi new` makes today, and a check keeps step 1 equal to it

## Context

Found by the docs audit of 2026-11-25. The book's project (the box
pusher, `site/tutorial/01-new` … `10-ship`) begins with `shuyi new
box-pusher --template blank --name "Box Pusher" --no-github`
(`site/content/learn/book/box-pusher/01-new.md`, about line 26), and the
docs map's row for the project says it is updated by a change that
"changes how a game starts (`shuyi new`, the template)". Two changes to
the template since then left step 1 behind, and nothing failed:

- **2225** gave every new game four menu sounds: `files/audio.ron`,
  `files/sfx/*.sfx.ron` and the `sfx` feature of `shuyi-assets` in the
  game crate (`templates/shuyi-default/base/crates/game/`). No step of
  the project has them. Chapter 3 (`assets-and-formats.md`, about line
  43) tells the reader a new game "already has four" sounds, so the book
  contradicts its own project. Step 1's file table
  (`01-new.md`, about line 89) still lists "its words, keys, colors and
  font".
- **3425** writes `created_with` (the template, its version and every
  answer) into a new game's `shuyi.ron` (`keep_created_with` in
  `crates/cli/src/template/compose.rs`). `site/tutorial/01-new/shuyi.ron`
  has none.
- Older: step 1's page says `shuyi new` asks only the GitHub question
  (about line 33), while the quick start (`quick-start.md`, about lines
  110–113) says it asks about worked examples first.

The tickets 2225, 2502 and 3425 each checked the quick start and not the
project, so the fix is the check as much as the files.

## Nick input

None.

## Scope

**In:**

- Every step, `01-new` to `10-ship`, gets what a new game has today:
  `files/audio.ron` and the four `files/sfx/*.sfx.ron` as the template
  writes them, the `sfx` feature in its game crate's `Cargo.toml`, and
  `created_with` in its `shuyi.ron`. The steps' `Cargo.lock` files
  follow. **Either order with 2277:** if 2277 is done, steps 08–10
  already have the menu sounds (and the push and win sounds): bring
  steps 01–07 up to them and leave 08–10's audio as 2277 left it; if
  not, give all ten steps the menu sounds, and 2277 adds only its push
  and win sounds.
- Step 1's page: its file table names `audio.ron` and `sfx/`, in one
  line each, linking chapter 3; its words on what `shuyi new` asks match
  the quick start's, and the command it shows is the one a reader runs
  today (with `--no-examples` if the quick start's flow asks the
  examples question).
- **The check**: a test (in `crates/xtask`, run by `cargo xtask gates`
  and CI, beside the tutorial's other checks) that runs the `shuyi new`
  of step 1's page, from the command written on the page, into a
  temporary folder with the engine at this checkout, and compares every
  file with `site/tutorial/01-new`. The differences it allows are a short
  list in the check, each with why (the engine's path, `Cargo.lock`,
  anything the reader is told to edit before step 1 ends). A file the
  template adds, drops or changes fails it, naming the file and saying
  to update step 1 and the steps after it.
- `site/tutorial/tries.toml`'s pictures: remade by `cargo xtask
  tutorial-tries` only if a picture changes (sound isn't drawn).
- `docs/README.md`'s row for the project names the check.

**Out (do not do):**

- the push and win sounds, `shuyi sfx` and chapter 08's part on them
  (2277);
- the wizard's terminal prompts (3423), beyond the words on step 1's
  page;
- checking steps 02–10 against the template file by file: each changes
  the game on purpose. The check covers step 1.

## Implementation steps

1. Read `site/README.md` (*The book's order*, the tutorial's build),
   `site/tutorial/01-new`, the template's base part, and
   `crates/xtask/src/exercises.rs` and `tries.rs` for how the xtask runs
   a step.
2. Bring the ten steps' files up to the template, and build and test
   each (as `site/tutorial`'s README says).
3. Write the check, and see it fail on the old step 1.
4. Step 1's page; the map's row.
5. Run the gates.

## Acceptance criteria

- [ ] Every step has `files/audio.ron`, `files/sfx/` with the template's
      four sounds, the `sfx` feature and `created_with`, and builds and
      passes its tests.
- [ ] The check passes, and fails when a file is added to the template's
      base part (shown by a test or in the completion notes).
- [ ] Step 1's file table and its words on what `shuyi new` asks match
      the template and the quick start.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The check itself, and a unit test of its allowed differences.
- Each step's own tests, unchanged.

## Completion notes

*(Filled in by the session that completes the ticket.)*
