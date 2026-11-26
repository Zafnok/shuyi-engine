---
id: "3432"
title: "The blind benchmark: fresh agents in empty folders get one line and an engine's link, Shuyi against Bevy, Godot and no engine named, scored on minutes, a blind judge's marks of the design, friction and lines"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2562"]
nick_input: decision
completed:
---

# 3432 — The blind benchmark against other engines

## Context

Nick, 2026-11-25 ([`docs/decisions.md`](../../docs/decisions.md), *the
benchmark is a blind race against other engines*): the engine's success
is measured by (1) the time from a prompt to a working game in the
intended design, (2) the follow-up prompts needed to refine it, and (3)
lines of code, against Bevy and other engines, in blind tests: "Agents
running in empty folders with no personalization etc and just given
prompt "Make X with Shuyi Engine - link" vs "Make X with Bevy - link"".
Agents leaning on the engine comes first, lines second.

What exists, and why it isn't this yet:

- 2524 (done) and 3421 (done): tasks as data and a grader through the
  contract. Its prompts are long and Shuyi-only: each says the folder
  holds a game made by `shuyi new`, which actions to use and which scene
  fields to expose.
- 2525 (open): `cargo xtask eval-run` runs a real agent in CI, in a
  fresh folder with an empty `HOME`, within a monthly budget, and records
  minutes, tokens, cost and lines. It starts each run *inside* a project
  `shuyi new` made, with the skills copied and the MCP server passed by
  `--mcp-config`: a warm start.
- 2562 (open): the A/B against Bevy on the maze chase. It adds a second
  arm, checks that work on any web build, a rubric judge over frame
  captures, a stream counter and a code/data line split. It too starts
  warm (Shuyi's arm from `shuyi new --template arcade`), and is one shot
  only.

This ticket adds the blind start and more arms, on the machinery of
2525 and 2562. The agent gets nothing but one line and a link, in an
empty folder: it must find the engine, install it, learn it and make the
game. That is what a stranger's agent does, and what decides which
engine agents lean on. The follow-up prompts (Nick's second number) are
3433, on top of this ticket.

**Briefs name no existing game.** Nick's example was "Make Doom". The
2026-10-26 rule ("a maze chase, not Pac-Man") and the 2026-11-03 one
(DOOM itself runs from its GPL code in `Zafnok/shuyi-doom`, never here)
mean a brief describes an original game in a genre's words: "a
first-person shooter in corridors" is the DOOM case. Nick may veto this,
as he may the 2026-10-26 rule.

## Nick input

**Decision** first, in one `ask-nick` round, with the cost of a round
estimated from 2525's recorded runs (briefs × arms × k runs, each up to
the time box):

1. **What may a round spend, and how often?** Offer, for example: one
   round when it lands with k = 1 (indicative); one a quarter with
   k = 3; one with each minor release of Shuyi. It comes out of 2525's
   monthly cap, or a cap of its own.
2. **Which arms?** Proposed: Shuyi, Bevy, Godot, and "no engine named"
   (the same line without "with X - link", so the agent picks; the
   report says what it picked). Unreal needs an Epic account, its EULA
   and a source build of the editor on Linux (tens of gigabytes per run):
   ask whether to add it, and if yes, write its ticket (step 1).
3. **The briefs:** the four below, or others in their place.

His answers go in `docs/decisions.md` and `evals/bench/bench.ron`.

## Scope

**In:**
- **Briefs as data,** `evals/bench/briefs/<id>.ron`, read by
  `crates/xtask/src/eval/bench.rs` (`deny_unknown_fields`):
  - `what`: the game in a player's words, one or two sentences. It is
    the only design the agent gets;
  - `rubric`: the items a judge marks, each `(item, from)`, where `from`
    is a phrase of `what` it follows from. A test fails an item whose
    `from` is not in `what`, so no brief has a hidden requirement;
  - `time_box_minutes`.

  The first four: `snake` (a snake that grows and dies on walls and
  itself), `box-pusher` (push boxes onto targets, a level counter, undo),
  `maze-chase` (pellets, four chasers that chase differently, power
  pellets, lives, a score), and `corridor-shooter` (first person, walls
  and doors, enemies that shoot back, health and ammunition pickups, an
  exit per level). The last is beyond what Shuyi shows today; a failing
  row is a finding, not a reason to drop it.
- **The arms as data,** `evals/bench/bench.ron`: per arm its `id`, its
  engine's name and link as the prompt shows them (`Shuyi Engine` and
  `https://github.com/Zafnok/shuyi-engine`, `Bevy` and
  `https://bevy.org`, `Godot` and `https://godotengine.org`, and `none`
  with no name or link), k, the agent and model (one of 2525's
  `agents`), and the judge's model.
