---
id: "3436"
title: "The MCP server's long tools run as tasks: start's build, playtest, autobalance, bake and export answer a task the client polls"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3435"]
nick_input: none
completed: 2026-11-27
---

# 3436 — The MCP server's long tools run as tasks

## Context

Anthropic's article *Skills over MCP, tasks and more* (2026-11-24) made
MCP **tasks** generally available in Claude and Claude Code: a
`tools/call` the server can't finish quickly answers a task at once, and
the client polls it. They are MCP's tasks extension,
`io.modelcontextprotocol/tasks` (SEP-2663; spec at
`github.com/modelcontextprotocol/ext-tasks`, overview at
`modelcontextprotocol.io/extensions/tasks/overview`). What Claude Code's
docs (`code.claude.com/docs/en/mcp`, *MCP tasks for server authors*,
read 2026-11-26) ask of a server:

- Only on a **2026-07-28** connection (3435), and only when the request's
  `_meta["io.modelcontextprotocol/clientCapabilities"].extensions` holds
  `io.modelcontextprotocol/tasks`: a task answered to a request without
  it is invalid. The server declares the extension in `server/discover`.
  **From 3435 (done):** a 2026-07-28 request is answered by
  `of_stateless` in `crates/cli/src/mcp/server.rs`, `server/discover`'s
  capabilities are `discover()`'s, and `of_2026(message, result_type)`
  sets `resultType` (`"task"` fits it) and `serverInfo` (ADR-0156).
- The `tools/call` result is `resultType: "task"` with `taskId` (up to
  128 printable ASCII characters, no spaces), `status`, `createdAt`,
  `lastUpdatedAt`, `ttlMs` (may be `null`) and, optionally,
  `pollIntervalMs` (clamped to 250–30,000 ms; 2,000 by default). The
  task exists before the answer is sent.
- **`tasks/get`** (`{ taskId }`) answers its status: `working`,
  `input_required` (with `inputRequests`, answered by **`tasks/update`**
  `{ taskId, inputResponses }`), `completed` (with `result`, the
  `tools/call` result it stands for), `failed` (with `error`) or
  `cancelled`. **`tasks/cancel`** is sent when the user stops it while
  connected. Claude Code uses no `tasks/result`, `tasks/list` or
  `notifications/tasks`.
- Keep a finished result at least an hour; poll every 10–30 s for jobs
  past a minute.

Read the extension's spec again when you start: the overview and the
2026-07-28 changelog disagree on small points (the changelog removed
`tasks/result` and `tasks/list`).

**Why we want it.** Some of our tools run for minutes, and today the
client waits on one `tools/call` the whole time:

- `start` launches the game's tool program with `cargo run … mcp-host`
  (`Cargo::launch`, `crates/cli/src/mcp/child.rs`), so its first call in
  a fresh worktree is a cold build of the game. 3414 found the
  server's own cold build outlasting the client's 300 s `MCP_TIMEOUT`
  (`.claude/settings.json`, `.codex/config.toml`) in a new worktree; a
  game's build is of the same order.
- `playtest`, `autobalance` (bots over many seeds), `bake` and `export`
  (a release build of a target) run as long, and `run` and `replay` can.
- A call forwarded to another workspace's server (ADR-0035 §2) waits on
  that server's build first.

A client that gives up on a call loses the work, and the agent has no
way to see progress or stop it.

**Why it isn't small.** The server is one synchronous loop over stdin
(ADR-0031 §1; `serve` reads on a thread and wakes every 200 ms to
`Server::poll`, ADR-0035 §4). To answer `tasks/get` while a build runs,
the work of a task must run off that loop, and the session tools must
not race it: `step` while `start` builds has no game yet.

## Nick input

None.

## Scope

**In:**
- **Which tools are tasks**: `start` (with no `link`), `playtest`,
  `autobalance`, `bake`, `export`, and any forwarded call, on a
  2026-07-28 request that declared the extension. Every other tool, and
  any tool without the extension, answers as today. A list in one place
  (`mcp::tasked(op) -> bool`), with its reason in a comment.
- **The task's run**: the work runs on its own thread; the loop answers
  `tasks/get` from the task's state. A task id is the server's own
  (`shuyi-<n>`, counting from 1 per process). `pollIntervalMs` 2,000
  for `start` and 10,000 for the rest; `ttlMs` one hour after it ends.
