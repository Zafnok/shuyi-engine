---
id: "2560"
title: "MCP's session attached to a running build over the live link: start with a link, then step, frame_png, scene, events, reset and stop"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2521"]
nick_input: none
completed:
---

# 2560 — MCP's session over the live link

## Context

2521 built the live link
([ADR-0047](../../docs/adr/0047-the-live-link-json-lines-over-loopback-tcp-to-a-running-debug-build-read-only-but-a-players-input.md)):
JSON lines over TCP on loopback between `shuyi` and a running debug
build, behind the backend's `link` feature and `SHUYI_LINK`, with a
token. It holds:

- `shuyi_platform::link`: `Request` (`Hello`, `Status`, `Frame`,
  `Screenshot`, `Play { steps }`, `Log { since }`), `Answer`, `Step`
  (`Keys`, `Pad`, `Act`, `Wait` in frames), `press_for`, `Server`, the
  base64 and `VERSION`;
- the listener in `shuyi-app` (`crates/app/src/link.rs`);
- `shuyi attach`, the contract's operation over it
  (`crates/cli/src/contract/attach.rs`): its private `Connection` (open,
  `ask`, `answer`, a `Drop` that shuts the socket down), its `hello`,
  its `screenshot`, its codes (`SH1005` no build, `SH0004` the token,
  `SH1006` the link version, `SH3006` busy or lost), and its test host
  (`attach/tests.rs`: the server on the toy game in a thread).

2521's scope also had MCP's session reach the link. Its PR passed the
ticket's size rule (about 600 changed lines), so that part is this
ticket, as 2521's size rule says. 1133's MCP server keeps one game
running headless between calls (`crates/cli/src/mcp/server.rs`: `start`
builds the game's tool program and talks to it as `shuyi mcp-host`;
the session tools are `Op::SESSION`).

## Nick input

None.

## Scope

**In:**
- **`start` takes `link`** (an address and a token, `{ "address":
  "127.0.0.1:7357", "token": "…" }`): the session attaches to that
  running build instead of building and starting the game headless.
  `project` is not needed then. It says hello as `attach` does, and
  answers as `start` does, its options empty, with the build's game. A
  game already running in the session is stopped first. The errors are
  `attach`'s, with their codes.
- **The session tools on an attached session:**
  - `step` plays its script over the link (the script's keys, pad
    buttons and waits; waits in the build's frames at 60 a second, with
    `attach`'s limit), then its idle frames as a wait, and answers as
    `step` does from `Status`, `Frame` and `Log`: `scene` none with the
    limit "no scene over the link", `events` and `sounds` empty with the
    limit "no events over the link";
  - `frame_png` answers the window's picture (`Request::Screenshot`), as
    an image, its size the window's; its `scale` is refused ("the
    window's picture has the window's size"). **Either order with
    2514:** if 2514 is done, its `every` and `count` are refused over a
    link ("one picture at a time over the link"); if not, 2514 refuses
    them (its own clause says so);
  - `scene` answers the frame's text in place of a scene, with the limit
    "no scene over the link";
  - `events` answers no events, with the limit "no events over the
    link";
  - `reset` is refused ("the link never changes a running build"), with
    `SH0001`;
  - `stop` detaches and leaves the build running.
- **Reuse, not a copy:** `attach`'s `Connection`, `hello` and
  `screenshot` become `pub(crate)` (or move to a module both use), so
  `attach` and the session speak the link one way.
- **2555's MCP part:** a session attached over the link answers 2553's
  `perf` tool through 2555's `Request::Perf`, with the same input and
  answer as a headless session. **Either order with 2555:** if 2555 is
  done, add it here; if not, 2555 adds it once this ticket is done (its
  own clause says so).
- **Schemas:** `session.start.input.json` regenerated, the contract's
  version raised by one with its line in `versions.json`.
- **Docs:** `docs/contract.md`'s *The MCP server* (the `link` argument
  and what each tool does over it) and *Attaching to a running build*
  (one line pointing at the session); the `shuyi-test` skill's section
  on a running build (2521's) gains the MCP way, its `metadata.version`
  raised.

**Out (do not do):**
- the web build and a device on the LAN (2522);
- the game's own scene, events and sounds over the link (ADR-0047's
  consequences: a ticket when an agent needs them);
- changing the build in any way but a player's input.

## Implementation steps

1. Read ADR-0047, `shuyi_platform::link`, `contract/attach.rs` and its
   tests, `mcp/server.rs`'s `start` and `in_session`, and 1133's
   scripted client test (`crates/cli/tests/it/mcp.rs`).
2. Share `attach`'s connection code; add an attached session to the
   server beside the hosted one.
3. Answer each session tool over the link as the scope says.
4. Add `link` to `StartInput`, regenerate the schemas, raise the version.
5. Give 1133's scripted client test a step that starts a session with
   `link` on a test host (the server on the toy game in a thread, as
   `attach`'s tests have it), steps it and reads its frame and picture.
6. Write the docs and the skill's lines; run the gates.

## Acceptance criteria

- [ ] `start` with `link` on a test host answers the build's game, and
      `step` with keys moves its screens as `attach` does.
- [ ] `frame_png` answers the host's picture; `scene` and `events` answer
      with their limits; `reset` is refused; `stop` leaves the host
      running and a second `start` with `link` attaches again.
- [ ] A wrong token, no build and another link version answer `attach`'s
      codes.
- [ ] 1133's scripted client test attaches to a test host, steps it and
      reads its frame.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the session's answers over the link from a test host, each tool;
  the refusals and their codes.
- Integration: 1133's scripted client test's step over the link.

## Completion notes

*(Filled in by the session that completes the ticket.)*
