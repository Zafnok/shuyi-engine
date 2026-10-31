---
id: "1143"
title: "Parked: Networking, step 4: LAN play — a UDP transport and finding games on the local network"
type: feature
milestone: "Engine: networking"
model: implementation
effort: high
status: blocked
blocked_by: ["1140"]
nick_input: none
completed:
---

# 1143 — LAN play: UDP transport and discovery

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1. On a local network, players need no server at all. One
player hosts, and the others find the game by a UDP broadcast and
connect directly. This is native only, because browsers have no UDP.

## Nick input

None.

## Scope

**In:**
- `UdpTransport` uses `std::net::UdpSocket`, with no new dependency. It
  has:
  - an unreliable channel, which is raw datagrams;
  - a reliable ordered channel, built from sequence numbers, acks and
    resends, with a configurable window;
  - connection timeouts;
  - a message size limit, with splitting for larger reliable messages.
- `LanDiscovery`: the host broadcasts a small announcement (game id,
  version, name, players) every second on a fixed port. Clients list
  what they hear.
- `LanSession`: the host is the session's authority.
- Tests:
  - two `UdpTransport`s on localhost play the toy lockstep game;
  - the reliable channel survives a fault-injecting socket wrapper (the
    same seeded faults as 1140);
  - discovery is tested on loopback.

**Out (do not do):**
- NAT traversal (not needed on a LAN);
- encryption on the LAN (documented as unencrypted).

## Implementation steps

1. Write the reliable layer over UDP, with property tests through the
   fault wrapper.
2. Write discovery and the session.
3. Write the docs.

## Acceptance criteria

- [ ] The property and integration tests pass on Windows, Linux and
      macOS CI.
- [ ] All gates pass.

## Tests required

- Property: reliable delivery under faults.
- Integration: localhost match and discovery.

## Completion notes
