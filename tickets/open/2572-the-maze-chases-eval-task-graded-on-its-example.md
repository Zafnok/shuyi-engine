---
id: "2572"
title: "The maze chase's eval task: evals/tasks/maze-chase.ron, graded on examples/maze-chase in CI"
type: infra
milestone: "Engine: real time"
model: implementation
effort: medium
status: todo
blocked_by: ["2776", "2524"]
nick_input: none
completed:
---

# 2572 — The maze chase's eval task

## Context

Split out of 2776 on 2026-10-30, when Nick unparked the demo chain
(snake, the maze chase and the A/B against Bevy, `docs/decisions.md`,
2026-10-30). 2776 makes the maze chase example and puts it on Pages.
This ticket gives the agent eval (2524) the task an agent is asked to
make, with that example as its reference solution. The A/B against
Bevy (2562) runs this task through an agent in each engine, so it waits
for this ticket.

Split so the game doesn't wait for the eval: 2524 waits for the
tutorial (2803) and one input script (2566), and 2776 needs none of
them.

**Not Pac-Man** (the 2026-10-26 rule): the prompt names no game, and the
task is called "maze-chase".

## Nick input

None.

## Scope

**In:**
- **The eval task** `evals/tasks/maze-chase.ron` in 2524's format:
  - `prompt`: "Make me a maze chase game with Shuyi Engine: a player who
    eats pellets in a maze, four chasers with different ways of chasing,
    power pellets that make them run, lives and a score, with a web
    build." Only the engine's name changes in 2562's Bevy run;
  - `template`: `"arcade"`, `name`: `"maze-chase"`,
    `time_box_minutes`: 60, `targets`: `desktop` and `web`;
  - `checks`: `Gates`, `Check`, `Targets`, `Web`, and `Responds` with
    `Act("Left")` and a wait. `Replay` too if 2524's grader can get a
    tick game's record by then; snake's task leaves it out because a
    tick game has no `bots` (2739's *The eval*), and this task follows
    whatever snake's does on the day;
  - `Scene` checks, through scene data the prompt names, as snake's task
    has (2739): the score goes up when the player eats a pellet, and a
    life goes when a chaser catches it;
  - `reference`: `Project("examples/maze-chase")`.
- The `eval` job's `cargo xtask eval-reference --all` grades it, and
  `cargo xtask eval-reference maze-chase` passes.

**Out (do not do):**
- The example itself: 2776.
- Running a real agent on the task: 2525, and the A/B's two arms: 2562.
- A Playdate target for the task: the maze chase example doesn't list
  one (2776).

## Implementation steps

1. Read 2524's task format and grader (`evals/tasks/`, `crates/xtask/src/eval/`),
   snake's task (`evals/tasks/snake.ron`, 2739) and the example's scene
   (`examples/maze-chase/src/tools.rs`, 2776).
2. Write `evals/tasks/maze-chase.ron`.
3. Run `cargo xtask eval-reference maze-chase`, then the `run-gates`
   skill.

## Acceptance criteria

- [ ] `evals/tasks/maze-chase.ron` reads with 2524's format, and
      `cargo xtask eval-reference maze-chase` passes on the reference.
- [ ] The `eval` job grades it with `--all`.
- [ ] The prompt names no game (the 2026-10-26 rule).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the grader on the reference, in the `eval` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
