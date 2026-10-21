---
id: "2803"
title: "The tutorial: a box-pushing puzzle built step by step, each step a project CI builds and tests, each playable in the browser"
type: infra
milestone: "Engine: public"
model: sonnet-5
effort: high
status: todo
blocked_by: ["2801", "2802", "2511"]
nick_input: none
completed:
---

# 2803 — The tutorial

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): "a robust
tutorial". The book (2802) explains each concept on its own. A tutorial
takes a reader from nothing to a finished small game, in order, and
each step must still work after every engine change. Bevy's Breakout
example and Godot's "Your first 2D game" are the model.

The game is a box-pushing puzzle on a grid: walk, push a box, put every
box on a target, undo, levels from files. It uses the engine's turn
games, grid, actions, assets, history, menus, saves, harness and web
build, and nothing of Visions of Shuyi (`AGENTS.md` rule 4). Call it "a
box-pushing puzzle", never by the trademarked name of the original game.

## Nick input

None.

## Scope

**In:**
- **`site/tutorial/`**, one folder per step, each a whole game project
  (`shuyi.ron`, `Cargo.toml`, `src/`, `assets/`) made with `shuyi new`
  from the `blank` template, on the short path (2511), with a path
  dependency on the engine's crates:
  1. `01-new`: `shuyi new`, run it, what each file is;
  2. `02-board`: a level as a text file, loaded through the registry,
     drawn by the grid skin; the level is `BasicData`'s own part
     (`BasicGame::with_own`);
  3. `03-walk`: actions for the four directions, a move command, walls;
  4. `04-push`: pushing a box, the win check, a test through the
     harness;
  5. `05-undo`: history and undo, as an action; undo is an action
     `UiAction` lacks, so the game's own `ActionSet` enum replaces it
     here;
  6. `06-levels`: a level list, a menu screen, a result when a level is
     won (ADR-0018);
  7. `07-save`: the furthest level reached, in storage;
  8. `08-polish`: a moves counter, the options screen with rebinding,
     and sound on push and win. Either order with 2225: if it is done,
     the sounds are `.sfx.ron` files made with `shuyi sfx`, which the
     short path plays from the step's files. If not, the short path has
     no sound yet (2511 leaves it to 2225), so the step has none, and
     2225's PR adds the two sounds and the page's part on them;
  9. `09-test`: a bot that solves level 1, a replay test, a frame
     snapshot. Either order with 2112: if 2112 is done,
     `shuyi_core::search::solve` finds level 1's line, and a test proves
     all five levels winnable. If not, the step plays a line kept in its
     files, and 2112's PR switches it to `solve`;
  10. `10-ship`: `shuyi check --target`, `shuyi web`, and putting it
      on Pages.
- **A page per step** in `site/content/learn/tutorial/`: what to do,
  and the diff from the step before shown as code that comes from the
  step's files (2802's snippet shortcode, pointed at the step's folder).
- **CI:** each step is built and tested (`cargo test` in each folder),
  in a matrix job of `ci.yml`. The steps aren't workspace members, so
  each has its own `Cargo.lock`; the job runs `cargo update -p
  shuyi-*` first, so the steps build against the engine's current code.
- **Playable:** `pages.yml` builds each step for the web into
  `/learn/tutorial/<step>/play/`, and each page links it.
- Levels are our own, written for the tutorial: five small ones.
- **The agent eval's reference.** Step 10 (`10-ship`) is the reference
  solution of 2524's box-pushing task. Every step is made with one
  `shuyi new blank --name <name>`, and step 01's page gives the name:
  2524's task and its line-count base reuse it. Its scene data (`cargo shuyi
  scene --json`, `result.scene.data`) has `moves` (the moves made on
  this level, from step 08) and `won` (true once every box is on a
  target, from step 04). Its actions keep `UiAction`'s six names, which
  the grid template uses too (`Up`, `Down`, `Left`, `Right`, `Confirm`,
  `Cancel`), plus its undo action. `BasicTools` has no scene, bot or
  replay, so by step 09 the game has its own `GameTools`, which calls
  `BasicTools` for the rest (2511's way on).
- **Either order with 3053:** if it is done, every step leaves (Cancel,
  and any Quit in step 06's menu) only where `ctx.platform.can_quit()`,
  as the templates do then. No step played in the browser then freezes
  when the player leaves. If not, 3053 changes the steps when it lands.

**Out (do not do):**
- a second tutorial (a tick game waits for 1119; the first tick example
  is snake, 2739, parked);
- an automatic browser check of each step's web build (2019, after the
  MVP, adds the steps to its `web-render` job); here each step is played
  by hand;
- video.

## Implementation steps

1. Build the finished game first (step 10), with its tests.
2. Cut it back into the earlier steps, each building and passing its own
   tests.
3. Write the pages.
4. Add the CI matrix and the Pages builds.
5. Run the gates.

## Acceptance criteria

- [ ] Ten steps, each building and passing its tests in CI.
- [ ] Each step plays in the browser from its page.
- [ ] Step 10's scene data has `moves` and `won`, and its actions keep
      the six names and undo (*The agent eval's reference*).
- [ ] Every code block on a tutorial page comes from a step's files.
- [ ] A change to the engine's API that breaks a step fails CI.
- [ ] Nothing names the original game's trademark, or anything of
      Visions of Shuyi.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Each step's own tests: the harness, and from step 9 the bot and replay.
- CI's matrix job over the steps.

## Completion notes

*(Filled in by the session that completes the ticket.)*
