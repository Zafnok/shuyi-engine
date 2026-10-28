---
id: "2568"
title: "A game's start options checked once, on every path: MCP's start refuses an option the game doesn't offer"
type: bug
milestone: After MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2568 — A game's start options checked once, on every path

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). A game offers start options (`GameTools::frame_options`,
`crates/cli/src/game.rs`): Hello, Grid's `--stage` and `--seed N`, the
toy's `--lobby`. They arrive as `GameArg { flag, value }`. Whether a
`GameArg` is one the game offers is checked in three copies, all on the
command line, and never when the options come as JSON:

- **Three copies of the same lookup**, each finding the flag in
  `options`, else `unknown option: {flag}`, then reading a value if the
  option takes one:
  - `run::parse_args` (`crates/cli/src/contract/run.rs:404-412`);
  - `frame_png::parse_args` (`crates/cli/src/frame_png.rs:241-251`);
  - `bots::game_arg` (`crates/cli/src/contract/bots.rs:252-266`), which
    `bots`, `playtest` and `autobalance` call
    (`bots.rs:236`, `playtest.rs:627`, `autobalance.rs:390`).
- **The JSON paths check nothing.** MCP's `start` and `reset` go to
  `Host::start` (`crates/cli/src/mcp/host.rs:310-311`), which hands
  `start` straight to `self.tools.session(&start)`. The one-off
  operations over MCP (`run`, `scene`, `bots`, `playtest`,
  `autobalance`, `host.rs:185-197`) read their input from JSON, so their
  `start` skips `parse_args` too. `RunInput::check` (`run.rs:100`)
  checks only the script and `frames`.
- **So a typo starts the wrong game, or blames the game.** The toy
  reads its options with `options.iter().any(|option| option.flag ==
  "--lobby")` (`crates/cli/src/toy.rs:56` and `:72`), so MCP's `start`
  with `--lobbby` starts the home screen and answers `ok`. Hello, Grid's
  `Start::from_options` (`examples/hello-grid/src/tools.rs:80-94`)
  refuses an unknown flag itself, but its error comes back as the
  game's (`code_of`, `game.rs:573`, gives `GAME_ERROR`), and `--seed`
  sent without a value is refused as `unknown option --seed`.

One check, beside `GameArg` in `game.rs`, used by every path, makes an
unknown option, a missing value and an extra value the same
`SH0001` (`codes::WRONG_ARGUMENTS`) everywhere.

## Nick input

None.

## Scope

**In:**
- **`shuyi_cli::game::game_arg`**: `bots::game_arg` moved as it is into
  `crates/cli/src/game.rs`. `run::parse_args`, `frame_png::parse_args`,
  `bots`, `playtest` and `autobalance` call it; the two copies go.
  `bots.rs` keeps `pub use crate::game::game_arg;` so the old path
  still works.
