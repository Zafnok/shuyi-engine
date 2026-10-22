---
id: "2527"
title: "shuyi soak: a seeded bot plays random legal commands, the state is saved and reloaded at every step, the TurnRules contract and the game's invariants are checked, and a failure shrinks to a short record"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2527 — shuyi soak: random legal play, reloaded and checked at every step

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and GameMaker. None of the first four ships a check like
this. Engines built on determinism do:

- Factorio's "heavy mode" saves and reloads the game every tick and
  reports any state that differs
  (https://wiki.factorio.com/Desynchronization).
- GameMaker's rollback system has a sync test that runs a frame's code
  twice and compares
  (https://manual.gamemaker.io/beta/en/GameMaker_Language/GML_Reference/Rollback/Rollback_System.htm).
- GGRS, which `bevy_ggrs` uses, has a `SyncTestSession` that simulates
  frames again and compares checksums.

Shuyi today:

- `crates/core/src/turn.rs`: `TurnRules`, with its contract in its docs
  only: deterministic; all or nothing; `check` gives `apply`'s answer
  without changing anything; `legal_commands` in a fixed order, each
  accepted by `check`; an ended game has no legal command. No generic
  code checks these clauses for a game.
- `crates/core/src/bots.rs`: `Bot`, `play`, `play_observed`, and
  `RandomBot`, which picks from `legal_commands` with its own `SimRng`.
- `crates/core/src/hash.rs`: `state_hash`, a hash of a state's serde
  form.
- `crates/core/src/history.rs`: `History`, a game's first state and its
  commands, saved as `(initial: …, commands: [ … ])`. A refused command
  replays as nothing.
- `crates/cli/src/contract/bots.rs` and `replay.rs` (1129, done):
  `shuyi bots` plays seeded random games and returns a record of each;
  `shuyi replay` runs one record twice and reports the first step whose
  hashes differ. `templates/grid/crates/game/src/tools.rs` and
  `examples/hello-grid/src/tools.rs` wire both.

What none of it catches is state that the serde form leaves out.
`state_hash` hashes the serde form, so it never sees a `#[serde(skip)]`
field or a cache. `bots` and `replay` rebuild the state by replaying
commands, so they never see it either. The bug shows only when play goes
on from a reloaded copy and is compared with a copy that was never
reloaded. Agent-written rules code is likely to have it. It breaks
saves, a game saved in progress, and later a peer that reconnects by
replaying history (1140). The other clauses of the contract also fail
quietly: a `check` that disagrees with `apply`, or a refused command that
changes something, passes unit tests and breaks replays.

Why it matters:

- **The game first.** Visions of Shuyi's rules are a `TurnRules` state,
  and its saves and replays rest on them. It doesn't ask for this. Once
  its tools implement one more `GameTools` method, it can soak its own
  rules in CI.
- **Other 2D games.** Every turn game made from the grid template gets
  the check in its CI from `shuyi new`.
- **The end goal** of 2026-10-19 (`docs/decisions.md`), measured on turn
  games until the launch (2026-10-20): an agent that writes a game's
  rules runs one op in its fix loop and gets a short failing record to
  read. An agent-made game then holds up under saves and replays, not
  only runs.

It is cheap because it reuses `RandomBot`, `History`, `state_hash` and
the `bots` input. Nick placed this group after the MVP, not parked, on
2026-10-20 (`docs/decisions.md`, *a second comparison*).

## Nick input

None.

## Scope

**In:**

**`shuyi_core::soak`** (`crates/core/src/soak.rs`, tests in
`crates/core/src/soak/tests.rs`), `no_std` with `alloc` (no `std::`
path, which `cargo xtask check-no-std` checks), beside `bots` and
`history`. No new dependency: serde is already the core's.

- `trait Codec { fn round_trip<T: Serialize + DeserializeOwned>(&self,
  value: &T) -> Result<T, String>; }`: a value written and read back, as
  a save would. The core has `ron` as a dev-dependency only, so the
  codec is handed in.
- `Checks<'c, R, C: Codec>`: the codec, and the game's invariants, a
  `fn(&R) -> Vec<String>` listing what is wrong with a state (empty when
  nothing is). `Checks::new(&codec)` has no invariants;
  `with_invariants(f)` adds them.
- `Check`, the check that failed, with `name()` (kebab case, for the
  contract) and `ALL`:

  | Check | Fails when |
  | ----- | ---------- |
  | `hash` | the state's own `Serialize` refuses (`HashError`) |
  | `round-trip` | the codec can't write or read the state, or the copy it reads hashes differently |
  | `reloaded` | the copy reloaded at every step and the copy never reloaded part: another hash, other events, or one accepts a command the other refuses |
  | `history` | the play's `History`, written and read by the codec and replayed, gives another hash at some step, or doesn't go through the codec |
  | `check-changed` | calling `check` changed the state's hash |
  | `check-disagrees` | `check` accepts a command `apply` refuses, or refuses one `apply` accepts |
  | `legal-refused` | `check` refuses a command `legal_commands` lists |
  | `legal-order` | `legal_commands` gives another list when asked twice |
  | `over-with-commands` | `outcome` is `Some` and `legal_commands` isn't empty |
  | `refusal-changed` | a refused command changed the state |
  | `stuck` | the game isn't over and lists no legal command, so no bot can go on (not a clause of the contract, but the soak stops there) |
  | `invariant` | the game's invariants list something, on either copy |

- `Failure<C> { check: Check, step: u32, detail: String, commands:
  Vec<C> }`. `step` counts as `replay` does: 0 is the start, `n` is
  after the `n`th command. A failure found while command `n` is applied
  (step 5 below) is at step `n`. `commands` runs from the start through
  the failing step's command, so replaying it reproduces the failure.
  `detail` names the copy, the command, the hashes or the invariant
  lines, with `Debug`.
- `play(start: &R, bot: &mut dyn Bot<R>, limit: u32, checks) ->
  Result<Vec<R::Command>, Failure<R::Command>>`, for `R: Sides`: the bot
  picks every side's commands until the game ends or turn `limit` has
  passed, as `bots::play` stops, and every step is checked. On a game
  with one side that passes, it applies exactly the commands
  `bots::play` would with the same bot for that side.
- `commands(start: &R, commands: &[R::Command], checks) ->
  Verdict<R::Command>`: a fixed list checked the same way, for a saved
  record and for shrinking. `Verdict` is `Passed`, `Failed(Failure)`, or
  `Unplayable { step }`: `check` refuses a command of the list at its
  point, so the list doesn't play as made.
- `shrink(start: &R, failure: Failure<R::Command>, checks, runs: u32) ->
  Shrunk<R::Command>`: the shortest list it finds that still fails the
  same `Check`, by delta debugging over the commands (ddmin). It starts
  from `failure.commands`, which already ends at the failing step, then
  tries removing chunks, making the chunks smaller when none can go,
  down to single commands. A candidate counts only if `commands` gives
  `Failed` with the same `Check`; `Unplayable` and other checks are
  dropped. At most `runs` candidates are run. `Shrunk { failure,
  played: u32, complete: bool }`: the shrunk failure, the length it
  started from, and whether it finished within `runs`.
- Bounds: `R: TurnRules + Clone + Serialize + DeserializeOwned`;
  `R::Command: Clone + PartialEq + Debug + Serialize + DeserializeOwned`;
  `R::Event: PartialEq + Debug`; `R::Error: Debug`. `History` already
  asks the serde bounds of a saved game.
- No re-exports at the crate's root: `play` would clash with
  `bots::play`. A game writes `shuyi_core::soak::play`.

**One step, exactly.** The module docs say it in this order. Copies A
and B start as clones of `start`.

1. Hash A (`hash`), and run the invariants on A (`invariant`).
2. Ask A for `legal_commands` twice. The lists must be equal
   (`legal-order`). Empty with no outcome is `stuck`; not empty with an
   outcome is `over-with-commands`. Call `check` on each listed command
   (each `Ok`, else `legal-refused`), then hash A again
   (`check-changed` if it moved).
3. **The refusal probe.** Of the previous step's legal commands, the
   first four in list order that `check` now refuses are applied to a
   clone of A. Each must return `Err` (else `check-disagrees`), and the
   clone's hash must stay A's (else `refusal-changed`). This tests "all
   or nothing" with commands the game itself once offered, so it needs
   no generator of the game's commands.
4. B goes through the codec. The copy's hash must equal B's
   (`round-trip`), the invariants run on the copy (`invariant`, "the
   reloaded copy"), and B becomes the copy.
5. The step's command (the bot's, chosen on A, or the list's) is applied
   to A and to B. In `commands`, `check` on A refusing the list's command
   ends the run first, as `Unplayable { step }`. A refusing a command
   `check` accepted is `check-disagrees`. A refusal on one copy only,
   other events, or another hash after is `reloaded`.

At the end of a play or a list, steps 1 to 4 run once more on the last
state, so the probe tries the ended game too. Then
`History::from_parts(start.clone(), commands)` goes through the codec
and is replayed, and each step's hash must equal A's at that step
(`history`).

**Choosing commands never touches the game's luck.** The bot has its own
seed (`bots.rs`: a bot's randomness "comes from a seed it was built
with"). The soak never draws from the state's `SimRng`. A debug-only
command that a game keeps out of `legal_commands` (2356) is never sent.

**The contract op `soak`** (`crates/cli/src/contract/soak.rs`):

- `SoakInput`: `games`, `seed`, `turns` and `start`, with the same
  defaults and limits as `BotsInput` (`MAX_GAMES`, the seed check), and
  `save: Option<String>`, a folder under the project's root.
  **Either order with 2544:** if 2544 is done, `bots` is folded into
  `playtest`: name the count `tries` and take the defaults and limits
  from `PlaytestInput`, and read "`bots`" in this ticket as
  `playtest`; if not, use `BotsInput`'s as written, and 2544 renames
  them.
- `SoakOutput { games: u32, steps: u64, unfinished: u32, failures:
  Vec<SoakFailure> }`. `SoakFailure { seed: u64, check: String, step:
  u32, detail: String, played: u32, shrunk: u32, complete: bool, record:
  Option<String> }`: the shrunk failure, the play's length before and
  after shrinking, and the shrunk record in the game's record format.
- `RonCodec`, the `Codec` through `ron::to_string` and `ron::from_str`
  (the CLI already depends on `ron` with `std`). Not `Ron`:
  `shuyi_assets::Ron<T>` is a format.
- `turns(input, codec, start: impl Fn(u64) -> Result<R, String>,
  invariants: fn(&R) -> Vec<String>, record: impl Fn(u64, &[R::Command])
  -> Result<String, String>) -> Result<SoakOutput, String>`: for each
  seed, `start(seed)` played by one `RandomBot::new(seed)` for every side
  (the seed rule of the template's `bots`), a failure shrunk with
  `SHRINK_RUNS` (500) candidates, and its record written by `record`. An
  error from `record` leaves the failure's `record` empty, as `bots`
  leaves a play's. It is to `soak` what `replay::turns` is to `replay`.
- `soak(game, root, input) -> Answer<SoakOutput>`: refuses `games` and
  `seed` as `bots` does, asks the game, and passes when there is no
  failure. With `save`, it makes the folder if it is missing and writes
  each failure's record to `<save>/soak-<seed>.record`, which `shuyi
  replay` reads. A record is a run's log, not an asset kind, so it gets
  no `Format` (as 2706 says of its records). Limits:
  - the plays the turn limit stopped;
  - a shrink that ran out of runs: "the record may not be the
    shortest";
  - a failure with no record: "the game's tools give no record of some
    failures";
  - always: "commands a game leaves out of `legal_commands` are never
    tried".
- `GameTools::soak(&self, input: &SoakInput) -> Result<SoakOutput,
  String>` (`crates/cli/src/game.rs`, after `bots`), defaulting to
  `unsupported("soak", "soak")`. `Fake` (`contract/fake.rs`) gets a
  `soak` field.
- `Op::Soak` (`soak`): in `Op::ALL` (`crates/cli/src/contract.rs:80`),
  `Op::name`, `schemas()` (`soak.input.json`, `soak.output.json`),
  `cli::runs_the_game` and `cli::with_game` (`contract/cli.rs`, and the
  list of ops in its test of `runs_the_game`), the usage in
  `crates/cli/src/lib.rs` (beside `bots`, line 55), and
  `docs/contract.md`'s two tables. `contract::VERSION` goes up by one,
  with its line in `crates/cli/schemas/versions.json`.
- The other places that list the ops gain `soak`: the table in
  `contract.rs`'s module docs, `lib.rs`'s module docs, the
  `description` in `crates/cli/Cargo.toml`, the two rows of `README.md`
  that list them, `examples/hello-grid/README.md` (its tools row and a
  line under `cargo shuyi bots 30`), and the lines in
  `templates/blank/AGENTS.md` and `templates/blank/crates/game/src/tools.rs`
  that say `scene`, `replay` and `bots` need a stage and rules.
- The command line: `shuyi soak <games> [--seed N] [--turns N] [--save
  DIR] [options] [--json]`. `parse_args` takes `--save DIR` out first and
  passes the rest to `bots::parse_args`. `usage` reads like
  `bots::usage`, with `--save`.
- `text`: one line for the soak, then for each failure the seed, the
  check, the step, the lengths before and after, the detail, and the
  record.

**Why an op and not a flag on `bots`.** `GameTools::bots` is the game's
own loop, with its own bot per side (Hello, Grid plays the foe with
`Chase`), so a `--soak` flag would need every game's `bots` to honour
it. A separate method with a default says plainly what a game offers.
Its input keeps `bots`' fields and limits.

**The grid template** (`templates/grid`):

- `rules::Stage` (`crates/rules/src/lib.rs:129`) derives `Deserialize`
  too. Its fields all do: `Grid`, `Pos`, `SimRng`, `Outcome`.
- `Stage::invariants(&self) -> Vec<String>`: both pieces stand on open
  tiles, and the turn is at least 1. It is a small example of the hook a
  game grows.
- `Tools::soak` in `crates/game/src/tools.rs`, through
  `shuyi_cli::contract::soak::turns` with `RonCodec`, `Stage::invariants`
  and `StageRecord` (`replay::turns` is already imported there as
  `turns`, so name the module).
- Its CI (`templates/grid/.github/workflows/ci.yml`) runs `cargo shuyi
  soak 20 --turns 100` after `cargo shuyi check-keys`. This repository's
  `templates` job (`.github/workflows/ci.yml`) is a matrix over `grid`
  and `blank`, and the blank game offers no `soak`. So the soak there is
  a step of its own after "Run its gates", with `if: matrix.template ==
  'grid'`, in the same working directory.
- Its `AGENTS.md`: a row in the tools table and a line in the gates.

**Hello, Grid** (`examples/hello-grid`): `rules::Stage`
(`src/rules.rs:156`) derives `Deserialize` (its fields all do), gets the
same two invariants (its hero and its foe on open tiles, the turn at
least 1), and `HelloTools::soak` is wired the same way
(`docs/contract.md` says its tools write every op). `tests/it/contract.rs`
soaks 20 seeds through `soak::soak`, as its other tests call `bots` and
`replay`.

**Either order with 1134:** if `skills/shuyi-test/SKILL.md` and
`skills/shuyi-debug-replay/SKILL.md` exist, `shuyi-test` gets a short
section (soak in CI; a shrunk record turned into a regression test with
`soak::commands`), and `shuyi-debug-replay` says to run `soak` first
when a save or a replay misbehaves, since `replay` can't see state the
serde form leaves out. If 1134 isn't done, 1134 adds them when it lands.

**MCP:** `shuyi mcp` (1133, done) makes its tools from `Op::ALL`; a new operation needs its arm in `mcp::host::one_off` and in `mcp::description`, which the compiler asks for, and is in `contract::cli::runs_the_game` if it runs the game (so it takes `project` and runs in the game's tool program). `soak` runs the game.

**Problem codes (2509):** `soak`'s refused arguments take `SH0001`,
the game's errors `SH3001` or `SH3002` through the same helpers as
`bots`, and a record `--save` can't write `SH3004`. A failure the soak
finds is a verdict (`ok: false`), not an error, and gets no code.

**The public API.** `Op` gains a variant, and `Op::ALL`'s type changes
from `[Op; 9]` to `[Op; 10]`. If 2010's job reports a break (a new
variant is one unless `Op` is `#[non_exhaustive]`), the PR has its
`Breaking:` line. A new `GameTools` method with a default breaks no
game's tools. If 2013's ADR on public types that can grow is in
`docs/adr/`, follow it for every new public type: `SoakInput` as an
input with a `Default`, `Check` and `Verdict` as enums that may grow,
and `Checks`, `Failure` and `Shrunk`.

**Out (do not do):**
- tick sims: 1119 (parked) gets an Out line for the same check over a
  `TickSim`, written as a follow-up when it lands;
- command sims: 2706 (parked) gets the same line;
- desyncs between several peers (1140, 2529);
- the same records on several platforms (2116);
- fuzzing the formats and the pack (2020);
- session records at the platform boundary (2518);
- a Harness test that suspends, saves, loads and resumes mid-play: a
  game's screens save through the game's own code, which this ticket
  doesn't see;
- shuffling an ECS schedule's systems to expose ordering bugs (2108
  compares a parallel schedule's hashes with the serial one's);
- saves from older versions of a game: the soak round-trips the current
  version only, and migrations are 2110's;
- commands outside `legal_commands`, and proptest strategies of a game's
  commands: the probe in step 3 uses the game's own;
- a game's own save files (its `SaveFormat` header and layout) in the
  soak: a game whose saves aren't its state in RON writes its own
  `Codec`;
- other seeds while shrinking: the shrink keeps the play's seed.

## Implementation steps

1. Read `turn.rs`, `bots.rs`, `history.rs` and `hash.rs` in the core,
   and `replay.rs` and `bots.rs` in `crates/cli/src/contract/`, for the
   house style. If 2112 is done, read `search.rs` too.
2. Write `soak.rs`: `Codec`, `Checks`, `Check`, `Failure`, `Verdict`,
   `Shrunk`, and one private `Run` holding A, B, the hashes so far and
   the previous legal list, with `step(command)` and `finish()`. Then
   `play`, `commands` and `shrink` over it. Add `pub mod soak;` and a
   line on it in the crate docs (`crates/core/src/lib.rs`).
3. Write the fixture games and the tests below. The tests' codec is RON
   (the core's dev-dependency).
4. Write `contract/soak.rs`, the `GameTools` method, `Fake`'s field,
   `Op::Soak`, the CLI parsing and usage, `docs/contract.md` and the
   other lists of ops. Run `cargo shuyi schema --out crates/cli/schemas`
   from the repository's root, raise `VERSION` and add its line to
   `versions.json`.
5. Wire Hello, Grid and the template. `templates/` isn't in the
   workspace, so `cargo fmt --all` doesn't format it: run the template's
   gates by hand (`cargo run -p shuyi-cli -- new grid <dir>
   --engine-path <repo>`, then fmt, clippy, the tests, `cargo shuyi
   check` and `cargo shuyi soak 20 --turns 100` in `<dir>`), as CI's
   `templates` job does. Make a blank game the same way and check its
   gates still pass.
6. Add the CI lines, do the either-order parts that apply, and run the
   gates.

Size: about 350 lines in the core and 250 in the CLI, each with its
tests besides, and 80 across the template, Hello, Grid, the lists of
ops and CI. The fixture games and the generated schemas don't count.
That is near one PR's 600 lines, so measure after step 3: if the core
and its tests already pass about 350 lines, the whole won't fit. Then
stop there and keep the core here (the acceptance criteria on fixture
games, shrinking and `no_std`). Write a follow-up ticket in this PR for
steps 4 to 6, with the other acceptance criteria and the either-order
parts with 1133, 1134 and 2509. In the same PR, point the tickets that
name 2527 for the op at the follow-up: 2509's either-order clause,
1134's, and the `shuyi soak` lines of 2735, 2736 and 2737.

## Acceptance criteria

- [ ] A fixture game whose `#[serde(skip)]` field changes what a command
      does fails `reloaded`, and shrinks to exactly the two commands
      that show it (one sets the field, the next uses it).
- [ ] Every other `Check` has a fixture game that fails it, and only it,
      at the step expected. A fixture whose `Command` skips a field in
      serde fails `history`.
- [ ] A correct fixture with one side passes 50 seeds, and `soak::play`
      applies the same commands as `bots::play` with a `RandomBot` of
      the same seed. A correct fixture with two sides passes too.
- [ ] Shrinking is deterministic, every list it keeps plays as made, a
      complete shrink is 1-minimal (no single command can go), and with
      `runs` 0 a failure of one command or more comes back whole with
      `complete: false`.
- [ ] Hello, Grid's `tests/it/contract.rs` soaks 20 seeds and passes, and
      CI's `templates (grid)` job runs `cargo shuyi soak 20 --turns 100`
      in a game made from the grid template, green. The
      `templates (blank)` job doesn't run it.
- [ ] In a game made from the template, `#[serde(skip)]` on `Stage`'s
      `turn` makes the soak fail at step 0 with `invariant` on the
      reloaded copy (a manual check, noted in the PR).
- [ ] `soak --save DIR` writes a record that `shuyi replay` reads.
- [ ] The schemas test passes with the new `VERSION`, and
      `docs/contract.md` lists `soak` in both tables, as do the other
      lists of ops named in Scope.
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`, and its
      tests pass without `std` (CI's `no-std` job).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, on fixture games in `soak/tests.rs`: one per `Check`; a passing
  game; a game with two sides; `Verdict::Unplayable`; a codec that
  fails; step numbers at 0 and at `n`; the refusal probe's first four,
  and the probe on an ended game.
- Unit, in the CLI: `soak::turns` over a small `TurnRules` game (as
  `replay.rs`'s tests use `Counter`), including a `record` that fails;
  `parse_args` with and without `--save`; `text`; and
  `GameTools::soak`'s default in `game.rs`'s test of the defaults.
- Property: on a passing fixture, `play` gives the same commands twice;
  a shrunk failure still fails the same check, is no longer than its
  input, and plays as made.
- Integration: `soak` through `Fake` (passes, fails, not offered, wrong
  arguments, `--save`); Hello, Grid's 20 seeds in `tests/it/contract.rs`;
  the template in CI.

## Completion notes

*(Filled in by the session that completes the ticket.)*
