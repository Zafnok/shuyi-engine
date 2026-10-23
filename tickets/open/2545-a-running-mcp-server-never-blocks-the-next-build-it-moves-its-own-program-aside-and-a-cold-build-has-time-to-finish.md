---
id: "2545"
title: "A running MCP server never blocks the next build: it moves its own program aside on Windows, and a cold build has time to finish"
type: bug
milestone: MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["1133", "2535"]
nick_input: none
completed:
---

# 2545 — A running MCP server never blocks the next build

## Context

1133 made `shuyi mcp`, and 2535 made it build before an agent connects
([ADR-0031](../../docs/adr/0031-the-mcp-server-hand-written-json-rpc-the-game-in-its-tool-program-and-session-tools-as-ops.md),
`docs/contract.md`, *The MCP server*). The entry an agent's project
registers runs the server through cargo: `cargo run -q -p shuyi-cli --
mcp` here (`.mcp.json`), `cargo shuyi mcp` in a game. Each Claude Code
session starts its own server process from that entry.

**On Windows the server fails to connect whenever its code moved while
another session's server runs.** Windows won't let a file be removed or
rewritten while a program runs from it, and cargo must do both to
relink. Found on 2026-10-22 on Nick's machine. All 13 failed connects in
Claude Code's MCP logs
(`%LOCALAPPDATA%\claude-cli-nodejs\Cache\<project>\mcp-logs-shuyi\*.jsonl`)
were this error, on stderr, about a second after start:

```
error: failed to remove file `D:\shuyi-engine\target\debug\shuyi.exe`
Caused by:
  Access is denied. (os error 5)
```

Cargo exits, stdio closes, and Claude Code reports `CONNECTION_CLOSED`.
Two older sessions' servers (`target\debug\shuyi.exe mcp`) were still
running from that file, and the checkout had moved to a new commit since
it was built. Every new session after that failed. It was reproduced by
hand: with `shuyi.exe mcp` running, `cargo build -p shuyi-cli` after a
`touch` of `crates/cli/src/main.rs` fails the same way.

**The way out, tested the same day:** Windows does allow a running
program's file to be *renamed*. Cargo writes the binary at
`target/debug/deps/shuyi-<hash>.exe` and hard-links it to
`target/debug/shuyi.exe`; both names are the one locked file. With the
server running, moving both names into another folder of the same
`target` let `cargo build` relink in 1 second. Cargo rebuilds a missing
output by itself (its fingerprint log says `FailedToReadMetadata` for
the `deps` file), so a moved program is never taken for a fresh one.

