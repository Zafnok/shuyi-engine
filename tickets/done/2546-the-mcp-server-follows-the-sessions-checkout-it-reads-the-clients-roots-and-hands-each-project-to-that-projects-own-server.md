---
id: "2546"
title: "The MCP server follows the session's checkout: it reads the client's roots and hands each project to that project's own server"
type: feature
milestone: MVP
model: reasoning
effort: high
status: done
blocked_by: ["2545"]
nick_input: none
completed: 2026-10-22
---

# 2546 — The MCP server follows the session's checkout

## Context

`shuyi mcp` (1133, 2535,
[ADR-0031](../../docs/adr/0031-the-mcp-server-hand-written-json-rpc-the-game-in-its-tool-program-and-session-tools-as-ops.md),
`docs/contract.md`, *The MCP server*) builds from the checkout it is
started in. It resolves a relative `project` against its working folder,
and its tool list and `lint` are that checkout's engine.

**Claude Code's desktop app starts a worktree session's server in the
repository's main checkout, not the worktree.** Found on 2026-10-22 on
Nick's machine, from Claude Code's MCP logs
(`%LOCALAPPDATA%\claude-cli-nodejs\Cache\<project>\mcp-logs-shuyi\*.jsonl`):

- worktree sessions failed on `D:\shuyi-engine\target\debug\shuyi.exe`,
  the main checkout's program, though `cargo metadata` in the worktree
  names the worktree's own `target`;
- worktree sessions connected in under a second when their worktree had
  no `target` yet (a cold build there takes 27 seconds);
- the running servers' command line was `target\debug\shuyi.exe mcp`,
  which cargo writes relative to its working folder: `D:\shuyi-engine`.

So an agent in a worktree that edits the engine or a game and passes
`project: "examples/hello-grid"` plays the *main checkout's* copy. Its
edits are never built, and nothing says so. Its tools are those of
whatever commit the main checkout has, which nobody updates on purpose.
2535's limit, when contracts differ, asks a person to run `--ready` and
reconnect. Nick wants the server to work in every new worktree with no
step of his. The same holds for a game made with the engine, and for its
worktrees: the game's entry is `cargo shuyi mcp`.

Two MCP features let the server do this itself (spec 2025-06-18):

- **Roots.** A client that declares `capabilities.roots` in
  `initialize` answers a `roots/list` request with its folders as
  `file://` URIs. Claude Code is expected to list the session's working
  folder; step 1 checks it.
- **`notifications/tools/list_changed`.** A server that declares `tools:
  { listChanged: true }` sends it, and the client fetches `tools/list`
  again.

2545 made the server started in the main checkout connect while
another runs: on Windows each server moves its program aside, so two
sessions in one worktree share its program without blocking a build.

2542 is not done: its item 1 changes the `stale` message, which is now
the limit written only when a fresh server fails to start.

## Nick input

None.

## Scope

**In:**
- **The session's folder.** When `initialize` declares
  `capabilities.roots`, the server sends a `roots/list` request after
  `notifications/initialized`, and takes the first root as the session's
  folder. A relative `project`, and the default project when `project`
  is left out, resolve against it; without roots, against the working
  folder, as today. `Server::handle` then returns the messages to write,
  not one response: the request, its answer, and later the
  notification below.
- **Each project gets its own server.** Each `tools/call` resolves its
  project to its cargo workspace root: `cargo locate-project --workspace
  --message-format plain`, run in the project's root, cached per root.
  When that isn't the server's own workspace (its working folder's), the
  call goes to that workspace's **forwarded server**:
  - it is the project's `command` (`shuyi.ron`, split into words as
    `install::run_ready` does) and `mcp`, run in the project's root,
    stdio piped, one per workspace, kept for the session;
  - it gets an `initialize` without `roots` (so it never asks for them),
    then `notifications/initialized`;
  - each call is sent with its `project` made absolute and its own id,
    and the answer goes back under the client's id, as is.
- **When the forwarded server starts.** At the roots answer, if the
  session's folder is in another workspace than the server's, it is
  started then (a spawn: its build runs in the background while the
  agent works). Otherwise, at its first call. A call waits for its
  `initialize` answer. If it fails to start, the call gets an error
  result with `codes::GAME_PROGRAM` (SH3005, what `start` answers when
  the tool program doesn't build) and the last lines of its stderr, as
  `mcp::child` keeps them. SH3005's help line and page
  (`contract/codes.rs`) then also name the project's own server.
- **The tool list follows.** The server declares `listChanged: true`.
  Once the session folder's forwarded server has answered `initialize`,
  the server fetches its `tools/list`. If it differs from its own, the
  server sends `notifications/tools/list_changed` and from then answers
  `tools/list` with that list.
- **A stale server restarts.** When a host answer's `contract` differs
  from the server's own `VERSION` (2535's `stale`), and the project is
  in the server's own workspace, the server starts a forwarded server
  for its own workspace and sends that call, and every later one, to
  it. The check-out moved under it, and the fresh build is the new
  engine. 2535's limit is written only if that fresh server fails to
  start.
