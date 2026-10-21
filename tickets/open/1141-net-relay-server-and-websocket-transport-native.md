---
id: "1141"
title: "Parked: Networking, step 2: a small relay server with room codes, and a native WebSocket transport"
type: feature
milestone: "Engine: networking"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1140"]
nick_input: none
completed:
---

# 1141 — Relay server and native WebSocket transport

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1, §3 and §5. A relay forwards messages between the players in
a room. Both players connect outward, so home routers are no problem, no
NAT traversal is needed, and the browser can join (1142).

It is the simplest internet play there is, and it is ours to host. A
$4–6 a month VPS runs it. Vendors have closed or shrunk in 2026, so we
don't depend on one.

## Nick input

None to build it. Running it publicly is a decision for when a game ships
online (0053). This ticket only gives a Docker image and instructions.

## Scope

**In:**
- `crates/relay` (binary `shuyi-relay`, tokio plus
  `tokio-tungstenite`, MIT). Its features:
  - rooms by a short invite code;
  - a cap on players per room;
  - messages forwarded with the sender's peer id;
  - rate and size limits;
  - rooms close when empty;
  - no game logic and no message storage;
  - a health endpoint.
- `WsTransport` (native) implements `Transport` over a WebSocket to the
  relay. The reliable channel is the socket. The unreliable channel is
  also the socket, documented as "reliable in practice on this backend".
- `RelaySession` creates a room, joins by code and lists peers. It is the
  first session backend (1145 generalizes it).
- A `Dockerfile`, and `docs/net-relay.md`: deploying to a small
  VPS with TLS (Caddy, Apache-2.0), and what it costs.
- Tests: two native clients and a relay in one test process play the
  toy lockstep game. Abuse tests cover oversize messages, flooding and a
  bad room code.

**Out (do not do):**
- the web client (1142);
- accounts (1148);
- running a public instance.

## Implementation steps

1. Write the relay server with unit tests of its room table.
2. Write the WebSocket transport.
3. Add the integration and abuse tests.
4. Add the Docker image and the docs.

## Acceptance criteria

- [ ] The integration and abuse tests pass in CI.
- [ ] The Docker image builds in CI.
- [ ] `cargo deny check` passes. TLS uses `rustls` with `ring`, never
      `aws-lc-rs`, because aws-lc-rs needs NASM on the GNU toolchain.
      Root certificates come from the system's store
      (`rustls-native-certs`, through `tokio-tungstenite`'s native-roots
      feature), never `webpki-roots` or `webpki-root-certs`: both are
      CDLA-Permissive-2.0, which is not on `deny.toml`'s allow list
      (2601 has the same rule). If `WsTransport` sits behind a cargo
      feature that is off by default, `cargo deny check` must see that
      feature, since `deny.toml`'s `[graph]` checks default features
      only.
- [ ] All gates pass.

## Tests required

- Unit: rooms and limits.
- Integration: relay plus two clients.

## Completion notes