- **The prompt,** built by `bench.rs` and nothing else:
  `Make {what} with {engine} - {link}`, or `Make {what}` for `none`,
  then one fixed sentence the same for every arm: "When it's done, write
  RUN.md with the one command that starts it on this Linux machine." The
  grader needs a way to start any engine's game, and the sentence names
  no engine or tool. A test checks that two arms' prompts differ in the
  engine's name and link alone.
- **The blind start** (`cargo xtask eval-bench`, reusing 2525's run code:
  its `Tools`, process group, time box, SIGINT then kill, the key scrub,
  the agent adapter, the `#[cfg(unix)]` and `GITHUB_ACTIONS` guards):
  - an empty folder and an empty `HOME`: no `CLAUDE.md`, no `~/.claude`,
    no memory, no skills, no plugins, no hooks, no `--mcp-config`, no
    `shuyi new` run first. The agent may use the network (it must, to
    follow the link), with the tools of 2525's command line;
  - the same base image for every arm, pinned by digest: `ubuntu:24.04`
    with `git`, `curl`, `ca-certificates`, `build-essential`, Node (for
    the agent's CLI), Python 3, and what the judge needs (`xvfb`,
    `xdotool`, Mesa's software GL and Vulkan, `mesa-vulkan-drivers` and
    `libgl1-mesa-dri`, so any engine's window draws). No Rust, no Godot,
    no engine preinstalled: the agent installs what it needs, and the
    minutes count;
  - the arms' runs interleaved in a shuffled order, one at a time, so
    no arm always runs first in the week's rate limits.
- **The grade, the same for every arm:**
  1. **It runs:** RUN.md exists; its command starts under Xvfb within
     five minutes (a first build allowed), runs 30 seconds without
     exiting with an error, and its frames are not blank.
  2. **The judge's marks:** a separate agent run, which sees neither the
     transcript nor the source nor the arm's name, plays the game under
     Xvfb through three tools of this ticket's own (a key press or hold,
     a screenshot, a wait), for up to ten minutes, and marks each rubric
     item met, partly met or not met, with one line of what it saw. Its
     prompt and the rubric are in `evals/bench/judge.md`. Nothing on
     screen names the engine but what the game itself draws (Xvfb has no
     window title bar); a splash screen that names one is noted in the
     report as a known leak.
  3. **The minutes:** the agent's wall time from the prompt to the end of
     its turn, and from the stream the time spent in its commands, apart.
  4. **Friction:** from the agent's stream (`agent.jsonl`), by rules in
     `bench.rs` tested on made-up streams: commands that failed (non-zero
     exit), the longest run of failures, a command repeated after it
     failed, pages fetched and searches made, the turns, tokens and cost.
     And whether the agent installed an MCP server or skills the engine's
     docs offered (in one non-interactive turn, a server installed
     mid-run doesn't connect; the report says so, and 3433's resumed
     turns are where it may).
  5. **Lines:** every text file the run left, without `.gitignore`d
     paths, lock files, build folders (`target/`, `.godot/`,
     `node_modules/`) and files over one megabyte. Split code, data and
     Markdown, as 2562 splits them, and the lines of files the agent wrote
     with its edit tools apart from files a command generated.
- **The results:** `evals/bench/results/<date>.json`, a row per run with
  the brief, the arm, the engine's version the agent installed, the
  model, the agent CLI's version and every number above; and
  `evals/bench/results/<date>.md`, a table per brief: each arm's runs
  that run, the judge's marks (met counts 1, partly ½), the median
  minutes, friction and lines, with k and the versions. Rows for `none`
  name what the agent picked. The games' source is a 30-day artifact,
  never committed: other agents' games may pull in code under any
  license.