- **The session while `start` runs**: a session tool called while a
  `start` task is `working` answers today's "no session" failure with a
  `why` that names the task; a second `start` cancels the first and
  starts again (decide in the ADR, with the reason).
- **`tasks/cancel`**: stops the work: the child's `cargo` process is
  killed (the `Process` the forwarding and the host already hold), and
  the task is `cancelled`. A cancel of a finished task is an empty
  result; of an unknown one `-32602`, as the spec says.
- **A failed tool** is `completed` with the error result `tools/call`
  would have answered, its `code` (`SH0001`) kept, so an agent still
  branches on it: the spec keeps `failed` for a JSON-RPC error (an
  internal one, `-32603`), which carries it as `error`.
- **An ADR** (the `write-adr` skill): tasks in the server, which tools,
  the thread per task, the session's rule while `start` runs. It amends
  ADR-0031 §1 (the loop is no longer the only thread that works).
- **Docs**: `docs/contract.md`'s *The MCP server* (which tools may answer
  a task, and that a client without tasks waits as before); the book's
  *Tools for agents* (one sentence).
- **Estimated size:** about 1,100 lines (code 500, tests 450, docs 150).
- **Split:** past 900 lines, split off the forwarded calls as tasks into
  a new ticket: a forwarded call then answers as today.

**Out (do not do):**
- The 2026-07-28 protocol, `server/discover` and `_meta`: 3435.
- `input_required` inside a task (`tasks/update`): no tasked tool asks
  the person anything. `new`'s questions (3437) are asked on the call
  itself, not in a task.
- `notifications/tasks`, `subscriptions/listen` for tasks,
  `tasks/list` and `tasks/result`.
- Keeping tasks across a server restart: a task lives in its process.
- The server's own build before it can answer at all (first contact):
  3414's.
- A task on the old protocol.

## Implementation steps

1. Read the tasks extension's spec and Claude Code's *MCP tasks for
   server authors*; `crates/cli/src/mcp/server.rs` (`serve`, `handle`,
   `poll`, `tool_call`, `call`, `start`), `mcp/child.rs`,
   `mcp/forward.rs`; ADR-0031 and ADR-0035.
2. The task table in `server.rs` (or a new `mcp/tasks.rs`): id, status,
   timestamps, result, the handle to stop it.
3. `tool_call` answers a task for a tasked tool when the request allows
   it; `tasks/get` and `tasks/cancel` arms in `handle`.
4. The session's rule while `start` runs.
5. Tests first (below), then the ADR and the docs.
6. **Check by hand**: in Claude Code on 2026-07-28, `start` on a game
   in a fresh worktree answers a task, Claude Code polls it, and the
   session plays once it completes; a stop in the client cancels the
   build. Write what you saw in the completion notes.

## Acceptance criteria

- [x] `server/discover` declares `io.modelcontextprotocol/tasks`.
- [x] `playtest` on a 2026-07-28 request with the extension answers
      `resultType: "task"` with every field Claude Code requires, then
      `tasks/get` answers `working`, then `completed` with the same
      `result` the call answers without tasks.
- [x] The same call without the extension, or on 2025-11-25, answers
      the result directly, as today.
- [x] A tool not in the list (`scene`, `docs`) never answers a task.
- [x] `tasks/cancel` of a working `start` kills its build and makes it
      `cancelled`; `step` then answers "no session".
