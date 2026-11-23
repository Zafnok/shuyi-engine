---
id: "2146"
title: "Parked: A game's whole rules can be a script: ScriptedTurnRules and ScriptedTickSim adapters in shuyi-script"
type: feature
milestone: "Parked: scripting"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3525"]
nick_input: none
completed:
---

# 2146 — A game's whole rules can be a script

> **Parked** (the blind-spot audit of 2026-11-23): it may start when Nick unparks it, in his answer to decision 3525 (what 1.0 is for, and whether people who write no Rust are an audience for gameplay). If he says no, the ticket is closed as not wanted, and the "Rust only" line in `crates/script/README.md` stays as it is. Nobody starts it until Nick says so. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

`shuyi-script` (ticket 2101, done;
[ADR-0093](../../docs/adr/0093-scripted-rules-rhai-integer-only-with-no-clock-no-luck-no-files-and-limits-views-in-and-values-out-by-serde.md))
runs Rhai, integers only, with no clock, no luck, no files and limits on
every run. A game's Rust rules call a script with a serde view and read
the values it returns (`Runtime::call`, `Runtime::run` in
`crates/script/src/lib.rs`). That is a hook a Rust programmer cuts. The
script can't be the rules themselves:

- `crates/script/README.md`, *What it is not for*: "The game's own
  rules. Write those in Rust, in the game's rules crate."
- `docs/modules.md`, *Scripted rules*: "None by default: a game's rules
  are Rust."
- `docs/research/2916-code-pad.md` (about line 181) names the gap: "rules
  a game writes as a script: a `TurnRules` whose `check` and `apply` call
  the script's functions … Neither exists."
- No open ticket plans it: `grep -rn "TurnRules.*script\|ScriptedTurn\|ScriptedTick" tickets/open`
  finds nothing.

Godot (GDScript), Defold and LÖVE (Lua) and GameMaker (GML) let someone
who writes no systems language make gameplay. Here the only path is
Rust. Visual scripting (1135, parked) compiles graphs to scripts, and
today those scripts have no standard place to be a game's rules.

The engine's two rules traits are in `shuyi-core`:

- `TurnRules` (`crates/core/src/turn.rs`): `check`, `apply`,
  `legal_commands`, `outcome`, with types `Command`, `Event`, `Error`,
  `Outcome`. `History` (replay, rewind, saving a game in progress), the
  bots (`Bot`, `Sides`) and lockstep are written against it.
- `TickSim` (`crates/core/src/tick.rs`): `step(&mut self, &TickInput) ->
  Vec<Event>` and `hash`. The audit called it "TickRules"; the trait is
  `TickSim`, so the adapter is `ScriptedTickSim`.

