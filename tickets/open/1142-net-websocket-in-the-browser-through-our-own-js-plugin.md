---
id: "1142"
title: "Networking: WebSocket in the web build through our own miniquad JS plugin"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2522", "2617"]
nick_input: none
completed:
---

# 1142 — WebSocket in the browser through our own JS plugin

> **Unparked** by Nick's decision of 2026-11-25 (`docs/decisions.md`): the next example game, trails, is played online, on the web as well as on desktop, so its browser client needs this. It was parked on 2026-10-14 with the rest of networking.

## Context

ADR-0006 §6, as amended by ADR-0155 (2613): real time over WebSocket in
the browser is built now, not left to 1147. macroquad's web loader doesn't use
`wasm-bindgen` (game ADR-0034), so crates like `ewebsock` and matchbox
can't run in our web build. The pattern that works is the one the
controller already uses: `crates/cli/web/gamepad.js`, a small miniquad
plugin. The plugin API is described at https://macroquad.rs/articles/wasm/.

- **2522** (the live link for the web build) writes
  `crates/cli/web/net.js`, a plugin named `shuyi_net`, and a safe
  `WebSocket` in `crates/app/src/web_socket.rs`: open, state, send,
  poll and close, text messages, the page's `location.hash`, and a cap
  of 64 MiB on queued bytes. This ticket adds binary messages, close and
  error events, and the page's query string to that plugin and that
  type. It writes no second plugin. 2522 is in `blocked_by`, so its
  clause for this ticket landing first never applies.
- **2617** writes the native client: `shuyi_app::net::WsClient` (behind
  `shuyi-app`'s `net` feature, with its own `WsEvent`s and no
  `shuyi-net` dependency, since a default crate may not depend on a
  module), adapted to `shuyi_net::Transport` by the front door's
  `shuyi::net::connect`. On wasm32 it leaves a stub that returns
  `ConnectError::Unsupported`. This ticket replaces that stub with the
  browser's WebSocket, behind the same API, so `shuyi::net::connect`
  works on the web and a game's code is the same on both.
- **2790** (the trails online client) reads the server's address from
  the page's `?server=` parameter first. This ticket gives it the
  reader.
- The browser has no UDP, so the web client talks only to a WebSocket
  server: 2616's `shuyi-server` (trails' rooms), or later 1141's relay.

## Nick input

None.

## Scope

**In:**
- **Binary messages in 2522's `net.js` and `WebSocket`**, beside its
  text messages: the socket's `binaryType` is `arraybuffer`;
  `shuyi_net_send_bytes(id, ptr, len)`; each received message keeps
  whether it was text or binary (`shuyi_net_next_kind(id)`), and the
  queue's byte cap counts both. Close and error events: the close code
  and reason are kept for `shuyi_net_close_reason`, and an error before
  open reads as a refused connect. Raise `net.js`'s `VERSION` and the
  Rust side's `PLUGIN_VERSION` together.
- **The page's query string:** `shuyi_net_query_len()` and
  `shuyi_net_query(ptr, len)` hand Rust `location.search` as UTF-8, as
  2522 hands over the hash.
- **`server_param(query: &str) -> Option<String>`**, a pure function
  compiled on every target (so native tests cover it), in `shuyi-app`:
  the `server` parameter, percent-decoded, accepted only as a `ws://` or
  `wss://` URL of at most 256 bytes; anything else is ignored and logged
  once. The front door exposes it as `shuyi::net::page_server()`, which
  is `None` on native.
- **The wasm32 `WsClient`**, in `crates/app/src/net/` behind the `net`
  feature, over the safe `WebSocket`: the same `connect`, `send`,
  `poll`, `close` and `stats` as 2617's native one, the same `WsEvent`s,
  polled once a frame. `connect` returns at once and `Open` or `Closed`
  arrives by `poll`; a connect that hasn't opened by its timeout closes
  with a timeout. It replaces 2617's stub.
