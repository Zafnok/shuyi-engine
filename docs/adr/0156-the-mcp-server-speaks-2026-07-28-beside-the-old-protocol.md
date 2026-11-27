# ADR-0156: The MCP server speaks 2026-07-28 beside the old protocol: a version per request, the folder by `input_required`, and list changes by subscription

- **Status:** Accepted; extended by ADR-0159 (tasks)
- **Date:** 2026-11-26
- **Related tickets:** 3435 (this); 3434 (skills), 3436 (tasks) and 3437
  (elicitation), which need it
- **Amends:** ADR-0035 §1 (the session's folder) and §4 (the tool list
  follows)
- **Extends:** ADR-0031 (the MCP server)

## Context

MCP revision 2026-07-28 is what Claude and Claude Code need for skills
served by a server, tasks and elicitation. Claude Code v2 asks a server
over stdio whether it speaks 2026-07-28 (it sends `server/discover`
first) and uses it if so; any other server connects on the old protocol.
Codex and older Claude Code speak only the old one. The server spoke the
old protocol only (`PROTOCOL_VERSIONS`, 2024-11-05 to 2025-11-25).

What 2026-07-28 changes, read on 2026-11-26 at `modelcontextprotocol.io`
(`specification/2026-07-28/`: `changelog`, `basic/versioning`,
`server/discover`, `basic/patterns/mrtr`, `basic/patterns/subscriptions`,
`basic/patterns/cancellation`, `client/roots`,
`server/utilities/caching`, and `schema.ts` in the specification's
repository):

- **No handshake.** Every request carries `params._meta` with
  `io.modelcontextprotocol/protocolVersion` and
  `io.modelcontextprotocol/clientCapabilities` (and SHOULD carry
  `io.modelcontextprotocol/clientInfo`). An unknown version is
  `-32022`, `data: { supported, requested }`. A server MUST NOT infer a
  client's capabilities from earlier requests. Results SHOULD carry
  `_meta["io.modelcontextprotocol/serverInfo"]`.
- **`server/discover`** (MUST): `resultType`, `supportedVersions`,
  `capabilities` (`tools: { listChanged }`, and `extensions`),
  `instructions`, `ttlMs`, `cacheScope`.
- **`resultType`** on every result, `"complete"` or `"input_required"`.
  `ttlMs` (milliseconds, 0 is stale at once) and `cacheScope`
  (`"public"`, `"private"`) are required on a complete `server/discover`,
  `tools/list` and the other lists (SEP-2549); an `input_required` result
  carries none.
- **No server-initiated requests** (SEP-2322, multi-round-trip): a
  `tools/call` (or `resources/read`, `prompts/get`, and nothing else) is
  answered `resultType: "input_required"` with `inputRequests`, a map of
  the server's keys to `{ method, params? }` (`roots/list`,
  `sampling/createMessage`, `elicitation/create`), and optionally an
  opaque `requestState`. The client retries the request under a new id
  with `inputResponses`, the same keys to each request's result (for
  `roots/list`, `{ roots: [{ uri, name? }] }`). Roots are deprecated
  (SEP-2577), not removed. `notifications/roots/list_changed`, `ping`
  and the `initialize` handshake are gone.
- **List changes** come on `subscriptions/listen`, a request with no
  response while it lasts: its `params.notifications` asks for
  `toolsListChanged` (and others), the server first sends
  `notifications/subscriptions/acknowledged` with what it agreed to, and
  each notification carries `_meta["io.modelcontextprotocol/subscriptionId"]`,
  the listen request's id. The client ends it with
  `notifications/cancelled` naming that id.
- **No protocol sessions**: state across calls is handles in tool
  arguments (SEP-2567).

The session tools (`start` … `stop`, ADR-0031) keep one headless game
per server process. ADR-0035 §1 asks the client's roots by the server's
own `roots/list` after `notifications/initialized`, and §4 tells the
client its tools changed by `notifications/tools/list_changed`: neither
exists on 2026-07-28.

## Decision

1. **Both generations, chosen per request.** `Server::handle` reads each
   request's `params._meta["io.modelcontextprotocol/protocolVersion"]`.
   - None, or the method `initialize` (whatever its `_meta`): the old
     protocol, exactly as before (`of_old`). `PROTOCOL_VERSIONS` stays
     the versions `initialize` negotiates; 2026-07-28 isn't one of them,
     because `initialize` never reaches it.
   - `"2026-07-28"` (`STATELESS`): `of_stateless` answers
     `server/discover`, `tools/list`, `tools/call` and
     `subscriptions/listen`; any other method (`ping` included) is
     `-32601`.
   - Anything else (a known old version, a number): `-32022`, `data:
     { supported: ["2026-07-28"], requested }`.

   A missing `clientCapabilities` is read as `{}`, not refused.