- **The workflow:** `.github/workflows/agent-bench.yml`, run by hand
  (`workflow_dispatch`) and on the schedule Nick chose, in 2525's
  `agent-eval` environment, within its budget plan (`eval-plan` reserves
  the round's runs, judges included, or skips it and says why). The
  judge is a separate job step with its own budget cap and without the
  builder's files but the built game. The rules of 2525's workflow hold:
  `permissions: {}`, pinned actions, inputs through `env:`, no `push` or
  `pull_request` trigger, results committed by a job with no secrets.
- **Pages:** the latest `<date>.md` as a page, linked from the site's
  page of 2562's A/B. Numbers and the judge's one-line notes only.
- **`evals/README.md`:** what the benchmark is, how it differs from the
  eval (2524) and the A/B (2562), and how to start a round.

**Out (do not do):**
- Follow-up prompts and the time to the full design across turns: 3433.
- Unreal, Defold, GameMaker or others: a ticket each, if Nick picks them
  in step 1.
- Changing 2524's tasks or 2562's maze chase A/B: they stay as they are,
  the warm start beside this cold one.
- Publishing transcripts or committing the arms' games.
- Any engine change the runs find: a ticket each (step 7).
- Running any of it on Nick's machine (`AGENTS.md`, 2525's line).

**Estimated size:** about 1,400 lines (code 700, tests 450, docs and
data 250).

**Split:** past 900 lines, split off the judge that plays the game
(grade 2) into a new ticket, and grade the first round with 2562's
judge over scripted frame captures.

## Implementation steps

1. Ask Nick (`ask-nick`) with the cost estimate; record his answers in
   `docs/decisions.md`. If he adds Unreal or another engine, write its
   ticket (`write-ticket`), blocked by this one.
2. Write `evals/bench/bench.ron`, the four briefs and `bench.rs`'s
   reading of them, with the tests on `from` and on the prompts.
3. Write the blind start in `bench.rs` on 2525's run code, and the base
   image's setup as a script, `scripts/bench-image.sh`, run by the
   workflow.
4. Write the grade: the run check, the judge's three tools and its
   prompt, the stream rules and the line count, each tested on fakes.
5. Write the results files, the workflow and the Pages step.
6. Run one round by hand from the Actions tab with k = 1, commit its
   results, and put the table in the PR description.
7. For each friction point Shuyi's arm hit in that round that another
   arm didn't (a missing doc, a failed install, a wrong first command),
   write a ticket (`write-ticket`) with what the stream showed.

## Acceptance criteria

- [ ] Nick's answers are in `docs/decisions.md` and `bench.ron`, and the
      tickets step 1 calls for exist.
- [ ] A test fails a rubric item whose `from` isn't in its brief's
      `what`, and two arms' prompts differ only in the engine's name and
      link.
- [ ] A run starts in an empty folder with an empty `HOME`, and the
      agent's command line has no `--mcp-config` and no project files
      (a test with a fake agent that lists its folder and environment).
- [ ] The stream rules count failed commands, repeats after a failure
      and fetches on a made-up stream.
- [ ] A made-up game whose RUN.md command exits with an error fails
      grade 1, and one that runs passes it (test, Linux only).
- [ ] One round ran in CI within the budget, and
      `evals/bench/results/<date>.json` and `.md` are committed with a
      row for every run of every arm.
- [ ] The page is on Pages, linked from the A/B's page.
- [ ] No brief names or draws from an existing game.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema,
      template, format or problem code, or the completion notes have a
      `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `bench.ron` and the briefs read and refused when wrong; `from`
  in `what`; the prompts; the stream rules on made-up streams; the line
  count's exclusions and split.
- Unit, Linux only: the blind start with fake agents and one-second
  limits, as 2525's tests of `eval-run`; grade 1 on a made-up game that
  runs and one that fails.
- Integration: the workflow read as YAML, with 2525's checks (triggers,
  `permissions: {}`, the environment and secret on one step only).
- The one round by hand.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
