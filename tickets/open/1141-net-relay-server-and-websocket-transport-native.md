---
id: "1141"
title: "Parked: Networking, step 2: friend rooms by invite code, a relay room kind in shuyi-server over the native and web clients"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["1140", "2613", "2616", "2617"]
nick_input: none
completed:
---

# 1141 — A relay with room codes, in shuyi-server

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1, §3 and §5. A relay forwards messages between the players in
a room. Both players connect outward, so home routers are no problem, no
NAT traversal is needed, and the browser can join (1142).

It is the simplest internet play there is for friends who already know
each other: one creates a room and passes its code. Vendors have closed
or shrunk in 2026, so we don't depend on one.

Nick's decision of 2026-11-25 (`docs/decisions.md`) unparked an online
example, trails, played in rooms of a server with quick play (ADR-0155).
That work builds what this ticket needed underneath it, so this ticket
stays parked and builds on it:

- `shuyi-server` (2616): a tokio listener with WebSocket, optional TLS
  (rustls on ring), limits on connections, addresses and messages,
  `/healthz` and `/stats`. The relay is a second room kind in that
  crate, not a new binary.
- The native client (2617): `shuyi_app::net::WsClient`, adapted to
  `Transport` by the front door's `shuyi::net::connect`. The web client
  is 1142's, behind the same call.
- Hosting: 2618's ADR runs a server on Oracle Cloud's Always Free
  tier, with a reproducible setup, TLS on an IP certificate and a
  systemd unit; 2856 is the book's page on hosting. A relay runs the same
  way, at no cost.

## Nick input

None to build it. Running it publicly is a decision for when a game ships
online with friend rooms (0053, or a later example).

## Scope

**In:**
- **A relay room kind** in `shuyi-server` (2616), for example
  `shuyi_server::relay` with `run_relay(config)` or a `RoomKind` the
  config picks; the session work is pure, in `shuyi_net::relay`, as
  2614's rooms are:
  - rooms by a short invite code;
  - a cap on players per room;
  - messages forwarded with the sender's peer id;
  - rate and size limits (2616's, reused);
  - rooms close when empty;
  - no game logic and no message storage;
  - 2616's `/healthz` and `/stats`, with relay counts.
- The client is 2617's `WsClient` through `shuyi::net::connect` (and
  1142's on the web). The reliable channel is the socket. The unreliable
  channel is also the socket, documented as "reliable in practice on this
  backend".
- `RelaySession` creates a room, joins by code and lists peers. It is the
  first session backend with codes (1145 generalizes it).
- Docs: how to host a relay and what it costs (nothing on the free host),
  where the docs map (`docs/README.md`) puts hosting a server.
- Tests: two native clients and a relay in one test process play the
  toy lockstep game (1140). Abuse tests cover oversize messages, flooding
  and a bad room code.

**Either order with 2606:** if 2606 is done, the relay's handshake
opens with `RELAY_PROTOCOL: u16`, a room keeps its creator's `Made`,
the relay starts with an `Accepts` list, and it refuses with
`Refused::Protocol`, `OtherBuild` and `UpdateRequired` before
forwarding anything, with tests; if not, nothing to do (2606 adds them
to the relay and `RelaySession`).

**Out (do not do):**
- quick play, seating a stranger in a room with a free seat: 2615;
- the web client (1142);
- accounts (1148);
- running a public instance;
- a UDP mode, encrypted datagrams and NAT traversal on the relay (2605
  adds them, reusing this relay's rooms, limits and TLS WebSocket for
  its connect token);
- voice chat (2910 researches it, and whether the relay's WebSocket
  is good enough for it). **Either order with 2910:** if 2910 is done,
  read its ADR and do any line it added to this scope for voice; if
  not, nothing to do (2910 builds on the relay and `WsTransport`).

## Implementation steps

1. Write the relay's room table in `shuyi-net`, with unit tests.
2. Add the relay room kind to `shuyi-server`, reusing its listener,
   limits and TLS.
3. Write `RelaySession` over `shuyi::net::connect`.
4. Add the integration and abuse tests.
5. Add the relay's section to the hosting docs.

## Acceptance criteria

- [ ] The integration and abuse tests pass in CI.
- [ ] `cargo deny check` passes. TLS uses `rustls` with `ring`, never
      `aws-lc-rs`, because aws-lc-rs needs NASM on the GNU toolchain.
      Root certificates come from the system's store
      (`rustls-native-certs`), never `webpki-roots` or
      `webpki-root-certs`: both are CDLA-Permissive-2.0, which is not on
      `deny.toml`'s allow list (2601 has the same rule). 2616 and 2617
      already hold this rule; keep it.
- [ ] All gates pass.

## Tests required

- Unit: rooms and limits.
- Integration: relay plus two clients.

## Completion notes
