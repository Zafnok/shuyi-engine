---
name: shuyi-debug-replay
description: Reproduce a bug of a game on Shuyi Engine from a record of its rules (its seed and commands), find where two runs part by the state's hash, and keep the record as a test. Use when a play went wrong and its record (from the bots, a test or a player) can show it, when two runs of one record disagree, or when a game's replays stop matching. Not for the checks before a commit (shuyi-gates), for writing tests from scratch (shuyi-test), or for changing the rules themselves (shuyi-stage-grid).
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# Reproduce a bug from a record

A turn game's rules are deterministic: the same seed and the same
commands always play the same game. So a **record**, the seed and the
commands, is the whole play. `cargo shuyi replay` re-runs a record by
the game's own rules and answers with a hash of the state at every
step. Two runs that should agree and don't part at the first step whose
hashes differ, and that step's command is where to look.

## When to use it

- A play ended wrongly, and you have its record or can make one (`bots`
  writes one per play).
- Two runs of one record disagree: the rules own something they
  shouldn't (an unordered map, a clock, luck of their own).
- Records made before a change no longer play the same, and you need to
  know from which step.

## When not to

- The checks before a commit: `shuyi-gates`.
- A test of something that never had a record: `shuyi-test`.
- Writing the rule that fixes the bug: `shuyi-stage-grid` (this skill
  finds the step; that one changes the rules).

## Steps

1. **Get the record.** A game's record is in its own format; the `grid`
   template's is a `StageRecord`, `(seed: 0, commands: [Step(Right),
   Step(Up)])`. The bots' plays carry theirs:

   ```bash
   cargo shuyi bots 20 --json
   ```

   Each of `result.plays` has its `seed`, `outcome` and `record`: write
   the record of the play that went wrong to a file. The bots stamp each
   record with what made it (`made`: the engine's version, the game's
   name and version, and a fingerprint of its files).

2. **Replay it:**

   ```bash
   cargo shuyi replay records/bug.ron --json
   ```

   The record runs twice. `result.runs[0].steps` has step 0 (the state
   it starts in) and one step per command: the `command`, its `events`,
   why the rules `refused` it if they did, and the state's `hash` after
   it. `result.runs[0].outcome` is how the game ended. Read the steps to
   find the command where it goes wrong.

3. **Compare two runs** when one record should play like another (the
   same game on two machines, a record made before and after a change):

   ```bash
   cargo shuyi replay records/before.ron records/after.ron --json
   ```

   `result.first_difference.step` is the first step whose hashes differ
   (or that only one run has), with both `hashes`. The command at that
   step is the first one that played differently. The answer passes
   when there is none.

4. **Keep the record as a test** in the rules crate: the record's seed
   and commands, applied to the state, and an assert of what must hold
   (an event, a refusal, an outcome). It fails while the bug is there.

5. **Fix the rules**, run the test, then replay the record again: it now
   plays as it should. A fix that changes the state changes its hashes,
   so records made before it part from the new runs at the step the fix
   changed: that is expected, and `first_difference` shows where. A fix
   in the game's files changes its fingerprint too, so `replay` refuses
   the records made before it (`SH3007`, below) until you pass
   `--any-stamp`.

## Records and hashes

- **A record is the game's.** Its tools read it in `GameTools::replay`
  and hand the rules' first state and the commands to
  `shuyi_cli::contract::replay::turns`, which re-runs them over any
  `TurnRules` state and hashes each step. A refused command changes
  nothing and says why; the answer's `limits` count the refusals ("the
  record doesn't play as it was made").
- **The hash is `shuyi_core::state_hash`**, over the state's
  `Serialize` form: the same on every platform and every run. It is the
  same each run only if the state serializes the same: its maps must be
  `BTreeMap` and its sets `BTreeSet`. A field left out of the state's
  serde form is left out of its hash too.
- **One record, run twice, that differs** means the rules own something
  a run doesn't control: a hash map's order, a float, the clock, luck
  that isn't the state's `SimRng`. The rules crate's fence (`cargo
  shuyi lint`, `shuyi-gates`) bans each of them; look there first.
- **A record says what made it.** A record with a stamp
  (`shuyi_core::Made`, a `made` field of the game's record, which its
  `replay` passes on) made by another engine, another build of the game
  or other files is refused before anything is compared: its runs could
  differ for that alone, which isn't a desync. `--any-stamp` runs it
  anyway, and the `limits` say a difference may come from the stamp. A
  record with no stamp (one written by hand, or before stamps) runs, and
  the `limits` say it has none. A game's tools give this build's stamp
  in `GameTools::made`, most simply `Made::new(name and version,
  bundle.fingerprint())`, as the `grid` template's `made()` does.
- **What differs from one machine to another** needs a second record
  made on the other machine: one process can't see it, and the answer's
  `limits` say so.
- A record replays the rules, not the screens: it says nothing of what
  was drawn. Look at a frame with `cargo shuyi frame-png` (`shuyi-test`).

Reproducing a player's crash from a crash report is still to come: today
a bug is reproduced from a record of the rules.

## When it fails

Branch on the `code`, and read its page with `cargo shuyi explain
<code>`:

- No code, and `ok` false: the runs differ. Read `first_difference`.
- `SH3007`: the record was made by another build or with other files;
  the error names each part that differs. Make the record again, or
  pass `--any-stamp` to see how it plays now.
- `SH3004`: the record's file isn't there (its path is from the game's
  folder).
- `SH3002`: the game's tools failed, most often on a record that
  doesn't read in the game's format; the error says where.
- `SH3001`: the game's tools give no `replay` (or no `bots`): a stage's
  `GameTools` writes them.
- `SH3005`: the game's tool program didn't build.

## Worked example

[`example.ron`](example.ron) runs on a new `grid` game: a bot's play is
written to `records/bot.ron` and replays the same twice, with its stamp;
two records of one seed, one of them stepping into a wall, part at step
2, where the rules refused the step; a record stamped with other content
fails with `SH3007` and runs with `--any-stamp`; a record that isn't
there fails with `SH3004`;
and the refused step becomes a test in the rules crate, which passes
with `fmt` and clippy.
