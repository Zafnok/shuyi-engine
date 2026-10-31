---
id: "2772"
title: "Parked: A soak of command sims: a CommandSim stepped as two copies, one saved and reloaded at every step, and a failure shrunk to a short record"
type: feature
milestone: "Engine: real-time strategy"
model: implementation
effort: high
status: blocked
blocked_by: ["2706", "2527"]
nick_input: none
completed:
---

# 2772 — A soak of command sims: reloaded every step, a failure shrunk

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches for all of these"): a follow-up of parked work is parked, and this one follows 2706. It waits until after Visions of Shuyi launches, which is later than the MVP in `docs/ROADMAP.md`. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2527 adds `shuyi_core::soak`. Two copies of a `TurnRules` game play the
same commands. One is written and read back through a `Codec` before
every step, as a save would be, and the two must stay equal. That
catches the state a save leaves out: a `#[serde(skip)]` field, a cache,
anything the serde form doesn't hold. `state_hash`, `bots` and `replay`
can't see that state, since they rebuild a game by replaying its
commands. A failure shrinks by delta debugging to a short list of
commands that still shows it. 2527 checks turn games only, and its Out
line sends command sims here.

[ADR-0015](../../docs/adr/0015-real-time-strategy-at-scale-command-sims-columns-flow-fields-and-ai-in-layers.md)
§2 adds a third shape of simulation, which 2706 builds: a `CommandSim`
steps 60 times a second, and its input is timed commands (`Timed<C>`),
kept in a `CommandRecord` that `replay_commands` plays. 2706's Out line
leaves this soak to this ticket.

A command sim needs the check more than a turn game does. A real-time
strategy game keeps a lot of state it can rebuild: grids of who is
where, cached paths, flow fields. Each is a field a save may leave out,
and a game that rebuilds it a little differently after a load drifts
from a copy that never loaded. Factorio's "heavy mode", which 2527
cites, is this check on a game of this shape: it saves and reloads the
game every tick.

What changes from 2527's soak:

- **No legal commands.** A `CommandSim` has no `legal_commands` and no
  `check`, so no bot picks its commands. The caller hands in a
  `CommandRecord`: one a Harness run made (`harness.command_record()`),
  one a `CommandDriver` made (`record()`), a saved one, or one it
  builds with `CommandRecord::push`.
- **Every command plays.** `CommandSim::step` returns no error, so a sim
  takes any command at any step, and an order it can't carry out does
  nothing. A shrunk record always plays, and there is no `Unplayable`.
  (A sim that panics on an order for a unit that isn't there has a bug
  of its own. The soak doesn't catch panics; in a test, a panic fails
  the test.)
- **A failure is a record.** Commands are timed, so a failure carries a
  `CommandRecord` cut after its step, not a bare list of commands.

## Nick input

None.

## Scope

**In:**

**`shuyi_core::soak::command`** (`crates/core/src/soak/command.rs`,
tests in `crates/core/src/soak/command/tests.rs`), `no_std` with
`alloc` (no `std::` path, which `cargo xtask check-no-std` checks),
beside 2527's soak of turn games. No new dependency.

- **Reused from 2527:** `Codec`; `Checks` (the codec and the game's
  invariants, a `fn(&S) -> Vec<String>`); and `Check`. If 2527
  bounded `Checks`' type by `TurnRules`, move that bound to the
  functions that need it. A looser bound breaks no caller.
- **Five of `Check`'s checks apply:** `hash`, `round-trip`, `reloaded`,
  `invariant`, and `record` (the run's record, written and read by the
  codec and replayed, gives another hash at some step, or doesn't go
  through the codec). The others ask `check`, `legal_commands` or
  `outcome`, which a `CommandSim` doesn't have. Each variant's docs say
  which soak gives it; these five add command sims.
  - **Either order with 2549:** if 2549 is done, `Check::Record` and
    the shared ddmin are there: use both, and add command sims to
    `Record`'s docs. If not, add `Check::Record` (`record`) here as
    2549 writes it, with its name, its place in `ALL` and its docs,
    and share the ddmin as implementation step 2 says; 2549 then uses
    both.
- **A `Failure` and a `Shrunk` of their own.** 2527's `Failure` counts
  commands in a `u32` and holds a bare list of them. A command sim
  counts steps in a `u64`, and a failure must keep the steps after the
  last command too, so here a failure holds a record.
