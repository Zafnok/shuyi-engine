---
id: "2564"
title: "The move-aside tests fail on Windows under load: find whether the program's path is ever empty, and make the tests hold"
type: bug
milestone: After MVP
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] The cause of each failure is written in the completion notes.
- [ ] Both tests pass 50 runs in a row on Windows while a workspace
      build runs.
- [ ] No assertion of either test is loosened.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the two tests as they are, and a test of any retry added (an
  error that clears after one try is retried; one that doesn't is
  returned).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
