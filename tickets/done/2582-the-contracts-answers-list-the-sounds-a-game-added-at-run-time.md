---
id: "2582"
title: "The contract's answers list the sounds a game added at run time"
type: feature
milestone: "Engine: demos"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 2582 — The contract's answers list the sounds a game added at run time

## Context

Ticket 2249 (done) let a game make a sound at run time from samples:
`AudioQueue::add_sound(cue, samples)` makes them a cue, and
`remove_sound(cue)` frees it ([ADR-0087](../../docs/adr/0087-sounds-made-at-run-time-samples-as-a-cue-through-the-queue-which-keeps-the-limits-and-a-provided-backend-method.md)).
The headless `Speaker` keeps them, and the Harness reads them
(`Harness::sounds_added`, `Harness::added_plays`).

**From 2249 (done):** the tool contract doesn't show them yet. `run`'s and
`events`' `sounds` list each play of an added cue as they list any cue's
(`crates/cli/src/contract/session.rs`, `fn sounds`), but no answer says
which cues the game made, or what their samples are. An agent playing a
game that makes its sounds (a synth toy, the DOOM port, 2780) can't tell
an added cue from a missing one.

## Nick input

None.

## Scope

**In:**
- A field of `run`'s answer (and `events`', which shares `sounds`) listing
  the sounds added and not removed at the end of the run: each one's cue,
  rate, channels, frames and milliseconds, in cue order. Never the
  samples. Additive, with a default when absent (ADR-0039), and the
  contract's version moved as `docs/contract.md` says a new field does.
- The `Session` trait's method for it, with a default of none, read from
  `Harness::sounds_added` and the speaker's samples.
- The schemas regenerated, `docs/contract.md`'s answer section, and the
  book's chapter 10 (tools for agents) if it lists `run`'s fields.

**Out (do not do):**
- The samples themselves in an answer, or a picture of them.
- Anything of how a game adds a sound (2249 did it).

## Implementation steps

1. Read `crates/cli/src/contract/run.rs` (`RunOutput::sounds`),
   `crates/cli/src/contract/session.rs` (`Session::sounds`, `Sound`) and
   `crates/platform/src/headless.rs` (`Speaker::added_sounds`).
2. Add the type (`AddedSound`, `#[non_exhaustive]`, `Serialize`,
   `Deserialize`, `JsonSchema`) and the field, with
   `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
3. Fill it from the Harness in the session; the fake session gives none.
4. Regenerate the schemas, update `docs/contract.md` and the printed form
   for people (`run`'s text: a line `sounds added: …` when any).

## Acceptance criteria

- [x] A game that adds a sound and runs answers `run` with it in the new
      field: its cue, rate, channels, frames and milliseconds.
- [x] A sound removed before the end of the run isn't listed.
- [x] An answer with no sounds added is the same JSON as before.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the field from a session whose game adds two sounds and removes
  one.
- Snapshot / integration: `run --json` on a test game that adds a sound,
  the field in the answer; the schema test that every answer matches its
  schema.

## Completion notes

- `shuyi_cli::contract::session::AddedSound` (`#[non_exhaustive]`): a
  sound's `cue`, `rate`, `channels`, `frames` and `millis`, never its
  samples. `Session::sounds_added` gives them, none by default; the
  headless session reads them from its speaker's
  `Speaker::added_sounds`, in cue order (the samples' own `rate`,
  `channels`, `frames` and `millis`).
- `RunOutput::sounds_added` and `EventsOutput::sounds_added`, with
  `#[serde(default, skip_serializing_if = "Vec::is_empty")]`: an answer
  with none is the JSON it was. `step`'s answer is `run`'s, so it has the
  field too; an attached session (the live link) gives none.
- `run`'s text has a line `sounds added: high (44100 samples a second, 2
  channel(s), 441 frames, 10 milliseconds)` when any (units written in
  full, as the engine's lint wants, rather than `Hz`).
- The contract's version is 38; the schemas regenerated
  (`run.output`, `session.events.output`, `session.step.output`).
- `docs/contract.md` (`run`'s and `events`' rows, the answer's example
  version) and the book's chapter 10 (`events`' row) say it.
- Tests: in `contract/session/tests.rs`, a session whose game adds two
  sounds and removes one on the next frame; `run`'s JSON answer with the
  field, its keys those of the schema, read back equal; an answer with
  none has no field and reads back equal. The `run --json` test drives
  `run_output` on a headless session of a test screen rather than the
  toy's tool program, as the toy adds no sound and the ticket keeps the
  toy as it is.

No follow-up tickets.

Breaking: shuyi-cli: mcp::EventsOutput: a new public field, `sounds_added`; a literal of it adds it.

```migration
since: 0.1.0
what: EventsOutput has a new field, sounds_added, the sounds the game added at run time.
old: EventsOutput { frames, events, sounds }
new: EventsOutput { frames, events, sounds, sounds_added }
before: |
  EventsOutput { frames, events, sounds }
after: |
  EventsOutput { frames, events, sounds, sounds_added: Vec::new() }
find: ["EventsOutput { frames, events, sounds }"]
```
