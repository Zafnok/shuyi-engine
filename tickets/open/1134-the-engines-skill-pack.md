---
id: "1134"
title: "The engine's skill pack: Agent Skills for making games on the engine, tested in CI and shipped as a plugin"
type: feature
milestone: "Engine: public"
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["1129", "2015", "2509", "2511", "2810"]
nick_input: none
completed:
---

# 1134 — The engine's skill pack

## Context

Nick, 2026-10-14: "Add plenty of shippable skills to it, similar to how
https://github.com/morluto/rea became huge by adding ai skill wrappers
around pre-existing tools."

ADR-0005 §2–§3 sets the shape. Each skill is a `SKILL.md` folder in the
open Agent Skills format, with:

- when to use it, and when not to;
- the steps;
- the contract operations it calls;
- reference files that load on demand.

The research found that no game engine ships official skills yet.
Remotion's official skills and plugin are the closest model.

The skill pack is what the agent eval judges (2524 grades tasks through
the contract; 2525 runs a real agent on them on a schedule), against the
end goal in `docs/decisions.md` (2026-10-19).

## Nick input

None.

## Scope

**In:**
- `skills/` (in the engine repository after 1131) holds the 15
  skills listed in ADR-0005 §3, from `shuyi-new-game` to `shuyi-net`.
  Four of them teach parked work and wait for it. Each says so in its
  body, lists the ids it waits for in its frontmatter (`waits_for`
  under `metadata`, the open format's place for a skill's own keys),
  and has no worked example until they are done:
  - `shuyi-net` waits for 1140, 2529 and 1145. Its worked example then
    plays two peers with the contract's `peers` op (2529) and a seeded
    loss, on `hello-grid` once 1145 gives it an online mode. It is the
    one example not run on a fresh `shuyi new` game, which offers no
    peers.
  - `shuyi-stage-field` waits for 1119, 1120 and 1122.
  - `shuyi-import-art` waits for 1123 and 1124.
  - `shuyi-animate` waits for 2231 (clips). Its section on particles
    waits for 2103 and says so until then.
- Each skill is generalized from this repository's practice and never
  copies our game's content. Sources: `run-gates`, `keyboard-input`, the
  ADRs named in the table.
