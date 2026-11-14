---
id: "2599"
title: "A game's MCP `start` can fail on Windows: cargo is refused removing the copy the move aside left in its program's place"
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] The cause is named in the completion notes, with how it was found.
- [ ] Twenty runs of the reproduction pass in a row.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the wait or the retry, with a fake build that answers busy
  once and then builds.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
