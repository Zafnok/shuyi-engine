---
id: "2617"
title: "A WebSocket client transport for native games, behind a net feature that is off by default"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2613"]
nick_input: none
completed:
---

# 2617 — A native WebSocket client, behind a net feature

## Context

- Nick's decision of 2026-11-25 (`docs/decisions.md`): trails is played
  online. Its desktop and Steam Deck clients talk to `shuyi-server`
  (2616) over `ws://` in development and `wss://` in production.
- Only platform backends touch sockets (`AGENTS.md` rule 5), so the
  socket lives in the backend `shuyi-app`.
- **A default crate may not depend on a module** (ADR-0009 §4;
  `cargo xtask check-modules` fails it, reading `cargo metadata
  --all-features`, so an optional dependency counts too). `shuyi-app` is
  a default crate and `shuyi-net` a module. So `shuyi-app` gets a
  WebSocket client with a byte-level API of its own, and the front door
  `shuyi` (which may depend on both) adapts it to `shuyi_net::Transport`
  behind its `net` feature. ADR-0152 (2613) records this split.
- 1141 (parked) had planned a native `WsTransport` over tokio. This
  ticket takes the native client, without tokio in a game; 1141 is
  amended to use it. The browser half is 1142, unparked on 2026-11-25,
  on 2522's `net.js` plugin; it comes after this ticket and replaces the
  wasm32 stub written here.
- Rule 13 (decision of 2026-11-24): a dependency a game doesn't strictly
  need, that makes a shipped game bigger, is a toggle. tungstenite,
  rustls, ring and the native roots are such dependencies, so they sit
  behind a `net` feature, off by default, and this ticket measures what
  they add.
- What exists, checked on 2026-11-25: no tungstenite, rustls or
  `rustls-native-certs` in `Cargo.lock`; `deny.toml`'s `[graph]` checks
  default features only (`all-features = false`), so a feature that is
  off by default is invisible to `cargo deny check` unless named there;
  `crates/shuyi/src/lib.rs` re-exports `shuyi_app` under its `app`
  feature, outside `target_os = "wasi"`.
