---
id: "2792"
title: "The template stage's map errors name their file once, with its line, as ContentErrors"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-27
---

# 2792 — The template stage's map errors name their file once, with its line

## Context

2269 (done, [ADR-0164](../../docs/adr/0164-the-short-path-reloads-its-own-files-and-restarts-on-a-new-own-part.md))
made a short-path game reload its own part under `cargo shuyi run --watch`,
and show a broken file's errors over the frame (`Ctx::reload_errors`) and
in `cargo shuyi check`. On a game made with `shuyi new --template grid`, a
broken `stage.map` shows:

```text
stage.map: stage.map: missing the "---" line between the header and the tiles
```

The file is named twice, by its bundle path rather than its display path
(`files/stage.map`), and a tile's line is inside the message, not in the
error's own `line`. The cause is in the template's stage part:

- `rules::parse_map`
  (`templates/shuyi-default/stages/top-down-grid/crates/rules/src/lib.rs`)
  takes the `Vec<ContentError>` of `shuyi_assets::grid_map::parse` and
  joins them into one `String`;
- `own` (`templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid.rs`)
  wraps that string in a new `ContentError::new(MAP, e)`, so the first
  file name is `own`'s and the second is the joined error's.

The errors `grid_map::parse` makes are already right: each names its file
(the `file` it was given) and line. Found while checking 2269 by hand on a
new grid game.

## Nick input

None.

## Scope

**In:**
- `rules::parse_map(file, source)` returns
  `Result<GridMap<TerrainId, Feature>, Vec<ContentError>>`, the parser's
  errors as they are (`shuyi_assets::ContentError`: the rules crate
  already depends on `shuyi-assets`).
- `own` hands `parse_map` the map's display path
  (`bundle.display_path(MAP)`), passes its errors on unchanged, and makes
  its own two errors (file not found, `Stage::new` refusing the map) on
  that display path, with no file name in their message.
- The rules crate's own tests (`lib.rs`'s `parse_map("t.map", …)?`) and the
  stage's tools (`tools.rs`) follow the new error type.

**Out (do not do):**
- `Stage::new`'s error type: it stays a `String`, a rule of the stage, not
  a file's; `own` puts it on the file.
- Any engine change: `grid_map::parse` is right.

## Implementation steps

1. Change `parse_map`'s error type and drop its join.
2. In `own`, take `let file = bundle.display_path(MAP);`, pass `&file` to
   `parse_map`, and make the not-found and `Stage::new` errors with
   `ContentError::new(&file, …)`.
3. Fix the callers the compiler names (`tools.rs`, the rules' tests,
   `tests/stage.rs` if it matches on the string).
4. Run the template job locally: `cargo xtask gates` runs
   `templates (grid)`.

## Acceptance criteria

- [x] A grid game whose `files/stage.map` lacks its `---` line fails
      `cargo shuyi check` with one error naming `files/stage.map` once.
- [x] A `stage.map` with an unknown tile on its fourth tile row has an
      error whose `line` is that row's line in the file (not 0, not in its
      message).
- [x] Under `cargo shuyi run --watch` (or the Harness over a disk layer,
      `BasicGame::over` and `Harness::change_file`), the broken map's
      `Ctx::reload_errors` line reads `disk/stage.map:<line>: …`, the file
      once.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (the template's game crate): `own` on a map with no `---` line and
  on one with an unknown tile: the errors' `file` and `line`.
- Integration: the grid template's `tests/stage.rs`, or a new test beside
  it, over a disk layer as above.

## Completion notes

- `rules::parse_map` returns `Vec<ContentError>`, `grid_map::parse`'s
  errors as they are, with their file and line.
- `own` reads the map's display path (`bundle.display_path(MAP)`) and
  hands it to `parse_map`; its missing-file error is
  `ContentError::not_found` (the engine's own message and its
  `MISSING_FILE` code, in place of a bare "file not found"), and
  `Stage::new`'s refusal is a `ContentError` on that path.
- Tests: `own`'s unit tests in `top_down_grid.rs` (no `---` line, an
  unknown tile on the fourth tile row and its line, a missing map, a map
  with no goal) and `tests/reload.rs`, the game over a disk layer with
  `Harness::change_file`, whose reload error starts
  `disk/stage.map:<the row's line>:` and names the file once. By hand on
  a made grid game, `cargo shuyi check` gives
  `files/stage.map: missing the "---" line between the header and the
  tiles` and `files/stage.map:14:7: error[SH2008]: '?' is not in the
  legend`.
- Deviation: the rules' own test helper and the stage's `map_of` test
  helper format the new error with `{:?}`; `tools.rs` needed no change
  (its `map` reads `data()`, not `parse_map`).

Behaviour: the grid template (a new game's `rules`, `game`): `parse_map`
returns `Vec<ContentError>`; a broken `stage.map`'s errors name its
display path (`files/stage.map`) once and carry their line; a missing
map's message is the engine's "file not found in asset bundle", coded
`MISSING_FILE`.

Book: none, no chapter shows the template's `parse_map` or a map error's
text.

- Skills: `shuyi-stage-grid`'s worked example expects `check`'s error on
  `files/stage.map` at line 12 (it read `stage.map` before), and its
  SKILL.md says `check` gives a map's errors with their code and line;
  `shuyi-test`'s example formats `parse_map`'s errors with `{:?}`.
- Deviations: `tools/imports.ron` allows `shuyi_assets::ContentError` in
  the rules crate beside its `grid_map` allow (the rules crate has no
  `shuyi` prelude), and `crates/cli/src/template/plans.golden` is
  blessed for the four template files.

No break: the engine's public API is unchanged; the change is in the
template's code, which a game owns once made.

Game: nothing to adopt, the game is not made from the grid template.
