---
id: "3409"
title: The test that a dropped forwarded server ends asks the child, not tasklist, whether it started
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3409 — The test that a dropped forwarded server ends asks the child, not tasklist, whether it started

## Context

On 2026-11-17, during a full `cargo xtask gates` run on Nick's Windows
machine (24 cores, the workspace's test binaries running at once), the
unit test `mcp::forward::tests::dropping_a_forwarded_server_ends_it`
(`crates/cli/src/mcp/forward/tests.rs`) failed at its first assertion:

```text
assert!(runs(pid), "it started");   // forward/tests.rs, line 286
```

Run alone three times right after, it passed each time. It was seen
while working 3316 (#501), which doesn't touch `shuyi-cli`, so it gets
its own ticket (`AGENTS.md` rule 2). It is a local sighting, not a CI
cell, so it has no `tools/flaky.ron` entry (the `run-gates` skill, *When
CI fails but local passes*: a second sighting in CI gets one).

**What 2063 left.** 2063 (*The MCP server's forwarding tests pass under
load*) ran the forwarding tests 200 times under load on Windows and Linux
and saw no failure, so it changed no test. Its notes read this test's
timing as safe because the test "asks about the process after
`Process`'s drop has killed and waited for it". That is true of the
second assertion only. 2063 read the tests for clocks (`sleep`,
`Duration`, `Instant`, `recv_timeout`) and didn't look at how `runs`
asks: it shells out to another program and trusts its answer. That
probe is what failed here, before the drop.

**The likely cause: the probe, not the child.** `Process::start`
(`crates/cli/src/mcp/forward.rs`) spawns the child with
`Command::spawn`; when `spawn` returns, the process exists (Windows'
`CreateProcess`, Unix's `fork` and `exec` have both returned), and
`ping -n 60` can't end on its own within a minute. So "the child had not
started yet" is not a possible state. `runs` (line 259) on Windows runs
`tasklist /FI "PID eq N" /NH` and answers whether its stdout contains
` N `. It ignores tasklist's exit status and stderr. `tasklist` reads the
process list through WMI, which can fail or time out when many processes
ask at once, as a 24-core test run does. A tasklist that failed prints
nothing to stdout, and `runs` reads that as "not running". The same flaw
makes the second assertion, `!runs(pid)`, pass when the probe fails:
the test can't tell "it ended" from "the probe broke". On Unix,
`kill -0` has the same shape (any failure, a permission error included,
reads as "not running").

## Nick input

None.

## Scope

**In:**
- The first assertion asks the test's own child handle, with no shell
  command: `try_wait()` on `process.child` is `Ok(None)` while it runs.
- The probe after the drop never reads its own failure as an answer.
  Split `runs` into a pure function that reads a probe's output and a
  thin one that runs the probe. The pure one answers *runs*, *doesn't
  run* or *the probe failed* (an `Err` with its status, stdout and
  stderr), and the test panics on the third with that text.
- Unit tests of the pure function on scripted outputs, so its verdicts
  don't depend on a real `tasklist` under load.

**Out (do not do):**
- `#[ignore]`, a retry loop, or a sleep around either assertion.
- A wait "until the process appears": `spawn` returning means it exists,
  and a bounded wait would hide a probe that fails rather than fix it.
- Changing `Process`'s `Drop` or `Process::start`: the drop's
  behaviour is what the test checks, and nothing shows it is wrong.
- A new dependency (`windows-sys` or the like) to open the process by
  handle: the pure reader of the probe's output is enough.
- The other MCP tests; 2063 measured them.

## Implementation steps

1. In `crates/cli/src/mcp/forward/tests.rs`, replace
   `assert!(runs(pid), "it started")` with
   `assert!(matches!(process.child.as_mut().unwrap().try_wait(), Ok(None)), "it started")`
   (the binding becomes `let mut process`). Keep `pid` from the child
   for the check after the drop.
2. Add `fn listed(pid: u32, status: ExitStatus, stdout: &str, stderr: &str) -> Result<bool, String>`
   beside `runs`, for each platform (`cfg!(windows)` as today):
   - Windows (`tasklist /FI "PID eq N" /NH`): `Ok(true)` when the
     status is success and a line's second column is `N`; `Ok(false)`
     when the status is success and the output (stdout or stderr) holds
     tasklist's line for an empty match, which starts `INFO:`; `Err`
     otherwise (a failed status, an `ERROR:` line, empty output).
   - Unix (`kill -0 N`): `Ok(true)` on success; `Ok(false)` when
     stderr says `No such process`; `Err` otherwise.
3. `runs(pid)` runs the probe with stdout and stderr captured and
   returns `listed(…)`; the test writes
   `assert!(!runs(pid).unwrap_or_else(|e| panic!("the probe failed: {e}")), "it ended with the drop")`.
4. Unit tests of `listed` (step 2) with scripted outputs, both
   platforms' readers testable on either host (make the platform a
   parameter, or two functions, so neither is `cfg`-gated out).
5. Check the test still catches a drop that doesn't end the child: with
   `child.kill()` and `child.wait()` commented out of `Process`'s drop,
   the test fails at "it ended with the drop" (then restore them). Write
   the output in the completion notes.
6. Run the 2063 loop (its completion notes give the command and the CPU
   load beside it) for this test, 200 runs on Windows, and record the
   count.

## Acceptance criteria

- [ ] `dropping_a_forwarded_server_ends_it` runs no shell command before
      the drop; "it started" reads the child handle's `try_wait`.
- [ ] A failed probe after the drop panics with the probe's status and
      output; no path reads a failed probe as "not running".
- [ ] Unit tests of the probe reader cover, for Windows: a listing
      with the pid → runs; tasklist's `INFO:` line → doesn't run; a
      failed status, an `ERROR:` line and empty output → the probe
      failed. For Unix: success → runs; `No such process` → doesn't run;
      any other failure → the probe failed.
- [ ] With `Process`'s kill and wait taken out, the test fails at "it
      ended with the drop" (the output in the notes).
- [ ] 200 runs of the test under the 2063 load pass on Windows (the
      command and count in the notes).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the probe reader's cases above, on scripted outputs.
- Unit: `dropping_a_forwarded_server_ends_it`, rewritten as above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
