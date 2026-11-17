---
id: "2063"
title: The MCP server's forwarding tests pass under load
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2063 — The MCP server's forwarding tests pass under load

## Context

#302 (3211) listed its gates as "fmt, clippy, `cargo test --workspace`
(one MCP forwarding test failed under load and passed alone)". No
ticket was written (`AGENTS.md` rule 2). The retro of 2026-11-08 found
it. A test that fails under load fails some CI run sooner or later, and
an agent then reruns it and learns to ignore it. 2577 (#311) shows the
fix pattern for a timing-dependent test: a seam that makes it
deterministic.

The forwarding tests are in `crates/cli/src/mcp/server/tests.rs`
(`forwarding`, and
`a_forwarded_server_that_doesnt_build_is_an_error_result_with_its_output`)
and `crates/cli/src/mcp/forward/tests.rs` (for example
`dropping_a_forwarded_server_ends_it`).

## Nick input

None.

## Scope

**In:**
- Find which test fails under load: run the forwarding tests in a loop
  beside a CPU load (`cargo test` of another crate in parallel), on
  Linux in CI or a container, and record the name and the failure.
- Remove the timing dependence: a channel or a ready signal in place of
  a sleep or a wall-clock wait, or a timeout raised to a bound that is
  stated and justified.

**Out (do not do):**
- `#[ignore]` or a retry loop around the test.

## Implementation steps

1. Read both test files for `sleep`, `Duration`, `Instant` and
   `recv_timeout`. Those are the likely spots.
2. Reproduce with `cargo test -p shuyi-cli forward -- --test-threads=16`
   repeated under load, and save the failure.
3. Replace the wait with an event the forwarded server sends, or give
   the test a seam as 2577 did with `fastest_of`.

## Acceptance criteria

- [x] The failing test is named in the notes with its failure output.
      Not met: no forwarding test failed in 860 runs under load, so there
      is no output to give; the notes name the likeliest test instead.
- [x] 200 runs of the forwarding tests under load pass (the command and
      count in the notes).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- The existing forwarding tests, made deterministic.

## Completion notes

- **No forwarding test fails under load, on Windows or Linux.** Every
  loop ran the test binaries of commit 9c73d37d (this PR changes a
  comment only), from `crates/cli` as `cargo test` does, at
  `--test-threads=16`, beside a CPU load of two loops that each built
  `shuyi-assets` from clean in a target folder of its own
  (`cargo clean --target-dir <dir>; cargo build -p shuyi-assets
  --target-dir <dir>`):

  | Where | What | Runs | Failed |
  | ----- | ---- | ---- | ------ |
  | Windows 11, 24 cores | `shuyi_cli-* forward` (the 10 tests the ticket's filter picks) | 200 | 0 |
  | Windows 11 | `it-* another_workspace` (the forwarded server as a real process) | 100 | 0 |
  | Windows 11 | the whole unit suite, the whole `it` suite | 40, 25 | 0, 0 |
  | Linux (WSL Ubuntu 24.04), 24 cores | `forward`, `another_workspace` | 200, 100 | 0, 0 |
  | Linux, 24 cores | the whole unit suite | 30 | 0 of its MCP tests |
  | Linux, all of it under `taskset -c 0,1` (2 cores) | `forward`, `another_workspace` | 200, 60 | 0, 0 |

  The Linux unit suite's 30 failed runs are the same three
  `export::tests::editions` tests every time, failing because the WSL
  setup has no `wasm32-unknown-unknown` target; no other test failed.
  CI has never seen it either: the logs of the failed jobs of the last
  120 failed CI runs (117 still downloadable, Linux, macOS and Windows)
  name no `mcp` test that failed.
- **Which test #302 saw (deviation: the first criterion can't be met as
  written).** #302's PR names no test and keeps no output, so the
  failing test isn't named with its failure output. The likeliest one is
  not a forwarding test: `mcp::aside::tests::a_program_is_at_its_path_at_every_moment_of_a_move`
  (2564, 2574). On 2026-11-07, when #302 ran its gates on Nick's machine
  beside another build, that MCP test failed about one run in 75 under
  load on Windows and passed alone, which is what #302's note says.
  2574 (#408, 2026-11-13) settled it: 0 failed in 300 runs under load.
- **Timing in the forwarding tests, read for `sleep`, `Duration`,
  `Instant` and `recv_timeout` (step 1).** The unit tests run on
  scripted pipes (`forward/tests.rs`) or fake servers
  (`server/tests.rs`): none of their verdicts waits on a clock.
  `dropping_a_forwarded_server_ends_it` asks about the process after
  `Process`'s drop has killed and waited for it. The one wall-clock bound
  is the 10-second loop at the end of
  `a_project_in_another_workspace_is_answered_by_its_own_server_which_ends_with_the_server`
  (`crates/cli/tests/it/binary.rs`). Its bound is now stated and
  justified in a comment there: the server killed and waited for the
  project's own server before it ended, so the loop waits only for
  Windows to let go of the ended program's file. Step 3 (an event in
  place of a wait) had no failing wait to replace, so no test's logic
  changed: a seam for a failure that doesn't happen would be a guess.
- **Linux in WSL, without root:** gcc 13 and libc6-dev with their
  dependencies extracted from Ubuntu's archive (a private, fresh apt
  index, since the system's was stale) under `~/vt`, a `cc` wrapper
  linking with `--sysroot`, and Rust 1.99.1 (minimal profile) from
  rustup. `shuyi-cli` needs no system library on Linux, only the linker.
- Book: none, no public API changed; the change is a comment in a test.
- No break.
- Docs: the comment in `binary.rs` and this ticket's line in
  `docs/ROADMAP.md`; nothing a game or the docs map describes changed.
- No follow-up ticket. No ticket: if a forwarding test fails again, the
  run-gates skill's rule for a flake (*When CI fails but local passes*)
  writes it down with its name and output, which this ticket lacked.
- Game: nothing to adopt, the change is a comment in an engine test.

Nick: nothing to do. The test failure the retro asked about doesn't
happen any more; it was most likely the one fixed on 2026-11-13.
