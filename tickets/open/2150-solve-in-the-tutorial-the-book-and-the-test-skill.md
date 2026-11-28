---
id: "2150"
title: "The level solver in the tutorial, the book and the test skill: level lines from solve, not files"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2112"]
nick_input: none
completed:
---

# 2150 — The level solver in the tutorial, the book and the test skill

## Context

2112 added the engine's level solver, `shuyi_core::search`
(`crates/core/src/search.rs`): `solve(&start, goal, limits)` returns
`Solve::Solved { commands, states }` (a shortest line),
`Solve::Exhausted` or `Solve::LimitReached`, or a `SolveError` for a bug
in the game. `Limits::DEFAULT` keeps 100 000 states with no depth limit.
The grid template's `check` calls it (`prove_stage`, in
`templates/shuyi-default/stages/top-down-grid/crates/game/src/stages/top_down_grid/tools.rs`).

2112 also held three parts left by done tickets (2803, 1134 and 2802),
and its size clause moved them here: its solver and the template's proof
came to about 1,050 counted lines alone. This ticket does them.

Today:

- `site/tutorial/09-test` and `10-ship` (2803) play a line kept in
  `crates/game/files/lines/1.ron` to `5.ron` (`10-ship` keeps `1.ron` to
  `3.ron`) with `LineBot` (`crates/game/src/tools.rs`), and
  `every_levels_line_wins_it` (`crates/game/tests/tools.rs`) plays each.
  The step's page, `site/content/learn/book/box-pusher/09-test.md`, shows
  the line file, and its checklist says the bot "isn't clever: it plays a
  line written by hand".
- `skills/shuyi-test/SKILL.md` (1134) says nothing of proving a level.
- The book's chapter 8, *Testing* (`site/content/learn/book/testing.md`,
  2802), has a *Bots* section and no word on `solve`. Its snippets live
  in `site/snippets/` (`src/testing.rs`); the book's garden
  (`site/snippets/src/rules.rs`) is a `TurnRules` game.

## Nick input

None.

## Scope

**In:**
- **The tutorial (from 2803).** In `09-test` and `10-ship`:
  - `LineBot` gets its line from `solve` on the level's start
    (`LineBot::new(puzzle: &Puzzle)` or a function beside it), not from
    a file; the files in `crates/game/files/lines/` go;
  - `every_levels_line_wins_it` becomes a test that proves every level
    winnable with `solve` and plays the line it returns to a win;
  - the tutorial's `Puzzle` must be `Clone + Eq + Serialize`. If it keeps
    a move counter or an undo history in the state, the states of two
    lines never compare equal, and the search may stop at its limit:
    then check what `solve` answers on the five levels, and leave the
    counter out of what the solver sees only if the tutorial's design
    allows it (a key that leaves part of the state out is 2112's
    **Out**). Say what was found in the completion notes;
  - the step's page (`site/content/learn/book/box-pusher/09-test.md`)
    shows the call in place of the line file, its table of files drops
    `files/lines/`, and its checklist's line on the shortest way is
    ticked;
  - the tutorial's step tests (`site/tutorial/tries.toml`, the steps'
    own tests) still pass.
- **The skill (from 1134).** A short section in
  `skills/shuyi-test/SKILL.md`: proving a level winnable with `solve` in
  a game's `check`, a `ContentError` when it isn't, and what
  `Exhausted` and `LimitReached` mean (a proof only when the game lists
  every command). `cargo xtask check-skills` passes.
- **The book (from 2802).** A short section in chapter 8 (*Testing*): the
  call, its three answers and a line on luck (a line is a line for that
  seed). Its code is a snippet in `site/snippets/` that CI builds and
  tests: proving the garden of `site/snippets/src/rules.rs` winnable, if
  it has a goal to reach, or a small puzzle of the snippet's own.

**Out (do not do):**
- changes to the solver (`crates/core/src/search.rs`);
- the grid template's proof (2112 did it);
- a `Bot` that plays by `solve` and sees the luck to come (`Bot`'s docs
  forbid it): a `LineBot` that plays a line `solve` found on a puzzle
  with no luck is fine.

## Implementation steps

1. Read `crates/core/src/search.rs` and the grid template's
   `prove_stage`.
2. Change `09-test`, then `10-ship`, as the Scope says, and run each
   step's tests.
3. Update `09-test.md`.
4. Add the skill's section; run `cargo xtask check-skills`.
5. Add the book's section and its snippet; run the snippets' tests.
6. Run the gates (`run-gates`).

## Acceptance criteria

- [ ] No `files/lines/` in `site/tutorial/09-test` or `10-ship`; their
      bot's line comes from `solve`.
- [ ] A test in each step proves every level winnable with `solve` and
      plays the line to a win.
- [ ] `09-test.md` shows the call, and builds.
- [ ] `skills/shuyi-test/SKILL.md` has the section; `check-skills`
      passes.
- [ ] Chapter 8 has the section, its code a snippet CI builds and tests.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The tutorial's steps' own tests, with the new `solve` test.
- The snippet's test in `site/snippets/`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
