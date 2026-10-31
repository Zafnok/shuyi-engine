---
id: "2562"
title: "An A/B against Bevy: the same agent makes the maze chase in each engine, scored on one-shot passes, friction, deterministic checks and lines, published on Pages"
type: infra
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2572", "2525"]
nick_input: decision
completed:
---

# 2562 — An A/B against Bevy on the maze chase

## Context

Nick, 2026-10-26 ([`docs/decisions.md`](../../docs/decisions.md)): "can
we do an A/B test how easy it is, how many LOC it takes, to make a
Pac-Man clone in say Bevy vs Shuyi Engine? We can host an example on our
github pages". And what it is for: "1) agent can one shot games easier
with 2) less friction and 3) more deterministic handling (i.e. we have
skills and MCP) and 4) approx equal LOC (this matters less since it's
agent first engine but would be a less subjective benchmark)".

On 2026-10-30 Nick unparked the demo chain ("we need better demos",
`docs/decisions.md`), this A/B last, "After the maze chase": it starts
once the maze chase and its eval task are done, and spends nothing
before his answer below.

The eval does most of the work already, for Shuyi alone:

- 2524 gives tasks as data, a grader through the contract, and the line
  count (`git diff --no-index --numstat` over the template's fresh
  project);
- 2525 runs a real agent on each task in CI (`cargo xtask eval-run`), in
  a fresh folder, with a time box and a budget, and records minutes,
  cargo's minutes (its `cargo` wrapper's `cargo.log`), tokens, cost and
  lines in `results.json`;
- 2776 gives the maze chase's reference solution, `examples/maze-chase`,
  and 2572 its task (`evals/tasks/maze-chase.ron`).

This ticket adds the second arm: the same agent making the same game
with Bevy, and the same scores for both. Bevy has no tool contract, so
2524's grader can't check a Bevy game. The A/B grades both arms with
checks of their own that work on any web build, and keeps 2524's
contract checks as Shuyi's extra column, never as a score Bevy is held
to.

The 2026-10-20 comparison (2524's *Context*, ROADMAP *Bevy's homepage*)
read Bevy 0.20. Read Bevy's release on the day: its version, its
examples, its fixed-timestep example and its web build (`wasm-bindgen`
through `trunk` or `wasm-server-runner`). Whatever agent tooling Bevy
itself ships on the day (its docs, its examples, the Bevy Remote
Protocol) is the Bevy arm's; a community MCP server is not, and the
report says which were there.

## Nick input

**Decision** first, in one `ask-nick` round:

1. **What may the A/B spend?** It runs the agent 2k times (k on each
   side) per round. The session estimates a round's cost from 2525's
   recorded runs, and offers: one round when it lands; one a quarter;
   one with each minor release of either engine. It comes out of the
   monthly cap Nick set for 2525, or a cap of its own.
2. **Which page says it?** The numbers and both games on Pages
   (decided 2026-10-26), with Bevy named. Ask whether the page may say
   which engine scored better in its own words, or shows the numbers
   only.

His answers go in `docs/decisions.md` and in `evals/ab/maze-chase.ron`.

## Scope

