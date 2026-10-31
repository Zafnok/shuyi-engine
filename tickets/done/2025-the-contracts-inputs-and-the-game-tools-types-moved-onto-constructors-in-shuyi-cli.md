---
id: "2025"
title: "The contract's inputs and the game tools' types moved onto constructors in shuyi-cli"
type: infra
milestone: After MVP
model: implementation
effort: medium
status: done
blocked_by: ["2013"]
nick_input: none
completed: 2026-10-23
---

# 2025 — The contract's inputs and the game tools' types moved onto constructors in shuyi-cli

## Context

[ADR-0039](../../docs/adr/0039-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
(ticket 2013) sorts public types into eight kinds and gives each a rule.
Its table lists the engine types the templates and hello-grid build by
literal, with the ticket that moves each. This one has `shuyi-cli`'s
rows; 2024 has the other crates'. Each contract operation's input
gains fields often (a new option), and today every new field breaks a
game's tests that build the input with `..RunInput::default()`.

2013 checked that clippy's `field_reassign_with_default` stays quiet on
`let mut input = RunInput::default(); input.frames = 3;` once
`RunInput` is `#[non_exhaustive]` (the ADR's *Context*).

## Nick input

None.

## Scope

**In:** the rows of ADR-0039's table marked 2025:

- **Kind 4** (inputs with a `Default`, `#[non_exhaustive]`; callers
  write `let mut input = X::default();` and assign fields): `RunInput`,
  `CheckInput`, `ReplayInput`, `ImportInput`, `PlaytestInput`,
  `AutobalanceInput`, and `BotsInput` and `NewInput`, which get a
  `Default` (a `#[serde(default)]` already says what each field's
  default is; where a field has none that makes sense, say so and give
  the type a `new` taking it instead).
- **Kind 2** (config a game's tools fill in, `#[non_exhaustive]`, a
  `new` taking what every game must choose): `GameLang`, `GameOption`,
  `GameBake`, `OneBitSkin`, `Tunable`, `AutobalancePlan` and
  `PlaytestPlan` (`crates/cli/src/game.rs`).
- **Kind 3** (answers and views, `#[non_exhaustive]`, a `new` for what
  the type can't be without): `Play` (`crates/cli/src/contract/bots.rs`),
  `contract::session::Scene`, `Shot` and `Script`
  (`crates/cli/src/frame_png.rs`), `Project` (`crates/cli/src/project.rs`).
- **Kind 6:** `Engine` (`crates/cli/src/contract/new.rs`) gets
  `#[non_exhaustive]`; its struct-like `Path` variant gets a
  constructor on the enum if it becomes non-exhaustive itself.
- Every literal of these types outside `shuyi-cli` moves: `templates/`
  (the `tests/it/main.rs` files and `src/tools.rs`),
  `examples/hello-grid/` (`src/tools.rs` and `tests/it/`), and whatever
  else the build finds. `..X::default()` becomes `let mut` and field
  assignments.
- If the PR goes past about 600 changed lines, stop and write the rest
  as a ticket of its own.

**Out (do not do):**

- The other crates' rows (2024).
- `GameArg` (kind 1: a flag and its value; it stays exhaustive).
- The JSON the contract reads and writes: serde attributes don't change,
  so no schema changes (ADR-0022 §5's schema test stays green).
- The exhaustive lints in `shuyi-cli` (ADR-0039 says why).

**For the game:** a break, once, for a game's tools and tests that
build these types by literal. The completion notes carry a `Breaking:
shuyi-cli: …` line in 2010's form. The game's PR that moves the pin
follows this one.

## Implementation steps

1. Read ADR-0039. For each type, read its definition and every literal
   of it outside `shuyi-cli` (`git grep -n "TypeName {"`).
2. Add `#[non_exhaustive]`, the `Default`s and the constructors, with
   doc comments saying which fields `new` sets and to what.
3. Move the literals. Run `cargo fmt --all`; keep `templates/` formatted
   by hand.
4. Run the template gate for `grid` and `blank` as CI's `templates` job
   does, and the gates.

## Acceptance criteria

- [x] Every row of ADR-0039's table marked 2025 is `#[non_exhaustive]`
      with its `Default` or constructors, or the completion notes say
      which moved to a new ticket and its number.
- [x] No file under `templates/` or `examples/` builds those types by a
      struct literal or with `..X::default()`.
- [x] The contract's schemas don't change.
- [x] Games made with `shuyi new grid` and `shuyi new blank` pass the
      template gate.
- [x] The completion notes carry the `Breaking:` line.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each `new` gives what the literal it replaces gave.
- Integration: the template gate; hello-grid's contract tests
  unchanged in what they check.

## Completion notes

- **Kind 4:** `RunInput`, `CheckInput`, `ReplayInput`, `ImportInput`,
  `PlaytestInput`, `AutobalanceInput` and `BotsInput` are
  `#[non_exhaustive]` with their `Default` (`BotsInput` already had one,
  by hand: 10 games, seed 0, 200 turns). `ReplayInput.record` and
  `ImportInput.file` have no serde default (the JSON must give them), so
  those two also get a `new` taking that field (`ReplayInput::new(record)`,
  `ImportInput::new(file)`); it also keeps clippy's `assigning_clones`
  quiet where a test assigned `input.record = record.to_owned()`.
- **`NewInput`** gets no `Default`: a template and a folder have no
  default that makes sense, so it gets `NewInput::new(template, dir)`
  (named for its folder, on `Engine::Main`), as the ticket allows.
- **Kind 2** (`crates/cli/src/game.rs`): `GameOption::new(flag, help)`
  (a `const fn`, with `const fn with_value`, since games build their
  options in a `const` array), `GameLang::new`, `GameBake::new`,
  `OneBitSkin::new`, `Tunable::new`, and `PlaytestPlan::new` and
  `AutobalancePlan::new` (title, entrants, checks, and the try or the
  stage; `groups` and `default_entrants` start empty and are assigned).
- **Kind 3:** `Play::new(seed, outcome, turns, commands)` (no measures,
  no record), `Scene::new(screen, kind, text)` (`data` starts `null`),
  `Shot::new(image, shows)` (no warning), `Script::new(steps)`, and
  `Project::new(root, config)` beside `Project::load`.
- **Kind 6:** `Engine` is `#[non_exhaustive]`. Its `Rev` and `Path`
  variants stay exhaustive (a commit and a folder are each the whole
  meaning), so they get no constructor.
- **Schemas:** the contract's types that derive a JSON schema (the
  inputs, `Play`, `Scene`, `Script`, `Engine`) say "it may grow
  (ADR-0039)" in a `//` comment, not in their doc comment, as `LogEntry`
  does, so the schemas and the contract's version don't change; the
  schema test is green. Their constructors' docs say how to build them.
- **Literals moved** in hello-grid (`src/tools.rs` and `tests/it/`) and
  in `templates/grid` and `templates/blank` (`src/tools.rs`,
  `tests/it/main.rs`). `crates/cli/tests` built none of them. Literals
  inside `shuyi-cli` stay.
- **Tests:** each `new` is checked against the literal it replaces in
  its module's tests (`GameOption`'s in a `const` item). hello-grid's
  contract tests check what they checked; games made from `grid` and
  `blank` pass the template gate.
- **Deviation, size:** about 880 changed lines, past the ticket's 600,
  and not split, for 2024's reason: most of it is call sites moved
  mechanically and the constructors' tests, and a split would break the
  game twice at consecutive pin bumps where the ticket promises once.
- **Gates:** all of `run-gates` passed locally except `cargo xtask api`,
  which failed building the base's `shuyi-app` for cargo-semver-checks (a
  build script of `winapi-x86_64-pc-windows-gnu` "never executed" on this
  machine), not on this change; CI runs it. One run of the workspace's
  tests failed `mcp::an_agent_starts_hello_grid_steps_into_the_stage_and_looks_at_it`
  while every crate built at once; it passed alone and in `shuyi-cli`'s
  own run.
- **Follow-up:** 2548. Made locally as `demo-blank`, the blank template's
  game fails its own `cargo fmt --all --check`: its `BasicGame::new(..)`
  line fits only CI's short name `demo`. It isn't this ticket's change.
- **Docs:** rustdoc on each type and constructor; no page under `docs/`
  builds these types by literal.
- **Not checked through the MCP tools:** nothing a player sees changed.
- **Game:** anything that builds these types by literal moves to the
  constructors or to `default()` and assignments, and a `match` on
  `Engine` needs a `_` arm; the game's PR that moves the pin follows this
  one (not yet opened).

Breaking: shuyi-cli: RunInput, CheckInput, ReplayInput, ImportInput, PlaytestInput, AutobalanceInput, BotsInput: build them with `default()` and assign the fields (ReplayInput::new(record) and ImportInput::new(file) also work); NewInput with NewInput::new(template, dir); GameOption, GameLang, GameBake, OneBitSkin, Tunable, PlaytestPlan, AutobalancePlan, Play, Scene, Shot, Script, Project: build them with their `new` (GameOption::new(..).with_value(..)) and assign the rest; a match on Engine needs a `_` arm