- **`shuyi_cli::game::check_start(options: &[GameOption], start:
  &[GameArg]) -> Result<(), String>`**: every `GameArg` names one of
  `options` (else `unknown option: {flag}`, the parsers' text); one
  that takes a value has it (else `{flag} needs a value`, the parsers'
  text); one that takes none has none (else `{flag} takes no value`).
  Both are added to `lib.rs`'s `pub use game::{…}`.
- **Every operation that starts the game calls `check_start` first**,
  failing with `codes::WRONG_ARGUMENTS`: `Host::start` (so MCP's
  `start` and `reset`), `run::played` (so `run` and `scene`, from the
  command line and MCP), `bots::bots`, `playtest::playtest` (so
  `kept`) and `autobalance::autobalance`. `frame-png` has no JSON path:
  its `parse_args` going through `game_arg` is its check.
- **`docs/contract.md`**: the paragraph on the game's own start options
  (around line 40) says an option the game doesn't offer, or one
  missing its value, is refused with `SH0001` on every path, MCP's
  `start` among them.
- **Either order with 2519:** if 2519 is done, `run::played` and
  `Host::start` can also start from a session record: call
  `check_start` before both ways (a session has no start options, as
  2519's `RunInput::check` makes sure, so it passes); if not, nothing to
  do.
- **Either order with 2544:** if 2544 is done, `bots.rs` is gone and
  `game_arg` lives where 2544 moved it (`grep -rn "fn game_arg"
  crates/cli/src`): move it from there, leave no re-export, and there
  is no `bots::bots` to call `check_start` in; if not, move it from
  `bots.rs` as written above.
- **Either order with 2527:** if 2527 is done, `soak` starts the game
  too: call `check_start` on its input's `start` at the top of its
  operation, beside its other checks; if not, nothing to do (2527 adds
  the call).
- **Either order with 2529:** if 2529 is done, `peers` and MCP's
  `start` with `peers` start the game too: call `check_start` on
  `PeersInput`'s `start` at the top of the `peers` operation, and keep
  the call at the top of `Host::start` ahead of its peers path; if not,
  nothing to do (2529 adds the call).
- **For the game:** no API break. `game::game_arg` and
  `game::check_start` are additive, and `bots::game_arg` stays as a
  re-export. A game whose own `session` refused a bad option now gets
  `SH0001` from the engine first, before its code runs.

**Out (do not do):**
- The games' own reading of their options (`Start::from_options` in
  Hello, Grid, `seed_of` in the grid template, `no_options` in
  `crates/cli/src/basic.rs`, the toy's `--lobby` match): they read the
  values; leave them.
- The other flags in either `parse_args` (the script's steps,
  `--scale`, `--target`, `--frames`, `--perf`): 2353, 2356, 2514, 2556
  and 2566 change those arms. Only the arm for the game's own options
  changes here.
- `attach`: it starts no game, and has no start options.
- Any change to the contract's types, so no schema change and no new
  `contract::VERSION`.

## Implementation steps

1. Move `game_arg` from `crates/cli/src/contract/bots.rs` into
   `crates/cli/src/game.rs`, with its docs. In `bots.rs` write
   `pub use crate::game::game_arg;`. Point `playtest.rs:24` and
   `autobalance.rs:24` at `crate::game::game_arg`.
2. Replace the lookup in `run::parse_args` (`run.rs:404-412`) and in
   `frame_png::parse_args` (`frame_png.rs:241-251`) with
   `game_arg(arg, options, value)?` (each keeps its own `value`
   closure).
3. Add `check_start` to `game.rs`, with unit tests in its `mod tests`,
   and add `game_arg` and `check_start` to `lib.rs`'s `pub use
   game::{…}`.
4. Call `check_start(game.frame_options(), &input.start)` (or
   `self.tools.frame_options()` and `&start` in the host) at the top of
   `Host::start`, `run::played` (after `input.check()`), `bots::bots`,
   `playtest::playtest` and `autobalance::autobalance`, each answering
   `Answer::failed(op, codes::WRONG_ARGUMENTS, e)`.
5. Write the tests below.
6. Update `docs/contract.md` (the `document` skill).
7. Settle the clauses with 2519, 2544, 2527 and 2529 with what is done.
8. Run the gates.

## Acceptance criteria

- [ ] `grep -rn "find(|option| option.flag ==" crates/cli/src` finds
      only `crates/cli/src/game.rs`.
- [ ] MCP's `start` on the toy with `start: [{"flag": "--lobbby"}]`
      answers `ok: false` with code `SH0001` and starts no game (a
      following `step` answers that there is no session).
- [ ] `run` over MCP with an unknown start option answers `SH0001`
      before the game's `session` runs.
- [ ] Every existing test of `run`, `scene`, `frame-png`, `bots`,
      `playtest`, `autobalance` and the MCP host passes unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/cli/src/game.rs`): `check_start` with no options and
  none given; an unknown flag; a flag that takes a value given none; a
  flag that takes none given one; every option given right.
- Unit (`crates/cli/src/mcp/host/tests.rs`, under `feature = "toy"` as
  the other toy tests are): `start` with `--lobbby` answers `SH0001`
  and leaves no session; `start` with `--lobby` still starts the lobby.
- Unit (`crates/cli/src/contract/run/tests.rs`): `run::run_at` with a
  JSON-built `RunInput` whose `start` holds an unknown flag answers
  `SH0001`, on the contract's `Fake` game (`contract/fake.rs`, which
  offers `--seed N`); so does one with `--seed` and no value.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
