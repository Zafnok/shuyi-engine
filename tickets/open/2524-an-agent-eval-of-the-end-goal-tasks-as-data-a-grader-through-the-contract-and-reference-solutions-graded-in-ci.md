---
id: "2524"
title: "An agent eval of the end goal: game tasks as data, a grader that checks any agent's game through the contract, and reference solutions graded in CI"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["1134", "2803"]
nick_input: none
completed:
---

# 2524 — An agent eval: tasks, a grader and reference solutions

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3.
Each of them ships tooling for coding agents: MCP servers, command-line
tools, instructions for agents. Defold's manual on AI agents describes
the loop: inspect, change, verify, then evaluate against acceptance
criteria, with a cap on repair attempts if wanted
(https://defold.com/manuals/ai-agents). None of them publishes a
measured result of an agent making a game. What engines track over time
is performance, like Bevy's compile times, sizes and benchmarks
(https://metrics.bevy.org).

Nick set the measure on 2026-10-19 ([`docs/decisions.md`](../../docs/decisions.md),
*the end goal*): an agent asked "make me a snake game with Shuyi Engine"
makes it, with a web build, a Playdate build and the other targets, in
under an hour and in few lines. "Every new API is judged against it."
On 2026-10-20 he parked real time until Visions of Shuyi launches, so
snake and a platformer wait, and until then the agent check of the end
goal measures turn-based games (the same file). Nothing checks any of it
today, so a change that makes the engine harder for an agent goes
unseen.

What the engine has (checked 2026-10-21):

- **The tool contract** (`crates/cli/src/contract.rs`, `VERSION` 4):
  `new`, `check`, `check --target`, `run`, `scene`, `replay`, `bots`,
  `import`, `bake` and `doctor`, each with `--json` answering an
  `Answer { contract, op, ok, result, error, limits }`, their schemas in
  `crates/cli/schemas/`. `cargo shuyi web` (`crates/cli/src/web.rs`)
  builds the folder that `shuyi.ron`'s `web` entry names (`web.dist`).
  In a game made by `shuyi new`, `cargo shuyi` is an alias for the
  game's own tool program, which calls `shuyi_cli`.
- **Scripts press keys, not actions.** `run` and `scene` play a
  `shuyi_cli::frame_png::Script` of `Step::Keys`, `Step::Pad` and
  `Step::Wait`, read by `Script::parse_step` from `--keys` (or
  `--script`), `--pad` and `--wait`. A grader can't know which keys an
  agent's game bound. `ActionSet::from_name`
  (`crates/assets/src/keymap/action_set.rs`) and `Keymap::primary` and
  `Keymap::primary_button` (`crates/ui/src/input/keymap.rs`) can find
  them, but no step uses them. 2521 plans an `act` step for its live
  link only.
- **The grid template** (`templates/grid`): a turn game whose tools
  (`crates/game/src/tools.rs`) give a session with a scene, `replay`,
  `bots` and the controls. Its actions are `Up`, `Down`, `Left`,
  `Right`, `Confirm` and `Cancel`. Its scene data already has a `turn`
  (a number). Its `shuyi.ron` lists the `desktop` and `web` targets but
  has no `web` entry, its game crate has no `build.rs` with the link
  argument the web shell needs (`crates/cli/web/README.md`), and its
  `rust-toolchain.toml` lists no `wasm32-unknown-unknown` target. So a
  web build costs an agent lines today, and the eval will put a number
  on it.
- **Tests of the tools, not of an agent.** 1134 (done) runs each
  skill's worked example, a script of contract operations
  (`crates/xtask/src/script.rs`), against a fresh `shuyi new` game in
  CI (`cargo xtask skill-examples`, the `skills` job). 1133 tests MCP with a scripted client. 2009 measures
  compile times, and 3008 run-time budgets.
- **Games to measure with.** 2803's finished tutorial
  (`site/tutorial/10-ship`) is a box-pushing puzzle made from the
  `blank` template, like every step of it, keeping the grid template's
  action names. Tic-tac-toe exists only as a test toy
  (`crates/core/src/bots/tests.rs`).

This ticket is the half that costs nothing to run: tasks as data, a
grader that checks any agent's game through the contract, and reference
solutions that CI grades on every pull request. A change that breaks a
reference fails its PR, and each reference's lines over the template are
the first numbers. 2525 runs a real agent on a schedule, which costs
money and waits for Nick.

Why it matters: the contract's runs and MCP sessions get `--act`, so
Visions of Shuyi's and every other game's scripts press actions by name.
And it is the only measure of the edge Nick chose for the engine.

## Nick input

None. Spending money on agent runs is 2525's question.

## Scope

**In:**
- **Actions by name in the contract's scripts** (`AGENTS.md` rule 8):
  - `Step::Act(String)` in `crates/cli/src/frame_png.rs`: an action by
    its `ActionSet::name`. `Step::check` accepts any name that isn't
    empty, since the game's actions are known only when it runs.
    `Script::play` looks the name up with `ActionSet::from_name` over
    the Harness's game (`G::Action`), and presses and releases its
    primary key in the player's keymap (`Keymap::primary` on
    `Harness::ctx().keymap`), or its primary button
    (`Keymap::primary_button`) when it has no key. The `Display` of
    `Chord` and of `ButtonChord` gives the names `Harness::keys` and
    `Harness::pad` take. A name the game has no key or button for
    panics there with the message below, as the Harness does on a key
    name it doesn't know; `run`, `scene` and MCP's `step` refuse it
    first, so only `frame-png`, which has no session to ask, can reach
    that panic.
  - `Session::actions(&self) -> Vec<String>`
    (`crates/cli/src/contract/session.rs`): the game's actions that have
    a key or a button, in `ActionSet::ALL` order. It is a default method
    answering an empty list, so no `Session` a game wrote itself breaks.
    `Headless` (from its Harness's keymap) and the test fake
    (`crates/cli/src/contract/fake.rs`) implement it.
  - `run` and `scene` refuse a script that names an action not in that
    list, in `played` (`crates/cli/src/contract/run.rs`) after the
    session starts and before it plays, and name the list: "act: no
    action `Jump` with a key or a button; the game's are Up, Down, Left,
    Right, Confirm, Cancel".
  - `Script::parse_step` reads `--act <Action>`. `run::parse_args` and
    `frame_png::parse_args` add `--act` to the step flags they match,
    and `run::usage` and `frame-png`'s usage list it, so `run`, `scene`
    and `frame-png` take it.
  - Change the contract as `docs/contract.md` says (regenerate
    `crates/cli/schemas/`, raise `contract::VERSION` by one, add the
    line to `versions.json`), and add `--act` to `docs/contract.md`'s
    table. The grid and blank templates' `AGENTS.md` tables show
    `run`, `scene` and `frame-png` with `--act` and their own action
    names in place of `--script` and key names.
  - The PR says the contract changed, that `Session` gained a default
    method, and that `Step` gained a variant: a game's exhaustive
    `match` on `Step` needs an arm for it (a `Breaking:` line, 2010,
    if `Step` isn't `#[non_exhaustive]` by then).
  - **MCP** (1133, done): its `step` tool plays the contract's
    `Script`, so it gets `act` with it. But the host's `step`
    (`crates/cli/src/mcp/host.rs`) calls `Session::play` itself, not
    through `played`, so the action check goes in one function both
    call. An MCP `step` naming an action the game lacks is then refused
    with `SH0001` and the list, and the session keeps running; without
    it the panic ends the game's tool program and the session
    (`SH3005`).
  - **Either order with 2521:** if 2521 is done, `attach` reads
    `--act` itself and looks actions up with
    `shuyi_platform::link::press_for`: move that `--act` into
    `Script::parse_step` and play `Step::Act` with `press_for`, so there
    is one parser and one lookup (2521's clause says so). If not, 2521
    reads `--act` through this ticket's `Script::parse_step`, and moves
    the lookup by name into `press_for`.
- **Tasks as data:** `evals/tasks/<id>.ron`, read by
  `crates/xtask/src/eval/task.rs` with `serde` and `ron`
  (`deny_unknown_fields`). These are the repository's tooling files, not
  a game's assets, so they don't go through the format registry
  (ADR-0007 is about a game's assets). The xtask already has `serde`
  (with `derive`), `serde_json` and `ron` (with `std`), from 2015.
  Fields:
  - `id`;
  - `prompt`: the words the agent gets, verbatim, and nothing else;
  - `template`: the template the agent's project starts from (`"grid"`),
    which is also the base the lines are counted over;
  - `name`: the game's name `shuyi new --name` gives the graded project
    and its base, so the names the template fills in count as no lines;
  - `time_box_minutes`: 60, the end goal's hour (2525 enforces it);
  - `targets`: the target ids `Targets` checks (`desktop`, `web`);
  - `checks`: below. Their steps are `Act(name)` and `Wait(seconds)`,
    the grader's own type, passed to the contract as `--act` and
    `--wait`. Never keys;
  - `reference`: `Overlay(path)`, a folder holding only the files that
    differ from a fresh `shuyi new <template>`, laid over one;
    `Project(path)`, a whole project graded where it is; or `None`;
  - `waits_for`: ticket ids; a task with any is listed but never graded;
  - `unmeasured`: parts of the end goal no check covers yet, each
    `(what, waits_for)`.
- **Checks.** Each gives one verdict (`Gates` one per command), and each
  goes through the contract or cargo, run in the project's folder:
  - `Gates`: the gates the template's `AGENTS.md` lists besides
    `check`: `cargo fmt --all --check`,
    `cargo clippy --workspace --all-targets -- -D warnings`,
    `cargo test --workspace` and `cargo shuyi check-keys`;
  - `Check`: `cargo shuyi check --json` passes;
  - `Targets`: `cargo shuyi check --target <id> --json` passes for each
    id in `targets`;
  - `Web`: `cargo shuyi web --release` exits 0, and the folder that
    `shuyi.ron`'s `web.dist` names holds `index.html` and one `.wasm`.
    **Either order with 3004:** if 3004 is done, `Web` runs `cargo
    shuyi export web` and reads its folder instead, and the turn-based
    tasks gain an `Export` check for `linux` (the package exists), run
    in the sniper container as CI's `linux` export is, since outside it
    3004's audit refuses a newer glibc; if not, nothing to do (3004's
    PR does both);
  - `Responds { steps }`: `run --json` with the steps answers `ok`, and
    its `frame` or its `scene` differs from a `run --frames <n> --json`
    with no input, where `n` is the first answer's `frames`. A panic in
    the game's tool program fails it, with the panic's message;
  - `Replay`: `bots 1 --json` answers a play with a `record`; the grader
    writes it to a file in a temporary folder of its own and runs
    `replay <that file's absolute path> --json`, which passes when the
    record run twice gives equal hashes (ADR-0022). `replay` joins the
    path to the project's root, so an absolute path is read as it is.
    **Either order with 2544:** if 2544 is done, `bots` is folded into
    `playtest`: run `playtest --tries 1 --json`, read its first try's
    `record`, and read the `bots` operation in this ticket (the
    prompt's too) as `playtest`; if not, use `bots` as written, and
    2544 moves the eval onto `playtest` when it lands;
  - `Scene { steps, has }`: `scene --json` after the steps has a scene,
    and each `(pointer, equals)` in `has` holds in `result.scene.data`:
    a value exists at that JSON pointer, and equals `equals` when one is
    given (a string compared without its quotes, anything else as JSON
    text).

  A check reads only the contract's answers and what the prompt asks the
  game to expose: its action names and its scene data. Never a
  reference's files, types or screens, so any agent's game can pass.
- **The grader:** `cargo xtask eval-grade <task.ron> <project> [--base
  <dir>] [--out <file>]` (`crates/xtask/src/eval/grade.rs`):
  - runs every check through 1134's worked-example runner,
    `script::run` in `crates/xtask/src/script.rs` (Implementation
    steps, 1), and goes on after a failure;
  - records for each check its kind and place in the task (`Scene 2`),
    whether it passed, the seconds it took, and a detail: the answer's
    `error`, the last 20 lines of a failed command's output, or what was
    equal that should have differed;
  - counts lines when `--base` names a folder made by
    `shuyi new <template> --name <name>`: `git diff --no-index
    --no-renames --numstat <base> <project>`, dropping from its output
    the paths under `target/`, `dist/`, `.git/` and the `web.dist`
    folder, and `Cargo.lock`; a binary file counts as a file changed
    only. Without `--base` there is no count, and a limit says so;
  - writes `--out` (by default `eval-result.json` in the current
    folder): `task`, `engine` (this checkout's `git rev-parse HEAD`),
    `checks`, `passed`, `total`, `lines_added`, `lines_removed`,
    `files_changed`, `seconds`, `unmeasured` and `limits`;
  - exits 0 when every check passed, 1 when one failed, and 2 when it
    couldn't grade (no such project, a bad task file, a task that
    waits);
  - reads the answers as `serde_json::Value` (`ok`, `error`,
    `result.frames`, `result.frame`, `result.scene.data`,
    `result.plays[0].record`), so `xtask` doesn't depend on `shuyi-cli`
    and the `tickets` job stays fast. A test checks those paths against
    `crates/cli/schemas/`, so a contract change that moves one fails
    here.
- **References:** `cargo xtask eval-reference (<id> | --all) [--out
  <dir>] [--template-only]` (`crates/xtask/src/eval/reference.rs`):
  - makes two games with `cargo run -q -p shuyi-cli --locked -- new
    <template> <dir> --name <name> --engine-path <this checkout>`, as
    `ci.yml`'s `templates` job does: `<tmp>/grade/<id>` to grade and
    `<tmp>/base/<id>` as the base;
  - lays an `Overlay` over the first, or takes a `Project` where it is,
    then grades it with `--base` and writes `<out>/<id>.json` (`--out`
    defaults to `eval-results`);
  - refuses an overlay that holds a `Cargo.toml` naming the engine's
    crates: their paths come from `--engine-path`;
  - with `--template-only`, grades the plain template, to show that a
    task's checks can fail;
  - with `--all`, grades every task that has a reference, lists the ones
    that wait, and warns when an id in a `waits_for` is in
    `tickets/done/` ("1127 is done: give `platformer` a reference and
    empty its `waits_for`").
- **The first tasks**, in `evals/tasks/`:
  - **`tic-tac-toe.ron`:** template `grid`, name `Tic-tac-toe`, targets
    `desktop` and `web`, reference
    `Overlay("evals/references/tic-tac-toe")`. Checks: `Gates`, `Check`,
    `Targets`, `Web`, `Responds { steps: [Act("Right"),
    Act("Confirm")] }`, `Replay`, `Scene { steps: [Act("Confirm")],
    has: [("/turn", Some("o")), ("/winner", Some(""))] }`, and a won
    game, x taking the middle, the top-left and the bottom-right cells:
    `Scene { steps: [Act("Confirm"), Act("Up"), Act("Confirm"),
    Act("Left"), Act("Confirm"), Act("Right"), Act("Right"),
    Act("Confirm"), Act("Down"), Act("Down"), Act("Confirm")], has:
    [("/winner", Some("x"))] }`.
  - **`box-pushing.ron`:** template `blank` (2803's steps are made
    from it), the name 2803's step 10 was
    made with, the same targets, reference
    `Project("site/tutorial/10-ship")` (2803). Its prompt asks for 2803's
    game in the words of the tic-tac-toe prompt below. Checks: `Gates`,
    `Check`, `Targets`, `Web`, `Responds { steps: [Act("Right")] }`,
    `Replay`, and `Scene { steps: <to level 1's stage>, has:
    [("/moves", Some("0")), ("/won", Some("false"))] }`, where the steps
    are `[]` if the game opens on that stage and `[Act("Confirm")]` if
    it opens on its level menu.
  - **`snake.ron`**, `waits_for: ["2739"]` (which builds on 1119), and
    **`platformer.ron`**, `waits_for: ["1127"]`: the end goal's own
    games. Their prompts and checks (a length, a score or a level in the
    scene data) are written now, with no reference, so they are on
    record. 2739 and 1127 give them their references and empty their
    `waits_for` when real time is unparked. **Either order with 2739:**
    if 2739 is done first, it has already written a snake task; keep
    it, and add only the platformer's.
    **Either order with 1127:** if 1127 is done, `platformer.ron` gets
    `reference: Project("examples/platformer")` and no `waits_for`, the
    example's scene data answers its `Scene` checks (the level and the
    coins its prompt names; add them to the example if it lacks them),
    and `cargo xtask eval-reference platformer` passes. If not, nothing
    to do (1127 does it).
  - **Either order with 2735, 2736, 2737 and 2738** (parked: the
    roguelike, the match-3, the card battle and the brick-breaker): if
    one is done, it added its task here: write that task file too,
    with its example as its `Project` reference. If not, nothing to
    do: each one writes its own task when it lands.
  - Every task has `unmeasured: [(what: "a Playdate build", waits_for:
    "3010")]`. **Either order with 3010:** if 3010 is done, the
    turn-based tasks whose references pass `cargo shuyi check --target
    playdate` list `playdate` in their `targets` instead and have no
    Playdate entry; a reference that doesn't pass keeps the entry, and
    gets a ticket for what stops it; if not, nothing to do (3010's PR
    adds `playdate` and drops the entry).
- **The tic-tac-toe prompt**, as a draft. The session may change the
  words, never the facts the checks read:

  > Make me a tic-tac-toe game with Shuyi Engine, with a web build. This
  > folder holds a new game made with `shuyi new grid`: turn it into
  > tic-tac-toe for two players on one keyboard or pad. X goes first.
  > Keep the actions named `Up`, `Down`, `Left`, `Right` (move the
  > cursor) and `Confirm` (put a mark where the cursor is). The cursor
  > starts on the middle cell, stops at the edges, and stays where it is
  > after a mark. `cargo shuyi scene --json` shows the board, which
  > stays on screen when the game ends, and the scene's data has `turn`
  > (`"x"` or `"o"`) and `winner` (`""` until the game ends, then `"x"`,
  > `"o"` or `"draw"`). Done means: the gates in `AGENTS.md` pass,
  > `cargo shuyi check --target all` passes, `cargo shuyi web --release`
  > builds, and `cargo shuyi bots 5` plays it, with records that
  > `cargo shuyi replay` replays.
- **The tic-tac-toe reference**, `evals/references/tic-tac-toe/`: only
  the files that differ from `shuyi new grid --name Tic-tac-toe`. The
  rules over `TurnRules` in `crates/rules/src/lib.rs` (so `bots` and
  `replay` work as the template's do), the stage screen and its scene
  data, the tools' bot, the template's tests changed to fit,
  `shuyi.ron` with a `web` entry, `crates/game/build.rs`, and
  `rust-toolchain.toml` with the `wasm32-unknown-unknown` target.
  Written as an agent should write it: the least change to the template
  that passes. Its lines over the template are the number the PR
  reports. The task files and this reference are test data, like
  fixtures.
- **`evals/README.md`:** what the eval is and the end goal it measures
  (quoting `docs/decisions.md`), how to add a task, how to grade a
  reference, and the rule that a check reads only the contract.
- **CI** (`.github/workflows/ci.yml`): a job `eval`, after `changes` and
  only on code changes, on `ubuntu-latest`, installing the macroquad
  libraries as the `wasm` job does, with `CARGO_TARGET_DIR` shared as in
  the `templates` job:
  - `cargo xtask eval-reference --all --out eval-results` must pass;
  - `cargo xtask eval-reference tic-tac-toe --template-only --out
    eval-template` must fail, and the step checks with `jq` in
    `eval-template/tic-tac-toe.json` that `Check` passed and that every
    `Scene` check failed;
  - it uploads `eval-results/` as an artifact and writes a table (task,
    checks passed, lines added, seconds) into the step summary;
  - `ci-result` needs it.
- **Size:** if the diff passes about 600 changed lines (the task files,
  the references, the schemas and the fixtures aside), the actions by
  name (`Step::Act`, `Session::actions`, `--act` and the contract
  change) go to a ticket of their own (`write-ticket`), After MVP,
  landed first as its own PR; this ticket then waits for it, and
  2521's either-order clause names it. The PR says so.

**Out (do not do):**
- running an agent, a schedule, tokens, cost and minutes, a results page
  (2525);
- references for snake and the platformer: `examples/snake` (2739) and
  `examples/platformer` (1127) become them when those tickets land
  (if 1127 is done first, the platformer's is set here, as above);
- a shortcut for `Gates` on a reference inside the engine's own
  workspace (the first is 2739's or 1127's): there they run over the
  whole workspace;
- the Playdate build (3010) and `shuyi export` checks (3004), beyond the
  either-order clauses with them above;
- the lines a template hands its author (2512);
- grading by a model (fun, looks): every verdict comes from the
  contract or cargo;
- a gate on new APIs: "every new API is judged against it" stays a
  convention until 2525 has a baseline;
- changing the grid template so the references get shorter (a `web`
  entry, a `build.rs`, the wasm target): a ticket of its own, if the
  numbers say so;
- playing session records back (2518).

## Implementation steps

1. Read 1134's completion notes and its worked-example runner,
   `crates/xtask/src/script.rs`. An eval check is what a worked example
   is: a contract command, and what its answer must say. Run the checks
   through that runner (`script::run`: the steps in, a `Verdict` per
   step out). It already gives each step's time, goes on after a
   failure, checks the answer at JSON pointers (`Expect::Is`, `Has`
   and `Holds`; `Is` compares a string without its quotes, as `has`
   does) and writes a value of an answer to a file (`save`, for
   `Replay`'s record). Add to it what the eval needs and it lacks
   (comparing two answers for `Responds`), so there is one runner, not
   two.
2. Add `Step::Act`, `Session::actions` and `--act`, with their tests.
   Change the contract as `docs/contract.md` says, and update its table
   and the templates' `AGENTS.md` tables.
3. Write `task.rs` and the four task files.
4. Write `grade.rs`, with the line count and the result file, and wire
   `eval-grade` and `eval-reference` into `crates/xtask/src/main.rs`,
   its usage text and the package's `description`.
5. Write `reference.rs` and the tic-tac-toe reference. Read 2803's step
   10: the name it was made with, the screen it opens on, its scene data
   and its action names. Write the box-pushing prompt and its `Scene`
   steps from them. If it doesn't expose `/moves` and `/won` in its
   scene data or doesn't keep the template's action names, add them to
   the tutorial's steps in this PR.
6. Add the `eval` job and `evals/README.md`, and run the gates.

## Acceptance criteria

- [ ] `Step::Act("Right")` played in a Harness gives the same frames as
      `Step::Keys` of that action's primary key, and an action that has
      only a button is pressed by that button (tests).
- [ ] `run` and `scene` with `--act Jump` are refused with the game's
      action names (a test over the fake session), and
      `cargo shuyi run --act Right --act Confirm --json` plays on a game
      from `shuyi new grid` (the `eval` job's `Responds` check).
- [ ] MCP's `step` with an `act` step naming `Jump` is refused with
      `SH0001` and the game's action names, and the session keeps
      running (a test in `crates/cli/src/mcp/host/tests.rs`).
- [ ] The contract's version is raised and the schemas regenerated (the
      schemas' test passes).
- [ ] `cargo xtask eval-reference --all` passes on tic-tac-toe and
      box-pushing in CI, and on snake or the platformer if 2739 or 1127
      is done; it lists the rest as waiting.
- [ ] `--template-only` on tic-tac-toe fails every `Scene` check and
      passes `Check`.
- [ ] The result file has every field above, and the PR description
      has each reference's lines added and seconds.
- [ ] A task file with an unknown field, and an overlay with an engine
      `Cargo.toml`, are refused with exit code 2.
- [ ] `grade.rs` reads no file of the project but `shuyi.ron` (for
      `web.dist`) and that folder's listing, and writes nothing into the
      project itself; every other input is a command's answer.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`xtask`): task parsing, each check, unknown fields refused;
  `has` matching (present, equal, a string without quotes, missing);
  the `--numstat` parsing and its exclusions; the waiting warning
  against a fake `tickets/done/`; the answer paths against
  `crates/cli/schemas/`.
- Unit (`shuyi-cli`): `Step::Act` with a key, with only a button, and
  with an unknown name; `Session::actions` on `Headless` and its
  default; `run`, `scene` and MCP's `step` refusing an unknown action;
  `Script::parse_step`, `run::parse_args` and `frame_png::parse_args`
  with `--act`; the `Display` of `Chord` and `ButtonChord` read back by
  `Chord::parse` and `ButtonChord::parse`.
- Integration: the `eval` CI job, both ways (the references pass, the
  plain template fails).

## Completion notes

*(Filled in by the session that completes the ticket.)*
