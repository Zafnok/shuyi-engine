---
id: "2558"
title: "BasicTools takes a short-path game's own formats, so import reads its own kinds"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2558 — `BasicTools` takes a short-path game's own formats

## Context

Found by the skill pack's authors (1134), writing `shuyi-cinematic`'s
worked example. A game on the short path (ADR-0036) reads a cinematic of
its own shot kinds in its stage's `own` function (`CinematicRon::<Shot>`
registered in a `Formats` of its own, then `cinematic::check`), so
`cargo shuyi check` sees its problems. But `cargo shuyi import
crates/game/files/intro.ron` can't read it:

- a blank game's tools are the engine's `BasicTools`
  (`crates/cli/src/basic.rs`, made by the generated `stages::tools()`),
  which don't override `GameTools::importers` (`crates/cli/src/game.rs`),
  so `import` tries `Importers::standard()`'s kinds alone;
- a kind of the game's own (`CinematicFile<Shot>`, or any file of a kind
  only the game knows) has no format there, and `import` answers that no
  format reads it;
- the grid template gives its stage's map to `import` only because its
  stage writes a `GameTools` of its own (`Tools::importers`,
  `templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid/tools.rs`).
  ADR-0036 says a game grows by adding a part, not by writing all of
  `GameTools` for one method.

`import` is where an agent reads one file and sees its errors with file,
line and code (`skills/shuyi-import-art`, `skills/shuyi-stage-grid`), so
a kind of the game's own should be readable there too.

## Nick input

None.

## Scope

**In:**
- **`BasicTools::importers(self, add: fn(Importers) -> Importers) ->
  Self`** (`crates/cli/src/basic.rs`), a builder like `BasicGame`'s:
  the game's own formats and kinds added to `import`'s, on top of
  `Importers::standard()`. `BasicTools` keeps the function in a private
  field (its fields are private already, so this is no break), and its
  `GameTools::importers` returns `add(Importers::standard())`, or
  `Importers::standard()` when none was given.
- A game adds its kind as the grid template's `Tools::importers` does:
  `importers.formats.register(CinematicRon::<Shot>::new())`, then
  `importers.kind::<CinematicFile<Shot>>(|c| format!("{} shot(s)",
  c.shots.len()))`.
- `check` is unchanged: a short-path game's own part is checked by its
  `own` function, as today (a test keeps that true).
- **Docs:**
  - the crate docs of `basic.rs` and `docs/contract.md`'s *What a game
    gives* (the short path's paragraph) say how a short-path game adds
    its own kinds to `import`;
  - `skills/shuyi-cinematic/SKILL.md` gets a step after "Read the file
    and check it": give `import` the game's kind with
    `BasicTools::new(crate::game()).importers(…)` in `stages::tools()`,
    so `cargo shuyi import crates/game/files/intro.ron` reads the file
    and names its errors. Its worked example (`example.ron`,
    `example/stages.rs`) adds the call and an `import` step expecting
    the game's kind in `/result/read_as`. `check-skills` passes, and
    the example passes in CI's `skills` job.
- **Either order with 2517:** if 2517 is done, `BasicTools::importers()`
  already registers `KeymapRon::<A>` and the `KeymapDef<A>` kind; the
  game's `add` runs on top of those. If not, nothing to do (2517 adds
  the keymap kind beneath the game's, by its own clause).

**Out (do not do):**
- giving `BasicGame` (`shuyi-platform`) a formats hook: `Importers` is a
  type of `shuyi-cli`, and the game's `own` already reads its own files;
- reading a game's own kinds in `check --baked` or `bake` (a game that
  bakes writes its own `GameTools::bake`);
- `shuyi kinds` (2515), which lists the kinds `import` knows.

## Implementation steps

1. Read `crates/cli/src/basic.rs`, `Importers` in
   `crates/cli/src/contract/import.rs`, the grid stage's `Tools`, and
   2517 if it is done.
2. Add the builder and the `importers` method, with the tests below.
3. Update the docs, the cinematic skill and its worked example.
4. Run `cargo xtask skill-examples shuyi-cinematic`, and the gates.

## Acceptance criteria

- [ ] A short-path game that adds its cinematic kind with
      `BasicTools::importers` reads its file with `cargo shuyi import`,
      and a broken one fails with its errors (the cinematic skill's
      worked example).
- [ ] Without the builder, `import` answers as today (a test).
- [ ] `check` still reports the game's own part's problems (a test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/cli/src/basic/tests.rs`): `importers()` with no `add`
  lists `Importers::standard()`'s kinds; with an `add` that registers a
  test kind, it lists that kind too, and reads a file of it.
- Integration: `shuyi-cinematic`'s worked example in CI's `skills` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
