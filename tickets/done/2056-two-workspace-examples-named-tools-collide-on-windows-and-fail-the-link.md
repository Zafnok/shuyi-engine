---
id: "2056"
title: Two workspace examples named `tools` collide on Windows and fail the link
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-11
---

# 2056 — Two workspace examples named `tools` collide on Windows and fail the link

## Context

CI's `test (windows-latest)` failed with `LNK1104: cannot open file
…\target\debug\examples\tools.exe` on #272, #283 and #307, and passed
after an unrelated push each time. The retro of 2026-11-08 found the
cause in CI's own log:

> output filename collision at …\target\debug\examples\tools.exe: the
> example target `tools` in package `hello-grid` has the same output
> filename as the example target `tools` in package `book-snippets`

`site/snippets/examples/tools.rs` came in with #233 (ticket 2819). On
MSVC, two parallel links of one output file fail. Locally the GNU `ld`
doesn't lock the file, so it never shows. Ticket 2822 (done) fixed the
tutorial's own `tools.exe`, a stale build of one copy of a step run for
another, not this collision.

## Nick input

None.

## Scope

**In:**
- Rename the `book-snippets` example (for instance to `snippet-tools`)
  and update the book pages and tests that run it by name.
- A check that fails when two workspace packages have a bin or example
  of the same name: a test in `crates/xtask` over `cargo metadata`, or
  a `check-layers`-style xtask step.

**Out (do not do):**
- Ticket 2822's tutorial issue.
- Renaming `hello-grid`'s `tools`: games copy that name from the
  templates.

## Implementation steps

1. `grep -rn "example tools\|--example tools\|examples/tools" site
   crates .github` to find every caller of the snippets' example.
2. Rename the file and its `[[example]]` entry, if there is one, in
   `site/snippets/Cargo.toml`, and update the callers.
3. Add the uniqueness check over `cargo metadata --format-version 1
   --no-deps` targets of kind `bin` and `example`, and run it in the CI
   job that runs the other xtask checks.

## Acceptance criteria

- [x] `cargo build --workspace --examples` warns of no output filename
      collision.
- [x] The new check fails when a second `tools` example is added, in a
      unit test over metadata JSON.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the uniqueness check over a metadata fixture.

## Completion notes

- `site/snippets/examples/tools.rs` is now `snippet-tools.rs` (no
  `[[example]]` entry existed, so the file name is the example's name).
  Its one caller by name, the book's tries in `cargo xtask
  tutorial-tries` (`crates/xtask/src/tries.rs`), runs `--example
  snippet-tools`. `cargo build --workspace --examples` now warns of no
  output filename collision.
- New `cargo xtask check-targets` (`crates/xtask/src/targets.rs`): over
  `cargo metadata --no-deps`, it fails when two workspace packages build
  a `bin`, or an `example`, of one name (a bin and an example of one
  name write different folders, so they don't count). Unit tests over a
  metadata fixture, including a second `tools` example, and one over the
  repository. CI runs it in the `layers` job, beside `check-layers`; the
  `run-gates` skill lists it.
- The pages that say `examples/tools.rs` (the quick start, *box-pusher*
  step 1) describe a reader's new game, whose tool program is still
  `tools`; they stay. `site/snippets/project/shuyi.ron` is the reader's
  Moon Garden project file, also still `tools`.

No break.

Book: none, the book's pages describe a new game's `examples/tools.rs`, which is unchanged; only the snippets crate's own copy was renamed.
