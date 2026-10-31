---
id: "3075"
title: "shuyi export builds a game's named features and ships the files it names beside the program"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3004"]
nick_input: none
completed:
---

# 3075 — shuyi export: a game's features, and files beside the program

## Context

1. **The game ticket:** `game#0910`
   ([github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0910-release-packages-with-shuyi-export.md](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0910-release-packages-with-shuyi-export.md)),
   found by the game's review of its pin move of 2026-10-30.
2. **The job:** package a game whose release build needs a feature on, and
   whose music and voice clips ship in folders beside the program.
3. **Friction numbers:** no friction review: a missing feature. The game's
   `release.yml` does each package by hand in about 200 lines of shell.
4. **The generic shape:** `shuyi.ron` names, for `export`, the package's
   features and files or folders copied beside the program. Today an export
   "turns on no feature of the game's package", though the web part of
   `shuyi.ron` already has `features`, and a package holds only the program
   and its notes.
5. **An example from a game that isn't tactics:** a platformer with its
   licensed art in a private repository behind a feature, and its soundtrack
   as loose files a player can replace.

## Nick input

None.

## Scope

**In:**
- An `export` part in `shuyi.ron`: `features` and `beside` (paths from the
  game's folder, copied into the package under their own names). A path that
  is missing is an error naming it, except one marked optional.
- The rule stays that `link` is refused, whatever `features` says.
- `export-report.json` lists what was copied, with its size.
- `.app`: files go where a macOS game can read them, stated in
  `docs/targets/desktop.md`.

**Out (do not do):**
- Reading the game's folders for anything else; any change to `check --target`.

No API break expected (`Config` is `#[non_exhaustive]`); the game's PR that
moves the pin follows this one.

## Implementation steps

1. The part in `project::Config`, with its parse errors.
2. Cargo arguments and the copy, in each exporter.
3. Docs: `docs/targets/desktop.md`, the `shuyi-ship` skill.

## Acceptance criteria

- [ ] A fixture game with a feature and a folder exports each target with the
      feature on and the folder beside the program.
- [ ] A missing `beside` path refuses the export, naming it.
- [ ] `link` in `features` is refused.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the part's parsing and errors; each exporter's cargo arguments.
- Integration: the fixture's packages, by `tools/export-check/check.sh`.