- `Failure<C> { check: Check, step: u64, detail: String, record:
  CommandRecord<C> }`:
  - `step` is the step whose checks failed. A check before step `s`
    runs, or while it runs, is at `s`. A check on the last state is at
    `record.steps`. The `record` check is at the first step whose
    replayed hash differs, or at `record.steps` if the codec refuses
    the record;
  - the `record` field is the run's record cut after the failing step.
    Its `steps` is `step + 1`, at most the run's own; the commands due
    later are dropped; `seed` and its other fields are kept. `run` over
    it fails the same `Check` at the same step;
  - `detail` names the copy ("the copy never reloaded" or "the
    reloaded copy"), the step's commands, and the hashes, the events or
    the invariant lines, with `Debug`.
- `run(start: &S, record: &CommandRecord<S::Command>, checks:
  &Checks<'_, S, K>) -> Result<Vec<u64>, Failure<S::Command>>`, for `K:
  Codec`: plays `record.steps` steps as **One step** says, then the end
  checks. On a pass it returns the hash after each step, the list
  `replay_commands` returns. The caller builds `start` from
  `record.seed`, as it does for `replay_commands`.
- `shrink(start: &S, failure: Failure<S::Command>, checks, runs: u32)
  -> Shrunk<S::Command>`: delta debugging (ddmin) over the timed
  commands of `failure.record`, with the ddmin 2527's `shrink` uses
  (implementation step 2). A candidate is that record with some
  commands dropped; its `steps` stays. It counts only if `run` fails it
  with the same `Check`, and then its own `Failure`, cut at its own
  step, is the one kept. Commands are only dropped: never moved to
  another step or player, never changed. At most `runs` candidates are
  run.
- `Shrunk<C> { failure: Failure<C>, played: usize, complete: bool }`:
  the shrunk failure, the number of commands it started from, and
  whether it finished within `runs`.
- Bounds: `S: CommandSim + Clone + Serialize + DeserializeOwned`;
  `S::Command: Debug`, beside the `Clone + Eq + Serialize +
  DeserializeOwned` that 2706's `CommandSim` already asks;
  `S::Event: PartialEq + Debug`.
- No re-export at the crate's root: a game writes
  `shuyi_core::soak::command::run`.

**One step, exactly.** The module docs say it in this order. Copies A
and B start as clones of `start`. For each step `s`:

1. The invariants run on A (`invariant`, "the copy never reloaded").
2. B goes through the codec. The codec must write and read B, and the
   copy's `hash()` and `state_hash` must equal B's (`round-trip`;
   `hash` if a `state_hash` refuses). The invariants run on the copy
   (`invariant`, "the reloaded copy"). B becomes the copy.
3. A and B step with `record.due(s)`. Their events must be equal, and
   after the step their `hash()` and their `state_hash` must be equal
   (`reloaded`; `hash` if a `state_hash` refuses). A's `hash()` goes in
   the list `run` returns.

After the last step, 1 and 2 run once more on the last state. Then the
record goes through the codec, and `replay_commands` over a clone of
`start` and the record read back must give A's list (`record`). If
`replay_commands` also takes a `Made`, pass the record's own `made`:
the soak tests the state, not the stamp.

Why `state_hash` beside the sim's own `hash()`: a game's `hash()` may
leave fields out to be fast, and two copies that differ only there
still make different saves.

**Nothing here chooses commands or draws luck.** The soak plays the
record it is handed. It never draws from the sim's `SimRng`.

**Docs.** The module docs of `soak::command`, with a doc example on a
tiny sim, and a line in `soak`'s module docs and in the crate docs
(`crates/core/src/lib.rs`). Follow the `document` skill.

**The public API.** `soak::command` and its items are new, so they are
additive, and so is `Check::Record` if this ticket adds it (`Check` is
`#[non_exhaustive]`, 2527). If `Check::ALL` is an array with its length
in its type, a longer one is a break: write the `Breaking:` line
(ADR-0038). Follow ADR-0039: `Failure` and `Shrunk` are
`#[non_exhaustive]` with public fields, as 2527's are.

**Out (do not do):**
- a `soak` operation in the tool contract, or `shuyi soak` over a
  command sim. The contract replays no command record today (2770,
  parked, adds that to `replay`). `run` and `shrink` are what such an
  op would call; no ticket plans one;
- making commands: no random commands and no bot. The records come
  from the caller: a person's Harness run, or a run with a game's
  computer players (2714, 2715);
- tick sims: 2549 (parked) does the same for a `TickSim`;
- the checks that need `TurnRules` (2527's `check-changed`,
  `check-disagrees`, `legal-refused`, `legal-order`,
  `over-with-commands`, `refusal-changed` and `stuck`);
- desyncs between peers: lockstep's hash check (2720) and several peers
  in one run (2529);
- a reload every N steps instead of every step: a caller with a long
  run hands in a shorter record;
- soaking a module's or an example's sim in its tests or CI;
- a game's own save files: a game whose saves aren't its sim in RON
  writes its own `Codec`, as 2527 says;
- shrinking by moving or changing commands, or with another seed.

## Implementation steps

1. Read 2527's `crates/core/src/soak.rs` and `soak/tests.rs`; 2706's
   command sims in `crates/core/src/tick.rs` (or `tick/command.rs`):
   `Timed`, `CommandSim`, `CommandRecord` and `replay_commands`; and
   `crates/core/src/hash.rs`.
2. **In `soak.rs`.** Share the ddmin: if `soak` already has a generic
   one that 2527's `shrink` calls, use it. If 2527 wrote it inside
   `shrink`, move it out to a private `fn ddmin<T: Clone>(items:
   Vec<T>, runs: u32, fails: impl FnMut(&[T]) -> bool) -> (Vec<T>,
   bool)` and make 2527's `shrink` call it. Don't copy it. Move any
   `TurnRules` bound off `Checks`, and add `Check::Record` unless it is
   there (the clause with 2549). Run 2527's tests: they pass unchanged.
3. Write `soak/command.rs`: `Failure`, `Shrunk`, and a private `Pair`
   holding A, B and A's hashes, with `step(s, due)` and `finish()`.
   Then `run` and `shrink` over it. Add `pub mod command;` to
   `soak.rs`.
4. Write the fixture sims and the tests below. The tests' codec is RON
   (the core's dev-dependency), as in 2527's tests.
5. Write the docs, and run the gates with `cargo xtask check-no-std`.

Size: about 200 lines in the core and 250 of tests, besides the fixture
sims.

## Acceptance criteria

- [ ] A fixture sim whose `#[serde(skip)]` field is set by one command
      and read by a later one, with a `hash()` over its serde form,
      fails `reloaded` at the later command's step. From a record of 20
      commands or more, it shrinks to exactly those two, each at its
      own step.
- [ ] The same sim with a `hash()` that covers the field fails
      `round-trip` at the step after the first command's, and shrinks
      to that one command.
- [ ] Each of `hash`, `round-trip`, `reloaded`, `invariant` and
      `record` has a fixture sim that fails it, and only it, at the
      step expected. A fixture whose `Command` skips a field in serde
      fails `record`. An invariant broken only on the reloaded copy
      says so in `detail`.
- [ ] A correct fixture sim passes 50 random records, and `run` returns
      the hashes `replay_commands` returns over the same record
      (property test).
- [ ] A failure's `record`, run again, fails the same `Check` at the
      same step.
- [ ] Shrinking is deterministic; a complete shrink is 1-minimal (no
      single command can go); a shrunk record has no more commands and
      no more steps than its input; and with `runs` 0 a failure comes
      back whole with `complete: false`.
- [ ] 2527's soak tests pass unchanged with the shared ddmin, and
      `Check::ALL` lists `record` once.
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`, its tests
      pass without `std` (CI's `no-std` job), and it has no float
      arithmetic outside tests (`clippy::float_arithmetic`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, on fixture sims in `soak/command/tests.rs`: one per check used;
  a correct sim; a record with steps after its last command; a record
  with no command; step numbers at 0 and at `record.steps`; a codec
  that fails; a failure's record run again.
- Property: on a correct sim, `run` gives what `replay_commands` gives
  over random records; a shrunk failure fails the same check, is no
  longer than its input, and comes out the same when shrunk twice.

## Completion notes

*(Filled in by the session that completes the ticket.)*
