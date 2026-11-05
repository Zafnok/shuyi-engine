---
id: "2566"
title: "One input script: the steps the Harness, the contract and the live link play, in shuyi-platform, with actions by name"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-04
---

# 2566 — One input script: the steps the Harness, the contract and the live link play

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-27). Three types say "a script of input steps", and a fourth
place plays them by hand:

- **`shuyi_cli::frame_png::{Script, Step}`** (`crates/cli/src/frame_png.rs`):
  `Keys(String)`, `Pad(String)`, `Wait(f32)` in **seconds**. `run`,
  `scene`, `frame-png`, the MCP session's `step` and every game's tool
  program use it (`GameTools::frame`, `Session::play`). It has no step
  that presses an action by name.
- **`shuyi_platform::link::Step`** (`crates/platform/src/link.rs`, from
  2521; ADR-0047 §7): `Keys`, `Pad`, `Act(String)`, `Wait(u32)` in
  **frames**. Its `plan` and `press_keys`/`press_pad` turn the steps into
  frames of `RawInputEvent`s, a second copy of what `Harness::keys` and
  `Harness::pad` do (`crates/platform/src/harness.rs`).
- **`shuyi_cli::contract::attach::AttachStep`**
  (`crates/cli/src/contract/attach.rs`): `Keys`, `Pad`, `Wait(f32)`,
  `Act`, the bridge between the two above (`AttachStep::of`,
  `to_link`, `frames_of`). `attach::parse_step` reads `--act` itself,
  beside `Script::parse_step`.

What this costs:

- A running build can be told "press Confirm"; a headless session can't,
  so an agent that plays the headless game names keys (`AGENTS.md`
  rule 8 asks for actions).
- Each new kind of step is written twice or more. Four open tickets add
  one: 2540 (`Crank`), 2353 (`Lang`), 2356 (`Command`, plus a request of
  its own on the link) and 2524 (`Act` on the contract's `Step`, and
  `attach` moved onto it).
- The two waits differ in unit, so `attach` converts and both ends check
  a different limit.

