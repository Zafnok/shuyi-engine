---
id: "2575"
title: Make the bake fastest-run test deterministic
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2575 — Make the bake fastest-run test deterministic

## Context

While running the workspace test gate for 2914, the existing
`shuyi-cli` test `times_are_whole_microseconds_and_the_fastest_run_counts`
failed with a measured minimum of 32,977 µs after 672 tests passed. It sleeps
for 20 ms on the first run and 2 ms on the remaining four, then asserts that
the fastest wall-clock measurement is below 20 ms. A busy Windows test host
can suspend every run for longer than
that bound, so the assertion can fail without a bake regression. The test is
in [`crates/cli/src/contract/bake/tests.rs`](../../crates/cli/src/contract/bake/tests.rs),
and the `fastest` helper is in
[`crates/cli/src/contract/bake.rs`](../../crates/cli/src/contract/bake.rs).
This ticket is independent of 2914's protected-rules research.

## Nick input

None.

## Scope

**In:**
- Verify that `fastest` takes exactly `LOAD_RUNS` measurements and chooses the
  smallest value, using controlled durations instead of a sleep bound.
- Keep whole-microsecond conversion coverage and the real bake timing path.

**Out (do not do):**
- Change the reported bake timing semantics, `LOAD_RUNS`, or public API.
- Change unrelated timing tests or loosen the repository's test gates.

## Implementation steps

1. In `crates/cli/src/contract/bake.rs`, give the minimum-selection part of
   `fastest` a testable measurement seam so controlled measurements exercise
   the same selection code as production's `Instant` measurements.
2. In `crates/cli/src/contract/bake/tests.rs`, replace the absolute
   2–20 ms sleep assertion with deterministic measurements whose first run is
   slower than later runs. Assert the selected minimum and exact run count;
   retain the `micros` conversion assertions.
3. Run the focused `shuyi-cli` test and the `run-gates` workflow.

## Acceptance criteria

- [ ] The test proves the fastest of `LOAD_RUNS` controlled measurements is
  selected, including when the first run is slower.
- [ ] The test no longer passes or fails according to OS scheduling delays.
- [ ] Bake timing output and public API stay unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: controlled durations check minimum selection, run count and whole
  microsecond conversion.
- Integration: the existing bake tests and full workspace suite pass.
- Property / snapshot: none; this change has no new format or visual output.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