**The same lock hits a game's `start`.** In a game, the server and the
game's tool program are the same binary: the entry runs `cargo shuyi
mcp`, and `start` (`mcp::child::Cargo::launch`) runs `cargo run … --
mcp-host` of that package. After the agent edits the game's code,
`start` must relink the binary the server is running from, and on
Windows it fails the same way. The `mcp-host` it started earlier holds
the file too.

**A cold build can outlast Claude Code's wait.** Claude Code waits 30
seconds for a stdio server to answer `initialize` (its logs say
`Starting connection with timeout of 30000ms`). Building `shuyi-cli` cold
took 27 seconds on Nick's machine, and longer after a big change. 2535
prints `MCP_TIMEOUT` advice from `mcp install` when its ready run takes
over 20 seconds, but that leaves the setting to a person. Nick wants the
server to work in every new session without him doing anything.

2546 builds on this ticket, so that each session's server is its own
checkout's.

## Nick input

None.

## Scope

**In:**
- **The server moves its own program aside on Windows.** At the start of
  `shuyi mcp` (the server arm of `mcp::main`, not `install` or
  `--ready`) and of `mcp-host` (`lib.rs`), on Windows only:
  `std::env::current_exe()` and its hard-linked twin in `deps/` are
  renamed into `<exe dir>/.shuyi-running/`. The process keeps running
  from the renamed file, and the next build writes fresh ones.
  - The twin is each `<exe dir>/deps/<exe stem>-*.exe` whose length and
    modified time equal the program's. Hard links share both. Give
    each moved file a name no other process picks:
    `<stem>-<pid>-<n>.exe`.
  - Do it only when such a twin exists. That is how a cargo build
    output is told apart from a `shuyi` installed with `cargo install`
    (2005), which must never be moved.
  - Before moving, delete every file in `.shuyi-running/`. A file still
    running refuses deletion; skip it. So the folder never holds more
    than the programs running now.
  - Any error is a line on stderr (`mcp: couldn't move the program
    aside: …`), never a failure. The server still serves; only the next
    build may meet the lock, as today.
- **A cold build has time to finish.** First check Claude Code's
  documentation (code.claude.com, *MCP* and *Settings*). Confirm that
  `MCP_TIMEOUT` sets the wait for a server to start, and that a
  project's `.claude/settings.json` `env` applies to it. If both hold:
  - this repository gets a `.claude/settings.json` with `"env": {
    "MCP_TIMEOUT": "300000" }` (5 minutes);
  - `mcp install` adds the same setting, when it writes Claude Code's
    `.mcp.json`, to the project's `.claude/settings.json`. It merges
    into a file that exists, keeps other keys, and goes through the same
    diff, ask and `.bak` as the other files it writes;
  - 2535's advice line about `MCP_TIMEOUT` goes, since install has
    written it.

  If the documentation doesn't confirm both, write none of this. Say
  what it says in the completion notes, and keep 2535's line.
- **Docs:** `docs/contract.md`, *The MCP server*, says in two sentences
  that on Windows the server moves its program aside so a build never
  waits for it, and that `mcp install` gives the server's first build
  time to finish (if it does).

**Out (do not do):**
- Starting the session's own checkout's server, or resolving `project`
  against the session's folder: that is 2546.
- A copy of the program, or a launcher process in front of it: the
  file running would still be the one cargo writes.
- Changing where cargo builds (`--target-dir`, `CARGO_TARGET_DIR`).
  Two sessions in one checkout still share one program, and a second
  `target` means a second cold build.
- Linux and macOS: removing a running program's file is allowed there,
  and moving it would only make the next `cargo run` relink.

## Implementation steps

1. Read `crates/cli/src/mcp.rs` (`main`, `ready`), `lib.rs` (the
   `mcp-host` arm, line ~156), `mcp/child.rs`, `mcp/install.rs` (`plan`,
   `Planned`, `Machine`, `install`, `run_ready`) and their tests.
2. A module `crates/cli/src/mcp/aside.rs` with `pub fn move_aside(exe:
   &Path, pid: u32) -> Result<Vec<PathBuf>, String>`, returning the new
   paths (empty when the program has no twin), and `fn sweep(dir:
   &Path)`. It works on any OS so its tests run everywhere; only the
   call sites are `if cfg!(windows)`.
3. Call it from the server arm of `mcp::main` and the `mcp-host` arm,
   with `std::process::id()`, writing the stderr line on an error.
4. Check the documentation (Scope). If it confirms the setting, add
   `.claude/settings.json` here and the settings file to `install`'s plan
   for Claude Code, with tests beside 2535's.
5. `docs/contract.md`, then the `document` skill. The templates'
   `AGENTS.md` *The tools* lines change only if `install`'s output did.

## Acceptance criteria

- [ ] On Windows, with `shuyi mcp` running from `target/debug/shuyi.exe`,
      a change to `crates/cli/src/main.rs` and `cargo build -p
      shuyi-cli` succeeds, and so does a second server's start.
- [ ] On Windows, a game's `start` after an edit to the game's code
      succeeds while the server runs (the toy game, `crates/cli/toy`,
      or hello-grid).
- [ ] A program with no twin in `deps/` is never moved.
- [ ] `.shuyi-running/` holds only files of programs running now after
      any start.
- [ ] Either `MCP_TIMEOUT` is in this repository's `.claude/settings.json`
      and `mcp install` writes it, with the documentation's words quoted
      in the completion notes, or the completion notes say why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`mcp/aside/tests.rs`, every OS, in a temp folder): a program and
  its `deps` twin are both moved and the original names are free; a
  `deps` file of another length or time is left; no twin, nothing moved;
  `sweep` deletes what it can and skips what it can't (on Windows, a
  file held open); two moves with the same pid don't collide.
- Integration (`tests/it/binary.rs`, `#[cfg(windows)]`): copy the built
  `shuyi` into a temp `debug/` with a hard link in `debug/deps/`, start
  it with `mcp` and its stdin held open, then write a new file at both
  original paths while it runs, and check an `initialize` still answers.
- `install` (if the setting is written): the plan holds the settings
  file; a file with other keys keeps them; `--dry-run` writes nothing.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
