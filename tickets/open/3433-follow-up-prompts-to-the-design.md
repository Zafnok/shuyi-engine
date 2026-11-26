---
id: "3433"
title: "Follow-up prompts in the blind benchmark: a blind player plays each arm's game and asks for what's missing, and the benchmark counts the prompts and minutes to the full design"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3432"]
nick_input: decision
completed:
---

# 3433 — Follow-up prompts to the design

## Context

Nick, 2026-11-25 ([`docs/decisions.md`](../../docs/decisions.md), *the
benchmark is a blind race against other engines*), named three numbers:
the time from a prompt to a working game in the intended design, the
"number of follow up prompts needed to refine the game", and lines.

3432 runs the blind benchmark one shot: one line and a link in an empty
folder, a blind judge's marks of the brief's rubric, minutes, friction
and lines. A game that one shot gets half right then needs a person to
say what's wrong, and how many times they must say it is the second
number. This ticket adds those turns: a player, as blind as 3432's
judge, plays the game, and when the design isn't met writes the next
prompt the way a person would. The builder carries on in the same
session. The benchmark counts the prompts and the minutes until the
player says the game is done.

It also measures what one non-interactive turn can't: an MCP server or
skills the agent installed in its first turn connect when the session is
resumed, so an engine whose tools help after setup shows it here.

## Nick input

**Decision**, in one `ask-nick` round, with the cost from 3432's first
round:

1. **The cap on follow-ups** (proposed: five), after which a run is
   "not done".
2. **What a round may spend:** a run now costs up to its cap of turns,
   each with a player's turn. Offer: the same rounds as 3432 with the
   cap; or follow-ups only for k = 1.

His answers go in `docs/decisions.md` and `evals/bench/bench.ron`.

## Scope

**In:**
- **The player** (`crates/xtask/src/eval/bench/player.rs`): 3432's
  judge, which plays the game under Xvfb through its three tools and
  marks the rubric, gains a second answer: either `done` (every item
  met, and the game ran), or the next prompt. The prompt:
  - is in a player's words, at most 120 words, about what it saw: what
    is missing or wrong, never how to fix it;
  - names no file, function, language, tool or engine, and has no code.
    `player.rs` refuses one that does (the arm's engine name, a code
    fence, a file extension, a word from a short list in
    `evals/bench/player.md`), and asks the player once more; a second
    refusal ends the run as `player_failed`, reported apart;
  - is written from the brief's `what`, the rubric and what the player
    saw. The player sees neither the builder's transcript, nor its
    source, nor the arm's name, nor the turn's number.
  Its instructions are `evals/bench/player.md`, the same for every arm.
- **The loop,** in `eval-bench`: after the builder's turn, grade it
  (3432's grade 1 and the player). If not done and under the cap, resume
  the builder's session with the player's prompt (for Claude Code,
  `claude -p <prompt> --resume <session id>` with the first turn's
  flags; another adapter's equivalent, read from its docs on the day),
  within the run's time box and budget, which cover every turn. A turn
  that ends the box or the budget ends the run as "not done".
- **The numbers,** added to 3432's rows:
  - `follow_ups`: how many prompts until `done` (0 is one shot), or
    "not done" with the cap;
  - `minutes_to_design`: the builder's wall minutes over every turn
    until `done`. The player's minutes are reported apart and never
    counted, as a person's time to look isn't the engine's;
  - the rubric's score after each turn (a short curve), the friction and
    lines of each turn, and the lines at `done`;
  - `mcp_connected` per turn, from each resumed turn's `system/init`
    (2525's field), so a server the agent installed shows when it first
    connects.
- **The table:** 3432's results page gains, per brief and arm, the median
  `follow_ups` (with "not done" counted as the cap plus one, and the
  number not done beside it) and the median `minutes_to_design`, before
  the one-shot columns.
- **`evals/README.md`:** the loop, the cap, and that the player's
  minutes don't count.

**Out (do not do):**
- A person as the player: the player is an agent, the same for every
  arm, so rounds can be repeated.
- Prompts the player writes with advice on how to fix the game.
- Changing 3432's one-shot grade: its columns stay as they were.
- Any engine change the turns find: a ticket each, as 3432's step 7.

**Estimated size:** about 1,100 lines (code 550, tests 400, docs 150).

**Split:** past 900 lines, split off the per-turn curve and the page's
new columns into a new ticket, keeping `follow_ups` and
`minutes_to_design` in `results.json`.

## Implementation steps

1. Ask Nick (`ask-nick`) with the estimate from 3432's round; record his
   answers.
2. Write the player's second answer and the prompt checks, with tests
   on made-up answers.
3. Write the loop on 2525's run code, resuming the session, with a fake
   agent that keeps a session id and a fake player.
4. Add the numbers, the table's columns and `evals/README.md`.
5. Run one round by hand with k = 1 and the cap; commit its results and
   put the table in the PR description.

## Acceptance criteria

- [ ] Nick's answers are in `docs/decisions.md` and `bench.ron`.
- [ ] A player prompt naming the arm's engine, a file or code is refused
      and asked again, and a second refusal ends the run as
      `player_failed` (tests on made-up answers).
- [ ] With fakes, a run whose player says `done` after two prompts has
      `follow_ups` 2, and one that never does ends "not done" at the cap
      (tests, Linux only).
- [ ] `minutes_to_design` sums the builder's turns and leaves out the
      player's (a test with fake timings).
- [ ] One round ran in CI within the budget, and its results and page
      have the new columns.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema,
      template, format or problem code, or the completion notes have a
      `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the prompt checks; the numbers from made-up turns.
- Unit, Linux only: the loop with a fake builder that resumes a session
  and a fake player, one run done and one at the cap.
- The one round by hand.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
