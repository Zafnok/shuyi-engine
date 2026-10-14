# ADR-0006: Networking in layers: transports, sessions, sync models, services; determinism is the contract

- **Status:** Accepted (Nick chose plan B, 2026-10-14; see `docs/decisions.md`). First proposed as the game repository's ADR-0067 (game ticket 1101)
- **Date:** 2026-10-14
- **Related tickets:** 1140–1150; 0053 (which multiplayer modes our game
  has, after launch)
- **Extends:** game ADR-0004 (a pure core), game ADR-0013 (licenses), game ADR-0034
  (the web build's own JS plugins), game ADR-0039 (a battle is its history),
  ADR-0003 (turn and tick simulations)

## Context

Nick, 2026-10-14: "if it turns out feasible without jeopardizing our own
velocity", add "easy, abstracted networking": P2P on a LAN or the
internet, sessions, hosted multiplayer (one player's machine, or a
company's servers), matchmaking and ELO, MMO, and global chat. Phase it
from easy to hard. Expose both the abstractions and "the lower level
stuff if a certain quirk needs to be used".

For our game, 0053 decides the multiplayer modes after the single-player
launch. Nick's first reactions: hotseat yes, live online PvP wanted,
play-by-link maybe.

What the research found (sources in the game repository's [feasibility report](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/feasibility.md),
*Networking*):

- **The web build is the constraint.** macroquad's web loader doesn't
  use `wasm-bindgen` (game ADR-0034). Almost every browser-capable networking
  crate assumes it: matchbox (WebRTC), renet2's web clients, ewebsock,
  web-transport, and GGRS on wasm (doubtful). naia has a macroquad
  feature. Otherwise the path is our own small JS plugin, as
  `web/gamepad.js` already is.
- **Lockstep fits a deterministic turn-based game perfectly.** Machines
  send commands, never state. Our `seed + commands` already serves as
  the save, the replay and the desync check (Age of Empires checksummed
  its world each turn the same way). Its weaknesses: every client holds
  everything, so a modified client can see through fog and preview rolls
  it can compute from the seed. The fixes are a referee server that
  holds the seed and the hidden state, or commit-reveal per roll between
  two peers.
- **Rollback (GGPO, GGRS in Rust) fits 2–4 player real-time games** on a
  fixed-step integer simulation, which ADR-0003 gives us.
  **Snapshot replication** fits many players and MMOs and needs no
  determinism.
- **Vendors vanish.**
  - Hathora's game hosting shut down on 2026-05-05.
  - Unity's Multiplay hosting ended on 2026-03-31.
  - PlayFab's free tier shrank to 1,000 lifetime players in March 2026.
  - SpacetimeDB is BSL-1.1, client SDK included, so game ADR-0013 denies it.
  - Nakama (Apache-2.0) is self-hostable, but its Rust client was
    archived.

  A $4–6 a month VPS runs a small relay.
- **Steam** gives free lobbies, relayed P2P that hides IP addresses
  (Steam Datagram Relay), lobby chat and leaderboards. `steamworks-rs`
  is MIT/Apache over the SDK. Whether it links on the Windows GNU
  toolchain is unverified.
- **Ratings:** `skillratings` (MIT/Apache, no dependencies) implements
  Elo, Glicko-2 and Weng-Lin (OpenSkill). TrueSkill is patented by
  Microsoft (estimated expiry 2029) and trademarked.
- **Chat with strangers is a legal commitment:**
  - EU DSA notice-and-action;
  - UK Online Safety Act (Ofcom put in-game chat and matchmaking with
    strangers in scope in 2025);
  - COPPA for mixed audiences from 2026-04-22;
  - GDPR for chat logs;
  - ESRB's "Users Interact" notice.

## Decision

### 1. Four layers, each public

Every layer is a public trait or type with its own docs and tests. A game
may use a high layer and drop to a lower one for a quirk, or use a low
layer alone.

| Layer | What it is | Backends, in the order built |
| ----- | ---------- | ---------------------------- |
| **Transport** | Bytes between peers: peer ids, connect and disconnect events, a reliable ordered channel and an unreliable channel, statistics | in-memory with seeded loss, delay and reordering (tests) → WebSocket to our relay (native, and web through our own JS plugin) → UDP on a LAN (native) → Steam networking messages (`steam` feature) → WebRTC or WebTransport later (1147 decides how, given the loader) |
| **Session** | Who is in a match: create, list, join by invite code, ready, start, leave; metadata | the relay's rooms → LAN discovery by UDP broadcast → Steam lobbies |
| **Sync model** | How a simulation stays the same on every machine; generic over `TurnRules` or `TickSim` (ADR-0003) | **Lockstep** (turns, peers) → **Referee** (turns, a server holds the seed and hidden state and sends each player their view) → **Rollback** (ticks, 2–4 peers) → **Snapshots** (ticks, client-server, many players) |
| **Services** | Accounts, a matchmaking queue, ratings, verified results and leaderboards, chat | our own small server first; Steam where it fits; Nakama only if self-hosted and a maintained client exists |

### 2. Determinism is the network contract

- A session's handshake checks the engine version, a hash of the loaded
  content (data files, mods), and the seed. A mismatch is refused with a
  message naming what differs.
- Each turn (lockstep) or every N steps (rollback) peers exchange a state
  hash (`TurnRules`/`TickSim` give one). A difference is a desync: the
  match stops, both sides keep the record, and `shuyi replay` (ADR-0005)
  finds the first step where they part.
- Reconnecting is replaying the history, the same way a battle loads
  (game ADR-0039).

### 3. One codebase for players and servers

`shuyi-core` has no I/O and no clock (game ADR-0004). So a server is the
same simulation in a headless binary (`shuyi-server`: tokio, no
macroquad). A listen server is the same code inside a player's game. A
referee, a dedicated server and a result checker (re-simulating a
submitted command list) are all this one program.

### 4. Hidden information and cheating, stated plainly

- **Lockstep between peers is for friends.** Both clients know
  everything, the seed included. Engine docs say so.
- **Casual P2P with dice:** an opt-in **commit-reveal** per roll. Each
  peer commits a hash of a nonce, and the roll comes from both. A peer
  who refuses to reveal forfeits.
- **Ranked or competitive play needs the referee.** The server holds the
  seed and the fog, checks every command, and records the result.
  Ratings come only from referee-checked or re-simulated matches.

### 5. Self-host first; vendors behind the traits

Everything the engine needs runs on servers we (or a game's developer)
can host. These are the relay, the referee, matchmaking and chat. They
are small Rust binaries with Docker images. Third-party services
(Steam, Edgegap for hosting, Nakama) are backends behind the same
traits, never the only way. Licenses follow game ADR-0013: no BSL
(SpacetimeDB). No proprietary SDK except Steam's behind its feature.

### 6. The browser

WebSocket through our own miniquad JS plugin, which is small and fits
the existing loader (game ADR-0034's pattern). The browser has no UDP, so LAN
and Steam are native-only. Real-time in the browser waits for 1147,
which decides between more JS plugins (WebRTC data channels or
WebTransport) and moving the web build to `wasm-bindgen`.

### 7. Ratings

Glicko-2 by default (public domain; via `skillratings`), Weng-Lin for
team or free-for-all games. Never TrueSkill.

### 8. Chat is scoped first; global chat is a decision, not a feature

The engine gives chat channels over sessions with these tools:

- a word filter (`rustrict`, MIT/Apache; Steam's own filter on Steam);
- block, mute and report;
- a moderation hook for the game's own service.

That covers lobby chat, match chat and friends. **Global chat** in a
shipped game needs moderators, a report process, legal notices and an
age rating answer. That is the game owner's decision (for us, Nick's,
in 0053 or a later ticket), not something the engine turns on.

### 9. MMO is research, not a promise

Zones, interest management, persistence, accounts, an economy, and
running it all are 40–80+ tickets and hundreds to thousands of dollars
a month. 1150 studies it on top of the snapshot model and recommends
whether to go further. Nothing else here depends on it.

### 10. When

Networking is engine work. Nothing networked reaches our game before
0053 is decided, which is after launch. The first piece (1140:
in-memory transport and lockstep, no real network) is cheap. It proves
the determinism contract, and it can run once 1114 has given us
`TurnRules`.

## Consequences

- Our game's online PvP (if 0053 picks it) is lockstep plus a relay: a
  few tickets once 1140–1145 exist, because the rules already replay.
  A ranked mode adds the referee (1146).
- Every networked feature is testable in one process with the in-memory
  transport and seeded network faults. Two Harness games play a match
  against each other in CI.
- We run a server once anything online ships (a relay at least). It is
  small, but it needs uptime, updates and a privacy policy. That is an
  ongoing cost for Nick, not only a build cost.
- Real-time online play in the browser is the hardest part, because of
  the web loader. It is deferred, not designed away.
- The engine offers chat tools but no global chat. A game turning it on
  takes on moderation and legal work that no library removes.

## Alternatives considered

- **One high-level "multiplayer" API and nothing below it** — easy to
  start, but Nick asked for the lower levels, and every real game hits a
  quirk.
- **A hosted backend as the foundation (Nakama, PlayFab, Photon,
  SpacetimeDB)** — fast to start. But two of 2026's vendors closed or
  shrank, Photon's free plan is non-commercial, SpacetimeDB is BSL, and
  Nakama's Rust client is archived. Kept as optional backends.
- **WebRTC P2P (matchbox) first** — no server for game traffic. But it
  needs `wasm-bindgen`, which the web build doesn't use, plus signaling
  and STUN/TURN servers anyway. A relay is simpler and works in the
  browser today.
- **Snapshot replication for everything** — no determinism needed, but
  it throws away our strongest asset and costs bandwidth for a game
  whose moves are a few bytes.
- **Building an MMO stack now** — no game of ours needs it, and its
  running costs are real.
