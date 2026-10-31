---
id: "2770"
title: "Parked: Command records in shuyi replay: replay::commands re-runs a CommandRecord with a hash per step"
type: feature
milestone: "Engine: real-time strategy"
model: implementation
effort: medium
status: blocked
blocked_by: ["2706", "2111"]
nick_input: none
completed:
---

# 2770 — Command records in shuyi replay

> **Parked** (Nick, 2026-10-18, for its parent 2706: "make tickets
> parked for after visions of shuyi launches for all of these"): a
> follow-up of the parked command sims (2706), and a follow-up of
> parked work is parked. So it starts after Visions of Shuyi launches,
> which is later than the MVP in `docs/ROADMAP.md`, and after 2706 is
> unparked and done. Nobody starts it until Nick unparks it. Whoever
> unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2706 adds a third shape of simulation to `shuyi_core::tick`
([ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§2). A `CommandSim` steps 60 times a second and is fed the timed
commands (`Timed<C>`) due at each step. A run is a `CommandRecord`: its
seed, its length in steps and its commands, in RON with a
`format_version`. `replay_commands` re-runs one and returns the hash
after each step. ADR-0015 §2 says `shuyi replay` works on such a
record, and 2706's Out leaves that to this ticket.

The contract's `replay` (1129, done;
`crates/cli/src/contract/replay.rs`) hands the record's file and text
to the game's `GameTools::replay`. The game re-runs it and gives back a
`ReplayRun`: step 0, then each step's hash, words and events. The
contract runs one record twice, or two records side by side, and
answers with the first step where their hashes part. `replay::turns`
builds that run for a `TurnRules` game. Hello-grid's `src/tools.rs`
calls it in three lines, and `docs/contract.md` (*What a game gives*)
tells a game to. A command sim has no such helper. So a game whose
input is orders writes the stepping loop itself, or offers no
`replay`.

Why it matters: a record run twice that parts shows the sim reading
something it doesn't own, such as a hash map's order or a clock.
ADR-0015 says that breaks replays and lockstep. `shuyi replay` shows an
agent the first step where it happens, without reading the sim.

A turn game's record is the game's own type (hello-grid's
`StageRecord`). A `CommandRecord` is the engine's. So the helpers here
read it too, and the game gives only how to build its sim from the
seed and how to say its outcome.

2111 stamps records with what made them. `CommandRecord` gets a
`made`, and `ReplayRun` a `made: Option<Made>` that the contract checks
against the game's before it compares hashes. This ticket fills that
field, so 2111 is needed first.

## Nick input

None.

## Scope

**In:**
- **`shuyi_cli::contract::replay::command_record`**:
  `command_record<C: DeserializeOwned>(record: &Record) ->
  Result<CommandRecord<C>, String>` reads the record's text as RON.
  - A record in another `format_version` than the one 2706 writes is
    refused, naming both. If 2706 gives a reader that checks the
    version, call it instead of writing a second check.
  - An unknown field is refused (2706's `deny_unknown_fields`).
  - Each error names the file first (`a.ron: …`), as hello-grid's
    `replay` does.
- **`shuyi_cli::contract::replay::commands`**, beside `turns`:
  `commands<S>(file: &str, initial: S, record:
  &CommandRecord<S::Command>, outcome: impl Fn(&S) -> Option<String>)
  -> Result<ReplayRun, String>`, where `S: CommandSim` and `S::Event:
  Debug`.
  - `initial` is the sim the caller built from `record.seed`, as
    `replay_commands`' caller does. `outcome` says how the run ended,
    in the game's words, since `CommandSim` has no `outcome()`.
  - Step 0 is the sim before any step. Then one `ReplayStep` per sim
    step, `record.steps` of them, each fed `record.due` of its step as
    `replay_commands` feeds it. Steps after the last command run too.
  - `step` counts the sim steps run: 0 before any, n after the n-th.
    `docs/contract.md` says which record step that is (n − 1 if 2706
    numbers a record's steps from 0).
  - `command` is the commands that step was fed, each as 2706's text
    dump writes it without the step (`player command`, the command in
    compact RON), joined by `; `. It is none for a step with no
    command.
  - `hash` is `CommandSim::hash` in sixteen hex digits, as
    `shuyi_core::StateHash` prints it. `events` are the step's events,
    each `{e:?}`, as `turns` gives them. `refused` is always none: a
    command sim refuses nothing, and its events say what a command did.
  - `made` is the record's `made`, so the contract's `replay` refuses a
    record stamped with other content than the game's (2111).
  - A record of more steps than a `u32` counts is refused, naming the
    file.
- **The docs:** `docs/contract.md`'s `replay` row (*What a game
  gives*) names `command_record` and `commands` for a command sim,
  beside the helpers it names already. `GameTools::replay`'s doc
  comment (`crates/cli/src/game.rs`) and `replay.rs`'s module docs name
  them too.
- **The contract stays as it is:** no type of the answer changes, so
  the schemas in `crates/cli/schemas/` and `contract::VERSION` don't
  move. If one must change after all, regenerate them and raise
  `VERSION` (ADR-0022 §5).
- **What changes for the game:** nothing. Two functions are new, and
  `turns` keeps its signature. The PR says so.

**Out (do not do):**
- the sim, the record and `replay_commands` themselves (2706);
- a replay of tick records, `replay::ticks` (2739's);
- lockstep and its desync reports (2720);
- the RTS module and its commands (2707);
- `bots` over a command sim, and a soak of one: 2772 (parked) checks a
  `CommandRecord` in `shuyi_core::soak::command`, with no operation;
- a shorter answer for long runs: every step is listed, as a tick
  replay lists every tick;
- a new operation, or a new field in an answer.

## Implementation steps

1. Read 2706's `shuyi_core::tick` (`Timed`, `CommandSim`,
   `CommandRecord` with its `due` and its text dump, and
   `replay_commands`), 2111's `Made` and `ReplayRun::made`, and
   `crates/cli/src/contract/replay.rs` with its tests.
2. In `crates/cli/src/contract/replay.rs`, add `command_record`. If
   `shuyi_core::tick` sits behind a feature, turn it on for
   `shuyi-core` in `crates/cli/Cargo.toml`. The edge from `shuyi-cli`
   to `shuyi-core` exists already, so `tools/layers.ron` doesn't
   change.
3. Add `commands`. Write a step's words through 2706's dump line if it
   has a function for one. If not, use `ron::to_string` (compact) in
   the same `player command` form.
4. Write the tests below, with a small `CommandSim` of the tests' own
   beside `Counter`.
5. Update `docs/contract.md`'s row, `GameTools::replay`'s doc comment
   and the module docs. Run the gates.

## Acceptance criteria

- [ ] `commands` on a record gives step 0 and then `record.steps`
      steps, and its hashes from step 1 on equal `replay_commands`'
      list over the same record.
- [ ] The contract's `replay` on one command record passes, with no
      difference between its two runs.
- [ ] On a record beside a copy with one command changed, `replay`
      fails, and its `first_difference` is the step that was fed the
      changed command.
- [ ] `command_record` refuses another `format_version` and an unknown
      field, naming the file.
- [ ] A run carries the record's `made`.
- [ ] `crates/cli/schemas/` and `contract::VERSION` are unchanged, and
      the schema test passes.
- [ ] `docs/contract.md`'s `replay` row names both functions.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, in `replay.rs`'s tests, over a `Tally` sim of their own: a
  total that each step's commands add to, `Command = i32`, an event
  `Added(i32)` per command, its hash the `state_hash` of its state, and
  an outcome `Reached` at 10:
  - step 0 and one step per sim step, the steps after the last command
    included;
  - a step with no command has `command` none, and a step with two
    commands lists both in record order;
  - the hashes equal `replay_commands`';
  - the run's `outcome` and `made`;
  - `command_record`: a round trip through RON, another
    `format_version`, an unknown field, and bad RON naming the file;
  - a record of more steps than a `u32` counts.
- Integration: the contract's `replay` over a test `GameTools` whose
  `replay` calls `command_record` and `commands` (its required methods
  delegated to `Fake`, `crates/cli/src/contract/fake.rs`). One record
  run twice passes. A record beside its changed copy fails at the
  changed command's step.
- Property: for random commands, the hashes of `commands` from step 1
  on equal `replay_commands`'.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
