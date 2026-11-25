---
id: "2787"
title: "The arcade template part: a `time` question, `stage-arcade-grid` on TickSim, the guide's In real time and the shuyi-stage-arcade skill"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2739", "3245"]
nick_input: none
completed:
---

# 2787 — The arcade template part, its guide section and its skill

## Context

2739 made `examples/snake`, the first grid game in real time
([ADR-0148](../../docs/adr/0148-a-grid-stage-may-run-in-real-time.md)), and
`shuyi_cli::contract::replay::ticks`. Its PR passed 2,800 counted lines
before the template part, so, as 2739's **Out** said, the template part,
its CI entry, the guide's section and the skill moved here.

ADR-0148 §1: a grid stage takes turns by default and ticks when the game
asks for real time, and the engine's own template asks it as its own
question `time`. This ticket writes that question and the part an agent
starts a real-time grid game from, so "make me a snake game" starts from
a stage that already steps in real time.

What there is:

- `templates/shuyi-default/template.ron` (2501, ADR-0042): parts with
  `when`, `conflicts` and `requires`, the short names `blank` and `grid`,
  and the template's own questions (`private-files`, `examples`);
  `wizard::questions` lists every question of it (3424), and
  `shuyi new --questions --json` gives them to an agent;
- `templates/shuyi-default/stages/top-down-grid/`: the grid part, a rules
  crate over `TurnRules` (`crates/rules/src/lib.rs`), a stage
  (`crates/game/src/stages/top_down_grid.rs`) drawn through `MapScene`
  and the glyph skin, its tools and `tests/stage.rs`;
- `examples/snake`: a `TickSim` stepped by `TickDriver` on the short
  path, its scene and its `replay` through `replay::ticks` — the pattern
  to follow;
- `docs/guides/top-down-grid.md` has a planned *In real time* section
  (2004);
- `evals/tasks/snake.ron` grades `examples/snake` with the `blank`
  template (2739);
- 3245 lets a game on the short path set its own `Background`; this
  part's game sets `Background::KEEP_RUNNING.paused()` with it.

## Nick input

None.

## Scope

**In:**
- **The question `time`** in `template.ron`'s `questions`: "On a grid:
  turns, or real time?", `pick: One`, answers `turns` (the default) and
  `ticks`. A test checks that `shuyi new --questions --json` lists it
  with its `asker` `template`, after the catalog's concerns and before the
  targets (3424's order).
- **The part `stage-arcade-grid`** in
  `templates/shuyi-default/stages/arcade-grid/`: `when: All([Picked
  ("perspective", "top-down-grid"), Picked("time", "ticks")])`,
  `conflicts: ["stage-top-down-grid"]`, `stage: Some("top-down-grid")`,
  `requires: ["base"]`; `stage-top-down-grid`'s `when` gains
  `Picked("time", "turns")`.
  - Its game is the grid part's small game in real time: the same map,
    pieces and goal, its rules a `TickSim` in a rules crate kept pure as
    the grid part's is. The hero steps a tile every 8 steps while a
    direction is held, the wanderer every 16 steps, timed with `Every`.
    It is not snake, so the eval's snake task stays a task.
  - Its stage steps the rules through `TickDriver` once for each of
    `input.ticks()`, records a `TickRecord`, and draws through `MapScene`
    and the glyph skin as the grid part does; its tools give the scene
    and `replay` through `replay::ticks`; `playtest` stays unsupported.
  - Its `Background` is `Background::KEEP_RUNNING.paused()` (3245's
    short-path setter).
- **`--template arcade`**, a short name for `perspective:
  ["top-down-grid"]` and `time: ["ticks"]`, beside `grid` and `blank`;
  `--set time=ticks` picks it too.
- **Its test**: with `Right` held for one second, the hero moves the same
  number of tiles in frames of 1/30 s (`Harness::frame`) as in frames of
  1/60 s.
- **CI**: the matrix of the `templates` job in `.github/workflows/ci.yml`
  gains `arcade`, and `crates/xtask/src/gates.rs` a `templates (arcade)`
  gate as `grid`'s.
- **The guide**: *In real time* in `docs/guides/top-down-grid.md` replaces
  the planned section: which part starts it (`--template arcade`),
  `TickSim` and `Every` against `TurnRules`, what not to use (`dt` in
  rules, a turn per tick), and `examples/snake` to read first; the
  guide's `Tickets:` line drops `2787`.
- **The skill `shuyi-stage-arcade`** in `skills/shuyi-stage-arcade/`, in
  ADR-0005 §2's form: a grid stage that steps in real time, with a worked
  example (`example.ron`, `template: "arcade"`) that makes a game with
  `--template arcade`, runs it, and replays a record. It passes
  `check-skills`, and CI's `skills` job runs its worked example.
- **The eval**: `evals/tasks/snake.ron`'s `template` becomes `arcade`,
  and its prompt says so.

**Out (do not do):**
- A bot trait for tick games, and `playtest` on a tick game: a ticket of
  its own if a game asks.
- Changing `examples/snake`, beyond what the eval needs.
- Free movement, collision and cameras (1120–1123).

**Estimated size:** about 1,800 lines (code 1,000, tests 400, docs 400).

**Split:** past 1,200 lines, split off the guide section and the skill
(steps 5 and 6) into a new ticket.

## Implementation steps

1. Read ADR-0148, ADR-0013 §1 and §3, ADR-0042, `template.ron`, the
   grid part, and `examples/snake` (`src/rules.rs`, `src/screens.rs`,
   `src/tools.rs`, `tests/it/frames.rs`).
2. Add the question `time`, the `when` on `stage-top-down-grid`, and
   the short name `arcade`; the wizard's test of the question's order.
3. Write `stages/arcade-grid/`: the rules crate as a `TickSim`, the
   stage, its tools and its tests, from the grid part's files.
4. Add `arcade` to the `templates` job's matrix and the gates.
5. Write the guide's *In real time*.
6. Write the skill and its worked example.
7. Set the eval's template, and run the gates.

## Acceptance criteria

- [ ] `shuyi new --questions --json` lists `time` among the template's
      questions, in 3424's order.
- [ ] `shuyi new` with `--template arcade` makes a game whose gates pass
      in the `templates` job, and its test passes at 30 and 60 frames a
      second.
- [ ] `--template grid` still makes the turn game.
- [ ] The guide's *In real time* is written and its `Tickets:` line drops
      2787.
- [ ] The skill passes `check-skills`, and CI's `skills` job runs its
      worked example.
- [ ] `evals/tasks/snake.ron` uses the `arcade` template, and `cargo
      xtask eval-reference snake` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the part's rules: a step every 8 while held, the wanderer every
  16, the goal and the catch.
- Integration: the game the part makes, its hero at 30 and 60 frames a
  second; the wizard lists `time`; the composer picks the part for
  `ticks` and the grid part for `turns`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
