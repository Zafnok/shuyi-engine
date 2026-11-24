---
id: "2605"
title: "Parked: Native games have an encrypted UDP internet path through the relay, with NAT traversal and the WebSocket as fallback"
type: feature
milestone: "Parked: networking"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["1141", "1143", "3525"]
nick_input: none
completed:
---

# 2605 — An encrypted UDP internet path with NAT traversal

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"), like 1140–1150. It may start when Nick unparks networking, or when his answer to decision ticket 3525 puts real-time online play into 1.0. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1 lists the transports: in-memory (1140), a WebSocket to our
relay (1141), UDP on a LAN (1143), Steam (1144), and WebRTC or
WebTransport for the browser (1147). None of them gives a native game
a fast path over the internet unless it ships on Steam.

- 1141's `WsTransport` carries the `Unreliable` channel over the same
  socket, "reliable in practice on this backend". That is TCP, so one
  lost segment holds back every later message until it is resent.
  Rollback (1147) exists to avoid exactly that stall.
- 1143's `UdpTransport` is LAN only. Its Out puts NAT traversal and
  encryption aside ("documented as unencrypted").
- 1147 weighs only the browser's options for real-time traffic.
- Only Steam (1144) gives relayed P2P that gets through home routers
  (Steam Datagram Relay).

So a native real-time game on itch.io, GOG or its own site has no
usable internet transport. LÖVE and Godot ship ENet; Bevy's ecosystem
has renet and netcode. (Source: the blind-spot audit of 2026-11-23,
checked against 1141 lines 46–47, 1143's Out and 1147's §3.)

This ticket adds that path on our own relay, self-hosted as ADR-0006 §5
asks:

1. a UDP mode in `shuyi-relay` (1141), beside its WebSocket;
2. datagrams encrypted and authenticated, keyed by a connect token the
   relay hands out over the TLS WebSocket;
3. optional hole punching, with the relay as the rendezvous and the
   fallback;
4. the WebSocket kept as the last fallback when UDP is blocked.

It reuses 1143's reliable layer over UDP, so the reliable channel is
not written twice.

Neither `crates/net` (1140) nor `crates/relay` (1141) exists yet
(`ls crates`: `alloc-count app assets cli core embed embed-macros
platform playdate script shuyi sound sound-out ui xtask`).

## Nick input

None to build it. Running a public relay stays the decision 1141
names (game ticket 0053). The UDP port adds nothing to its cost: the
same $4–6 a month VPS runs it.

## Scope

