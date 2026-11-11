---
id: "2063"
title: The MCP server's forwarding tests pass under load
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] The failing test is named in the notes with its failure output.
- [ ] 200 runs of the forwarding tests under load pass (the command and
      count in the notes).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- The existing forwarding tests, made deterministic.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