- Each skill has a worked example as a script of contract operations.
  CI runs every example against a fresh `shuyi new` game (all but
  `shuyi-net`'s). If an example fails, the skill is wrong. The runner of
  these scripts is a library function in `crates/xtask` (a script of
  contract commands in, a verdict per step out), not steps inline in a
  workflow: the agent eval's grader (2524) runs its checks through it.
- Skills branch on a failed answer's `code` (2509), never on its
  message, and send the reader to `shuyi explain <code>`.
- Skills that write data files (`shuyi-new-game`, `shuyi-import-art`,
  `shuyi-ship`) tell the agent to read a file's shape with
  `shuyi kinds <kind> --json` before writing it. Either order with
  2515: if 2515 is done, write it here; if not, 2515 adds the line.
- Skills that send an agent to the engine's docs send it to the
  project's `AGENTS.md`, which names the engine's rules and docs at the
  version the game builds with (2808's `{{engine.docs}}`), never to a
  fixed URL on `main`. Either order with 2808: until it is done, that
  `AGENTS.md` links `blob/main`.
- **One source for what is true of every game.** Some of these skills
  teach what a skill of this repository already teaches:
  `shuyi-input` and `input-actions`, `shuyi-gates` and `run-gates`.
  The shipped skill becomes the source for what holds in every game.
  Ours keeps only what holds in this repository alone (the engine's
  own crates, CI's jobs, the game at the next pin, Nick's machine) and
  links the shipped skill for the rest, in this PR. A rule is never
  written in both. The docs map (`docs/README.md`, 2810) gets:
  - a row for `skills/` under *People making games*;
  - each such pair in its *Pairs* table;
  - a line in *What each task reads*: the `25xx` block reads
    `skills/`.

  A later skill that overlaps one of ours follows the same rule.
- What particular skills teach beyond ADR-0005 §3's table:
  - `shuyi-new-game` teaches the short path first (2511): `shuyi new
    blank`, `BasicGame`, `BasicData`, `run_game`, `BasicTools`. It says
    when to leave it: an action set of the game's own, a part of its own
    (`with_own`), a `GameTools` of its own, or `GameData` by hand.
  - `shuyi-new-game` says how an agent gets the MCP server
    (`docs/contract.md`, *The MCP server*): `cargo shuyi mcp install`
    once, `.mcp.json` checked in, and after updating the engine, run
    `cargo shuyi mcp --ready` (2535), then reconnect the server.
  - `shuyi-new-game` has a short section on sound effects (2225). A cue
    names a `.sfx.ron` file, `shuyi sfx <preset> --out <file>` writes
    one, `--wav` renders it to listen to, and these are the fields to
    change. Either order with 2225: if it isn't done when this runs,
    2225 adds the section.
  - `shuyi-gates` names the rules crate's guards from 2015:
    `crates/rules/clippy.toml`, the `deny(clippy::float_arithmetic)` in
    `src/lib.rs` and `tests/layers.rs`. It says what each error means
    and what to use instead (`BTreeMap`, `SimRng`, a `Command`, whole
    numbers), and that a failure is fixed in the code, never by
    loosening the file or the test.
  - `shuyi-stage-grid` says a game's luck comes from a `SimRng` kept in
    its state, never from the `rand` crate (its minor releases may
    change its values) or a clock. Either order with 2114: if 2114 is
    done, the skill names its helpers (`range`, `chance`, `pick`,
    `shuffle`, `pick_weighted`, `Bag`) and one stream per purpose
    (`SimRng::stream`, `fork`). If it is not, 2114 adds that section
    when it lands.
  - The skills that teach rules (`shuyi-stage-grid`,
    `shuyi-stage-field`) say how to time a rule: `shuyi_core::timer`'s
    `Countdown` and `Every`, advanced by one each turn or once per step
    of `FrameInput::ticks()`, never by `dt` (2115). If 2115 isn't done,
    they say that rules count turns or steps and never read `dt`.
  - `shuyi-test` covers saved forms (2110): a save format's `VERSION`,
    its `Migrate` steps, and the `v<N>_*.ron` fixtures
    `check_saved_forms` checks, with what its failure asks. If 2110
    isn't done when the skill is written, the skill leaves saves out and
    2110 adds the paragraph.
  - Search: `shuyi-balance-bots` shows `Determinize`, the rollout bot,
    the playtest runner with its bands, and the autobalance search
    (`shuyi_core::autobalance` and the game's `Knob`s). Either order
    with 2112 and 2113: if they are done, `shuyi-test` shows how to
    prove a level winnable with `shuyi_core::search::solve` in a game's
    `check`, and `shuyi-balance-bots` shows the minimax and MCTS bots.
    If not, each of those tickets adds its section when it lands.
    `shuyi-balance-bots` also shows targets by tier
    (`shuyi_core::bots::playtest::Targets`) and personas as data
    (planner settings and weights: `shuyi_core::bots::weigh`). Either
    order with 2131, 2132, 2532, 2533 and 2021: if they are done, it
    also shows the tactics kit's threat areas and candidate order,
    `shuyi playtest`'s options and history, and the reusable playtest
    workflow. If not, each adds its lines when it lands.
  - Motion, either order with 2514: if 2514 is done, `shuyi-test`,
    `shuyi-animate` and `shuyi-cinematic` say to check motion with a
    sheet (`frame-png --every <seconds> --count <n>`, or the MCP
    `frame_png` with `every` and `count`), and `shuyi-ship` says how to
    make an APNG or a GIF (`--apng`, `--gif`) for a README or a store
    page. If 2514 isn't done, 2514 adds those lines.
  - The live link, either order with 2521 (`shuyi attach`): if 2521 is
    done, `shuyi-test` gets a section on attaching to a running build
    (when to use it rather than the headless `run`, the `link` feature,
    `SHUYI_LINK`, the token, and `--screenshot`), with a worked example
    that CI can't run (it needs a window), marked as manual. If not,
    2521 adds that section.
  - Soak, either order with 2527: if `shuyi soak` exists, `shuyi-test`
    gets a short section on running it in CI and on turning a shrunk
    record into a regression test with `shuyi_core::soak::commands`,
    and `shuyi-debug-replay` says to run `soak` first when a save or a
    replay misbehaves (`replay` rebuilds the state by replaying, so it
    can't see what the serde form leaves out). If 2527 isn't done, 2527
    adds these.
  - Debug commands, either order with 2356: if 2356 is done,
    `shuyi-debug-replay` (and `shuyi-test`) tell agents to reach a bug's
    state with a debug command the game declares (`--command`), rather
    than a long key script. The command is a debug-only variant of the
    game's own command enum, never in `legal_commands`. If 2356 isn't
    done, 2356 adds that section.
  - `shuyi-debug-replay` also starts from a player's crash report
    (3046): `shuyi crash <report> --symbols <file>` reads it,
    `shuyi replay --crash <report>` reproduces it (`ok: false` while it
    panics), and the skill loops fix, then replay, until `ok: true`
    (2520). If 2520 isn't done when the skill is written, its body says
    the crash path waits for 2520.
  - `shuyi-ship` covers a game's notices and credits: the `NOTICES.txt`
    a build carries (3039, and 3004 for every package), the credits
    file, the `credits` part of `shuyi.ron` and `check`'s credits pass
    (2223), and that an agent never widens a game's `licenses` list to
    make a check pass (that is the game owner's call). It teaches what
    exists when the skill is written; 2223 adds the rest if it lands
    later.
  - `shuyi-ship` teaches releasing a game from a tag if 3050 is done:
    `cargo shuyi export --plan`, `git tag -a vX.Y.Z -m <notes>`, the dry
    run from the Actions tab, and turning on itch.io with the
    `ITCH_GAME` variable and the `BUTLER_API_KEY` secret. If not, 3050
    adds that section.
- `shuyi-cli check-skills` checks six things:
  - frontmatter (`name`, `description` with when and when-not, a
    version);
  - every operation a skill names exists in the contract;
  - every file a skill links exists;
  - the length is within the format's guidance;
  - every id in `waits_for` is a ticket in `tickets/open/`, and only a
    skill with a `waits_for` has no worked example;
  - every `SHnnnn` code a skill names is a row of 2509's table
    (`shuyi explain --json`) and isn't retired.
- Packaging:
  - a Claude Code plugin (a marketplace entry in the engine repository);
  - the layout the common skill installers read;
  - `shuyi new` copies the skills into new projects.

**Out (do not do):**
- skills for our game (they stay in the game's repository);
- session records in `shuyi-debug-replay`: 2519 adds a section
  (`shuyi run --session`, `first_difference`) once debug builds write
  them. Until then the skill reproduces a bug from a rules record
  (`shuyi replay`);
- `shuyi-upgrade`, the skill that moves a game to a newer engine, and
  the `shuyi migrations` op it runs: 2523, after 2005 and 2804;
- `shuyi-stage-arcade`, the real-time grid skill: 2739 (parked) writes
  it in this pack's form;
- the agent eval: its tasks, grader and reference solutions (2524), and
  scheduled agent runs (2525).

## Implementation steps

1. Write `check-skills`, the worked-example runner, and their tests.
2. Write the skills and their worked examples, three or four per PR if
   the diff grows past one PR's worth (split into follow-up tickets).
3. Do the packaging, and add the book's page on it (2802).

## Acceptance criteria

- [ ] `check-skills` passes on every skill.
- [ ] CI runs every worked example green, except one marked manual.
      Only the skills that wait for parked tickets have none.
- [ ] Installing the plugin in Claude Code lists the skills (manual
      check noted in the PR).
- [ ] `input-actions` and `run-gates` hold only what is this
      repository's and link `shuyi-input` and `shuyi-gates`, and
      `docs/README.md` lists both pairs.
- [ ] All gates pass.

## Tests required

- Unit: `check-skills` and the worked-example runner.
- Integration: worked examples in CI.

## Completion notes
