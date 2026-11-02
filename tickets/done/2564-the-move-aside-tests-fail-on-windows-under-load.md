---
id: "2564"
title: "The move-aside tests fail on Windows under load: find whether the program's path is ever empty, and make the tests hold"
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-01
---

# 2564 — The move-aside tests fail on Windows under load

## Context

2561 (done, `tickets/done/2561-moving-a-running-servers-program-aside-never-leaves-its-path-empty.md`)
moves a running MCP server's program aside so a rebuild can write a new
one, and promises its path is never empty during the move
(`crates/cli/src/mcp/aside.rs`, `put_aside`). Its tests are in
`crates/cli/src/mcp/aside/tests.rs`.

While working 2111 (2026-10-27, Nick's machine: Windows 11, GNU
toolchain), two full `cargo test --workspace --locked --no-fail-fast`
runs each failed one of them, while another cargo build ran at the same
time:

- `a_running_program_is_put_aside_and_its_path_left_replaceable`:
  `...\shuyi-aside-running-<pid>\debug\ping.exe: Access is denied. (os
  error 5)` at `tests.rs:202` (`put.unwrap()`), and once
  `The process cannot access the file because it is being used by
  another process. (os error 32)` at `tests.rs:203`;
- `a_program_is_at_its_path_at_every_moment_of_a_move`: the watcher
  thread saw the path missing 2839 times (`left: 2839, right: 0` at
  `tests.rs:179`).

Each passes alone (`cargo test -p shuyi-cli --lib mcp::aside`, three
runs in a row). So either the tests race with something under load (an
antivirus scan of a new `.exe`, the indexer), or `put_aside` really
leaves the path empty or fails when the machine is busy; the second
would be a bug in 2561's promise, which a rebuild while an agent's
server runs depends on.

## Nick input

None.

## Scope

**In:**
- Reproduce both failures on Windows under load (a loop of the
  `mcp::aside` tests beside a `cargo build` of the workspace, or a
  stress tool), and find the cause of each.
- If `put_aside` can leave the path empty or fail where a retry would
  succeed: fix it (a retry on `ERROR_ACCESS_DENIED` and
  `ERROR_SHARING_VIOLATION` with a bound, or another order of the
  moves), and say why in its doc comment.
- If only the tests race: make them hold under load without weakening
  what they check (the path is never missing; the old program is put
  aside and the path is writable).

**Out (do not do):**
- Changing what 2561 promises, or skipping a test on Windows.
- Any change to the MCP server's restart beyond `put_aside`.

## Implementation steps

1. Read 2561 and `crates/cli/src/mcp/aside.rs`.
2. Reproduce: run the two tests in a loop (`for i in $(seq 50)`) while
   `cargo build --workspace` runs; note which call fails and with which
   OS error.
3. Fix the cause found (code or test), as *Scope* says.
4. Run the loop again under load: 50 runs, no failure.

## Acceptance criteria

- [x] The cause of each failure is written in the completion notes.
- [x] ~~Both tests pass 50 runs in a row on Windows while a workspace
      build runs.~~ (not done: the running-program test passes 300 of 300;
      the watcher test fails about one run in 75, because the system itself
      leaves the gap; 2574 settles it)
- [x] No assertion of either test is loosened.
- [x] All gates in the `run-gates` skill pass (the ones this change can
      reach; see the notes).

## Tests required

- Unit: the two tests as they are, and a test of any retry added (an
  error that clears after one try is retried; one that doesn't is
  returned).

## Completion notes

**Half fixed: one failure is gone, the other is real and still happens.**

- **`a_running_program_…`, fixed.** Both errors are a file held for a
  moment. `Access is denied` came from `put_aside`, on `from`, and the
  error doesn't say which call, but the rename is the likely one; it was
  seen once and not caught again by the labelled run. `os error 32` was the
  test's own rewrite of the path, which cleared in about 28 ms both times
  it was timed. An antivirus scan of a new `.exe` fits, but wasn't proved.
  `put_aside` now retries its steps after the copy on those two errors, for
  at most two seconds (`retrying`), and the test's rewrite does too. Under
  load it failed 2 of 300 runs before, and 0 of 300 after. The retry changes
  no assertion: a refusal that lasts still fails.
- **`a_program_is_at_its_path_…`, not fixed.** The count wasn't an artifact
  of `exists()`: the watcher got `os error 2` and the folder didn't list the
  name, for 59 ms in the run timed. So **2561's promise doesn't strictly
  hold.** Every gap fell in the step that replaces the name, and three
  different calls all had it (standalone program, under load):
  `fs::rename` 25 gaps in 900,000 moves, a POSIX-semantics rename 5 in
  150,000, a replacing hard link 3 in 750,000. No order of the moves avoids
  it. The test failed 4 of 300 runs after this change, one of them run 13,
  so the 50-run criterion is not met. It can pass 50 by luck, and that
  wouldn't mean anything.
- **Deviation:** the assertion stays strict, as the ticket says, so this test
  can still fail on a busy Windows machine, CI's included. Loosening it, or
  restating the promise, is what *Out* forbade, so 2574 holds that choice,
  with the numbers. `aside.rs`'s module docs now state the gap, since they
  would otherwise claim what was measured false; 2561's promise is otherwise
  unchanged.
- **Not established:** whether the gap is NTFS's own or a filter driver's.
  Telling them apart needs real-time scanning off, which wasn't tried.
- **For a person,** the gap is rare: about one server start in 20,000 opens
  one, and something must start the program inside its tens of milliseconds.
- **Tests:** `a_busy_answer_that_clears_is_retried`,
  `a_busy_answer_that_lasts_is_returned_after_every_try`,
  `any_other_error_is_returned_at_once`.
- **Gates run:** fmt, clippy, the workspace's tests, docs, deny, machete, the
  root lint and the repository's checks all pass. The web build, the no-std
  builds, the templates and the skill examples were left to CI: the change is
  in `shuyi-cli`'s Windows move only.
- **Docs:** none beyond `aside.rs`'s own; nothing a game or the docs map
  describes changed.
- **Follow-up:** 2574.
- No break.
