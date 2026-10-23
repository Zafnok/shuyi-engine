---
id: "2025"
title: "The contract's inputs and the game tools' types moved onto constructors in shuyi-cli"
type: infra
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2013"]
nick_input: none
completed:
---

# 2025 — The contract's inputs and the game tools' types moved onto constructors in shuyi-cli

## Context

[ADR-0037](../../docs/adr/0037-public-types-that-can-grow-non-exhaustive-config-and-views-with-constructors-plain-values-left-open.md)
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

**In:** the rows of ADR-0037's table marked 2025:

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
- The exhaustive lints in `shuyi-cli` (ADR-0037 says why).

**For the game:** a break, once, for a game's tools and tests that
build these types by literal. The completion notes carry a `Breaking:
shuyi-cli: …` line in 2010's form. The game's PR that moves the pin
follows this one.

## Implementation steps

1. Read ADR-0037. For each type, read its definition and every literal
   of it outside `shuyi-cli` (`git grep -n "TypeName {"`).
2. Add `#[non_exhaustive]`, the `Default`s and the constructors, with
   doc comments saying which fields `new` sets and to what.
3. Move the literals. Run `cargo fmt --all`; keep `templates/` formatted
   by hand.
4. Run the template gate for `grid` and `blank` as CI's `templates` job
   does, and the gates.

## Acceptance criteria

- [ ] Every row of ADR-0037's table marked 2025 is `#[non_exhaustive]`
      with its `Default` or constructors, or the completion notes say
      which moved to a new ticket and its number.
- [ ] No file under `templates/` or `examples/` builds those types by a
      struct literal or with `..X::default()`.
- [ ] The contract's schemas don't change.
- [ ] Games made with `shuyi new grid` and `shuyi new blank` pass the
      template gate.
- [ ] The completion notes carry the `Breaking:` line.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each `new` gives what the literal it replaces gave.
- Integration: the template gate; hello-grid's contract tests
  unchanged in what they check.

## Completion notes

*(Filled in by the session that completes the ticket.)*
