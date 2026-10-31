---
id: "1142"
title: "Parked: Networking, step 3: WebSocket in the web build through our own miniquad JS plugin"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["1141"]
nick_input: none
completed:
---

# 1142 — WebSocket in the browser through our own JS plugin

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §6. macroquad's web loader doesn't use `wasm-bindgen`
(game ADR-0034), so crates like `ewebsock` and matchbox can't run in our web
build. The pattern that works is the one the controller already uses:
`crates/cli/web/gamepad.js`, a small miniquad plugin. The plugin API is
described at https://macroquad.rs/articles/wasm/.

2522 (the live link for the web build) writes `crates/cli/web/net.js`
and a safe `WebSocket` in `crates/app/src/web_socket.rs`: open, state,
send, poll and close, text messages, and a cap on queued bytes. This
ticket adds binary messages to that plugin and that type, then writes
`WsTransport` over it. It writes no second plugin. If 2522 isn't done
when this is unparked, write the plugin as 2522 describes, and 2522
builds on it.

ADR-0006's Context names play-by-link as a maybe. Only our own plugin
can read the page's address, so the web half of a join link is here
too (3055 notes it).

## Nick input

None.

## Scope

**In:**
- Binary messages in 2522's `net.js` and `WebSocket`, beside its text
  messages (open, send, close, poll and the byte cap exist already), and
  its error events.
- A license header in `net.js` and a line in `THIRD_PARTY_ASSETS.md` if
  what this ticket adds borrows from any upstream file. It doesn't need
  one if it is all ours.
- The `wasm32` side of `WsTransport` uses 2522's safe `WebSocket`, whose
  file holds the `unsafe` boundary in `shuyi-app`.
- **A join link:** at start the plugin hands Rust the page's query
  string (`location.search`), as it hands over the hash for 2522. Rust
  reads its `join` parameter (`?join=<token>`) with a pure function
  compiled on every target, so native tests cover it, and gives it to
  the platform as the launch join token (1145's). If 1145 isn't done,
  it is the relay room code that `RelaySession` joins by (1141). A
  malformed one is ignored and logged. So a link to the web build with
  a room code opens the game straight into that room.
- A test page in the browser pane: two tabs join the same relay room,
  running locally, and play the toy lockstep game.

**Out (do not do):**
- WebRTC and WebTransport (1147);
- the plugin's text messages, its place in the web shell, and the live
  link over it (2522);
- `--join` on native, the launch join token in `Setup`, and the lobby
  joining it (1145).

## Implementation steps

1. Read 2522's `net.js`, `web_socket.rs` and their `shuyi-cli` tests,
   and check whether 1145 is done. If 2522 isn't done, write the plugin
   as it describes first.
2. Add binary messages, error events and the join reader, then the
   `wasm32` side of `WsTransport`.
3. Run the local relay (1141), open two tabs in the browser pane, and
   play. Open the second tab at `?join=<code>`.
4. Write the docs.

## Acceptance criteria

- [ ] Two browser tabs and a native client play one match through a
      local relay. Screenshots of both tabs go in the PR.
- [ ] Opening the build at `?join=<code>` in a second tab joins the
      first tab's room with no typing.
- [ ] The WASM build and all gates pass.

## Tests required

- Manual: browser pane, two tabs plus native.
- Unit: binary messages through the Rust side's queue, beside text
  ones.
- Unit: the token read from an address with the parameter, without it,
  and malformed (ignored, logged).

## Completion notes