**In:**
- **An ADR** (the `write-adr` skill) for the native internet path. It
  amends ADR-0006 §1's transport row and says:
  - the packet format, the connect token and the keys (below), with
    the threat model: the relay is trusted, since it forwards every
    message anyway; an eavesdropper or a spoofer on the path is not;
  - why not DTLS (an OpenSSL-shaped dependency, and a TLS stack per
    peer) or Noise through `snow` (a handshake per peer that the
    relay-issued keys make unneeded). If the ADR's own weighing picks
    Noise instead, the steps below change only in how the keys are
    made;
  - that a direct path shows each peer the other's IP address, so it
    is off unless the game turns it on (Steam hides addresses; we
    don't pretend to).
- **The relay's UDP mode**, in `crates/relay`:
  - a UDP socket on a configured port, beside the WebSocket;
  - a `ConnectToken` the relay issues to each peer that joins a room,
    sent as a control message over its TLS WebSocket. It holds the
    room, the peer id, an expiry, the client's keys for each direction
    and the relay's address and port. Its sealed half is encrypted
    with a key only the relay holds, so the UDP side checks a token
    without any state shared with the WebSocket side;
  - datagrams forwarded between the peers of a room, with 1141's rate
    and size limits, and its rule of no game logic and no storage;
  - the source address of each peer's first good datagram kept as its
    public address (for the rendezvous).
- **Encrypted datagrams** in `shuyi-net`, behind the feature 1141's
  `WsTransport` sits behind:
  - ChaCha20-Poly1305 through the `chacha20poly1305` crate
    (Apache-2.0 OR MIT, pure Rust, no `ring` or `aws-lc-rs`). Check
    the licences of it and its tree with `cargo deny check` and name
    them in the PR;
  - the nonce is the packet's sequence number per direction, and a
    replay window of 256 refuses a repeated or stale packet;
  - a packet that fails to decrypt is dropped and counted in `stats()`.
- **`RelayTransport`** (native) implements 1140's `Transport`. Its
  constructor takes the relay's URL, the room code and a `RelayPaths`
  value (`#[non_exhaustive]`, with a constructor and builder methods,
  ADR-0039): `udp` on by default, `direct` off by default.
  - It always opens 1141's WebSocket first. That carries the token,
    the control messages and, while no UDP path works, all traffic.
  - With `udp` on, it sends both channels over UDP to the relay, the
    `Reliable` one through 1143's reliable layer. If no datagram comes
    back within a timeout (3 s by default), it stays on the WebSocket
    and says so.
  - With `direct` on, it punches through to each peer (below) and moves
    that peer's traffic to the direct path when it works, back to the
    relay when it stops.
  - A path change is a `PathChanged { peer, path }` event (`path` one of
    `WebSocket`, `RelayUdp`, `Direct`), and `stats()` reports each
    peer's path. 1140's `net` debug channel shows it.
- **Hole punching** with the relay as the rendezvous: the relay sends
  each peer the other's public address and a shared start time. Both
  send probes for a few seconds, encrypted with a per-pair key the
  relay issued in the token. A probe answered both ways opens the
  direct path. Keep-alives keep it open; three missed ones fall back
  to the relay.
- **Tests and docs:** below, and a section in `docs/net-relay.md`
  (1141): the UDP port to open on the VPS, the token, and what
  `direct` shows other players.
- **Either order with 1145:** if 1145 is done, expose `RelayPaths` as an
  option of its relay session backend, and show the path on its lobby
  screen beside each player. If not, `RelaySession` (1141) takes
  `RelayPaths`, and 1145 carries the option over.
- **Either order with 1147:** if 1147 is done, measure the toy game
  under its setup (100 ms of delay, 5% loss) on `RelayTransport` beside
  `WsTransport`, and add the row to the ADR. If 1147's ADR asks a
  native transport of its rollback ticket, name `RelayTransport`
  there. If not, measure here, and 1147 weighs its browser options
  against this row.
- **Either order with 2606:** if 2606 is done, the UDP handshake carries
  the relay's protocol version and refuses an old one the same way the
  WebSocket does. If not, nothing to do (2606 adds the version to both
  handshakes).
- **Either order with 2920:** if 2920 is done, link its comparison of
  EOS's P2P and relays with this UDP path from the ADR. If not, nothing
  to do (2920 compares them when it runs and writes a section this
  ticket can link).

**Out (do not do):**
- the browser: it has no UDP, so its real-time path is 1147's;
- Steam's relay (1144) and Epic Online Services' P2P, which 2920
  researches as a possible backend behind the same trait;
- the LAN: 1143 stays unencrypted and serverless;
- a relay on more than one machine, regions, or picking the nearest
  relay;
- a TURN server or STUN beyond what the relay itself sees;
- the referee (1146) over UDP;
- running a public instance.

The new public types (`RelayTransport`, `RelayPaths`, `ConnectToken`,
the `PathChanged` event) are new and additive. `RelayPaths` and the
token are `#[non_exhaustive]` with constructors; the path enum is
`#[non_exhaustive]`.

**Estimated size:** about 2,600 lines (code 1,300, tests 1,000, docs 300).

**Split:** past 1,400 lines, split off steps 6 and 7 (hole punching and
the direct path) into a new parked ticket, blocked by this one.

## Implementation steps

1. Write the ADR (Scope), and link it from ADR-0006 §1.
2. In `crates/relay`, add the connect token: issue it on join over the
   WebSocket, seal it with the relay's key, check it on the UDP side.
   Unit tests of the token: expiry, a wrong room, a tampered byte.
3. Add the encrypted packet layer in `shuyi-net` (sequence nonce,
   replay window, drop and count), with property tests.
4. Add the relay's UDP socket and forwarding, with 1141's limits.
5. Write `RelayTransport` over the WebSocket and the relay's UDP, with
   1143's reliable layer for the `Reliable` channel, the UDP timeout
   and the fall back to the WebSocket, and the `PathChanged` event.
6. Add the rendezvous to the relay and the punching to the transport.
7. Add the direct path's keep-alives and its fall back to the relay.
8. Write the integration and abuse tests, the docs and the skill's
   section (1134's `skills/shuyi-net`: when to pick `RelayTransport`,
   and what `direct` shows).

## Acceptance criteria

- [ ] Two native clients and a relay in one test process play 1140's
      toy lockstep game over `RelayTransport` on loopback, over UDP,
      and end with the same state hash.
- [ ] With the relay's UDP port closed, the same match falls back to the
      WebSocket within the timeout, raises `PathChanged` to `WebSocket`,
      and ends with the same state hash.
- [ ] With `direct` on, two clients on loopback reach `Direct`. With
      the direct probes dropped by a fault wrapper, they stay on
      `RelayUdp`. After three missed keep-alives on a direct path,
      they go back to `RelayUdp`.
- [ ] `direct` is off unless the game sets it.
- [ ] The relay drops a datagram with a bad, expired or other room's
      token, a replayed sequence number, or a tampered byte, and counts
      it; the match goes on.
- [ ] Under 1140's seeded faults (loss, delay, reordering, duplicates)
      on the UDP socket, reliable messages arrive once and in order.
- [ ] `cargo deny check` passes, with the new crates' licences named in
      the PR. 1141's TLS rules still hold: no `aws-lc-rs`, and no
      `webpki-roots` or `webpki-root-certs`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the token (issue, seal, check, expiry, tamper); the replay
  window; the path state machine (`WebSocket` → `RelayUdp` → `Direct`
  and back) over a fake clock and a fake socket.
- Property: the packet layer under seeded loss, reordering and
  duplicates delivers each reliable message once and in order, and
  never accepts a packet twice.
- Integration: relay plus two clients over UDP; the UDP port closed;
  hole punching on loopback; abuse (flooding the UDP port, oversize
  datagrams, forged tokens).

## Completion notes
