---
id: "3431"
title: The test that perf on an attached session answers as on a headless one doesn't depend on how many frames the link's host ran
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-25
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

- [x] The limits' comparison ignores only the frames-dropped limit;
      every other limit is still compared between the two sessions.
- [x] When the attached answer dropped frames, the test asserts its
      limit is `perf::dropped_limit` of the answer's `summary.dropped`;
      when it dropped none, no such limit is there.
- [x] The attached session starts after the headless reads.
- [x] With a made-up extra limit in `perf_once`, the test fails (the
      output in the notes).
- [x] With the host forced past 600 frames (step 6), the test passes,
      and the old comparison failed (both in the notes).
- [x] 50 runs under a full workspace test run on Windows pass (the
      command and count in the notes).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `perf_on_an_attached_session_answers_as_on_a_headless_one`,
  rewritten as above.

## Completion notes

**What was done.** In `crates/cli/src/mcp/attached/tests.rs`:

- `dropped_prefix()` takes the limit's opening words from
  `perf::dropped_limit(0)` itself (up to its first `": "`), so the test and
  the sentence can't drift. `limits_but_dropped(answer)` is the limits
  without that one, and `dropped_limits(answer)` is that one alone.
- `perf_on_an_attached_session_answers_as_on_a_headless_one` compares
  `limits_but_dropped` of the two sessions. It also asserts that the
  headless answer has no dropped limit, and that the attached one has
  exactly `dropped_limit(n)` when its answer counts `n > 0` dropped
  frames, and none when `n` is 0.
- The host (`LinkHost::start()`) and the attached session start after
  the headless reads. A comment says the filter, not this order, is what
  makes the test independent of the machine's speed.

**Deviations (the ticket corrected).**

- The ticket put the limit in `perf_once` (`contract/attach.rs`). That
  is the `attach` operation's one-shot read. The `perf` tool of both MCP
  sessions answers through `perf_answer` (`crates/cli/src/mcp/host.rs`,
  about line 510), and that is where the limit comes from. A made-up
  limit pushed in `perf_once` left the test passing, since the test
  never calls it. One pushed in `perf_answer` (on `!process_heap`)
  passed too: both sessions pass `process_heap: false` and so both got
  it. So the check was made by adding the limit in `AttachedSession::perf`
  (`crates/cli/src/mcp/attached.rs`), on the attached answer only.
- The ticket reads the count at `result.summary.dropped`. In the MCP
  `perf` answer, `summary` comes from `perf::summary(&records)`, whose
  `dropped` is always 0. The count is `result.dropped`, which the test
  reads.
- The host's start moved along with the attached session's (step 4).
  The host runs frames from its own start, attached or not (`serve`),
  so moving only the `start` call would have saved no frames.

**Fault checks (each change restored afterwards).**

1. A made-up limit in `AttachedSession::perf`
   (`.map(|mut a| { a.limits.push("a made-up extra limit".to_owned()); a })`):
   the test failed at the limits' comparison:

   ```text
   panicked at crates\cli\src\mcp\attached\tests.rs:313:9:
     left: [String("times are this machine's, …"), String("a made-up extra limit")]
    right: [String("times are this machine's, …")]
   ```

2. The perf test's host started with `LinkHost::running(700)`: the new
   test passed, with the attached answer's dropped branch taken. With
   the old `assert_eq!(attached["limits"], headless["limits"])` added
   back beside it, the test failed:

   ```text
   panicked at crates\cli\src\mcp\attached\tests.rs:319:9:
     left: Array [String("times are this machine's, …"), String("the trace keeps the last 600 frames: 5000 earlier frame(s) were dropped")]
    right: Array [String("times are this machine's, …")]
   ```

**50 runs under load (Windows, Nick's machine).** `cargo test --workspace`
ran in the background. Twenty seconds after it started, this ran 50
times in the foreground, using the workspace's own `shuyi-cli` test binary:

```text
shuyi_cli-<hash>.exe --exact mcp::attached::tests::perf_on_an_attached_session_answers_as_on_a_headless_one
```

Result: 50 passed, 0 failed. The loop and that workspace run were started
from Git Bash. There, one other test failed every time, in the workspace
run and when run alone:
`mcp::forward::tests::dropping_a_forwarded_server_ends_it`, at
`assert!(runs(pid), "it started")` (ticket 3409's test). From PowerShell
it passes. This change doesn't touch it, so it is 3409's.

Book: none, a test-only change: no public item, operation or answer changed.

No break. No behaviour change: only a test changed.
