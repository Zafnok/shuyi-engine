---
id: "2599"
title: "A game's MCP `start` can fail on Windows: cargo is refused removing the copy the move aside left in its program's place"
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2599 — A game's MCP `start` can fail on Windows: cargo is refused removing the copy the move aside left in its program's place

## Context

Found while working 2074, on 2026-11-14, on Nick's machine. In one full
`cargo xtask gates` run, the `shuyi-perf` skill example's step 10
(`cargo shuyi mcp: start, perf, step, perf, perf (fails), stop`) failed
on its first call:

```
call 1 (start): its isError is true, not false; error: the game's tool
program stopped before it answered (exit code 101): error: failed to
remove file `…\target/skill-examples-build\debug\examples\tools.exe`

Caused by:
  Access is denied. (os error 5); code: SH3005
```

The same example passed alone right after (`cargo xtask skill-examples
shuyi-perf`), and the whole `skill-examples` run passed the next time.
No `tools.exe` was still running after the failed run.

In a game the MCP server and the game's tool program are one binary
(`examples/tools.exe`). On Windows the server moves its running file
aside and leaves a copy in its place (`crates/cli/src/mcp/aside.rs`,
2545, 2561, 2564, 2574), so `start`'s `cargo run … -- mcp-host` can
relink. Here cargo's own remove of that path was refused. The copy is a
new `.exe` made a moment before. A guess, not checked: an antivirus scan
of the new copy holds it while cargo removes it. `aside.rs` retries its
own steps while the file is busy, but cargo's remove isn't retried.
2574 is about a different race (a replaced name read as missing).

Whether 2074 made this likelier is not known: the games now build in a
target folder of their own (`target/skill-examples-build`), with no
incremental cache. It is not caused by the flag: what links, and when,
is the same.

## Nick input

None.

## Scope

**In:**
- Find what holds `examples/tools.exe` when cargo removes it (Process
  Monitor or Sysinternals `handle`, or a log in `aside.rs` of the copy's
  time against the child's build).
- Make `start` hold when the path is busy for a moment, for example by
  waiting in `mcp::child` until the copy can be opened for delete
  (the same `BUSY` codes and limits as `aside.rs`), or by retrying the
  child's build once when its stderr is cargo's `failed to remove file`
  with os error 5 or 32.

**Out (do not do):**
- Changing the move-aside order (ADR-0115).
- A skip or a retry in the skill example's script.

## Implementation steps

1. Reproduce: run `cargo xtask skill-examples` in a loop (or the
   `shuyi-new-game` and `shuyi-perf` examples one after the other), and
   count the failures.
2. Find the holder (step 1 of the scope).
3. Fix in `crates/cli/src/mcp/child.rs` or `aside.rs`, as the finding
   says.

## Acceptance criteria

- [x] The cause is named in the completion notes, with how it was found.
- [x] Twenty runs of the reproduction pass in a row.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the wait or the retry, with a fake build that answers busy
  once and then builds.

## Completion notes

**The cause.** After the move aside, the program's name
(`examples/tools.exe`) and its twin's (`examples/tools-<hash>.exe`) held
two separate copies. Cargo links a program's name to its twin on every
build, a fresh one too (its `link_targets` runs for fresh units), and
when the two names are different files it first removes the program's
name. So every `start` after a move aside removed the copy made a moment
before, and anything holding that new `.exe` then (most likely a scan)
refused it: `failed to remove file`, os error 5.

How it was found: a scratch crate with one example, built, then both
names replaced by copies as `put_aside` leaves them. `cargo build -v`
said `Fresh`, yet `tools.exe`'s file id changed to the twin's: removed
and linked again. Held open without delete sharing, the same build
failed with `failed to remove file … (os error 32)`; with the two names
hard-linked to one file, held the same way, it passed. In the real
reproduction, after the MCP step, `tools.exe` and its twin now show one
file id (`fsutil file queryfileid`). No holder was caught by name
(Process Monitor isn't installed here): the fix doesn't depend on who it
is, since cargo no longer removes anything on a fresh `start`.

**The fix** (`crates/cli/src/mcp/aside.rs`), the order of the moves kept
(ADR-0115):

- a twin's copy is the program's copy under one more name (a hard link),
  not a second new file: cargo's own layout, so cargo leaves it alone;
- before `move_aside` returns, it waits until the copy can be opened for
  delete (`BUSY` codes, at most `TRIES` × `PAUSE`, two seconds), so a
  build that does relink right after (the game's code changed) finds it
  free. A hold that lasts is left to the build: the move is done.

Deviations: the fix is in `aside.rs` rather than `mcp::child` (the
ticket allowed either): the cause is what the move aside leaves, and
removing it there means no `start` waits or retries at all when the
build is fresh. The unit test of the wait holds a real file shared for
reading only and lets go after 50 milliseconds, rather than a fake
build: the wait is on the file, not on a build.

Tests: `a_copy_held_for_a_moment_is_waited_for_until_it_can_be_deleted`
(Windows), and the first test now checks the two copies are one file.

Reproduction: `cargo xtask skill-examples shuyi-new-game shuyi-perf`,
20 runs in a row, all passed (about 3 minutes each).

Docs: `docs/contract.md`'s paragraph on the move aside says the copy is
one file under both names.

Behaviour: shuyi-cli: on Windows, the copy `mcp` and `mcp-host` leave in
their program's place is one file under the program's name and its
twin's, and the server waits up to two seconds at start for it to be
free to delete. Searched the game for `shuyi-running` and `move_aside`
(`gh search code`): no hit.

Book: none, no public item changed and the book doesn't describe the
move aside (it's the tool's own housekeeping, in `docs/contract.md`).
