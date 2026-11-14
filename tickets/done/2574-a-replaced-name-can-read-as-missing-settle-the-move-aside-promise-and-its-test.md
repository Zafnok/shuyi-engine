---
id: "2574"
title: "A replaced name can read as missing on Windows: settle the move-aside promise and its test, which still fails under load"
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-13
---

# 2574 — A replaced name can read as missing on Windows

## Context

2561 promises that `put_aside` (`crates/cli/src/mcp/aside.rs`) never leaves
a moved program's path empty, and
`a_program_is_at_its_path_at_every_moment_of_a_move` asserts it: a watcher
thread must see the path missing 0 times through 200 moves.

2564 measured that promise on Windows 11 (NTFS, temp folder on `C:`), beside
a workspace build. The path really is empty for a moment: `os error 2`, the
folder doesn't list the name, for tens of milliseconds (59 ms in the one
timed). It happens only inside the call that replaces the name:

| Replacing call | Moves | Gaps |
| -------------- | ----- | ---- |
| `fs::rename` | 900,000 | 25 |
| POSIX-semantics rename (`FileRenameInfoEx`) | 150,000 | 5 |
| Replacing hard link (`FileLinkInformationEx`) | 750,000 | 3 |

So no order of the moves and no call tried keeps the promise strictly. At
200 moves a run, the test fails about one run in 75 under load (4 of 300
after 2564). 2564's *Out* forbade loosening the assertion or changing the
promise, so the test stays as it was and still fails sometimes. CI's Windows
runners can hit it too.

Not established: whether the gap is NTFS's own or a filter driver's, such as
Defender's. Telling them apart needs real-time scanning off, which only Nick
can do on his machine.

The gap is rare for a person: a server start moves two files, so about one
start in 20,000 opens a gap of tens of milliseconds, and something must try
to start the program inside it.

## Nick input

None.

## Scope

**In:** choose one of these, say why in an ADR (the `write-adr` skill), and
do it:
- **State the promise as what the system allows.** The order leaves no gap
  of its own; only the system's replacing can, rarely. The test then tells
  the two apart, so 2561's bug still fails it: the old order opened a gap in
  every move, so count the separate gaps and hold them to a small number
  (work out the number from the rates above), or mark the step the watcher
  saw and assert no gap outside the replacing.
- **Find a replacing with no gap,** if one exists (a Windows call not in the
  table, or a design where the name never has to be replaced). A direct
  system call needs `unsafe`, which the CLI forbids; a crate that holds one
  sets `unsafe_code = "deny"` with one `allow` and a test on it
  (`crates/alloc-count`, ADR-0021). Measure it as 2564 did, for at least
  1,000,000 moves, first. The replacing hard link had fewer gaps than
  `fs::rename` but not none.

**Out (do not do):**
- Skipping the test on Windows, or retrying a failed spawn (2561's *Out*).
- A change to the MCP server's restart, unless the ADR chooses it and says
  why.

## Implementation steps

1. Read 2561 and 2564, and `aside.rs`'s module docs, which already describe
   the gap.
2. Choose, and write the ADR.
3. Change the test (and the code, if the choice needs it).
4. Check the test still fails with 2561's old order: rename `from` to `to`,
   then the copy to `from`.
5. Run the `mcp::aside` tests beside a `cargo build --workspace`, 300 runs.

## Acceptance criteria

- [x] An ADR says what `put_aside` promises and why.
- [x] `a_program_is_at_its_path_at_every_moment_of_a_move`, or its
      replacement, fails with 2561's old order.
- [x] 300 runs of the `mcp::aside` tests beside a workspace build pass.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the watcher test as the ADR settles it.

## Completion notes

- **Chose the first option: the promise is what the system allows**
  (ADR-0115). `put_aside`'s order leaves no gap of its own; the system's
  replacing of the name may, rarely, on Windows. No call measured by 2564
  replaces a name without a gap, and a design with no replacing at all
  needs `unsafe` or a launcher stub, for a gap about one server start in
  20,000 meets; the ADR says why that lost.
- **The test** `a_program_is_at_its_path_at_every_moment_of_a_move` now
  counts separate gaps (each time the path, there on the watcher's last
  look, isn't), not missing looks, and allows `GAPS = 2` in `MOVES = 200`.
  From 2564's rates (4 failed runs in 300 under load, a gap in a run about
  one run in 75), three gaps in one run is about one in 2,500,000 if they
  fall independently.
- **It still fails 2561's bug:** with the old order (rename `from` to `to`,
  then the copy to `from`) put back by hand, three runs each saw
  `the path was missing 200 separate times in 200 moves`. The order was
  then restored, unchanged.
- **Under load:** the built `mcp::aside` tests ran 300 times while
  `cargo build --workspace` compiled in a separate target folder beside
  them: 0 failed.
- **Code:** unchanged; only `aside.rs`'s module docs and `put_aside`'s now
  state the promise as ADR-0115 does (and correct "one move in 40,000" to
  2564's 25 in 900,000, about one in 36,000).
- **Not established, still:** whether the gap is NTFS's own or a filter
  driver's. No ticket: ADR-0115's promise holds either way, and telling
  them apart needs Nick's real-time scanning off for no change here.
- **Docs:** ADR-0115 and `aside.rs`'s docs; nothing a game or the docs map
  describes changed.
- No break.
- Game: nothing to adopt, the change is a test and docs in `shuyi-cli`.
