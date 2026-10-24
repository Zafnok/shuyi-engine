---
name: shuyi-net
description: Online play for a game on Shuyi Engine, by lockstep and sessions, through the engine's online module. The module isn't built yet. This skill says how a turn game stays ready for it (deterministic rules, commands as the only input, a state hash, records that replay) and how to find a desync by hand today. Use when someone asks for online or networked play, or when shaping a game that will go online later. Not for local play (shuyi-stage-grid) or a replay that differs from one run to the next (shuyi-debug-replay).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
  waits_for: "1140 2529 1145"
---

# Online play

**Not built yet.** Online play is a module of the engine, turned on with
`cargo shuyi add net` once it is built. `cargo shuyi modules` lists its
answers under Online play (`lockstep`, `a referee server`) as `parked`
today, and `add` refuses it with `SH0003`. When it is `ready`, this
skill gets its steps and a worked example. Until then, tell the person
who asked that online play isn't in the engine yet, and keep the game
ready for it.

## When to use it

- Someone asks for online, LAN or networked play.
- A turn game is being shaped, and will go online later.
- Two players' games came out different, and you need to find where.

## When not to

- Two players on one machine, taking turns: an ordinary turn game
  (`shuyi-stage-grid`).
- A record that replays differently on its own: `shuyi-debug-replay`.

## How lockstep will work

Each peer runs the same rules from the same seed and sends only its
commands. Every peer applies every command in the same order, so every
peer's state is the same, and each turn they compare a hash of it. A
hash that differs is a desync: the match stops, both sides keep their
record, and `cargo shuyi replay` finds the first step where the two
records part. Joining late or coming back is playing the record again.

Lockstep between peers is for friends: every peer holds the whole state,
the seed included, so it can see what the others can't. A game with
hidden information, or one ranked, needs a server that holds the seed
and the hidden state and sends each player their view. Plan for that
before promising a ranked mode.

## Steps, meanwhile

A turn game that keeps these rules can take lockstep without a rewrite:

1. **The state changes only through commands.** The rules implement
   `TurnRules`: `apply` takes a command, and nothing else moves the
   state. The screen turns the player's actions into commands.
2. **The rules are deterministic.** The rules crate's fence
   (`shuyi-gates`): no floats, no clock, no I/O, `BTreeMap` for maps,
   and the only luck a `SimRng` kept in the state. The same seed and
   the same commands always give the same game.
3. **The state hashes.** `shuyi_core::state_hash` over the state (it is
   `Serialize`) is the same on every machine and every run when its
   maps are ordered.
4. **A record is the seed and the commands.** The game's tools give
   `replay` (`GameTools::replay`; `replay::turns` re-runs one over any
   `TurnRules`), so two records compare:

   ```bash
   cargo shuyi replay a.ron b.ron --json
   ```

   The answer's hashes per step, and the first step where the two runs
   differ, are what a desync report will show.
5. **Commands are plain data** (serde), so they can cross a network as
   they cross a file.

## When it fails

Branch on the answer's `code`, never its message, and read its page
with `cargo shuyi explain <code>`:

- `SH0003`: `cargo shuyi add net` before the module is built: online play
  isn't in the engine yet.
- `SH2103`: the rules use what keeps them from being the same on every
  machine (a float, a clock, a `HashMap`): fix it now, or lockstep
  desyncs later.
- `replay` answers `ok: false` with no code when two runs differ: its
  answer names the first step where they part.

## Worked example

None yet: it comes with the engine's online module.