2. **Every 2026-07-28 result** gets `resultType` and `_meta.serverInfo`
   (`of_2026`); an error stays a JSON-RPC error, as on the old protocol.
   `server/discover` answers `supportedVersions: ["2026-07-28"]`,
   `capabilities: { tools: { listChanged: true } }`, `INSTRUCTIONS`,
   `ttlMs: 3600000` and `cacheScope: "public"`: nothing in it changes
   while the process runs. `tools/list` answers the old list, in the
   same order, with `ttlMs: 0` and `cacheScope: "private"` when the
   server forwards (its list follows the session's folder, so it may
   change and is this client's), and `3600000`, `"public"` when it
   doesn't (a forwarded server, whose list never changes).
3. **The session's folder by `input_required`** (amends ADR-0035 §1).
   The first `tools/call` of a client whose `clientCapabilities` hold
   `roots` is answered `resultType: "input_required"`, `inputRequests:
   { "shuyi-roots": { "method": "roots/list" } }` (the key is `ROOTS`,
   the old request's id), with no `requestState`: the retry carries the
   tool's name and arguments itself, and nothing in it is trusted. The
   retry's `inputResponses["shuyi-roots"]` is taken as the old answer
   was (`rooted`: the first root is the folder, and its workspace's
   server starts), then the call is answered. **It is asked once per
   process.** A retry without the roots is answered from the server's
   folder, and so is every later call: there is no `list_changed` to say
   the roots moved, and a client that declares `roots` but never
   answers would otherwise loop. A later request that carries
   `inputResponses["shuyi-roots"]` is still taken. `tools/list` never
   asks (the specification allows `input_required` only on
   `tools/call`, `resources/read` and `prompts/get`). A client without
   `roots` is answered at once, from the server's folder.
4. **List changes by subscription** (amends ADR-0035 §4). A
   `subscriptions/listen` gets its acknowledgment at once, agreeing to
   `toolsListChanged` alone (or nothing), and no response. Each one that
   asked for `toolsListChanged` is kept by its id until a
   `notifications/cancelled` names it. When the tools change (`poll`,
   or a folder taken back in the server's own workspace), the server
   sends `notifications/tools/list_changed` with
   `_meta.subscriptionId` on each, and the old notification only if a
   client said `initialize`. The server never ends a subscription
   itself.
5. **Forwarding stays on the old protocol** (ADR-0035 §2). A forwarded
   server is started and asked as before, with `initialize` on
   `PROTOCOL_VERSIONS[0]`, whatever the client speaks; its answer to a
   2026-07-28 call gets `resultType` and `serverInfo` as the server's
   own do. A forwarded server that is an older `shuyi`, with no
   2026-07-28, works unchanged. No request's `_meta`, `inputResponses`
   or `requestState` is passed on.
6. **The session stays the process's.** On stdio one client has a
   process, so the game `start` runs is the process's, on both
   protocols, and no tool takes a session handle (SEP-2567 is for
   servers many clients share, as over HTTP, which this one isn't:
   ADR-0031).

## Consequences

- Claude Code v2 connects on 2026-07-28, which 3434 (skills), 3436
  (tasks) and 3437 (elicitation) build on; Codex and older clients are
  served as before, and the old tests pass unchanged.
- On 2026-07-28 the folder's server starts at the first call, not at
  connection: the first call in a worktree waits for that worktree's
  build, where the old protocol's started it while the agent read.
- A 2026-07-28 client whose session folder moves mid-process keeps the
  first one (the old protocol reads them again on `list_changed`).
- A 2026-07-28 client that lists tools before its first call sees the
  server's own list, then hears of the folder's on its subscription.
- `Server::handle` and `Server::poll` keep their signatures; the
  messages are built as JSON and written as text at the edge.
- `notifications/subscriptions/acknowledged` is always sent, also for
  a subscription that asked for nothing the server sends.

## Alternatives considered

- **Add `"2026-07-28"` to `PROTOCOL_VERSIONS`.** Lost: `initialize`
  answers the first of them to a client asking for one it doesn't know,
  and the forwarded servers are started on it, so it would answer, and
  ask, a handshake 2026-07-28 doesn't have. A constant of its own
  (`STATELESS`) keeps both lists true.
- **Ask for the roots on every call until they come.** Lost: a client
  that declares `roots` and never answers would never be answered; the
  specification's "ask again while input is missing" assumes a client
  that answers.
- **Ask for the roots on `tools/list`, so the folder's build starts
  sooner.** Lost: `input_required` isn't allowed there.
- **A `requestState` holding the call.** Lost: the retry carries the
  call itself, and a state the server would have to sign buys nothing.
- **Speak 2026-07-28 to the forwarded servers when the client does.**
  Lost: an older `shuyi` there doesn't speak it, the parent would need
  both clients, and mapping the old answer is a few lines.
- **A session handle in the session tools' arguments.** Lost: stdio has
  one client per process, and every session tool would change shape for
  no client that needs it.
