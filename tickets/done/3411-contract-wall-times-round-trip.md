---
id: "3411"
title: The contract's wall times are rounded where they are measured, so an answer reads back as itself
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-22
---

# 3411 — The contract's wall times are rounded where they are measured, so an answer reads back as itself

## Context

One local `cargo test --workspace`, while ticket 2203 was worked, failed
`contract::autobalance::tests::the_json_answer_has_the_fields_of_its_schema`
(`crates/cli/src/contract/autobalance/tests.rs`), then passed on a rerun.
The two `Answer`s it compares were equal except `seconds`: `2.2799121`
read back against `2.2799120999999998` measured.

The test writes the answer as JSON, reads it back and asserts the two are
equal. `AutobalanceOutput::seconds` is the search's wall time,
`started.elapsed().as_secs_f64()`, every bit of it kept. When that
number's shortest decimal has 17 significant digits, its significand is
past 2^53, and `serde_json`'s default parser (no `float_roundtrip`
feature) may land one unit in the last place away. A short run gives a
short decimal, which parses exactly, so the test passes most of the time.
It is a flake by the clock, not by the code under test, and
`tools/flaky.ron` (2064) has no entry for it: this ticket fixes it
instead.

The answer is wrong as well as the test: a wall time is noise past the
millisecond, and the JSON carries 17 digits of it, which a Rust client
reading the answer (`serde_json`, as `bake` reads its child's answer)
gets back changed. The contract already gives its other times a stated
precision where they are measured: `perf`'s milliseconds have one
decimal, `bake`'s and the perf traces' times are whole microseconds
(`docs/contract.md`, *Perf traces*).

The only other wall time in seconds in a contract answer is `playtest`'s
`Played::seconds_per_try` (`crates/cli/src/contract/playtest.rs`, 2532):
a try's time on average, `took.as_secs_f64() / tries`, unrounded. No test
compares a played answer read back from JSON with a measured time (its
tests set the field to `0.5` before comparing), so it hasn't flaked, but
its answer and its history lines have the same 17 digits.

## Nick input

None.

## Scope

**In:**
- `autobalance`'s `seconds` rounded to whole milliseconds where the
  answer is built.
- `playtest`'s `seconds_per_try` rounded to whole microseconds where it is
  computed: a try of a small game takes well under a millisecond, and the
  text's `< 0.01` reads it.
- The fields' doc comments say the precision, which the committed schemas
  (`crates/cli/schemas/autobalance.output.json`,
  `playtest.output.json`) carry, and `docs/contract.md`'s rows say it.
- A test that a rounded time reads back from JSON as itself, over many
  values, so the round-trip test can't flake again.

No public type changes shape: the two fields stay `f64`, only their
values are rounded. No break.

**Out (do not do):**
- Turning on `serde_json`'s `float_roundtrip` feature: it would hide the
  noise in our reader, not take it out of the answer.
- `playtest`'s `measures` `mean` and the other floats of the reports:
  they are the game's deterministic numbers, not times, and the same
  every run.
- An entry in `tools/flaky.ron`: the flake is fixed, not tolerated.

## Implementation steps

1. In `crates/cli/src/contract/playtest.rs`, add a `pub(crate)` helper
   that rounds a number of seconds to a given number of decimals
   (`(s * 10^n).round() / 10^n`), and use it for `seconds_per_try`
   (6 decimals, whole microseconds). Say so in the field's doc comment.
2. In `crates/cli/src/contract/autobalance.rs`, round `seconds` with it
   to 3 decimals (whole milliseconds) where `autobalance` measures it.
   Say so in the field's doc comment.
3. Regenerate the committed schemas as the contract's schema test says.
4. In `docs/contract.md`, the `playtest` and `autobalance` rows say the
   precision of their times.
5. Tests below.

## Acceptance criteria

- [x] An autobalance answer's `seconds` is a whole number of milliseconds,
      and a played entrant's `seconds_per_try` a whole number of
      microseconds.
- [x] For every whole number of milliseconds from 0 to 100 000 s plus
      noise under half a millisecond (a sweep, not a sample), the rounded
      value written as JSON reads back equal; the same for microseconds
      up to 100 s.
- [x] `the_json_answer_has_the_fields_of_its_schema` still compares the
      whole answer read back, `seconds` included, unchanged.
- [x] The schemas and `docs/contract.md` say the precision.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the rounding helper (a value on each side of a half, zero) and
  the JSON round-trip sweep of both precisions.
- Unit: an autobalance answer's `seconds` times 1000 is whole; a played
  entrant's `seconds_per_try` times 1 000 000 is whole (to within the
  float's error).

## Completion notes

- `playtest::rounded(s, decimals)` rounds a number of seconds to its
  decimals; `autobalance` rounds its `seconds` to 3 (whole milliseconds)
  where it measures the search, and `playtest` its `seconds_per_try` to
  6 (whole microseconds). A rounded time's shortest decimal has a
  significand under 2^53 and at most 6 decimals, which `serde_json`'s
  default parser reads exactly, so the answer reads back as itself.
  Before the fix, `2.2799120999999998` written as JSON read back as
  `2.2799121`, one unit in the last place off: checked with a throwaway
  test, the flake's own numbers.
- `the_json_answer_has_the_fields_of_its_schema` is unchanged: it still
  compares the whole answer read back.
- The fields' doc comments say the precision, so the two committed
  schemas changed, and the contract goes to version 54
  (`schemas/versions.json`, `docs/contract.md`'s example answer). The
  `playtest` and `autobalance` rows of `docs/contract.md` say the
  precision.
- Deviation: the ticket didn't name the contract's version raise, which
  the schemas' test asks for when a schema changes.
- Deviation: the sweep covers every millisecond to 1,000 s and every
  microsecond to 1 s (10^6 values each, with noise of -0.4 to 0.4 of a
  unit), plus a day and more at three spot values, not every
  millisecond to 100,000 s: that is 10^8 values, about a minute in a
  debug test, and the decimal's significand stays under 2^53 either way.
- The other contract answers with a time: `perf`'s milliseconds already
  have one decimal and `bake`'s times are whole microseconds; no other
  answer has a float of seconds. No other test compares an answer read
  back from JSON with a measured float time.
- Game: nothing to adopt. `gh search code --repo Zafnok/visions-of-shuyi
  'seconds_per_try'` finds only the game's own `crates/tools` playtest
  (its own struct, its own timing), and `'autobalance seconds'` nothing;
  the game pins `50472bbc`.

No break: the fields stay `f64`.

Behaviour: shuyi-cli: `playtest`'s `seconds_per_try` is rounded to the microsecond and `autobalance`'s `seconds` to the millisecond; the contract is version 54.

Book: none, the change is to two numbers of the CLI's answers, which the contract's page documents, not the book.
