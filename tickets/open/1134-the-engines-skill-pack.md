---
id: "1134"
title: "The engine's skill pack: Agent Skills for making games on the engine, tested in CI and shipped as a plugin"
type: feature
milestone: "Engine: public"
model: opus-5.5
effort: xhigh
status: in-progress
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
    peers. **Either order with 2551:** if 2551 is done, a game made
    with `shuyi new <dir> --online lockstep` offers peers: the skill
    says what that template part gives and shows `cargo shuyi peers` on
    such a game, and its worked example stays on `hello-grid`. If not,
    nothing to do (2551 adds that to the skill when it lands).
    If this ticket lands first, each of 1140, 2529 and 1145 drops its
    id from `waits_for` when it lands, and the later of 2529 and 1145
    writes the worked example (their clauses say so).
    **Either order with 2550** (a relay or a referee inside a `peers`
    run): if 2550 is done, the skill's `peers` section also teaches
    `--via`, the hub in the answer, and `MemoryPeers::through_relay`
    (and `under_referee`, if 2550 kept it); if not, nothing to do
    (2550 adds them when it lands).
  - `shuyi-stage-field` waits for 1119, 1120 and 1122. If they are
    done, write its worked example here, with no `waits_for`. If not,
    1119 and 1120 each take their id out of `waits_for` when they land,
    and 1122, the last of the three, writes the worked example.
  - `shuyi-import-art` waits for 1123 and 1124.
  - `shuyi-animate` waits for 2231 (clips): if 2231 is done, it
    teaches clips with its worked example; if not, 2231 adds them (its
    clause says so). Its section on particles waits for 2103. If 2103
    is done, write it here. If not, the skill says particles are to
    come, and 2103 writes the section.
- **Skills of later modules**, which ADR-0005 §3's table doesn't list,
  wait the same way. A module's ticket writes its skill if this pack is
  done, or adds it to this list if not, and the module's example
  finishes it with a worked example. A skill this pack writes from such
  an entry lists the ids of the tickets still to come in `waits_for`.
  - **Either order with 2726, 2732, 2735, 2741 and 2754:** if one is
    done, write what it put in this list: the worked example of
    `shuyi-vn` (2726), `shuyi-mystery` (2732) or `shuyi-procgen`
    (2735), the `shuyi-crawl` skill (2741), or the worked examples of
    `shuyi-crawl` and `shuyi-rpg` (2754). If not, nothing to do: each
    writes or finishes its skill.