- [x] A failing `start` (a game that doesn't build) is `completed` with
      the error result and the `SH` code `start` answers today.
- [x] `tasks/get` of an unknown id is `-32602`.
- [x] ~~Checked by hand in Claude Code (step 6).~~ (not done: the session's server predates the change and reconnecting it is the user's; checked over stdio with the same messages, see the notes)
- [x] All gates in the `run-gates` skill pass.

## Tests required

- `server/tests.rs`: each acceptance case through `handle` and `poll`,
  with a fake launcher whose build blocks until the test releases it
  (so `working` is seen, then `completed`), and one that the cancel
  must stop.

## Completion notes

**What was done.** On a 2026-07-28 request whose client capabilities
declare `io.modelcontextprotocol/tasks`, `start` (without `link`),
`playtest`, `autobalance`, `bake` and `export` answer a task at once
(`mcp::tasked`, with its reasons). `server/discover` declares the
extension. `tasks/get` and `tasks/cancel` are new arms of
`of_stateless` (`Server::of_task`). The task table is
`crates/cli/src/mcp/tasks.rs` (ids `shuyi-<n>`, times as ISO 8601 UTC,
`pollIntervalMs` 2,000 for `start` and 10,000 for the rest, `ttlMs` null
while it works, then its run plus an hour, and forgotten an hour after it
ends). A session tool while `start`'s task works answers `SH0002` naming
the task, and a second `start` cancels the first (ADR-0159 §4). A cancel
drops the host: `cargo` is killed. ADR-0159 records it and amends
ADR-0031 §1; `docs/contract.md` (*The MCP server*, **Tasks**) and the
book's *Tools for agents* say it.

**Deviations.**

- **No thread per task** (the ticket's *The task's run*). `Launch` and
  `Link` aren't `Send` (`InProcess` borrows a game's `GameTools`), so a
  thread running the call would need `Send + 'static` bounds, a break for
  games' tests of the server. Every tasked tool's long work is a wait on
  a host process already running, so `Link` gained `send` and `reply`
  (defaults that keep today's behaviour), `child::Program` reads its
  stdout on a thread of its own, and the loop polls (`Server::advance`,
  from `tasks/get`, `tasks/cancel`, the session tools and each `poll`).
  ADR-0159 §2, and *Alternatives considered*.
- **A failing tool is `completed`, not `failed`.** The specification
  (2026-07-28, and its `schema.json`) forbids `failed` for a tool's error
  result: a game that doesn't build is `completed` with the error result
  `start` answers today, `SH3005` kept. `failed` carries a JSON-RPC
  error. The ticket's Scope and acceptance line were fixed before the
  code.
- **A cancel of an unknown task is `-32602`**, as the specification says
  (the ticket said an empty result); of an ended task, an empty result.
  `tasks/get` and `tasks/cancel` from a client without the extension are
  `-32021`, as the specification says. Fixed in the Scope before the
  code.
- **Forwarded calls split off** at the size valve, into 3440: a call
  another workspace's server answers answers as today (the PR was at
  1,263 counted lines before its docs). The engine-moved restart still
  waits on the fresh server, inside the task's completion: 3440 too.

**Checked by hand**, at `503abbb5` (the tip of `main`; the branch has no
merge of it since): the branch's `shuyi mcp`, driven over stdio by a
script as a 2026-07-28 client with the extension, on
`examples/hello-grid` with its sources touched so its tool program
rebuilt:

- `server/discover`'s capabilities held the extension;
- `start` answered `resultType: "task"`, `shuyi-1`, `working`, in 0.09 s;
  three seconds later the server had two `cargo` and seven `rustc`
  processes under it; `step` answered `SH0002` naming `shuyi-1`;
- `tasks/cancel` answered empty, and a second later no process was left
  under the server (cargo's job object took the `rustc`s); `tasks/get`
  answered `cancelled`, `step` the plain "no session";
- a second `start` (`shuyi-2`) was polled every 2 s, `working` for
  37 s, then `completed` with `start`'s ok result; `step` then played 5
  frames;
- `tasks/get` without the extension was `-32021`, of `shuyi-99`
  `-32602`.

**Not checked: in Claude Code** (acceptance line left unticked). The
session's own `shuyi` server was built before this change, and
reconnecting a project's MCP server is the user's (`/mcp`); a headless
`claude -p` couldn't sign in. The stdio run sends what Claude Code's
*MCP tasks for server authors* says it sends (`tasks/get` at
`pollIntervalMs`, `tasks/cancel` on a stop).

**Size:** 1,482 counted lines (code 678, tests 614, docs 190), under
1,500; the forwarded calls (3440) were where it split.

Behaviour: shuyi-cli: on 2026-07-28, `server/discover`'s capabilities
hold `extensions: { "io.modelcontextprotocol/tasks": {} }`, and a client
that declares the extension gets a task (`resultType: "task"`) from
`start`, `playtest`, `autobalance`, `bake` and `export`, then answers
`tasks/get` and `tasks/cancel`.

Behaviour: shuyi-cli: a session tool called while `start`'s task builds
answers `SH0002` with an `error` that names the task; a `start` while
another's task builds cancels it and adds a limit saying so.

No break: `Link` gains two methods with defaults, `mcp::tasked` is new;
`gh search code --repo Zafnok/visions-of-shuyi` for `impl Link for`,
`server::Server`, `InProcess`, `server/discover` and `no game is running`
found nothing (the game pins `96eb097c`).

Game: nothing to adopt, the game implements none of the server's traits
and its agents get tasks from the engine's server as it moves its pin.
