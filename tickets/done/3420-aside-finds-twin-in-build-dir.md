---
id: "3420"
title: "A running MCP server moves its program aside again under Rust 1.100's build layout"
type: bug
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-23
---

# 3420 — A running MCP server moves its program aside again under Rust 1.100's build layout

## Context

Found on 2026-11-23 on Nick's machine, by a session of the game (no
game ticket: the session that found it filed this one). Every
new Claude Code session of the game failed to start its `shuyi` server
with `Connection closed`. Claude Code's MCP log held cargo's stderr:

```
error: failed to remove file `D:\tactical-rpg\target\debug\trpg-tools.exe`

Caused by:
  Access is denied. (os error 5)
```

22 earlier `trpg-tools.exe shuyi mcp` processes, from 2026-11-20 and
2026-11-21, were still running from that path, and nothing of
`trpg-tools` was in `target/debug/.shuyi-running/`. The newest file
there was the engine's own `xtask`, from 2026-11-18.

The cause: Rust 1.100 (pinned on 2026-11-18) builds in cargo's new
build-dir layout. A program's hard-linked twin is no longer
`<profile>/deps/<stem>-<hash>.exe` but
`<profile>/build/<package>/<hash>/out/<crate name>.exe`, the crate name
with `_` for `-` (`build/trpg-tools/9f110cbb8b53d455/out/trpg_tools.exe`).
There is no `deps/` folder at all. `aside::twins` (`crates/cli/src/mcp/aside.rs`,
tickets 2545, 2561, 2599) looks only in `deps/` and beside the program,
finds no twin, and `move_aside` takes "no twin" to mean a program
installed with `cargo install`: it moves nothing and says nothing. So
every server on Windows holds its program, and the first `cargo run` that
must relink it (any change of the game's code or pin in that checkout)
fails until every server started from it ends. This is every game on the
engine, and the engine's own `cargo run --bin shuyi -- mcp`.

## Nick input

None.

## Scope

**In:**
- `aside::twins` also finds the twin in the new layout:
  `<profile>/build/*/*/out/<name>` for a binary at `<profile>/<stem>`, and
  for an example at `<profile>/examples/<stem>`, where `<name>` is the
  stem, or the stem with `_` for `-`, with the program's extension, and
  the same length and modified time (as the old layout's twins are
  matched).
- A program cargo built (a `<stem>.d` beside it) whose twin isn't found
  says so on stderr, once, instead of silently serving with its file
  held: the next build in that checkout may fail to relink it.
- The module docs say both layouts.

**Out (do not do):**
- A `build-dir` configured outside the target folder
  (`CARGO_BUILD_BUILD_DIR`, `build.build-dir`): the warning covers it.
- Ending stale servers, or anything in the game's repository: its pin
  move comes by the engine watcher.

## Implementation steps

1. In `aside.rs`, add the new layout's candidates to `twins`: the
   profile folder (the program's folder, or its parent for
   `examples/`), each `build/<package>/<hash>/out/` under it, and in each
   only the one or two file names that could be the twin (checked with
   `stamp`, no folder listing of `out/`).
2. In `move_aside`, when no twin is found and `<stem>.d` is beside the
   program, return `Ok` as before but print one line to stderr naming
   the program and that its next relink may be refused.
3. Docs at the top of `aside.rs`.

## Acceptance criteria

- [x] A program at `debug/trpg-tools.exe` with its hard-linked twin at
      `debug/build/trpg-tools/<hash>/out/trpg_tools.exe` and no `deps/`
      is moved aside with its twin, and a copy left in both places.
- [x] The same for an example at `debug/examples/tools.exe`.
- [x] A file in `build/*/*/out/` of another length or time is left.
- [x] A program with no twin in either layout is still never moved.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: in `crates/cli/src/mcp/aside/tests.rs`, a binary and an example
  in the new layout are moved with their twins; a look-alike of another
  length is left; the no-twin test still holds with a `build/` folder.

## Completion notes

Done on 2026-11-23, from the game's session that found it.

- `aside::twins` also looks in `<profile>/build/*/*/out/` for the stem
  or its crate name (`_` for `-`), with the same length and time
  (`build_dir_candidates`), for a binary and for an example. Only those
  names are checked, so no `out/` folder is listed.
- `move_aside` says on stderr when a cargo build output (`<stem>.d`
  beside it) has no twin it can find.
- Checked on Nick's machine with the game built on this checkout: its
  `trpg-tools.exe shuyi mcp` moved both files into
  `target/debug/.shuyi-running/`, and `cargo build -p trpg-tools` relinked
  while that server ran. Before this, the same build was refused.
- On the machine, the held `D:\tactical-rpg\target\debug\trpg-tools.exe`
  was renamed aside by hand so new sessions could start again.
- Docs: `docs/contract.md`'s paragraph on the move aside names both
  layouts.
- No public API changes. The game takes this with its next pin move
  (the engine watcher files it).

Book: no chapter covers moving the server's program aside; the contract
doc is where it is described.
