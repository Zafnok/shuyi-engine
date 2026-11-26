# ADR-0155: Real-time rooms: an authoritative server broadcasts each step's inputs, with bots in empty seats and quick play without accounts

- **Status:** Accepted
- **Date:** 2026-11-26
- **Related tickets:** 2613 (this, and `shuyi-net`'s transport), 2614
  (rooms), 2615 (the director and quick play), 2616 (`shuyi-server`),
  2617 (the native client), 2618 (the host), 2619 (watching it), 2788–2791
  (trails), 2854–2856 (the book and the skill pack), 1140–1148 (what stays
  parked)
- **Amends:** [ADR-0006](0006-networking-in-layers-transports-sessions-sync-models-services.md)
  §1, §5 and §6; [ADR-0008](0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
  §2 (a headless server backend)

## Context

- Nick's decision of 2026-11-25 (`docs/decisions.md`) makes the next
  example game, trails, an online light-trail arena: rooms on a server,
  bots in empty seats, drop-in and drop-out, quick play. It is a fixed-tick
  game (`TickSim`, ADR-0003) of up to eight seats (2612's `TickInput`).
- ADR-0006 laid networking out in four layers and listed sync models in
  the order built: lockstep, a referee, rollback, snapshots. Its §6 left
  real time in the browser to 1147's research. Its §1 put the in-memory
  transport first, then a WebSocket to our relay.
- The web build has only macroquad's loader and our own JS plugins
  (ADR-0006 §6, game ADR-0034): no `wasm-bindgen`, so no WebRTC data
  channels without more plugins. A WebSocket runs over TCP: a lost packet
  is resent by TCP itself, in order, before anything after it.
- Bots, drop-in and matchmaking need a server running the rules whatever
  the sync model. A peer-to-peer model would still need it.
- The simulation is deterministic and replays (ADR-0002 §2, ADR-0003): the
  same inputs from the same seed give the same states on every target,
  checked by `TickSim::hash()`.
- A seat's input has a byte form (2612). A step's whole `TickInput` for
  eight seats is about 12 B of payload.

## Decision

### 1. The sync model: the server decides, and sends each step's inputs

- **The server runs the real `TickSim`, headless, and decides
  everything**: who sits where, which input counts at which step, the
  bots in empty seats, the rounds and their seeds.
- **Clients send `Input` only when their actions change**, and `Alive`
  once a second so a quiet player isn't taken for a gone one.
- **Each step the server sends a `Step` frame**: the step number and the
  step's whole `TickInput` in 2612's byte form; every 15 steps it adds
  `TickSim::hash()`.
- **Clients step their own copy of the same deterministic sim** with
  exactly those inputs. ADR-0006 §2's contract (determinism) makes the
  copies equal; the hash proves it.
- **Joining, or a resync after a hash mismatch, is a catch-up**:
  `RoundStart { round, seed, wins }` and every step's inputs since the
  round began, at most `MAX_ROUND_STEPS` of them (2614; about 32 KB). No
  game needs a state codec.
- **Clients show confirmed steps** behind a jitter buffer of 1–2 steps,
  and don't predict.

ADR-0006 §1's sync-model row gains this model for ticks, called
**server-authoritative input broadcast**, ahead of rollback.

### 2. Why not rollback

- Rollback suits 2–4 peers with no server. This game needs a server
  anyway: bots, drop-in and matchmaking.
- The browser has only WebSocket, over TCP, where a resend on loss gains
  little: TCP has resent already, in order.
- Server authority settles ADR-0006 §4's cheating concern for inputs: a
  client can't change what counted.
- A game with no hidden information may show every client the whole
  state. A game with hidden information needs 1146's referee; prediction
  of a player's own rider stays 1147's.

### 3. The bandwidth

Per `Step` message: 36 B of payload, 2 B of WebSocket header, 22 B of
TLS 1.3 record and 52 B of TCP/IPv4 with timestamps, 112 B in all. At 15
steps a second that is **1,680 B/s per human**:

| Load | A month (30 days) |
| ---- | ----------------- |
| 80 humans around the clock | about 348 GB, plus about 7 GB of catch-ups |
| At a realistic 10% of that | about 35 GB |
| The whole grid sent each step instead | about 6.6 TB |

So input broadcast is required, not an optimization. Bots cost no
traffic: the server and every client step them from the same inputs.

### 4. The crates

- **`shuyi-net`**, pure: no I/O, no clock, no threads, no floats; it
  depends on `shuyi-core` only. It holds the `Transport` trait, the
  in-memory network (2613) with its seeded faults (2620), the rooms
  (2614) and the director (2615). A module (ADR-0009 §4), behind the front door's `net`
  feature, off by default.
- **`shuyi-server`**, a tokio library (2616): it runs a director's rooms
  over WebSocket with TLS, limits and a health check.
- **The client's sockets live in the backend `shuyi-app`**, behind its own
  `net` feature, with a byte-level API of its own (2617 native, 1142
  web). The front door adapts it to `Transport`, because a default crate
  may not depend on a module (ADR-0009 §4, `cargo xtask check-modules`).

`Transport` is single-threaded (no `Send` bound): a game's transport
lives on its main thread, and a backend with a thread of its own keeps
the thread inside. The in-memory network moves on only by `step`, and
its faults draw from one `SimRng` in a fixed order per message, so tests
of rooms are exact on every target, wasm32 included (CI's `determinism`
job).

### 5. The server is a headless platform backend

Under ADR-0008 §2, `shuyi-server` is a platform backend with no window:
it touches sockets and the clock, and hands time to the rules as an
argument. It never holds a rule; the rooms in `shuyi-net` do.

### 6. Quick play without accounts

No names, no chat, no room browser, no codes: a player presses play and
the director seats them. Free text from strangers would bring moderation
duties (ADR-0006 Context and §8), and accounts bring personal data.

### 7. What stays where

1141's relay, 1145's lobbies, 1146's referee, 1147's rollback and
prediction, and 1148's accounts stay parked; 1140 keeps lockstep and
commit-reveal, built on `shuyi-net`'s transport.

### 8. What it amends in ADR-0006

- **§1:** the Transport row's backends, in the order built, are now:
  in-memory with seeded loss, duplicates, delay and reordering (tests) →
  WebSocket to our server (native, and web through our own JS plugin) →
  then the rest as listed. The sync-model row gains server-authoritative
  input broadcast for ticks (§1 here), built first.
- **§5:** a server may run as a systemd service on a free host, beside
  Docker images; 2618's ADR records the host.
- **§6:** real time over WebSocket in the browser is built now (1142),
  not left to 1147. 1147 keeps WebRTC, WebTransport and prediction.

## Consequences

- One server serves every client kind with one protocol, and the
  in-memory network tests it in one process.
- A tick game goes online by keeping its rules deterministic, which it
  already must; it writes no state codec.
- Clients see the game a step or two late, with no prediction: right for
  a grid arena at 15 steps a second, wrong for a twitch game, which needs
  1147.
- A desync is found within 15 steps and healed by a catch-up, not hidden.
- We run a server, with its uptime, updates and costs (ADR-0006
  Consequences); 2618 and 2619 host and watch it.
- The client's sockets split across two places (the backend and the front
  door's adapter), because of the module rule.

## Alternatives considered

- **Rollback (GGPO-style) over peers** — no server for game traffic, and
  prediction. But this game needs a server anyway, the browser has only
  TCP, and rollback needs peers to trust each other's inputs.
- **Snapshots of the state each step** — no determinism needed, but about
  6.6 TB a month at the load above, against 348 GB, and a state codec per
  game.
- **Lockstep among peers (1140)** — exact, but the slowest peer sets the
  pace, and bots and drop-in still need a host. Kept for turn games.
- **Clients predicting their own rider** — smoother, but the rewinds it
  needs are 1147's, and at 15 steps a second the buffer's delay is small.
- **The client's sockets in `shuyi-net`** — one place, but a pure crate
  can't touch sockets, and a default crate may not depend on a module.