- Each skill is generalized from this repository's practice and never
  copies our game's content. Sources: `run-gates`, `input-actions`
  (ADR-0005's table calls it `keyboard-input`), the ADRs named in the
  table.
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
    once, and `.mcp.json` checked in. The server follows the session's
    folder and the engine by itself (2546), so there is nothing to
    rebuild or reconnect after updating the engine; `cargo shuyi mcp
    --ready` (2535) is for a first build before connecting.
  - **Either order with 2004** (the guides): if 2004 is done,
    `shuyi-new-game` sends an agent to the direction guide
    (`docs/guides/`) before it chooses a template; if not, 2004 adds
    that line when it lands.
  - The wizard and templates from anywhere, either order with 2502
    and 2505: if 2502 is done, `shuyi-new-game` calls `shuyi new <dir>
    --questions --json`, asks its user, and passes the answers as
    flags. If 2505 is done, it teaches `--template` sources (a folder,
    or `git+<url>@<rev>`) and shows the user a template's source and
    license before it passes `--yes`. If not, each adds its lines when
    it lands.
  - `shuyi-new-game` has a short section on sound effects (2225). A cue
    names a `.sfx.ron` file, `shuyi sfx <preset> --out <file>` writes
    one, `--wav` renders it to listen to, and these are the fields to
    change. Either order with 2225: if it isn't done when this runs,
    2225 adds the section.
  - **Either order with 2232:** if 2232 is done, `shuyi-new-game`
    says to run `shuyi map-types` after changing the game's things
    enum (the type its Tiled and LDtk objects read into); if not,
    nothing to do (2232 adds that line when it lands).
  - `shuyi-gates` starts from `cargo shuyi lint` (2534): one answer with
    every finding's code, file and line, from `check-keys`, `check-text`
    and the rules crate's fence, and `shuyi explain <code>` for each
    code's page. Then it names the rules crate's guards from 2015 that
    `lint` reads: `crates/rules/clippy.toml`, the
    `deny(clippy::float_arithmetic)` in `src/lib.rs` and
    `tests/layers.rs`. It says what each error means
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
    (`shuyi_core::autobalance` and the game's `Knob`s). **From 2541
    (done):** a game runs that search with `shuyi autobalance`
    (`--stage`, `--entrant`, `--budget`, `--time`, `--threads`,
    `--apply`) over its `autobalance_plan`, an `AutobalancePlan` whose
    stage is a `Tunable<S>` (the stage, its `Knob<S>`s, `play` and
    `write`), and writes no command, threads or time limit of its own.
    The grid template has no `autobalance_plan`, so the skill shows how
    to write one, and its worked example on a fresh game leaves
    `autobalance` out. Either order
    with 2112 and 2113: if they are done, `shuyi-test` shows how to
    prove a level winnable with `shuyi_core::search::solve` in a game's
    `check`, and `shuyi-balance-bots` shows the minimax and MCTS bots.
    If not, each of those tickets adds its section when it lands.
    `shuyi-balance-bots` also shows targets by tier
    (`shuyi_core::bots::playtest::Targets`) and personas as data
    (planner settings and weights: `shuyi_core::bots::weigh`), and
    `shuyi playtest`'s options (`--stage`, `--every-stage`, `--entrant`,
    `--threads`, `--report`) and a game's `PlaytestPlan`. Either order
    with 2131, 2132, 2533 and 2021: if they are done, it also shows the
    tactics kit's threat areas and candidate order, `shuyi playtest`'s
    history, and the reusable playtest workflow. If not, each adds its
    lines when it lands.
    Either order with 2544: if 2544 is done, every skill plays a game's
    bots through `playtest` (`bots` is gone); if not, a skill may show
    `bots` for a plain bot run, and 2544 moves it onto `playtest`.
  - Motion, either order with 2514: if 2514 is done, `shuyi-test`,
    `shuyi-animate` and `shuyi-cinematic` say to check motion with a
    sheet (`frame-png --every <seconds> --count <n>`, or the MCP
    `frame_png` with `every` and `count`), and `shuyi-ship` says how to
    make an APNG (`--apng`) for a README or a store page, and a GIF
    (`--gif`) once it exists (2514 may leave it to a ticket of its
    own). If 2514 isn't done, 2514 adds those lines.
  - `shuyi-cinematic` teaches the cinematic file as the engine has it
    when the skill is written. **Either order with 2364:** if 2364 is
    done, it also teaches keyed tracks, sound cues, markers and shot
    enters (`check_with`, `Tracked`, `play_cues`, `markers_crossed`),
    with them in its worked example; if not, nothing to do (2364 adds
    that recipe when it lands).
  - The live link, either order with 2521 (`shuyi attach`): if 2521 is
    done, `shuyi-test` gets a section on attaching to a running build
    (when to use it rather than the headless `run`, the `link` feature,
    `SHUYI_LINK`, the token, and `--screenshot`), with a worked example
    that CI can't run (it needs a window), marked as manual. If not,
    2521 adds that section.
  - The editor, either order with 2507 (parked): if 2507 is done,
    `shuyi-test` also gets a section on `shuyi edit` (the live view,
    the tree and the inspector over a running build). If not, 2507
    adds it.
  - Soak, either order with 2527: if `shuyi soak` exists, `shuyi-test`
    gets a short section on running it in CI and on turning a shrunk
    record into a regression test with `shuyi_core::soak::commands`,
    and `shuyi-debug-replay` says to run `soak` first when a save or a
    replay misbehaves (`replay` rebuilds the state by replaying, so it
    can't see what the serde form leaves out). If 2527 isn't done, 2527
    adds these.
  - Tick soak, either order with 2549 (parked): if 2549 is done,
    `shuyi-test`'s soak section also shows a tick game soaking its
    `TickSim` in its own tests (`shuyi_core::soak::tick::play` with
    `RandomInputs` over the game's actions) and a shrunk `TickRecord`
    kept as a regression test (`soak::tick::inputs`). If not, nothing
    to do: 2549 adds it when it lands.
  - Debug commands, either order with 2356: if 2356 is done,
    `shuyi-debug-replay` (and `shuyi-test`) tell agents to reach a bug's
    state with a debug command the game declares (`--command`), rather
    than a long key script. The command is a debug-only variant of the
    game's own command enum, never in `legal_commands`. If 2356 isn't
    done, 2356 adds that section.
  - Session records, either order with 2519: if 2519 is done,
    `shuyi-debug-replay` also reproduces a bug from a session record of
    a real build: `shuyi run --session <file>`, read
    `session.first_difference`, fix, then keep the file as a golden
    session. If not, the skill reproduces a bug from a rules record
    (`shuyi replay`), and 2519 adds that section when it lands.
  - Crash reports, either order with 2520: if 2520 is done,
    `shuyi-debug-replay` also starts from a player's crash report
    (3046): `shuyi crash <report> --symbols <file>` reads it,
    `shuyi replay --crash <report>` reproduces it (`ok: false` while it
    panics), and the skill loops fix, then replay, until `ok: true`. If
    not, its body says the crash path is still to come, and 2520 writes
    it when it lands.
  - A debug build's crash report naming its session, either order with
    3060: if 3060 is done, `shuyi-debug-replay` also says that such a
    report's `session` names a file in `<data dir>/sessions/`, which
    `shuyi run --session <file>` plays to the frame before the panic,
    screens and skins included, where a rules replay runs only the
    rules; MCP's `start` from it lets an agent `step` on. If not,
    nothing to do (3060 adds it when it lands).
  - `shuyi-ship` covers a game's notices and credits: the `NOTICES.txt`
    a build carries (3039, and 3004 for every package), the credits
    file, the `credits` part of `shuyi.ron` and `check`'s credits pass
    (2223), and that an agent never widens a game's `licenses` list to
    make a check pass (that is the game owner's call). It teaches what
    exists when the skill is written. **Either order with 3039, 3004
    and 2223:** if they are done, teach their parts here; if not, each
    adds its part when it lands: 3039 the web build's `NOTICES.txt`,
    3004 `shuyi export <target>` and the notices in every package, 2223
    the credits (each one's scope says so).
  - `shuyi-ship` teaches releasing a game from a tag if 3050 is done:
    `cargo shuyi export --plan`, `git tag -a vX.Y.Z -m <notes>`, the dry
    run from the Actions tab, and turning on itch.io with the
    `ITCH_GAME` variable and the `BUTLER_API_KEY` secret. If not, 3050
    adds that section.
  - Ease curves and tweens, either order with 2109: if 2109 is done,
    `shuyi-animate` has a section on them (`shuyi_core::ease`: `Ease`,
    `Tween`, `Lerp`, `finished_between`). If not, nothing to do: 2109
    adds the section when it lands.
  - Sequences of tweens and curves through points, either order with
    2135: if 2135 is done, that section names `Sequence` and `Points`
    too. If not, nothing to do: 2135 adds them to the section when it
    lands.
  - Level generators, either order with 2119 (parked): if 2119 is done,
    the `shuyi-procgen` skill (2118 adds it to this pack's list) names
    its generators (`caves`, `rooms`, `walk`), `regions`, `repair` and
    `WeightedTable`. If not, nothing to do: 2119 adds them to the skill
    when it lands.
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
  - `shuyi new` copies the skills into new projects. **From 2501
    (done):** they join `compose`'s plan (`shuyi_cli::template::compose`,
    called by `contract::new::plan`), read from the engine's `skills/`
    (not a second copy in the template), so every composed game has
    them and `new`'s answer lists them.
  - **From 2003 (done):** `shuyi add <module>` copies a module's skill
    into the game's `.claude/skills/<crate name>`, from the package's
    own `skill/` or else the pack's `skills/<crate name>`
    (`catalog::skill_of`), and only when the game has `.claude/skills/`
    (`catalog::SKILLS`, ADR-0041 §5). So `shuyi new` copies the pack
    into `.claude/skills/`. A module's skill (`shuyi-net`) is named
    after its crate and left out of what `shuyi new` copies: a game
    gets it from `shuyi add` (ADR-0009 §4). `shuyi-new-game` names
    `shuyi modules` and `shuyi add` for turning a module on.
- **Either order with 2802:** if 2802 is done, replace the line in the
  book's chapter 10 (*Tools for agents*) that says the skill pack is
  coming with a section on installing and using it; if not, nothing
  to do (2802 writes that section from this pack, as its chapter 10
  says).

**Out (do not do):**
- skills for our game (they stay in the game's repository);
- `shuyi-upgrade`, the skill that moves a game to a newer engine, and
  the `shuyi migrations` op it runs: 2523, after 2005 and 2804;
- `shuyi-stage-arcade`, the real-time grid skill: 2739 (parked) writes
  it in this pack's form;
- `shuyi-rts`, the real-time strategy skill: 2707 (parked) writes it
  in this pack's form, or adds it to the list above if this ticket
  isn't done. Either order with 2719: if 2719 is done, the skill's
  worked example reads `examples/rts` and it links
  `docs/guides/rts.md` (2719 adds that to its entry); if not, nothing
  to do (2719 points the skill at them). Either order with 2769: if
  2769 is done, the skill also says the module serves isometric and
  names the `isometric` stage of `examples/rts` (2769 adds that to its
  entry); if not, nothing to do (2769 adds it to the skill);
- rollback in `shuyi-net` (ADR-0005 §3's table gives the skill
  lockstep, rollback and sessions): 1147 writes the tickets that build
  rollback, and the rollback one adds that section;
- the agent eval: its tasks, grader and reference solutions (2524), and
  scheduled agent runs (2525).

## Implementation steps

1. Write `check-skills`, the worked-example runner, and their tests.
2. Write the skills and their worked examples, three or four per PR if
   the diff grows past one PR's worth (split into follow-up tickets).
3. Do the packaging, and the book's section if 2802 is done (above).

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
