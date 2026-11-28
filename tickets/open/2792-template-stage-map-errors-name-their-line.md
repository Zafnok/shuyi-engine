---
id: "2792"
title: "The template stage's map errors name their file once, with its line, as ContentErrors"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: ["2773"]
nick_input: none
completed:
---

# 2792 — The template stage's map errors name their file once, with its line

**Done by 2773.** The redundancy sweep of 2026-11-28 found that 2773
plans the same change: `parse_map` returns `Vec<ContentError>`, and
`own` uses `bundle.display_path(MAP)`. 2773 is the larger ticket (it
adds the codes, the golden file and the skills), and its acceptance
criteria now hold this ticket's. 2773's PR moves this ticket to
`done/`. Don't work it on its own.

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

- [ ] A grid game whose `files/stage.map` lacks its `---` line fails
      `cargo shuyi check` with one error naming `files/stage.map` once.
- [ ] A `stage.map` with an unknown tile on its fourth tile row has an
      error whose `line` is that row's line in the file (not 0, not in its
      message).
- [ ] Under `cargo shuyi run --watch` (or the Harness over a disk layer,
      `BasicGame::over` and `Harness::change_file`), the broken map's
      `Ctx::reload_errors` line reads `disk/stage.map:<line>: …`, the file
      once.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (the template's game crate): `own` on a map with no `---` line and
  on one with an unknown tile: the errors' `file` and `line`.
- Integration: the grid template's `tests/stage.rs`, or a new test beside
  it, over a disk layer as above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
