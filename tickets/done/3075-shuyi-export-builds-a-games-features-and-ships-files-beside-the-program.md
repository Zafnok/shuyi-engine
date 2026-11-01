---
id: "3075"
title: "shuyi export builds a game's named features and ships the files it names beside the program"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["3004"]
nick_input: none
completed: 2026-11-01
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

- [x] A fixture game with a feature and a folder exports each target with the
      feature on and the folder beside the program.
- [x] A missing `beside` path refuses the export, naming it.
- [x] `link` in `features` is refused.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the part's parsing and errors; each exporter's cargo arguments.
- Integration: the fixture's packages, by `tools/export-check/check.sh`.

## Completion notes

`shuyi.ron` has an `export` part: `features` and `beside` (`path`, optional
`name`, `optional`). Windows, Linux, macOS and web build with the features and
ship what `beside` names; `export-report.json` lists it under `beside`.
Hello, Grid has the part, with a feature that does nothing, so CI's `export`
job and `tools/export-check` check it on all four. Windows was run here by
hand; Linux and macOS are checked by that job only.

Deviations and additions, each because the game's release needs it:

- `export --without <feature>`: a dry run or a fork has no bought art, and its
  build must still run with the placeholders.
- `name` in a `beside` row: the game's `audio-licenses/` is
  `assets/audio/licenses` in its repository.
- macOS: a folder goes in `Contents/Resources`, and `shuyi-app`'s
  `folder_beside` looks there too, so `music/` and `voice/` play from a `.app`
  and the signature covers them. A file goes beside the `.app`.
- The Playdate's exporter takes none of the part: a `.pdx` has its own build
  and files. So "each exporter" in the steps means the other four.

Game PR that moves the pin: game#0910 (the ticket's own PR).

Book: updated (`targets-and-export`, the Export section).

No break of what a game writes: `shuyi.ron`'s new part is optional. The
contract's version is 34 (`export`'s `without` and `beside`).

Breaking: shuyi-cli: export::ExportCx: two new public fields, `turned_on` and `beside`; a literal of it adds them.

```migration
since: 0.2.0
what: ExportCx has two new fields; ExportOutput has one.
old: ExportCx { .. }
new: ExportCx { turned_on, beside, .. }
before: |
  ExportCx { features, .. }
after: |
  ExportCx { features, turned_on: Vec::new(), beside: Vec::new(), .. }
find: ["ExportCx { features, .. }"]
```