- The `unsafe` boundary stays in `web_socket.rs` (2522's file with
  `#[allow(unsafe_code)]`); nothing else here is unsafe.
- **Rule 13:** the wasm32 client sits behind the same `net` feature,
  off by default. Measure the web build of `examples/snake`'s `.wasm`
  without `net` and with `shuyi/net` plus a throwaway call to
  `shuyi::net::connect` (not committed), and write both sizes in the
  completion notes and the feature's doc comment beside 2617's native
  numbers. `net.js` itself ships in every web build (2522), so also say
  how many bytes this ticket adds to it.
- A license header in `net.js` and a line in `THIRD_PARTY_ASSETS.md` if
  what this ticket adds borrows from any upstream file. It doesn't need
  one if it is all ours.
- **A dev echo server** to try it by hand:
  `crates/app/examples/ws-echo.rs` (needs `net`), the sync tungstenite
  echo server of 2617's tests as a small program on `127.0.0.1:<port>`.
- **Either order with 1145:** if 1145 is done, `net.js` already hands
  Rust the page's query for its `?join=` reader: read `server` from that
  same text and add no second call. If not, write the query hand-off
  here, and 1145 reads its `join` parameter from it.
- No `wasm-bindgen`, `js-sys` or `web-sys`, at any depth (`cargo tree
  --target wasm32-unknown-unknown`).
- The completion notes get a `Book:` line (2854) and a `Skills:` line
  (2855).

**Out (do not do):**
- WebRTC and WebTransport (1147);
- the plugin's text messages, its place in the web shell, and the live
  link over it (2522);
- the `?join=` invite reader, `--join` on native, the launch join token
  in `Setup`, and the lobby joining it (1145);
- trails' online screen and its address fallbacks (`server.ron` next to
  the page, the dev default): 2790;
- the native client (2617).

**Estimated size:** about 700 lines (code 350, tests 220, docs 130).

## Implementation steps

1. Read 2522's `net.js`, `web_socket.rs` and their `shuyi-cli` tests,
   2617's `crates/app/src/net.rs` and the front door's `net` module, and
   check whether 1145 is done.
2. Add binary messages, close and error events, and the query hand-off
   to `net.js` and `web_socket.rs`; raise the plugin version.
3. Write `server_param` and `shuyi::net::page_server`, with tests.
4. Write the wasm32 `WsClient`, replacing 2617's stub.
5. Write `ws-echo`, make a web build that connects (snake with the
   throwaway call), and open two tabs in the browser pane against it.
6. Measure the sizes; write the docs (the section on `net.js` in
   `crates/cli/web/README.md`) and the `Book:` and `Skills:` lines.

## Acceptance criteria

- [ ] In the browser pane, two tabs of a web build open
      `ws://127.0.0.1:<port>` from `?server=` against `ws-echo`; each
      sends 100 binary messages and gets them back in order. A
      screenshot of both tabs goes in the PR.
- [ ] Stopping `ws-echo` gives each tab a `Closed` event (a
      `Disconnected` through `shuyi::net::connect`).
- [ ] A connect to a closed port gives `Closed` within the timeout.
- [ ] `server_param` accepts `?server=wss%3A%2F%2F1.2.3.4%2Fplay` and
      ignores `?server=http://x`, an empty value, and one over 256
      bytes.
- [ ] Text and binary messages share the queue's byte cap.
- [ ] The web build has no `wasm-bindgen` anywhere in its tree.
- [ ] The sizes with and without `net` are in the completion notes.
- [ ] The completion notes have a `Book:` line naming 2854 and a
      `Skills:` line naming 2855.
- [ ] The WASM build and all gates in the `run-gates` skill pass.

## Tests required

- Manual: browser pane, two tabs against `ws-echo`.
- Unit: binary and text messages through the Rust side's queue; close
  reasons; the plugin version check.
- Unit: `server_param` with the parameter, without it, encoded,
  malformed and too long (ignored, logged).

## Completion notes
