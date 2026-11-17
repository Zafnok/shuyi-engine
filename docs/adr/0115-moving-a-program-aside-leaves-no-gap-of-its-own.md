# ADR-0115: Moving a program aside leaves no gap of its own; the system's replacing of a name may, rarely

- **Status:** Accepted
- **Date:** 2026-11-13
- **Related tickets:** 2545, 2561, 2564, 2574

## Context

On Windows, `shuyi mcp` and `mcp-host` move their own program aside so the
next build can write a fresh one (ticket 2545, `crates/cli/src/mcp/aside.rs`).
Ticket 2561 promised that `put_aside` never leaves the program's path
empty: it hard-links the running file to its new name, then renames a copy
over the old name, rather than renaming the file away and the copy in after
it. `a_program_is_at_its_path_at_every_moment_of_a_move` held it strictly: a
watcher thread had to see the path missing 0 times in 200 moves.

Ticket 2564 measured that promise on Windows 11 (NTFS, temp folder on `C:`),
beside a workspace build. The path really is empty for a moment, inside the
call that replaces the name: `os error 2`, the folder doesn't list it, for
tens of milliseconds (59 ms in the one timed). Every call tried has it:

| Replacing call | Moves | Gaps |
| -------------- | ----- | ---- |
| `fs::rename` | 900,000 | 25 |
| POSIX-semantics rename (`FileRenameInfoEx`) | 150,000 | 5 |
| Replacing hard link (`FileLinkInformationEx`) | 750,000 | 3 |

So the test failed about one run in 75 under load (4 of 300), on Nick's
machine and possibly on CI's Windows runners. Whether the gap is NTFS's own
or a filter driver's (Defender's) was not established. For a person it is
rare: a server start moves two files, so about one start in 20,000 opens a
gap, and something must start the program inside it.

## Decision

1. **The promise is what the system allows.** `put_aside` leaves no gap of
   its own: no step of its order removes the name before another puts it
   back. The one step that touches the name, replacing it with the copy in
   one call, may on Windows read as missing for a moment, rarely, and that
   is the system's. `aside.rs`'s module docs and `put_aside`'s say so.
2. **The test tells the two apart by counting separate gaps.** The watcher
   counts each time the path, there on its last look, isn't (a gap's
   start), not each look that misses. The old order opens one gap in every
   move, 200 in a run of 200 moves (measured: 200 every run). The system's
   replacing opens one in a run about one run in 75 under load, so the
   test allows `GAPS = 2` in `MOVES = 200`: a run with three is about one
   in 2,500,000 if the gaps fall independently. A change that brings back
   any order opening a gap per move fails the test every run.
3. The code is unchanged: `fs::rename` stays the replacing call.

## Consequences

- The watcher test stops failing on busy Windows machines, CI's included,
  and still fails 2561's bug in every run.
- A program started inside one of the system's gaps can still fail to find
  `shuyi.exe`, about one server start in 20,000 opening such a gap. Retrying
  a failed spawn stays out (2561's *Out*).
- A regression that opens gaps only now and then (a few per run, not one per
  move) could pass. Nothing today does that; an order either removes the
  name between steps in every move or never does.
- If gaps grow common (a new filter driver, a new Windows), the test fails
  with the count in its message, and this ADR's numbers are the ones to
  re-measure.

## Alternatives considered

- **A replacing with no gap.** None of the three Windows calls measured has
  none; the replacing hard link had the fewest but not none. Another call
  (`MoveFileTransacted`, deprecated) or a design where the name is never
  replaced (a launcher stub at the path that never changes) would need a
  crate holding `unsafe` and a million-move measurement first, or a change
  to what cargo builds at that path, for a gap a person meets about once in
  20,000 starts. Not worth it.
- **Mark the step the watcher saw, and assert no gap outside the
  replacing.** Stronger, but `put_aside` would take a hook for the test to
  see its steps, production code shaped for one test, and the old order's
  gap also spans the replacing step, so the marks would need care to tell
  them apart. Counting gaps fails the old order every run without any of
  that.
- **Skip the test on Windows, or retry a failed spawn.** Forbidden by 2561's
  *Out*: Windows is the only system where a program is moved aside at all.
- **Keep the strict assertion.** It fails about one run in 75 under load
  for a gap no change to this code can close.