**In:**
- **The A/B as data,** `evals/ab/maze-chase.ron`, read by
  `crates/xtask/src/eval/ab.rs` (`deny_unknown_fields`): the task id
  (`maze-chase`), k, the agent and model (one of 2525's `agents`), and
  per arm its name, how its project starts, and its prompt. The prompt
  is the task's `prompt` with only the engine's name changed; a test
  checks that the two differ in that name alone.
- **How each arm starts:** Shuyi's as 2525 starts a run (`shuyi new
  --template arcade`, its skills and MCP server). Bevy's from `cargo
  new maze-chase` with `bevy` at the version on the day as its one
  dependency, in the same empty folder, with the same `cargo` wrapper
  and no MCP config. The fresh project is each arm's base for the line
  count.
- **`cargo xtask eval-ab --ab <file> --out <dir>`** (`ab.rs`): for each
  arm, k runs through 2525's `eval-run` code with the arm's start and
  prompt; the same `#[cfg(unix)]` and `GITHUB_ACTIONS` guards; the same
  time box and per-run cap. Then the scores below for each run.
- **The scores, the same for both arms** (Nick's order):
  1. **One shot:** the share of runs that pass every shared check, with
     no human turn. The shared checks need nothing but the arm's own
     build: it builds for the desktop and the web; the web build loads
     in headless Chromium with no console error (a Playwright script of
     this ticket's own, run the same way on both games); pressing the
     prompt's directions moves the player (two canvas captures differ);
     and an agent judge, a separate run with no access to the transcript,
     marks the task's features (pellets, four chasers that chase
     differently, power pellets, lives, a score) from scripted frame
     captures, against a written rubric in `evals/ab/rubric.md`.
  2. **Friction:** per run, the agent's minutes and cargo's minutes, the
     number of failed `cargo` runs and the longest run of failures in
     `cargo.log`, the agent's turns, tokens and cost.
  3. **Deterministic checks:** per run, how the agent checked its work:
     tool calls that check by replay, by the contract or MCP, or by tests
     the run added, against those that only look at a frame. Counted from
     The agent's stream (`agent.jsonl`), by tool name and command, with
     the rules written in `ab.rs` and tested on made-up streams. And
     whether the game itself is deterministic: the same scripted input
     played twice in headless Chromium gives the same last canvas.
  4. **Lines:** lines added over the arm's base, code (`.rs`) and data
     (everything else) apart, so a Shuyi game's `.ron` files count as
     Bevy's constants do.
  Shuyi's 2524 checks (`Check`, `Targets`, `Replay`) go in a column of
  their own.
- **The games kept:** for each arm, the source of the run that passed the
  most shared checks (fewest lines on a tie) goes to
  `evals/ab/maze-chase/<date>/<arm>/`, with its `results.json` rows. Not
  in the workspace: the root `Cargo.toml` gains `exclude = ["evals/ab"]`,
  and the Bevy game keeps its own `Cargo.lock`.
- **Licenses** (`AGENTS.md` rule 7): the kept Bevy game goes on Pages, so
  it ships. Run `cargo deny check licenses` on it with the root
  `deny.toml`. A crate outside the allow list (an MPL audio decoder
  under one of Bevy's default features, for example) is turned off by
  feature in the kept game's `Cargo.toml`, and the report says what was
  turned off. Its web build's `wasm-bindgen` is fine: it is that game's,
  not the engine's (the engine's web build stays macroquad's loader,
  game ADR-0034).
- **The report and Pages:** `evals/ab/maze-chase/<date>/README.md`, the
  four scores per arm with k, the model, both engines' versions and what
  agent tooling each had. `pages.yml` builds both kept games for the web
  and publishes them at `/ab/maze-chase/shuyi/` and
  `/ab/maze-chase/bevy/`, with the report as a page linked from the
  index, or from the site's examples section if the site has one by
  then.
- **The workflow:** `.github/workflows/agent-ab.yml`, run by hand
  (`workflow_dispatch`) and on the schedule Nick chose, in 2525's
  `agent-eval` environment, within its budget plan (`eval-plan`
  reserves the 2k runs, or skips the round and says why). Pinned actions
  and zizmor as every workflow.

**Out (do not do):**
- Other engines (Godot, Defold): a ticket each, after this one, if Nick
  asks.
- A Bevy game written by hand: both arms are an agent's.
- Publishing the agents' transcripts (2026-10-26 rule).
- Any engine change the A/B finds: a ticket each (`write-ticket`), with
  the friction it measured.

## Implementation steps

1. Ask Nick (`ask-nick`) with the cost estimate, record his answers.
2. Write `evals/ab/maze-chase.ron`, `evals/ab/rubric.md` and `ab.rs`'s
   reading of them, with tests.
3. Write the Bevy arm's start, reusing 2525's run code.
4. Write the shared checks, the stream counter and the line split, each
   tested on fakes (made-up streams, a made-up web build).
5. Write `eval-ab`, the workflow and the Pages steps.
6. Run one round by hand from the Actions tab, commit the kept games and
   the report, and check both games play on Pages.

## Acceptance criteria

- [ ] One round ran in CI with k runs per arm, within the budget.
- [ ] `evals/ab/maze-chase/<date>/` holds both kept games and the report
      with the four scores per arm, and Shuyi's contract column.
- [ ] Both games play on Pages, and the report is linked from the index.
- [ ] The kept Bevy game passes `cargo deny check licenses` with the
      root `deny.toml`.
- [ ] Nothing in the games or the report is named or drawn from Pac-Man
      (the 2026-10-26 rule).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `maze-chase.ron` read and refused when wrong; the two prompts
  differ in the engine's name only; the stream counter on made-up
  streams; the line split by extension.
- Property: none.
- Snapshot / integration: `eval-ab` with fake agents and one-second
  limits, as 2525's tests of `eval-run`; the shared checks on a made-up
  web build that passes and one that fails each check.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