- **Nothing outlives the session.** A forwarded server ends when its
  stdin closes, as `serve` does today, so it ends when the server ends,
  even if the server is killed. Its own game host (`mcp-host`) ends the
  same way. Then nothing runs from a worktree's `target` once its
  session ends, and the app can delete the worktree after its PR merges.
  The worktree's builds go with it, so nothing needs cleaning after a
  merge.
- **Docs and the words agents read:**
  - `docs/contract.md`, *The MCP server*: the session's folder, the
    forwarded servers, the list that follows, the restart. Delete the
    "run `--ready`, then reconnect" advice: `--ready` stays, for a
    first build before connecting.
  - The `project` property's description (`mcp.rs`, `project_schema`)
    says it is relative to the session's folder.
  - The server's `instructions` (`server.rs`, `initialize`) keep "call
    `start` again after changing the game's code": `start` still
    rebuilds the game. Only the tool list follows by itself.
  - The templates' `AGENTS.md` *The tools* line for `mcp install`
    (`templates/blank`, `templates/grid`) drops "after updating the
    engine, run `cargo shuyi mcp --ready` and reconnect the server".

**Out (do not do):**
- Asking Claude Code to start the server in the worktree: that is the
  app's behavior, outside this repository.
- `roots/list_changed` (a session that changes folder): read the roots
  once. Write a follow-up ticket only if step 1 finds that Claude Code
  sends it.
- A forwarded server for a project outside any cargo workspace: there
  is no engine to build there; serve it in the server, as today.
- Moving the program aside: 2545.

## Implementation steps

1. **Check what Claude Code sends.** On the branch, add a temporary
   stderr line in `initialize` with the client's `capabilities`. Build,
   then run Claude Code headless with a config naming the built program:
   `claude -p "list your tools" --mcp-config <file>`, from a scratch
   folder, with `<file>` an `.mcp.json` whose `command` is the built
   `shuyi.exe` and `args` `["mcp"]`. Read that run's
   `mcp-logs-shuyi` file for the line (Claude Code logs a server's
   stderr). Then do the same with a `roots/list` request, and note
   whether the first root is that scratch folder. Write both findings in
   the completion notes, and delete the line. If Claude Code declares no
   roots, the session's folder is the working folder (unchanged), and
   the rest still holds for an absolute `project`. Then the `project`
   description says an absolute path is safest.
2. Read `crates/cli/src/mcp/server.rs` (`handle`, `initialize`,
   `answer_of`, `stale`), `mcp.rs` (`serve`, `project_schema`),
   `mcp/child.rs` (`Cargo`, `Program`, `keep_tail`), `mcp/install.rs`
   (`run_ready`) and `project.rs` (`Project::find`, `Config::command`).
3. `handle` returns `Vec<String>`; `serve` writes each. Responses to the
   server's own requests (`roots/list`'s answer, by id) are taken, not
   answered.
4. A module `mcp/forward.rs`: `Forwarded` (the child, its stdin, a
   reader, its stderr tail, its next id), behind a trait as `Launch` is,
   so the tests use a fake that answers from a script. Its `call` sends
   one request and reads lines until the reply with that id.
5. The workspace lookup behind the same seam, so the tests don't run
   cargo.
6. Roots, forwarding, the list that follows, the restart, each with its
   tests (below).
7. `docs/contract.md`, the descriptions, the templates' lines, then the
   `document` skill. Raise the contract's version only if a schema
   changed (`docs/contract.md`, *The schemas*).

## Acceptance criteria

- [x] With a client that declares roots, a relative `project` resolves
      against the first root, not the working folder.
- [x] A call whose project is in another workspace is answered by that
      workspace's own server (the fake sees it, with `project` made
      absolute), and the client gets the answer under its own id.
- [x] When the session folder's server lists other tools, the client gets
      `notifications/tools/list_changed`, then that list.
- [x] A host answer of another contract in the server's own workspace
      starts a fresh server for it, and the call is answered by it.
- [x] A forwarded server that doesn't build gives an error result with a
      code and its stderr's last lines.
- [x] Closing the server's stdin ends every forwarded server (the
      integration test).
