---
id: "2773"
title: "The grid template's map errors keep their line, column and code in check"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2773 — The grid template's map errors keep their line, column and code

## Context

Found by the skill pack's authors (1134), writing `shuyi-stage-grid`'s
worked example. On a game made with `shuyi new --template grid`, a
`.map` with a character its legend lacks gives two different answers:

- `cargo shuyi import crates/game/files/stage.map` answers each error on
  its own, with its line, column and `SH2008` (`grid_map::parse`,
  `crates/assets/src/grid_map.rs`, through the stage's map format);
- `cargo shuyi check` answers one error on `stage.map`, with no line, no
  column and no code, all the parser's messages joined into its text.

The template loses them in two places:

- `rules::parse_map`
  (`templates/shuyi-default/stages/top-down-grid/crates/rules/src/lib.rs`)
  returns `Result<GridMap<TerrainId, Feature>, String>`: it joins
  `grid_map::parse`'s `Vec<ContentError>` into one `String`;
- the stage's `own`
  (`templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid.rs`)
  wraps that `String`, and `Stage::new`'s, in one
  `ContentError::new(MAP, e)`. It names the file `stage.map`, where the
  registry names the game's other files `files/…`
  (`Bundle::display_path`).

So `skills/shuyi-stage-grid/SKILL.md` (*When it fails*) sends agents to
`import` to find a map's errors, and its worked example expects
`check`'s one error on `stage.map`.

## Nick input

None.

## Scope

**In:**
- **`rules::parse_map(file, source) -> Result<GridMap<TerrainId,
  Feature>, Vec<ContentError>>`**: `grid_map::parse`'s errors as they
  are, each with its line, column and code. The rules crate already
  depends on `shuyi-assets`, which holds `ContentError`, so its fence
  holds.
- **The stage's `own`** names the file as the registry does
  (`bundle.display_path(MAP)`). It returns `parse_map`'s errors as they
  are, gives a missing file `codes::MISSING_FILE` (`SH2003`), and gives
  `Stage::new`'s problems (goals, pieces, a piece off the floor)
  `codes::OUT_OF_RANGE` (`SH2010`).
- **The template's tests:** the rules crate's `stage` helper and its
  tests (`crates/rules/src/lib.rs`), and the game crate's
  (`crates/game/tests/stage.rs`), take the new error type. A test reads
  a broken map through `check` (the contract's `check` with the stage's
  `Tools`) and finds the error's line, column and `SH2008`.
- **The plans' golden file** (`crates/cli/src/template/plans.golden`),
  written again with `SHUYI_BLESS=1 cargo test -p shuyi-cli --lib
  template`.
- **The skill pack:**
  - `skills/shuyi-stage-grid/SKILL.md`, *When it fails*: `check` names a
    map's errors with their line, column and code, as `import` does.
    Its worked example (`example.ron`) expects `check`'s error on
    `files/stage.map` with `SH2008`, its line and its column, and
    `SH2010` for the second goal;
  - every worked example that calls `parse_map` and reads its error as
    text: `skills/shuyi-test/example/rules.rs` (`panic!("{e}")` becomes
    `{e:?}`, since `Vec<ContentError>` has no `Display`) and
    `skills/shuyi-debug-replay/example/record.rs`;
  - `cargo xtask skill-examples shuyi-stage-grid shuyi-test
    shuyi-debug-replay` passes, and `check-skills` passes.
- The grid template's `AGENTS.md` files need no change. Check that none
  says `check` gives a map's errors without a line.

**Out (do not do):**
- Hello, Grid's own `parse_map` (`examples/hello-grid/src/rules.rs`): an
  example of its own, with its own tests;
- the hex grid template part (2771, parked), which copies the grid
  part's `parse_map` and follows this ticket by its own clause;
- new codes: `SH2003`, `SH2008` and `SH2010` exist;
- lines for `Stage::new`'s problems: they are about the whole map (two
  goals), not one tile.

## Implementation steps

1. Change `parse_map` and its tests in the template's rules crate.
2. Change the stage's `own`, and the game crate's tests.
3. Make a game with `cargo run -p shuyi-cli -- new <dir> --template grid
   --engine-path .`, break its map, and look at `cargo shuyi check
   --json`'s answer.
4. Write the golden file again, update the skills and their worked
   examples, and run them.
5. Run the gates, the `templates` job's steps among them (the
   `run-gates` skill, *A new game from each template*).

## Acceptance criteria

- [ ] On a new `grid` game, `cargo shuyi check` answers a map's unknown
      character on `files/stage.map` with its line, column and
      `SH2008`, as `import` does.
- [ ] A map with two goals fails `check` with `SH2010`.
- [ ] The template's tests and CI's `templates` job pass.
- [ ] `shuyi-stage-grid`'s skill and worked example say so, and the three
      worked examples pass.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (the template's rules crate): `parse_map` of a bad map gives each
  error with its line, column and code.
- Integration (the template's game crate): `check` over the stage's
  `Tools` on a bad map finds the line and `SH2008`.
- Integration: CI's `templates` and `skills` jobs.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