- `grep -rnw` over `tickets/open`: `WsClient` and `ConnectError` are
  named by no other ticket; `WsTransport` by 1141, 1142, 2522, 2550 and
  2605 (all reading it as the future native or web transport; 1141 and
  1142 are amended to this ticket's names).

## Nick input

None.

## Scope

**In:**
- **`shuyi_app::net::WsClient`**, native only, behind `shuyi-app`'s `net`
  feature, with no `shuyi-net` dependency:
  - `WsClient::connect(url: &str, options: WsOptions) -> Result<WsClient,
    ConnectError>` takes `ws://` and `wss://` URLs; `WsOptions`
    (`#[non_exhaustive]`, `WsOptions::new()`, ADR-0039) holds the
    connect timeout (default 5 s), the outbound cap and, for tests only,
    extra trusted roots;
  - `send(&mut self, bytes: &[u8]) -> Result<(), WsError>` (binary
    messages); `poll(&mut self) -> Vec<WsEvent>` with `WsEvent { Open,
    Binary(Vec<u8>), Closed(WsClose) }` (`#[non_exhaustive]`);
    `close(&mut self)`; `stats(&self)` (bytes and messages both ways);
  - it runs `tungstenite` (`default-features = false`, feature
    `handshake`) in sync mode on one background thread, joined by
    `std::sync::mpsc` channels. No tokio in a game;
  - TLS is our own `rustls::ClientConnection` in a
    `rustls::StreamOwned` under tungstenite, so no TLS feature of
    tungstenite decides rustls's features: rustls with `default-features
    = false, features = ["ring", "std", "tls12"]`, and roots from
    `rustls-native-certs`. Never `webpki-roots` (CDLA-Permissive-2.0),
    never aws-lc-rs (it needs NASM on the GNU toolchain);
  - a close, an error or a timeout becomes `WsEvent::Closed` with the
    reason; the thread ends, and dropping the client closes the socket
    and joins the thread.
- **On wasm32**, `WsClient::connect` returns `ConnectError::Unsupported`.
  1142 replaces this stub with the browser's WebSocket behind the same
  API.
- **The front door**, `crates/shuyi`:
  - its `net` feature (2613: `["std", "dep:shuyi-net"]`) also turns on
    `shuyi-app?/net`;
  - **From 2613 (done):** `shuyi::net` is today `pub use shuyi_net as
    net;` in `crates/shuyi/src/lib.rs`. To add `connect` beside it, make
    `net` a module (`crates/shuyi/src/net.rs`) that re-exports
    `shuyi_net::*` and adds `connect`, so `shuyi::net::MemoryNetwork`
    and the rest keep their paths;
  - `shuyi::net::connect(url: &str) -> Result<Box<dyn
    shuyi_net::Transport>, ConnectError>` in `crates/shuyi/src/net.rs`:
    a small adapter, `WsTransport`, that implements `Transport` over
    `WsClient` with one peer, the server (`PeerId(0)`): `Open` becomes
    `Connected`, `Binary` a `Message` on `Reliable`, `Closed` a
    `Disconnected` with the matching `DisconnectReason`. `Unreliable`
    sends go on the same socket, documented as reliable on this backend.
  - A game calls it from a screen; which socket runs is the backend's
    choice, never a target `cfg!` in the game (ADR-0060).
- **The toggle** (rule 13): `net` is **off by default** on `shuyi-app`
  and on `shuyi`. Measure the stripped release size of `examples/snake`
  for `x86_64-pc-windows-gnu` and `x86_64-unknown-linux-gnu`, built
  without `net`, and built with `shuyi/net` and a throwaway local call
  to `shuyi::net::connect` so the linker keeps the client (not
  committed). Write both numbers and the difference in the completion
  notes and in the feature's doc comment in both `Cargo.toml`s. 2790
  measures trails itself.
- **`cargo deny` sees it:** set `deny.toml`'s `[graph]` so the `net`
  features of `shuyi-app` and `shuyi` are on when it checks (for example
  `features = ["shuyi-app/net", "shuyi/net"]`, or a second `cargo deny
  check --features …` step in the gate and in CI). Check with `cargo
  deny list` that tungstenite, rustls and ring appear, then that the
  check passes.
- **Either order with 2616:** if 2616 is done, `deny.toml`'s `[bans]
  deny` already lists `aws-lc-rs`, `aws-lc-sys`, `webpki-roots` and
  `webpki-root-certs`, and they cover this graph too: nothing to add. If
  not, add those four entries here.
- **Licenses:** tungstenite (MIT OR Apache-2.0), rustls (Apache-2.0 OR
  ISC OR MIT), ring (Apache-2.0 AND ISC), `rustls-native-certs`
  (Apache-2.0 OR ISC OR MIT) and what they pull in on each desktop OS
  (`schannel`, `security-framework`, `openssl-probe`), all on the allow
  list; checked by `cargo deny check licenses` after the `[graph]`
  change.
- The completion notes get a `Book:` line (2854) and a `Skills:` line
  (2855).

**Out (do not do):**
- The browser's WebSocket, binary messages in `net.js`, and the
  `?server=` reader: 1142, after this ticket, replaces the wasm32 stub.
- Relay sessions and room codes: 1141.
- A reconnect policy beyond one attempt: the game decides (2790).
- HTTP proxies, `permessage-deflate`.
- The Playdate: no network there (trails' Playdate build is offline,
  2788).

**Estimated size:** about 600 lines (code 300, tests 220, docs 80).

## Implementation steps

1. Add the dependencies to `crates/app/Cargo.toml` as optional, under
   `net`, with their default features off and the features above; run
   `cargo tree -e features -p shuyi-app --features net` and check that
   neither aws-lc-rs nor webpki-roots appears.
2. Write `crates/app/src/net.rs` (with a `net/` folder if it grows): the
   thread, its channels, the TLS stream, `WsClient`, `WsEvent`,
   `WsOptions`, `ConnectError` and the wasm32 stub.
3. Write the front door's `net` module and `WsTransport` adapter.
4. Change `deny.toml`'s `[graph]` and run `cargo deny check`.
5. Measure the sizes and write them in the doc comments and the notes.
6. Write the tests, then the `Book:` and `Skills:` lines.

## Acceptance criteria

- [ ] Against a test echo server (`tungstenite` on `127.0.0.1:0` in a
      thread, a dev-dependency), 100 binary messages round-trip in order
      through `WsClient`, and through `shuyi::net::connect` as `Message`
      events on `Reliable`.
- [ ] A close from the server gives `Closed` (`Disconnected` through the
      front door).
- [ ] A connect to a port that refuses, and to one that never answers,
      each gives an error within the connect timeout plus 1 s.
- [ ] `wss://` works against an `rcgen` certificate trusted through
      `WsOptions`' test roots, and is refused without them.
- [ ] A build without `net` links neither tungstenite nor rustls:
      `cargo tree -p snake -e normal` finds neither.
- [ ] `cargo xtask check-modules` passes: `shuyi-app` has no edge to
      `shuyi-net`.
- [ ] The size difference is in the completion notes and both feature
      doc comments.
- [ ] `cargo deny check` sees the `net` graph and passes.
- [ ] Tests pass on Windows GNU and Linux.
- [ ] The completion notes have a `Book:` line naming 2854 and a
      `Skills:` line naming 2855.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration (`crates/app/tests/`, with `net`): echo round trip,
  close, refused connect, timeout, TLS with and without the test root.
- Integration (`crates/shuyi/tests/`, with `net`): the adapter's event
  mapping over the echo server.
- Build check: `cargo tree` without `net`.

## Completion notes
