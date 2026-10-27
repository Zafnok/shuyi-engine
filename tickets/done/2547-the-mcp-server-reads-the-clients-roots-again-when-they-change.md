---
id: "2547"
title: "The MCP server reads the client's roots again when they change"
type: feature
milestone: MVP
model: sonnet-5
effort: medium
status: done
blocked_by: ["2546"]
nick_input: none
completed: 2026-10-26
---

# 2547 — The MCP server reads the client's roots again when they change

## Context

2546 ([ADR-0035](../../docs/adr/0035-the-mcp-server-follows-the-sessions-folder-roots-a-forwarded-server-per-workspace-and-a-restart-when-the-engine-moves.md),
`docs/contract.md`, *The MCP server*) made the server take the client's
first root as the session's folder. It asks for the roots once, after
`notifications/initialized`, and its answer resolves a relative
`project` and starts the session folder's own server (a forwarded
server, `crates/cli/src/mcp/forward.rs`) when that folder is in another
cargo workspace.

Claude Code 2.1.291 declares `capabilities.roots.listChanged: true` in
`initialize` (2546's completion notes), so it says when its roots
change: it sends `notifications/roots/list_changed`. The Claude desktop
app can move a session to another folder while it runs. Today the
server ignores that notification, and keeps resolving against the old
folder and following the old folder's tool list.

## Nick input

None.

## Scope

**In:**
- On `notifications/roots/list_changed`, from a client that declared
  `roots`, the server sends `roots/list` again (the same id,
  `server::ROOTS`), and handles the answer as the first one: the new
  first root is the session's folder.
- If the new folder is in another workspace than the server's and than
  the old folder's, that workspace's forwarded server starts then, and
  the tool list follows it (`Server::poll`). If the new folder is in the
  server's own workspace, the server lists its own tools again, and
  sends `notifications/tools/list_changed` if that changes the list.
- A session game keeps running where it runs; forwarded servers already
  started stay until the session ends.
- `docs/contract.md`, *The MCP server*: one sentence on the change.

**Out (do not do):**
- More than the first root: the session's folder stays one folder.
- Stopping forwarded servers of a folder left behind.
- A change to how the first roots are asked for.

## Implementation steps

1. Read `crates/cli/src/mcp/server.rs`: `handle` (the notification
   branch), `rooted`, `poll`, `forward_to`, the fields `follow`,
   `listing` and `tools`.
2. In `handle`'s notification branch, answer
   `notifications/roots/list_changed` like `notifications/initialized`
   when `self.roots` is set.
3. In `rooted`, when the new folder's workspace is the server's own (or
   none), set `self.tools` back to `tools()` and `self.listing` to none,
   and return the notification if the list changed; else start and
   follow as today.
4. Tests in `crates/cli/src/mcp/server/tests.rs`, with the `Workspaces`
   fake.
5. `docs/contract.md`, then the `document` skill.

## Acceptance criteria

- [x] A client that declared roots and sends
      `notifications/roots/list_changed` gets a `roots/list` request;
      one that didn't declare roots gets nothing.
- [x] After the new answer, a relative `project` resolves against the
      new first root.
- [x] Moving to a folder of another workspace whose server lists other
      tools sends `notifications/tools/list_changed`, then that list;
      moving back to the server's own workspace lists the server's own
      tools again, with the notification.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`mcp/server/tests.rs`): the request sent only to a client with
  roots; the folder changed; the list followed both ways.

## Completion notes

Done as written. `notifications/roots/list_changed` from a client with roots sends `roots/list` again; `rooted` now returns to the server's own list (with `tools/list_changed`) when the new folder is in its own workspace. One addition: `poll` repoints `listing` when another workspace's server lists the same tools as the one followed, so unknown tools do not go to the folder left behind. No break, no follow-ups, nothing for Nick.