- [x] The completion notes say what Claude Code sent in step 1.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`mcp/server/tests.rs`, `mcp/forward/tests.rs`, with fakes): the
  `roots/list` request is sent only to a client that declares roots;
  a `file://` URI (with a Windows drive and with a percent-escape) to a
  path; a relative and a missing `project` against the root; a call
  forwarded and its id mapped back; one forwarded server per workspace;
  the list change and the notification; the stale restart; a forwarded
  server that exits before answering.
- Integration (`tests/it/binary.rs`): the built `shuyi mcp` with a
  `roots/list` answer naming a copy of the toy game (`crates/cli/toy`)
  in a temp workspace of its own. Its `explain` call is answered by a
  forwarded server (one more `shuyi` process while it runs), and
  closing stdin leaves none.

## Completion notes

**What Claude Code sent (step 1).** Claude Code 2.1.291, headless
(`claude -p … --mcp-config <file>`), a probe build logging each message
to a file (Claude Code logs a server's stderr only while it connects):

- first a `server/discover` request (protocol `2026-07-28`), which the
  server answers "Method not found", then `initialize` with protocol
  `2025-11-25` and `capabilities: { "roots": { "listChanged": true },
  "elicitation": { "form": {}, "url": {} } }`;
- it sends `tools/list` right after `notifications/initialized`,
  *before* it answers the server's `roots/list`; so the first list is
  always the server's own, and a different one comes by
  `notifications/tools/list_changed`;
- the `roots/list` answer is one root, the session's working folder:
  `file:///C:/Users/…/scratchpad/probe/work` (a drive, forward slashes;
  a worktree session gave `file:///D:/shuyi-engine/.claude/worktrees/…`).

Claude Code declares `roots.listChanged`, so it will say when the
session's folder changes: follow-up **2547** reads the roots again then.

**What was done.**
- `mcp/forward.rs`: the `Forward` and `Forwarding` seam, `Processes`
  (cargo's `locate-project --workspace`, canonical paths without the `\\?\` prefix)
  and `Process` (the child, its stdin, its stdout's lines on a thread, its
  stderr's tail through `child::keep_tail`), `words` (how a project's own
  server is run), `path_of_uri`.
- `Server::handle` returns `Vec<String>`; `Server::poll` sends
  `notifications/tools/list_changed`; `serve` reads stdin on a thread and
  polls every 200 ms. `Server::forwarding` turns forwarding on (`mcp::main`
  does, unless `SHUYI_MCP_FORWARDED` is set).
- The tools list `listChanged: true`; SH3005's help line and page name
  the project's own server; `docs/contract.md`, the usage text and the
  templates' `AGENTS.md` lines drop "run `--ready` and reconnect".
- [ADR-0035](../../docs/adr/0035-the-mcp-server-follows-the-sessions-folder-roots-a-forwarded-server-per-workspace-and-a-restart-when-the-engine-moves.md)
  records the design.
- Checked live: a server started in `D:\shuyi-engine` (as the desktop app
  does) with Claude Code in this worktree started the worktree's own
  server at the roots' answer, and Claude Code's `explain` call was
  answered by it.

**Deviations.**
- **A folder with no `command` uses the workspace's `.mcp.json`.** The
  engine repository has no `shuyi.ron` at its root, so an engine
  worktree's session folder named no command. The forwarded server is the
  project's `command` and `mcp`; else the `shuyi` entry of the
  workspace's `.mcp.json` (the engine's is `cargo run -q -p shuyi-cli --
  mcp`), run in the workspace; else `shuyi mcp` in a project; else the
  server answers itself.
- **`SHUYI_MCP_FORWARDED`.** A forwarded server forwards nothing, so a
  stale restart can't chain.
- **Session tools follow `start`.** A `start` forwarded to a workspace
  keeps the session there (`step`, `scene`, … go to it), and a `start`
  elsewhere stops it first.
- **A tool only the followed list has** goes to the followed server.
- **The contract is 13.** The `start` input's `project` description now
  says "from the session's folder"; that changed
  `session.start.input.json`, so `VERSION` rose (`versions.json`, the
  example answer in `docs/contract.md`). No shape changed.
- **SH3005's help line** is rewritten with single spaces, so 2542's
  item 2 now only has `lint`'s description left (its text says so).
- The `project` description doesn't say "an absolute path is safest":
  Claude Code declares roots.

**API change for the game.** `mcp::server::Server::handle` returns
`Vec<String>` instead of `Option<String>`, and `mcp::serve` takes an
input that is `Send + 'static`. The game's PR that moves the pin follows
if a game test drives either.

**Notes for Nick.** A worktree session's tools now build and run that
worktree's engine and game by themselves: no `--ready`, no reconnect.
The first call in a fresh worktree waits for its build (about 27
seconds cold); the build starts as the session connects.
