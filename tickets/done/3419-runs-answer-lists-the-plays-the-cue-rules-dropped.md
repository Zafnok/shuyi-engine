---
id: "3419"
title: "run's answer lists the plays the cue rules dropped"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2345"]
nick_input: none
completed: 2026-11-23
---

# 3419 — `run`'s answer lists the plays the cue rules dropped

## Context

2345 (cue rules as data, ADR-0139) runs a sound cue's rules in
`shuyi-platform`'s `App`: a play less than its cue's `min_gap_ms` after
the last play that sounded is dropped. `App::dropped_sounds` gives a
frame's drops as `DroppedSound { cue, why: DropWhy }`
(`crates/platform/src/audio/cues.rs`), and the Harness records them
with their frames (`Harness::dropped_sounds() -> Vec<(u32,
&DroppedSound)>`, cleared by `clear_audio`). `DropWhy::word` gives each
reason's name in the contract (`gap` today).

2345 planned to answer them in `run` too, but its PR passed 1,500
counted lines (`cargo xtask pr-size`), and this part lands apart from
the rest: until it does, an agent sees a dropped play under `sounds`
only, as before. 2378 (cue rules, part two) adds the reasons `copies`,
`voices` and `cut`, and its contract clause reads them from here.

## Nick input

None.

## Scope

**In:**
- `Dropped { frame: u32, cue: String, why: String }` beside `Sound` in
  `crates/cli/src/contract/session.rs` (`Serialize`, `Deserialize`,
  `JsonSchema`, `deny_unknown_fields`), `why` being `DropWhy::word`.
- `Session::dropped_sounds(&self) -> Vec<Dropped>`, with a default body
  that answers none (ADR-0039 rule 7), implemented by `Headless` from
  `Harness::dropped_sounds` and by `FakeSession`
  (`crates/cli/src/contract/fake.rs`, which lists two clicks in frame 1
  under `sounds` and the second under `dropped`).
- `RunOutput::dropped: Vec<Dropped>`, `#[serde(default,
  skip_serializing_if = "Vec::is_empty")]` as `sounds_added` is, filled
  in `run::run_output` (which the MCP server's `step` answers through
  too, 1133) and empty in `crates/cli/src/mcp/attached.rs`'s answer. The
  text answer gets a line `dropped: click (frame 5, gap)` when there are
  any, after `sounds:`.
- The contract's version (`crates/cli/src/contract.rs`) one over
  `main`'s, the schemas regenerated (`cargo shuyi schema --out
  crates/cli/schemas`: `run.output.json` and `session.step.output.json`
  change) and the line in `crates/cli/schemas/versions.json` (ADR-0022
  §5).
- `docs/contract.md`: `run`'s `dropped` (each one's frame, cue and why;
  each is under `sounds` too, as the game asked for it; left out when
  there are none).
- **2378's clause:** if 2378 is done, `why` gives `copies`, `voices` and
  `cut` through `DropWhy::word` with no change here; if not, nothing to
  do (2378 does it).

**Out (do not do):**
- any change to the cue rules or to the Harness: 2345 and 2378;
- a `dropped` list in `events` or another operation's answer.

**Estimated size:** about 130 lines.

## Implementation steps

1. Read ADR-0139, `crates/platform/src/audio/cues.rs` and
   `Harness::dropped_sounds`, then `crates/cli/src/contract/run.rs`
   (`RunOutput`, `run_output`, `text`), `session.rs` (`Sound`,
   `Session`, `Headless`) and `fake.rs`.
2. `Dropped`, `Session::dropped_sounds`, its two implementations,
   `RunOutput::dropped` and the text line; fix the `RunOutput` literals
   (`crates/cli/src/mcp/attached.rs`, `crates/cli/src/contract/run/tests.rs`).
3. The version, the schemas and `versions.json`; `docs/contract.md`.

## Acceptance criteria

- [x] With the toy game's `click` given `min_gap_ms: 40` (`Toy::audio`
      is public; `CueRules::DEFAULT.with_min_gap_ms(40)`), `keys f f f`
      on a `Headless` session answers `dropped: [{frame: 5, cue:
      "click", why: "gap"}]`, both clicks under `sounds`, and the JSON's
      `dropped` entry has the keys the schema's `Dropped` lists.
- [x] An answer with nothing dropped has no `dropped` field, and reads
      back as itself.
- [x] The fake session's text answer has `sounds: click (frame 1), click
      (frame 1)` and `dropped: click (frame 1, gap)`.
- [x] `contract::VERSION` is one over `main`'s, with the schemas and
      `versions.json` updated, and the schemas' test passes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: `run_output` over the toy game with a gap (above); the
  fake session's text answer.
- Snapshot: the contract's schemas.

## Completion notes

- `Dropped { frame, cue, why }` beside `Sound` in
  `crates/cli/src/contract/session.rs`, `#[non_exhaustive]` as
  `AddedSound` is (an answer's type that may grow, ADR-0039).
  `Session::dropped_sounds` answers none by default; `Headless` reads
  `Harness::dropped_sounds`, and `FakeSession` lists two clicks in frame 1
  and drops the second.
- `RunOutput::dropped`, left out when empty, filled by `run_output` (so
  the MCP server's `step` answers it too); `attached.rs`'s `run` answer
  holds none. The text answer prints `dropped: click (frame 5, gap)` after
  `sounds:`.
- Contract version 55; `run.output.json` and `session.step.output.json`
  regenerated, and the fingerprint in `versions.json`.
- `docs/contract.md`'s `run` row; the book's sound paragraph
  (`saves-and-settings.md`) names `run`'s `dropped` beside
  `h.dropped_sounds()`.
- Tests: `run_lists_the_plays_the_cue_rules_dropped`
  (`contract/session/tests.rs`: the toy with a 40 ms gap, `f f f`, the
  JSON's keys against the schema's, and an answer with none dropped reads
  back as itself); the fake's text in `run/tests.rs`. The test of
  `run`'s keys against its schema leaves `dropped` out of the schema's
  list, as that run drops nothing.
- **2378's clause:** 2378 is not done, so nothing here; its ticket's
  clauses now say what holds (the contract part says to add its words to
  `Dropped::why`'s doc comment and raise the version).
- Deviation: none.

No break: `RunOutput` is `#[non_exhaustive]`, and `Session` gains a
method with a default body.

Behaviour: shuyi-cli: `run`'s (and `step`'s) answer has a `dropped` list
when the cue rules dropped a play, and its text a `dropped:` line;
contract version 55. Searched the game for `RunOutput`
(`gh search code --repo Zafnok/visions-of-shuyi RunOutput`): it builds
none; `dropped_sounds`: no hit.

Book: the book's sound paragraph in `saves-and-settings.md` now names
`run`'s `dropped`.

Nick input: none.
