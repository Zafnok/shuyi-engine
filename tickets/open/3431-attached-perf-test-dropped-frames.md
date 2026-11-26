---
id: "3431"
title: The test that perf on an attached session answers as on a headless one doesn't depend on how many frames the link's host ran
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3431 — The test that perf on an attached session answers as on a headless one doesn't depend on how many frames the link's host ran

## Context

On 2026-11-25, during a full `cargo xtask gates` run on Nick's Windows
machine (the workspace's test binaries running at once), the unit test
`mcp::attached::tests::perf_on_an_attached_session_answers_as_on_a_headless_one`
(`crates/cli/src/mcp/attached/tests.rs`, about line 258) failed at

```text
assert_eq!(attached["limits"], headless["limits"], "{attached}");
```

The attached session's `perf` answer had one more limit than the
headless one's:

```text
the trace keeps the last 600 frames: 3824 earlier frame(s) were dropped
```

Run alone (`cargo test -p shuyi-cli --lib -- perf_on_an_attached_session`)
it passes. It was seen while working 2278 (#590), which doesn't touch
`shuyi-cli`, so it gets its own ticket (`AGENTS.md` rule 2). It is a
local sighting, not a CI cell, so it has no `tools/flaky.ron` entry (the
`run-gates` skill, *When CI fails but local passes*).

**The cause.** The headless session steps only when asked. The attached
one doesn't: `LinkHost::start()` (`crates/cli/src/toy/link.rs`) is
`LinkHost::running(1)`, whose thread (`serve`) runs one frame, sleeps
1 ms, and loops, whether or not anyone asks, from the moment it starts.
The test starts the host and attaches **before** it runs the headless
reads (`perf_reads(&mut headless)`), so the attached game keeps running
all through them, and then through its own reads. `perf_reads` reads
`perf` with `since: 0` first. Once more than `PERF_FRAMES` (600,
`crates/cli/src/contract/perf.rs`) frames have run, that read's answer
says `dropped > 0`, and `perf_once` (`crates/cli/src/contract/attach.rs`,
about line 564) adds `perf::dropped_limit(answer.dropped)`. A headless
toy that ran 10 frames never drops one. Alone, the test finishes in well
under 600 of the host's frames; under load, the headless reads and the
host's sleeps take longer (Windows' sleep can round 1 ms up to a timer
tick) and the host ran about 4,400 frames (600 kept, 3,824 dropped).

So the compared limit depends on the machine's speed, not on the code.
The other comparisons hold under load: `fields` compares field names,
not values (`summary.dropped` is a field on both sides), and the
assertions after the loop read only frames, snapshots and codes.

## Nick input

None.

## Scope

**In:**
- The comparison of limits leaves out the frames-dropped limit, the one
  limit an attached game that runs in real time may add, and compares
  every other limit as now.
- Where the attached answer says frames were dropped, the test checks
  that the dropped limit is the one `perf::dropped_limit` writes for the
  answer's own `summary.dropped`, so the line is still tested, not only
  ignored.
- Start the attached session after the headless reads, so the host's
  frames run only while the attached reads do (fewer frames, nearer to
  what the test means), with a comment saying the order matters less
  than the filter.

**Out (do not do):**
- `#[ignore]`, a retry, or a sleep.
- Changing `LinkHost`, `serve`, `PERF_FRAMES`, `perf_once` or what a
  `perf` answer says: the answer's limit is right, the game did drop
  frames.
- A paused or stepped `LinkHost` mode (bounding the host's frames). It
  would make the attached session unlike a real debug build, which runs
  in real time, and the test's point is that the answers agree despite
  that. If the filter turns out not to be enough, that is a new ticket.
- The other attached tests and `tests/it/toy.rs`: they don't compare
  limits across sessions.

## Implementation steps

1. In `crates/cli/src/mcp/attached/tests.rs`, add a helper beside
   `fields` and `kept`:
   `fn limits_but_dropped(answer: &Value) -> Vec<Value>`, the answer's
   `limits` array without the entries that start with
   `"the trace keeps the last "` (the prefix of
   `crate::contract::perf::dropped_limit`; build the prefix from
   `PERF_FRAMES` or from `dropped_limit` itself so the two can't
   drift, rather than a second copy of the sentence).
2. In `perf_on_an_attached_session_answers_as_on_a_headless_one`,
   compare `limits_but_dropped(attached)` with
   `limits_but_dropped(headless)` instead of the raw `limits`.
3. In the same loop, for the attached answer: if its
   `result.summary.dropped` is a number above 0, assert its `limits`
   hold exactly `perf::dropped_limit(dropped)`; if 0 or absent, assert
   no limit starts with the prefix. The headless answer must hold no
   such limit.
4. Move the attached session's `start` (the `Fake` game, `Server::new`
   and `called(&mut attached, "start", …)`) to after
   `let alike = perf_reads(&mut headless);`, with a one-line comment
   on why.
5. Check the test still fails when the limits truly differ: make
   `perf_once` push a made-up extra limit, see the test fail at the
   limits' comparison, then restore it. Write the output in the
   completion notes.
6. Reproduce the race on purpose to check the fix: temporarily make the
   test sleep (or step the host with `LinkHost::running(700)`) so the
   host runs past 600 frames before the attached reads; see the old
   assertion fail and the new one pass; restore. Then run the test 50
   times during a full `cargo test --workspace` on Windows and record
   the count in the notes.

## Acceptance criteria

- [ ] The limits' comparison ignores only the frames-dropped limit;
      every other limit is still compared between the two sessions.
- [ ] When the attached answer dropped frames, the test asserts its
      limit is `perf::dropped_limit` of the answer's `summary.dropped`;
      when it dropped none, no such limit is there.
- [ ] The attached session starts after the headless reads.
- [ ] With a made-up extra limit in `perf_once`, the test fails (the
      output in the notes).
- [ ] With the host forced past 600 frames (step 6), the test passes,
      and the old comparison failed (both in the notes).
- [ ] 50 runs under a full workspace test run on Windows pass (the
      command and count in the notes).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `perf_on_an_attached_session_answers_as_on_a_headless_one`,
  rewritten as above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