2524 already plans part of this ("let `attach` take the contract's
`Step` instead of its `AttachStep`, so there is one parser and one
lookup"), but 2524 is the agent eval and waits for the tutorial (2803).
This ticket does that part now and goes further: one type for the
Harness, the contract and the link. 2524 is blocked by this ticket and
keeps the rest of its scope.

## Nick input

None.

## Scope

**In:**
- **`shuyi_platform::script`** (`crates/platform/src/script.rs`, new):
  - `Step`, `#[non_exhaustive]` (ADR-0039): `Keys(String)`,
    `Pad(String)`, `Act(String)`, `Wait(f32)` in seconds. Serde form as
    the contract's today (`rename_all = "kebab-case"`: `{"keys": "Enter
    Up"}`, `{"pad": "South+East"}`, `{"act": "Confirm"}`, `{"wait":
    0.5}`).
  - `Script { steps: Vec<Step> }`, `#[non_exhaustive]`, with `new`,
    `check` and, with the `harness` feature, `play(&self, h: &mut
    Harness<G>)`: the same names the game calls today
    (`script.play(&mut h)`, `Script::new`).
  - `Step::check`: names `Chord::parse` and `Button::parse` know, an
    action name that isn't empty, a wait that is finite and not
    negative. The longest wait is the player's to check, not the step's:
    `harness::MAX_WAIT` (seconds, today's `frame_png::MAX_WAIT`, moved)
    and `link::MAX_WAIT` (now seconds: `1_800.0`, half an hour).
  - `press_for` moves here from `link` (one lookup of an action by name:
    `ActionSet::from_name`, then `Keymap::primary`, else
    `Keymap::primary_button`), with `Press`.
  - `frames(steps, keymap, pad) -> Result<Vec<Planned>, String>`: the
    one plan, `Planned::Events(Vec<RawInputEvent>)` (one frame) or
    `Planned::Idle(u32)` (that many frames with no input), each frame
    1/60 s. An error names the step (`step 3: act: …`). The Harness,
    the link and 2018's capture run all play it.
  - **One rule for a wait**, `wait_frames(seconds) -> u32`: the seconds
    at 60 frames a second, rounded up, so a wait above 0 is at least
    one frame and a wait of 0 is none. A product within a thousandth of
    a whole number counts as that number (0.5 s is 30 frames, 1/60 s
    is 1). It replaces `attach::frames_of`, which rounds to the
    nearest, and the shorter last frame `Script::play` runs today
    through `Harness::wait`: only a script's wait that isn't whole
    frames, or is 0 (one frame of no time today), can change a
    `frame-png` picture. `Harness::wait`, which a game's tests call,
    keeps its seconds.
  - Serde behind a new feature `script` (`dep:serde`), which `link`
    turns on; `JsonSchema` behind a new feature `schema`
    (`["script", "dep:schemars"]`), which `shuyi-cli` turns on. Add
    `schemars` to `shuyi-platform`'s `outside` list in
    `tools/layers.ron`, with its reason, as `shuyi-ui`'s has it.
- **The Harness** plays a `Script` through the plan: `Harness::keys` and
  `Harness::pad` are built on the plan's helpers too, so a key press is
  written once. `Act` presses the action's primary key or button in the
  game's keymap; a name the game has no key or button for panics, as a
  key name the Harness doesn't know does today.
- **The live link:** `Request::Play { steps: Vec<script::Step> }`.
  `link::Step`, `link::plan`, `press_keys` and `press_pad` go; the
  server plays the plan, a `Planned::Idle(n)` as `n` of the build's
  frames (the waits on the wire stay seconds). Raise `link::VERSION` by
  one.
- **The contract** (`crates/cli`):
  - `frame_png` re-exports `shuyi_platform::script::{Script, Step}`, so
    `shuyi_cli::frame_png::Script` still names the type. Its own `Step`,
    `Script` and `MAX_WAIT` go.
  - `Script::parse_step` becomes `frame_png::parse_step(flag, text)`
    (an inherent method can't stay on a type of another crate), and
    reads `--act <Action>` too. `frame_png::parse_args`,
    `run::parse_args` and `attach` call it; `frame-png --help` and
    `run`'s usage list `--act`.
  - `AttachStep`, `AttachStep::of`, `to_link` and `attach::parse_step`
    go: `AttachInput::steps` is a `Vec<Step>`.
  - `Session::actions(&self) -> Vec<String>`, a default method answering
    an empty list: the game's actions that have a key or a button, in
    `ActionSet::ALL` order. `Headless` (from its Harness's keymap) and
    `contract/fake.rs` implement it. `run`, `scene` and MCP's `step`
    refuse a script that names an action not in the list before it
    plays, naming the list ("act: no action `Jump` with a key or a
    button; the game's are Up, Down, Left, Right, Confirm, Cancel"). The
    check is one function `played` (`contract/run.rs`) and the MCP host's
    `step` (`crates/cli/src/mcp/host.rs`) both call, so an MCP `step`
    with a bad name answers `SH0001` and the session keeps running.
  - Regenerate `crates/cli/schemas/`, raise `contract::VERSION` by one,
    add its line to `crates/cli/schemas/versions.json`
    (`docs/contract.md`, *The schemas*). `attach.input.json` keeps its
    shape (`AttachStep` becomes `Step`, the same JSON).
- **Docs:** ADR-0047 §7 amended (the `write-adr` skill): waits on the
  wire in seconds, played as the build's frames at 60 a second, rounded
  up (`script::wait_frames`), and the steps are
  `shuyi_platform::script::Step`. `docs/contract.md` (its
  steps, `--act`, the MCP `step` row's "waits in the build's frames")
  and the grid and blank templates' `AGENTS.md` tables (`--act` with
  their own action names) through the `document` skill.
- **From 2540 (done):** `Step::Crank(f32)` is in `frame_png::Step`
  (played with `Harness::crank`, read from `--crank`), and
  `AttachStep::of` refuses it ("crank: not over the live link yet").
  Move it into `shuyi_platform::script::Step` with the others, and make
  the link's plan (`script::frames`) refuse it the same way.
- **Either order with 2353:** if 2353 is done, move `Step::Lang` into
  `shuyi_platform::script::Step`, played with `Harness::set_lang`; the
  link's plan refuses it ("lang: not over the live link yet"); if not,
  nothing to do (2353 adds it to `shuyi_platform::script::Step`).
- **Either order with 2356:** if 2356 is done, move `Step::Command` into
  `shuyi_platform::script::Step`, played with `Harness::command`; the
  link's plan refuses it and names the link's own `command` request; if
  not, nothing to do (2356 adds it to `shuyi_platform::script::Step`).
- **Either order with 2556:** if 2556 is done, `frame_png::parse_args`
  already hands `--script` to the step parser: keep it when it calls
  `frame_png::parse_step`; if not, nothing to do (2556 hands `--script`
  to `frame_png::parse_step`).
- **From 2018 (done):** fold its `capture::Step` (`keys`, `pad`,
  `wait`, `capture`, in `crates/platform/src/capture.rs`) into
  `script::Step`. A plan's steps become `capture::PlanStep`s,
  `Input(script::Step)` or `Capture(String)` (`#[non_exhaustive]`),
  read from the same RON (`act("Confirm")` too); `Plan::parse` checks
  each input step with `Step::check`. `Plan::frames` takes the keymap
  (`Plan::frames(&self, keymap) -> Result<Vec<PlannedFrame>, String>`)
  and builds on `frames`, a `Planned::Idle(n)` as `n` frames with no
  events. `capture::wait_frames` and `capture::FRAME_DT` are the wait
  rule above: one copy, in `script`. The key and pad helpers 2018
  moved out of `link` (the crate-private `press::keys`, `press::pad`
  and `press::buttons` in `crates/platform/src/press.rs`, behind `link`
  or `capture`) become the plan's helpers, and `link::press_keys` and
  `link::press_pad` go with `link::plan`. `Plan`'s fields are private
  and `Plan::parse` alone builds one, so only `Plan::steps`, `Step` and
  `Plan::frames` change: a `Breaking:` line for `shuyi-platform`.
- **Either order with 2029:** if 2029 is done, its session form of
  `capture::Plan` stays as it is: only the step form's steps move (the
  part from 2018 above); if not, nothing to do (2029 adds its session
  form beside the plan's steps as they are then).
- **From 2019 (done):** its web capture run plays 2018's plan through
  `Plan::frames` (`crates/app/src/capture.rs`, the same code on the
  web), so nothing changes there beyond the part from 2018 above.
- **Either order with 2514:** if 2514 is done, its `Script::then_frames`
  (an inherent method of the type that becomes a re-export) becomes the
  free function `frame_png::then_frames(script: &Script, n: u32) ->
  Script`, and its test that `n` waits of `FRAME_DT` run exactly `n`
  more frames passes under `wait_frames`; if not, nothing to do (2514
  writes the free function).
- **Either order with 3008:** if 3008 is done, the inputs of its
  `.bench.ron` scenarios move onto `script::Step` with the contract's
  steps; if not, nothing to do (3008 writes its scenarios' inputs as a
  `script::Script`).
- **Either order with 2569:** if 2569 is done, `shuyi-platform` already
  has a `schema` feature (`["dep:schemars"]`, turned on by `shuyi-cli`)
  and `schemars` in its `outside` list in `tools/layers.ron`, with
  `link::LogEntry` deriving `JsonSchema` behind it: add `script` to that
  feature and derive `JsonSchema` on the script's types behind it, and
  leave the layers line as it is; if not, nothing to do (this ticket
  adds the feature as above, and 2569 derives `JsonSchema` on
  `link::LogEntry` behind it).
- **For the game:** an API break, which CI's `api` job reports and the
  completion notes carry as `Breaking:` lines (ADR-0038 §6):
  `frame_png::Step` gains `Act` and is `shuyi_platform::script::Step`
  (a game's exhaustive `match` adds an arm); `Script::parse_step` is
  `frame_png::parse_step`; `link::Step` and `AttachStep` are gone. The
  game calls `Script::new`, `frame_png::Step::Keys` and `script.play(&mut
  h)` (its `crates/xtask/src/game_tools.rs`), which keep compiling. The
  game's PR that moves the pin follows this one.

**Out (do not do):**
- The rest of 2524 (tasks as data, the grader, reference solutions):
  2524 is blocked by this ticket.
- New kinds of step beyond `Act`: each is its own ticket's (2540, 2353,
  2356, above).
- Holding a key (`Harness::hold`, `hold_pad`) or typing (`type_text`)
  as script steps.
- The xtask's own script runner (`crates/xtask/src/script.rs`), whose
  `Step` is a script of contract commands, not of input.

## Implementation steps

1. Write `crates/platform/src/script.rs`: `Step`, `Script`, `check`,
   `press_for` and `Press` (moved from `link.rs`), `Planned` and
   `frames`, with the features above. Add the features to
   `crates/platform/Cargo.toml` and `schemars` to `tools/layers.ron`.
2. Build `Harness::keys`, `Harness::pad` and `Script::play` on the
   plan's helpers; move `MAX_WAIT` into `harness`.
3. Move the link onto `script::Step` and the plan; delete `link::Step`,
   `plan`, `press_keys`, `press_pad`; `link::MAX_WAIT` in seconds; raise
   `link::VERSION`. Fix `crates/app/src/link/tests.rs` and the toy's
   link host (`crates/cli/src/toy/link.rs`). Move 2018's capture plans
   onto `script::Step` as the part from 2018 says, and fix
   `crates/app/src/capture.rs` and Hello, Grid's plan if it changes.
4. In `crates/cli`: the re-export in `frame_png`, `frame_png::parse_step`
   with `--act`, `AttachStep` removed, `Session::actions`, the action
   check before `run`, `scene` and MCP `step` play, the fake.
5. Regenerate the schemas, raise `contract::VERSION`, add the
   `versions.json` line.
6. Amend ADR-0047 §7; update `docs/contract.md` and the templates'
   `AGENTS.md` tables; `cargo xtask skill-examples` for any skill whose
   worked example plays a script.
7. Settle the either-order clauses above with what is done.
8. Run the gates.

## Acceptance criteria

- [x] One type of input step in the engine:
      `grep -rn "enum Step" crates/cli crates/platform` finds only
      `shuyi_platform::script::Step` (and the dialogue's and text box's,
      which are not input; and `export::playdate::Step`, a build step of
      the Playdate exporter). 2018's `capture::Step` is gone too: a
      plan's steps are `capture::PlanStep`s over `script::Step`.
- [x] `RawInputEvent::Down` for a script's key is built in one place:
      the link and the Harness both play `script::frames`.
- [x] `cargo shuyi run --act Confirm` and `scene --act Confirm` on a
      game made with `shuyi new --template grid` press its Confirm key;
      `--act Jump` is refused before anything plays, naming the game's
      actions.
- [x] `frame-png --act Confirm` draws the same frame as `--keys` with
      Confirm's primary key.
- [x] `shuyi attach --act Confirm --wait 0.5` plays the same frames as
      before on the toy's link host (`act`, then 30 frames).
- [x] An MCP `step` naming an action the game lacks answers `SH0001` and
      the session stays up.
- [x] `contract::VERSION` and `link::VERSION` are each one higher; the
      schemas are regenerated; `attach.input.json` describes the same
      JSON as before (and lists `crank` too, which `attach` refuses).
- [x] `cargo xtask check-layers` passes with `schemars` in
      `shuyi-platform`'s list.
- [x] Each either-order clause above is settled in the PR.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/platform/src/script.rs`): `check` on every kind of bad
  step; `frames` of keys, buttons together, an action by key, an action
  by button, a wait, and a wait of 0 (no frame); `wait_frames` of 0,
  1/60, 0.5 and 0.51 s (0, 1, 30 and 31 frames); an unknown action's
  error names its step.
- Property: for random scripts of keys, buttons and waits, the Harness
  playing `Script::play` records the same input frames as the link's
  server playing the same steps over the toy (the link's tests already
  play random scripts; point them at the shared plan).
- Unit (`crates/cli`): `parse_step` reads `--act`; `run` refuses an
  unknown action with the list; `attach`'s input reads `--act`, `--keys`,
  `--pad` and `--wait` into `Step`s.
- Integration: the MCP session test plays `{"act": "Confirm"}` on the
  toy; `crates/app/src/link/tests.rs` passes with seconds.

## Completion notes

One input script for the engine: `shuyi_platform::script` (new) holds
`Step` (`Keys`, `Pad`, `Act`, `Wait` in seconds, `Crank`), `Script`
(`new`, `check`, and `play` on a Harness), `Planned` and `frames` (the
one plan of a script's frames), `wait_frames` (seconds at 60 a second,
rounded up, within a thousandth of a whole number), `check_waits` (the
player's longest wait), `press_for` and `Press` (moved from `link`), and
`pressable` (the actions with a key or a button). `Harness::keys`,
`Harness::pad` and `Script::play`, the link's server and
`capture::Plan::frames` are built on the plan, so a key press's events
are built in one place. Recorded as ADR-0082, which amends ADR-0047 §7
and ADR-0052.

- **Harness:** `Script::play` plays `Act` by the game's keymap (an
  action it can't press panics, naming the step) and `Crank` by
  `Harness::crank`; `harness::MAX_WAIT` (33.3 s) is the longest wait it
  plays; `harness::FRAME_DT` is `script::FRAME_DT`.
- **Link:** `Request::Play { steps: Vec<script::Step> }`, waits in
  seconds on the wire, `link::MAX_WAIT` 1 800 s, a crank refused
  (`link::check_play`, the one check both ends run); `link::Step`,
  `plan`, `press_keys`, `press_pad` and the crate-private `press` module
  are gone; `link::VERSION` is 3.
- **Capture:** a plan's steps are `capture::PlanStep`s
  (`Input(script::Step)` or `Capture`), read from the same RON with
  `act("Confirm")` (a crank refused); `Plan::frames(&keymap)` answers a
  `Result`; `shuyi-app`'s capture run plans a plan that presses an action
  once the game has loaded (`Plan::presses_actions`).
- **Contract:** `frame_png` re-exports `Script` and `Step`;
  `frame_png::parse_step` reads `--act` too, for `frame-png`, `run`,
  `scene` and `attach`; `frame_png::check` adds the Harness's longest
  wait. `AttachStep` is gone (`AttachInput::steps` is `Vec<Step>`; the
  frames after are waits that play exactly that many frames,
  `attach::idle`). `Session::actions` (default: none; `Headless` from its
  game's keymap; the fake's `Up, Down, Left, Right, Confirm, Cancel`);
  `run::check_actions` refuses a script naming an action not in the list
  before anything plays, in `run`, `scene` and MCP's `step` (`SH0001`,
  the session kept). `contract::VERSION` is 36, the schemas
  regenerated.
- **Docs:** ADR-0082 (and the status lines of ADR-0047 and ADR-0052),
  `docs/contract.md` (the steps, `--act`, the waits' rule, the MCP `step`
  rows), the template's `AGENTS.md` table (`run --act Confirm`), the
  book's *Testing* and *Tools for agents* chapters, and the
  `shuyi-test` and `shuyi-cinematic` skills.

Checked on Hello, Grid through the MCP session: `step` with `{"act":
"Jump"}` answers `SH0001` naming the game's actions with nothing played,
and `{"act": "Confirm"}, {"wait": 0.5}` opens the stage in 2 + 30
frames; `frame-png --act Confirm --wait 0.25` writes the same PNG as
`--keys Enter --wait 0.25`; `scene --act Confirm` reaches the stage and
`run --act Jump` is refused.

Deviations:

- No `script` feature: `serde` is already a non-optional dependency of
  `shuyi-platform` (session records), so the feature would gate nothing.
  The `schema` feature is `["dep:schemars"]`, and 2569's clause says so.
- The check of the longest wait is `script::check_waits(steps, max)`,
  called by each player (`frame_png::check` for the Harness,
  `link::check_play`, `capture::Plan::parse`), rather than a method.
- `attach.input.json`'s step now lists `crank` too (it is
  `script::Step`), which `attach` refuses ("crank: not over the live link
  yet"); every JSON it read before reads the same. `attach` waits may be
  up to 1 800 s (they were held to the Harness's 33.3 s before).
- A capture plan that presses an action by name waits for the game to
  load before its frames start, since the keymap comes with the game;
  plans without `act` play as before.
- The capture plan's longest wait stays its own `capture::MAX_WAIT`
  (600 s).

Either-order clauses: 2540 is done (the crank moved into
`script::Step`, the link's plan and a capture plan refuse it); 2353,
2356, 2556, 2029, 2514, 3008 and 2569 are not done, so their clauses
here needed nothing, and each of those tickets now says what holds
("From 2566 (done)"). 2524 keeps the rest of its scope.

Nick input: none.

Breaking: shuyi-cli: frame_png::Step, frame_png::Script: re-exports of `shuyi_platform::script::{Step, Script}`; `Step` gains `Act`, and `Script` is built with `Script::new` (its fields can't be named from another crate).

```migration
since: 0.2.0
what: The contract's script is shuyi-platform's, and a step can press an action by name.
old: 'Script { steps: vec![Step::Keys(..)] }'
new: 'Script::new(vec![Step::Keys(..)])'
before: |
  let script = Script { steps: vec![Step::Keys("Enter".to_owned())] };
after: |
  let script = Script::new(vec![Step::Act("Confirm".to_owned())]);
find: ["Script { steps"]
```

Breaking: shuyi-cli: frame_png::Script::parse_step, frame_png::MAX_WAIT: now `frame_png::parse_step(flag, text)`, which reads `--act` too, and `shuyi_platform::harness::MAX_WAIT`; `Script::check` no longer checks the longest wait: `frame_png::check(&script)` does.

```migration
since: 0.2.0
what: A script flag is read by a free function, and the Harness's longest wait is checked with the script.
old: 'Script::parse_step(flag, text); script.check()'
new: 'frame_png::parse_step(flag, text); frame_png::check(&script)'
before: |
  if let Some(step) = Script::parse_step(arg, &text) { steps.push(step?); }
  script.check()?;
after: |
  if let Some(step) = frame_png::parse_step(arg, &text) { steps.push(step?); }
  frame_png::check(&script)?;
find: ["Script::parse_step", "script.check()"]
```

Breaking: shuyi-cli: contract::attach::AttachStep, AttachStep::of, frames_of, FRAMES_PER_SECOND: gone; `AttachInput::steps` and `link_steps` are `Vec<frame_png::Step>`, waits in seconds.

```migration
since: 0.2.0
what: attach takes the contract's steps.
old: 'AttachStep::Act(name)'
new: 'Step::Act(name)'
before: |
  input.steps = vec![AttachStep::Act("Confirm".into()), AttachStep::Wait(0.5)];
after: |
  input.steps = vec![Step::Act("Confirm".into()), Step::Wait(0.5)];
find: ["AttachStep"]
```

Breaking: shuyi-platform: link::Step, link::Press, link::press_for, link::MAX_WAIT, link::VERSION: the steps are `script::Step`, waits in seconds (`link::MAX_WAIT` is 1 800.0 seconds), `press_for` and `Press` are `script`'s, `Request` is no longer `Eq`, and the link speaks version 3.

```migration
since: 0.2.0
what: The link's steps are a script's, with waits in seconds.
old: 'link::Step::Wait(30)'
new: 'script::Step::Wait(0.5)'
before: |
  let steps = vec![link::Step::Act("Confirm".into()), link::Step::Wait(30)];
after: |
  let steps = vec![script::Step::Act("Confirm".into()), script::Step::Wait(0.5)];
find: ["link::Step"]
```

Breaking: shuyi-platform: capture::Step, capture::Plan::steps, capture::Plan::frames, capture::FRAME_DT, capture::wait_frames: a plan's steps are `capture::PlanStep`s (`Input(script::Step)` or `Capture`), `Plan::frames` takes the game's keymap and answers a `Result`, and the frame and wait rule are `script::FRAME_DT` and `script::wait_frames`.

```migration
since: 0.2.0
what: A capture plan's frames are planned with the game's keymap.
old: 'plan.frames()'
new: 'plan.frames(game.input().keymap())?'
before: |
  let frames = plan.frames();
after: |
  let frames = plan.frames(game.input().keymap())?;
find: ["plan.frames()"]
```

Breaking: shuyi-cli, shuyi-platform: a script's wait: played as `script::wait_frames` frames (rounded up, none for 0) by `Script::play`, not by `Harness::wait`'s shorter last frame, so a `frame-png` picture after a wait that isn't whole frames, or of 0, can change; `Harness::wait` keeps its seconds.

```migration
since: 0.2.0
what: A script's wait plays whole frames, rounded up.
old: 'Step::Wait(0.51) as 30.6 frames'
new: 'Step::Wait(0.51) as 31 frames'
before: |
  Script::new(vec![Step::Wait(0.51)]).play(&mut h);
after: |
  Script::new(vec![Step::Wait(31.0 / 60.0)]).play(&mut h);
```