This ticket adds an adapter for each, so a game's rules crate can be a
few lines of Rust (the types and the file) and a script. The state stays
a Rust value, so saves, `state_hash`, replays and lockstep work as for
any Rust rules. A script still changes nothing directly: it returns the
new state and the events as values (ADR-0093's views in, values out),
and the adapter applies them all or nothing.

Found by the blind-spot audit of 2026-11-23 (scripting, hot reload and
modding). Decision 3525 asks Nick whether script-written rules are for
1.0.

## Nick input

None here. The product question is decision 3525.

## Scope

**In:**
- `shuyi_script::rules::ScriptedTurnRules<S, C, Ev, E, O>`: implements
  `shuyi_core::TurnRules` by calling the script's functions `check`,
  `apply`, `legal_commands` and `outcome` with `#{ state, command }` as
  the view. `S` is the game's state type (`Serialize +
  DeserializeOwned + Clone`), the others its command, event, error and
  outcome types (`Serialize + DeserializeOwned`).
- `ScriptedTurnRules` also implements `shuyi_core::bots::Sides` when the
  script defines `side_to_move` and `turn`, so `bots::play` and the
  search bots run over it (a further type parameter for the side,
  `Copy + Eq + Debug + DeserializeOwned`).
- `shuyi_script::rules::ScriptedTickSim<S, Ev>`: implements
  `shuyi_core::TickSim` by calling `step` with `#{ state, input }`;
  `hash` is `state_hash` of `S`.
- The call shape, written in the new ADR and the README: `apply` returns
  `#{ state: <new state>, events: [...] }` or `#{ refused: <error> }`;
  `check` returns `()` or `#{ refused: <error> }`; `legal_commands` a
  list; `outcome` `()` or a value; `step` `#{ state, events }`.
- A script that fails (a compile error is caught at `new`; a run error, a
  limit, or a value that doesn't read) changes nothing:
  - `TurnRules::Error` is `Refusal<E>`: `Refused(E)` (the script
    refused) or `Script(ScriptError)` (the script failed).
  - `TickSim::Event` is `TickEvent<Ev>`: `Game(Ev)` or
    `Failed(ScriptError)`. The state stays as it was, so the step is
    still deterministic.
  - `legal_commands` and `outcome` have no error in their signatures:
    a failure gives an empty list and `None`, and `last_error()` holds
    the error. The ADR records this.
- `new(runtime: Rc<Runtime>, script: Script, state: S)` checks that the
  script defines each function it needs (`Script::has_function`) and
  names the missing one in its error. `Runtime` isn't `Send`, and the
  bots need `Clone`, so the adapters share the runtime by `Rc`.
- The new edge `shuyi-script` → `shuyi-core` in `tools/layers.ron`
  (today its `engine` list is empty).
- An example, `examples/scripted-rules/`: a small turn game whose rules
  are one `.rhai` file and a 30-line Rust crate of types, with a replay
  test through `History`, a bot game through `bots::play`, and a
  `frame-png` picture.
- A new ADR (the `write-adr` skill) that amends ADR-0093 §6: the call
  shape, the error mapping and the cost.
- Docs: `crates/script/README.md` (*What it is not for* loses "the
  game's own rules"; a section shows both adapters), `docs/modules.md`
  (*Scripted rules*), and the book's scripting chapter if there is one.
- New public types are `#[non_exhaustive]` where they may grow
  (ADR-0039): `Refusal` and `TickEvent` are, with constructors; the two
  adapters are structs with private fields.

**Out (do not do):**
- Luck inside a script. ADR-0093 gives scripts no random numbers, and
  this ticket keeps that. A scripted game that needs luck keeps a seed in
  its state and draws in the script with integer maths. A later ticket
  may hand the script draws from the state's `SimRng`.
- Scripts attached to scene entities or ECS systems (2221, 2107). That
  is a later ticket, written once this one shows the shape.
- A script as an asset kind that hot reloads: that is 2145. The adapters
  take a compiled `Script`, so a reload builds a new adapter from the
  same state.
- Helper functions a game registers, and imports between scripts: that
  is 2144.
- Visual scripting (1135, parked): its graphs may compile to the
  functions this ticket calls, which its ADR decides.
- 2102's example (one scripted rule inside Rust rules) stays as it is.
- Any change to `TurnRules` or `TickSim` themselves.

**For the game:** additive. Nothing in the game must change, and no pin
bump follows.

**Estimated size:** about 1,800 lines (code 500, tests 600, example 400,
docs and ADR 300).

**Split:** past 900 lines, split off `ScriptedTickSim`, its tests and its
README section (steps 4 and part of 7) into a new parked ticket.

## Implementation steps

1. Add `shuyi-core` to `shuyi-script`'s dependencies (no default
   features beyond what `TurnRules`, `TickSim` and `state_hash` need), and
   its row in `tools/layers.ron`. Run `cargo xtask check-layers`.
2. `crates/script/src/rules.rs`: the view types sent in (`TurnView<'a, S,
   C>`, `StepView<'a, S>`) and the value types read back (`Applied<S,
   Ev, E>`, `Checked<E>`, `Stepped<S, Ev>`), all serde, private.
3. `ScriptedTurnRules`: `new` (checks the four functions), `state()`,
   `last_error()`, and the `TurnRules` impl. `apply` deserialises the
   whole reply before it replaces the state, so a bad value changes
   nothing. Implement `Clone`.
4. `ScriptedTickSim`: `new` (checks `step`), `state()`, the `TickSim`
   impl with `TickEvent`.
5. Re-export both under `shuyi::script` (the `script` feature).
6. `examples/scripted-rules/`: the game, its `.rhai` file, its tests
   (replay, a bot game, a refused command, a failing script that changes
   nothing), and its `frame-png` picture.
7. Measure the cost of one `apply` and one `step` on the example's state
   (natively, release build) and write it in the README beside the
   existing third of a microsecond.
8. The ADR, the README, `docs/modules.md` and the book (the `document`
   skill).

## Acceptance criteria

- [ ] A turn game whose rules are only a script plays through
      `History`: the replay of its commands gives the same
      `state_hash` at each command.
- [ ] `bots::play` runs a bot game over `ScriptedTurnRules` to an
      outcome.
- [ ] A command the script refuses returns `Refusal::Refused` and the
      state's hash doesn't change.
- [ ] A script that errors, hits `Limit::Operations`, or returns a value
      that doesn't read leaves the state's hash unchanged, in both
      adapters, and `TickEvent::Failed` carries the error.
- [ ] `new` on a script without `apply` (or `step`) fails and the error
      names the missing function.
- [ ] A `ScriptedTickSim` run recorded in a `TickRecord` replays to the
      same hashes.
- [ ] `cargo xtask check-layers` passes with the new edge.
- [ ] The README states the measured cost of `apply` and `step`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each reply shape read back; each failure kind leaves the state
  unchanged; the missing-function error at `new`.
- Property: for generated command lists, `ScriptedTurnRules` over a
  small script agrees with the same rules written in Rust (state hash and
  events after each command).
- Snapshot / integration: the example's replay test, bot game and
  `frame-png` picture; a row in the crate's golden table
  (`crates/script/tests/golden/scripts.ron`) for the example's final
  state hash, checked on wasm32 by CI's `determinism` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
