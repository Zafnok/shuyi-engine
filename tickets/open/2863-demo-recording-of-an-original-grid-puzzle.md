---
id: "2863"
title: "The demo: an uncut recording of an agent making, testing and screenshotting an original small grid puzzle, and the puzzle on the website"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2861", "2005"]
nick_input: setup
completed:
---

# 2863 — The demo: an agent makes an original grid puzzle, recorded uncut

## Context

Nick asked for "a Fireship video type thing" (2859). Channels like that
cover what people already share; an unknown project that pitches them
reads as spam, so nobody is contacted (2859, *Context*). What this ticket
makes is the clip people can share: the engine's one idea shown in about a
minute, an agent making a game and checking it with no window
(`start`, `step`, `frame_png`, a test, a replay).

The research found that one-prompt game clips drew two replies in 2026:
"it's a clone from the training data" and "it isn't fun". So the clip
uses a puzzle with rules invented for it, not Snake, Tetris, Sokoban or
any known game (the 2026-10-26 rule on naming games, `docs/decisions.md`),
shows the prompt uncut, and states the attempts that failed. Nothing of
Visions of Shuyi appears (`AGENTS.md`, rule 4).

## Nick input

**Setup** (about 1–2 hours, once the script is ready):

1. Install OBS Studio (free, <https://obsproject.com>) if you don't have
   it.
2. Open `docs/launch/demo/recording-script.md` and follow it: a fresh
   folder, the fixed prompt, the agent runs, and you record the whole run.
3. In the edit, speed up the waiting at 4x and keep the "4x" label the
   script gives; cut nothing else.
4. End on you playing the result in the browser, then the end card.
5. Export a 60–90 second cut and a vertical cut of 15 seconds or less,
   put both where the script says, and tell the agent.

## Scope

**In:**
- `docs/launch/demo/prompt.md`: one fixed prompt for an original small
  grid puzzle (3 levels), with rules invented for it.
- `docs/launch/demo/rehearsals.md`: a row per rehearsal run (model,
  minutes, the agent's tool calls, what failed). At most five runs, to
  cap their cost.
- `docs/launch/demo/recording-script.md`: the shots (the prompt; the agent
  calling `start`, `step`, `frame_png`; the PNG files it looked at; a test and
  a replay passing; Nick playing it in the browser) and the end card:
  "Grid games today. Free and open source." with the site's address.
- `docs/launch/demo/captions.md`: model, minutes, attempts including the
  failed ones, "nothing edited by hand".
- The game the recorded run made, as an example in `examples/` and on the
  website's Examples page, its README stating its rules and the prompt it
  came from.
- A GIF from Nick's recording, under 8 MB, for the README and the itch
  page (2864).
- `docs/launch/` and its row in the docs map, if not there yet.

**Out (do not do):**
- Any game, name, art or level of an existing game, and anything of
  Visions of Shuyi.
- Free movement, side-scrolling or anything the engine doesn't do.
- Any comparison with another engine.
- Contacting any YouTuber, streamer or press.

## Implementation steps

1. Design the puzzle's rules (a grid, one invented mechanic, three
   levels) and write the prompt.
2. Rehearse in a fresh folder with `shuyi new` and the release's MCP
   server; log each run; change only the prompt between runs.
3. Write the recording script and the captions sheet.
4. After Nick records: make the GIF; add the recorded run's game to
   `examples/` and the Examples page, unedited but for what the example
   rules require (its `shuyi.ron`, shots and web build), each change listed
   in its README.

## Acceptance criteria

- [ ] `prompt.md`, `rehearsals.md`, `recording-script.md` and `captions.md` exist in `docs/launch/demo/`.
- [ ] `rehearsals.md` shows at least one run that worked on the first try, on the released version.
- [ ] `captions.md` states the model, the minutes and the attempt count.
- [ ] The puzzle plays on the website's Examples page, and its README states its rules and its prompt.
- [ ] The GIF is under 8 MB; the main cut is 60–90 seconds; the vertical cut is 15 seconds or less.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The completion notes have a `Skills:` line.
- [ ] All gates in the `run-gates` skill pass (the example's included).

## Tests required

- The example's own tests and snapshots, as every example has (its
  levels solvable, its replay passing).

## Completion notes

*(Filled in by the session that completes the ticket.)*
