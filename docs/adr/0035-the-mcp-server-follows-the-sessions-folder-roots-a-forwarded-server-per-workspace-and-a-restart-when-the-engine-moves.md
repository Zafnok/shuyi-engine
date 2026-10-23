# ADR-0035: The MCP server follows the session's folder: roots, a forwarded server per workspace, and a restart when the engine moves

- **Status:** Accepted
- **Date:** 2026-10-22
- **Related tickets:** 2546 (this), 2535 (`--ready` and the stale
  limit), 2545 (the program moved aside), 2542 (the stale limit's two
  ways)
- **Extends:** ADR-0031 (the MCP server)

## Context

`shuyi mcp` builds from the checkout it is started in, and resolved a
relative `project` against its working folder. Claude Code's desktop app
starts a worktree session's server in the repository's main checkout,
not the worktree (found on Nick's machine from Claude Code's MCP logs).
An agent in a worktree then played and checked the main checkout's copy
of its game, and listed the main checkout's tools, and nothing said so.

What Claude Code sends (2.1.291, checked headless for 2546):
`initialize` declares `capabilities.roots` (with `listChanged: true`);
it sends `tools/list` before answering the server's `roots/list`; the
answer's first root is the session's working folder as a `file:///C:/…`
URI.

The server is one synchronous loop over stdin (ADR-0031 §1). It can't
rebuild itself in place: a running program can't relink over its own
file (2545), and its tools and schemas are compiled in.

## Decision

1. **The session's folder is the client's first root.** A client that
   declares `roots` gets a `roots/list` request (id `shuyi-roots`) after
   `notifications/initialized`. `Server::handle` returns every message
   to write (`Vec<String>`): a response, and the server's own requests
   and notifications. A response to the server's request is taken, not
   answered. A relative `project`, the default project, `new`'s `dir`
   and `import`'s folder resolve against the session folder.
2. **A project in another cargo workspace is answered by that
   workspace's own server.** `mcp::forward` holds the seam:
   `Forwarding` (`workspace(folder)`, by `cargo locate-project
   --workspace`, cached per project root, and `spawn(words, root)`) and
   `Forward` (`ready`, `request`), with `Processes` and `Process` for
   real and fakes in the tests. A forwarded server is one process per
   workspace, kept for the session: the project's `command` and `mcp` in
   its root; with no `command`, the workspace's `.mcp.json` `shuyi` entry
   in the workspace (the engine repository has no `shuyi.ron` at its
   root, and its `.mcp.json` is how its own server runs); else `shuyi
   mcp`. It gets `initialize` without roots at once, so its build runs in
   the background; a call waits for its answer. Each call goes with its
   `project` (and `dir`) made absolute, and its answer goes back under
   the client's id, as it is. A forwarded server that stops is forgotten,
   and its call answers `SH3005` with its stderr's last lines, so the
   next call builds it again.
3. **No chains.** A forwarded server runs with `SHUYI_MCP_FORWARDED`
   set, and a server with it set forwards nothing.
4. **The tool list follows.** The server declares `listChanged: true`.
   `serve` reads stdin on a thread and wakes every 200 ms to
   `Server::poll`: once the session folder's forwarded server has
   answered `initialize`, the server fetches its `tools/list` and, if it
   differs, sends `notifications/tools/list_changed`, then lists it and
   hands a tool it doesn't know to that server.
5. **A stale server restarts.** When a host's answer speaks another
   contract than the server's `VERSION` and the project is in the
   server's own workspace, a forwarded server of its own workspace
   answers that call and every later one. The stale limit (2535) is
   written only if that one doesn't start.
6. **Nothing outlives the session.** A forwarded server ends when its
   stdin closes; dropping it closes its stdin, waits a moment, then
   kills it.

## Consequences

- A worktree session works with no step of a person's: its calls build
  and run the worktree's engine and game, and its tool list is the
  worktree's. The same holds for a game's worktrees (`cargo shuyi mcp`).
- `--ready` and a reconnect are no longer needed after the engine moves;
  `--ready` stays for a first build before connecting.
- The first call in a worktree waits for that worktree's build (27 s
  cold on Nick's machine); the build starts at the roots' answer.
- `Server::handle` returns a `Vec`: a game's test that drove it directly
  changes with its pin move.
- Each project root costs one `cargo locate-project` per session.
- The MCP `_meta` of a forwarded call (a progress token) is not passed
  on.
- `roots/list_changed` is not read: the roots are read once (ticket
  2547 reads them again when they change).

## Alternatives considered

- **Re-exec the server from the worktree** (the server replaces itself
  with the worktree's build). Lost: on Windows there is no `exec`, the
  client's pipe belongs to the first process, and the server would have
  to hand over a session mid-flight.
- **Only resolve `project` against the root**, without forwarding. Lost:
  the game's tool program would build from the worktree, but `lint`,
  `explain`, `new` and the tool list would still be the main checkout's.
- **Ask the client to start the server in the worktree.** Lost: that is
  the app's behavior, outside this repository.
- **Forward with the server's own command line.** Lost: the server
  doesn't know how it was started, and a game's `cargo shuyi` is its
  own tool program, not `shuyi-cli`.
