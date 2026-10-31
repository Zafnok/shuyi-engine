---
id: "2561"
title: Moving a running server's program aside never leaves its path empty, so the CLI's tests don't flake when they run in parallel
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-26
---

# 2561 — Moving a running server's program aside never leaves its path empty

## Context

2545 made a running MCP server move its own program aside, so the next
build can write a new one (ADR-0035, `crates/cli/src/mcp/aside.rs`).
`put_aside(from, to)` copies `from` to `<to>.copy`, renames `from` to
`to`, then renames the copy back to `from`. Between the two renames
nothing is at `from`.

`cargo test -p shuyi-cli --test it` runs its tests on several threads.
`crates/cli/tests/it/mcp.rs` starts `shuyi mcp` from
`env!("CARGO_BIN_EXE_shuyi")`, and that server moves its program (the
very `shuyi.exe` the other tests run) aside as it starts. A
`crates/cli/tests/it/binary.rs` test spawning the same path in that
moment fails with "spawn shuyi: The system cannot find the file
specified. (os error 2)". Seen on Windows (GNU) while working 3064: in
four runs of the test binary, two failed, each in a different
`binary::` test (`mcp_ready_builds_what_the_server_needs_and_exits`,
`the_commands_that_run_the_game_need_its_tool_program`), and once
`mcp::an_agent_starts_hello_grid_steps_into_the_stage_and_looks_at_it`
answered `isError` on its `run`. With `--test-threads=1` all 40 pass.
CI's Windows runners can hit it too.

The same gap reaches a person: an agent's server starting while they run
`cargo shuyi …` in a terminal can make that command fail the same way.

## Nick input

None.

## Scope

**In:**
- `put_aside` leaves a program at `from` at every moment. On Windows a
  running program's file can be renamed but not replaced or deleted, so
  copy first (as now), then move the running file aside, then put the
  copy in its place with a rename that replaces nothing (nothing is
  there). The gap between the two renames is the bug. Options, in the
  order to try:
  1. a hard link: `fs::hard_link(from, to)` (the running file gains a
     second name in `.shuyi-running/`), then rename the copy over
     `from` (`fs::rename` replaces the target on Windows with
     `MOVEFILE_REPLACE_EXISTING`; check that it does when the old file
     is a running program whose other name keeps it alive). If that
     works there is never a moment without a file at `from`;
  2. otherwise, a retry in the tests' spawn helpers is **not** the fix
     (it would hide the same failure from a person's terminal).
- Say in `aside.rs`'s module docs why the order is what it is.
- If the `mcp::` test's `isError` is another cause (its answer's error
  text says), find it and fix it, or write a ticket for it.

**Out (do not do):**
- Running the CLI's tests on one thread, or retrying a failed spawn.
- Changing what moves aside or where it goes (`.shuyi-running/`).

## Implementation steps

1. Reproduce: `cargo test -p shuyi-cli --test it` a few times on
   Windows; note the failures.
2. Change `put_aside` (`crates/cli/src/mcp/aside.rs`) as *Scope* says,
   keeping its modified-time copy so cargo still sees the program as
   fresh.
3. Add a unit test in `crates/cli/src/mcp/aside/tests.rs`: while
   `put_aside` runs on a file in a scratch folder, another thread that
   checks `from.exists()` in a loop never sees it missing.
4. Run the test binary in parallel ten times; none fails.

## Acceptance criteria

- [x] At no moment during `put_aside` is there no file at `from` (the
      unit test).
- [x] Ten parallel runs of `cargo test -p shuyi-cli --test it` on
      Windows pass.
- [x] `aside.rs`'s tests still pass, and a running server still lets
      `cargo build` replace its program.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `put_aside` never leaves `from` empty, watched from a second
  thread.
- Integration: the existing `mcp::` and `binary::` tests, in parallel.

## Completion notes

- `put_aside` now hard-links the running file to its name in
  `.shuyi-running/`, then renames the copy over `from`, instead of
  renaming `from` away and the copy in after it. Option 1 of *Scope*
  works: on Windows the rename replaces a running program's name while
  its other name keeps the file, so there is never a moment without a
  program at `from`. The modified-time copy is kept. `aside.rs`'s module
  docs and `put_aside`'s say why the order is what it is.
- Tests in `crates/cli/src/mcp/aside/tests.rs`:
  `a_program_is_at_its_path_at_every_moment_of_a_move` (a second thread
  watches `from` through 200 moves; with the old order it saw the path
  missing 2773 times, with the new one never), and, on Windows,
  `a_running_program_is_put_aside_and_its_path_left_replaceable` (a
  running copy of the system's `PING.EXE` is put aside, then its path
  overwritten as a build would).
- Reproduced before the fix by the unit test, not by the integration
  tests (they flaked in 3064's session). After it, ten runs of
  `cargo test -p shuyi-cli --test it` on its default threads passed,
  40 of 40 each. The `mcp::` `isError` didn't come back in them, so it
  was taken for the same gap (the server's `run` spawns the same
  program); no ticket written.
- `binary::a_running_server_moves_its_program_aside_so_a_build_can_write_a_fresh_one`
  still passes: a running server still lets a build replace its program.
- No docs beyond the module docs: nothing a game or the docs map
  describes changed.
- No break.
