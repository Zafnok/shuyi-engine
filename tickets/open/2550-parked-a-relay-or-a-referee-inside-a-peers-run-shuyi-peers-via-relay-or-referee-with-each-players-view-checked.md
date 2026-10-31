---
id: "2550"
title: "Parked: A relay or a referee inside a peers run: shuyi peers --via relay or referee, in the same headless process, with each player's view checked"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2529", "1141", "1146"]
nick_input: none
completed:
---

# 2550 — A relay or a referee inside a peers run

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need, 2529 among them): a follow-up of the parked peers run (2529), the parked relay (1141) and the parked referee (1146), and a follow-up of parked work is parked. The game doesn't ask for it, so it waits until after the MVP (`docs/ROADMAP.md`) and until all three are unparked and done. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

2529 adds `shuyi peers`: several copies of a turn game in one headless
process, joined by 1140's `MemoryNetwork` with seeded faults, with
each peer's `NetStats` and a desync report, as an operation of the
tool contract and through MCP. Its peers talk straight to each other,
as lockstep between friends does (ADR-0006 §4). Its **Out** leaves "a
relay or a referee inside the run (1141, 1146)" to this ticket.

Online play has two more shapes (ADR-0006 §1 and §3–§5):

- **Through a relay** (1141). Every peer connects outward to a small
  server, which forwards each message to the room with the sender's
  peer id. ADR-0006's Consequences make an online turn game "lockstep
  plus a relay". 2529's run has no relay in it, so a bug in joining a
  room, in the forwarding, or in a game's use of the relay's peer ids
  shows only against a real server.
- **Under a referee** (1146). A server holds the seed and the full
  state, checks each command, and sends each player only what that
  player may see (`PlayerView::view_for`). Its point is hidden
  information: a card game's hands, a board game's hidden tiles,
  tactics in fog. 1146's tests prove that no hidden cell reaches a
  client, for its own toy game, in Rust. An agent can't check that its
  game's views leak nothing, or that each client rebuilds its view
  right under loss and delay, without writing Rust.

This ticket puts the relay's room table, or 1146's `RefereeSession`,
on one more endpoint of the same `MemoryNetwork`: the **hub**. Every
peer is linked to the hub only, so each message really goes through
it. `shuyi peers --via relay` and `--via referee` choose the hub. Each
peer's `NetStats` then measures its hop to the hub, the answer adds
the hub's own stats, and under a referee the check is no longer "every
peer agrees with peer 0" but "every peer's view equals the referee's
view for it".

The end goal (`docs/decisions.md`, 2026-10-19: the most AI-friendly
2D engine) is why it goes through the contract: an agent checks a
game's online shapes with the tools it already uses.

By the time this starts, everything it builds on exists. 1146 comes
after 1145, which comes after 1141 and 1143, so the relay, sessions,
`hello-grid`'s online mode and its `peers` (2529's or 1145's part,
whichever landed later) are all done.

ADR-0009 §4 still holds: `shuyi-cli` never depends on `shuyi-net`.
The contract gains plain data and defaulted methods only, and
`shuyi-net` builds the hubs behind its `tools` feature.

## Nick input

None.

## Scope

**In:**
- **The contract** (`crates/cli/src/contract/peers.rs`), all of it
  additive. The new types follow ADR-0039 as 2529's do:
  - `Via`: `Direct` (the default), `Relay`, `Referee`,
    `#[non_exhaustive]`, lowercase in JSON. `PeersInput` gains `via:
    Via` with `serde(default)`, so an input without it reads as
    before.
  - `Hub { net: NetStats, turns: Option<u32> }`,
    `#[non_exhaustive]` with a constructor: the hub transport's stats,
    and the commands the referee applied (none for a relay).
    `PeersOutput` gains `via: Via` and `hub: Option<Hub>`.
  - `Peers` gains three defaulted methods: `via(&self) -> Via`
    (`Direct`), `hub(&self) -> Option<Hub>` (`None`), and
    `expected_hashes(&self, peer: u32) -> Option<Vec<StateHash>>`
    (`None`): under a referee, the hash of the referee's view for that
    peer before the first command and after each one.
  - `GameTools::peers_via(&self, options: &[GameArg], count: u32, net:
    &NetConditions, via: Via) -> Result<Box<dyn Peers>, String>` in
    `crates/cli/src/game.rs`. Its default calls `GameTools::peers` for
    `Direct` and refuses any other party with "the game's tools don't
    offer peers through a relay (GameTools::peers_via)" (or "under a
    referee"). `peers::peers` always calls it. `GameTools::peers` stays
    as it is.
  - `peers::compare`: when every peer has `expected_hashes`, each
    peer's `turn_hashes` is compared with its own expected list (both
    cut to the shorter) by `replay::first_differing`, instead of with
    peer 0's. The desync's `at.hashes` are then the referee's and that
    peer's. A peer behind the referee is no desync; its limit says
    "peer N is K turn(s) behind the referee's view for it: the turns
    after T weren't compared".
  - The limits always name the party. Through a relay: "the peers met
    in an in-memory relay's room (the relay's room table): its
    WebSocket, TLS and server aren't tried". Under a referee: "the
    peers played under an in-memory referee (RefereeSession):
    `shuyi-server` and its sockets aren't tried". 2529's line on the
    in-memory network stays.
  - `peers::text` names the party on its first line ("2 peers through
    a relay: ok").
- **The CLI:** `--via direct|relay|referee` in `peers::parse_args` and
  `peers::usage`. An unknown party is `SH0001`. `peers_via` left at its
  default for that party is `SH3001`; any other error of the game's is
  `SH3002`, as in 2529.
- **The schemas:** `peers.input.json` and `peers.output.json`
  regenerated, `contract::VERSION` raised by one, and its line added
  to `crates/cli/schemas/versions.json`.
- **`FakePeers`** (`crates/cli/src/contract/fake.rs`): a settable
  party, hub and expected hashes.
- **`shuyi-net`** (`crates/net`):
  - **Links to the hub only.** If 1140's `MemoryNetwork` links every
    transport to every other, add a way to link chosen pairs (for
    example `MemoryNetwork::star(hub)`), so a peer's one link is the
    hub's. A peer that could reach another peer directly would pass
    the relay test without the relay.
  - **The relay's pieces run with no I/O.** The room table (rooms by
    code, the cap, forwarding with the sender's peer id, the rate and
    size limits) and the client's side of the relay's protocol must
    run with no tokio and no clock. If 1141 already built them so
    inside `shuyi-net`, use them. If they sit in `crates/relay` with
    tokio, move the pure part into a `shuyi_net::relay` module, and
    `crates/relay` calls it. The room table takes the time as a whole
    number from its caller: `crates/relay` passes its clock's, and the
    in-memory relay passes the network's step count, with limits a
    peers run never meets (the limits stay 1141's abuse tests' job).
    `shuyi-net` never depends on `crates/relay`, which depends on it.
  - **`RelayLink<T: Transport>`**, a `Transport` over a transport to
    the relay, whose peers are the room's other members, unless 1141
    already has one. If `WsTransport` can sit on it with a small
    change, make it do so, so the protocol has one implementation.
  - **`MemoryPeers::through_relay(count, net, peer, hashes)`**
    (`tools` feature), beside 2529's `MemoryPeers::start`. The hub
    runs the room table on its own `MemoryTransport`. Seat 0 creates a
    room, and the other seats join it by its code before frame 0. The
    network steps until every seat is in the room, at most `64 + 4 ×
    delay` steps, or the start fails naming the seats still out.
    `peer` gets a `RelayLink<MemoryTransport>` already in the room;
    `hashes` reads the game's lockstep hashes as in 2529.
  - **`MemoryPeers::under_referee(count, net, referee, peer,
    hashes)`**, for `R: PlayerView` whose `R::View` is `Serialize`:
    - `referee: impl FnOnce(MemoryTransport, &[PeerId]) ->
      Result<RefereeSession<R>, String>` builds 1146's session on the
      hub's transport, with the seats' peer ids in seat order;
    - `peer: impl Fn(Seat, MemoryTransport) -> Result<Headless<G>,
      String>` starts one player's client, linked to the referee only;
    - `hashes: impl Fn(&Harness<G>) -> Vec<StateHash>` reads the
      hashes of the views the client holds, one per turn;
    - after each command the referee applies, `MemoryPeers` keeps
      `state_hash` of `view_for(seat)` for every seat (and one before
      the first command): `Peers::expected_hashes`. It reads the rules
      through 1146's accessor; if there is none, add a read-only
      `RefereeSession::rules(&self) -> &R`.
  - Each network step, the hub polls its transport after the network
    moves and handles every message in the order the network delivered
    them, so the same input gives the same answer every time.
    `Peers::hub` is the hub transport's stats and, under a referee, the
    referee's turn count. Nothing here uses floats.
  - `crates/net/README.md` names the two constructors.
- **`hello-grid`:** `examples/hello-grid/src/tools.rs` offers
  `GameTools::peers_via` for `Direct` and `Relay` through
  `MemoryPeers`, and a CI test runs `cargo shuyi peers 2 --via relay
  --net seed=1,loss=10,delay=2` on it. It has no hidden information,
  so it offers no referee: `--via referee`, if step 7 adds it, answers
  `SH3001`.
- **The new game's online part, either order with 2551:** if 2551 is
  done, its `online-lockstep` part of the built-in template
  (`templates/shuyi-default/`) also offers `GameTools::peers_via` for
  `Direct` and `Relay`, through `MemoryPeers::through_relay` for the
  relay, as `hello-grid`'s tools do. `WithPeers` (2551, in
  `contract/peers.rs`) answers `peers_via` itself, `Direct` through the
  `peers` it is given, and never passes it on to the tools it wraps.
  CI's leg that makes a game with `--online lockstep` also runs `cargo
  shuyi peers 2 --via relay --net seed=1,loss=10,delay=2` on it, which
  must answer `ok`. If not, nothing to do (2551 offers `--via relay`
  when it lands).
- **MCP:** the `peers` tool gains `via` from its schema, with no code
  here. If the MCP session holds peers (2529 kept them at its step 9,
  or its MCP follow-up ticket is done), `start` takes an optional
  `via` beside `peers` and `net`, `reset` starts the same party again,
  and the `net` tool's answer adds the hub's stats. If that follow-up
  ticket is still open, add those three lines to its scope in this PR
  instead, naming 2550.
- **The `shuyi-net` skill. From 1134 (done):** add `--via relay` (and
  `--via referee`, if step 7 adds it) to the `peers` section of
  `skills/shuyi-net/SKILL.md` (2529 wrote it): what each checks, the hub
  in the answer, and `through_relay` (and `under_referee`) for a game's
  tools. `check-skills` passes.
- **Docs** (the `document` skill): `docs/contract.md`'s *Peers*
  section (`via`, the hub, what is compared under a referee, the
  limits) and its MCP lines if the session took `via`; `docs/net.md`
  (the two constructors); `docs/net-relay.md` (the room table also
  runs in memory in `shuyi peers --via relay`).
- **For the game:** additive. `GameTools` and `Peers` gain defaulted
  methods, `PeersInput` and `PeersOutput` gain fields, and
  `MemoryPeers` gains constructors. If CI's `api` job calls any of it
  a break, the completion notes carry its `Breaking:` line. The game's
  PR that moves the pin follows this one.
- **Size and the split:** the contract, its wiring and unit tests are
  about 200 changed lines; the links and the relay's hub about 150
  (more if the room table moves); the referee's hub with its test game
  about 200; `hello-grid` and the docs about 70. That is near one PR's
  worth, so the referee is the last code step (step 7). It stays in
  this PR only if the diff, fixtures and snapshots aside, is under 450
  changed lines when step 7 starts. Otherwise step 7 writes a follow-up
  ticket with the `write-ticket` skill instead: parked like this one,
  `blocked_by: ["2550"]`, holding the referee's pieces above
  (`Via::Referee`, `Peers::expected_hashes`, `compare`'s branch and
  its limit, `MemoryPeers::under_referee` and its tests). Then none of
  them, and no line on the referee in the CLI, the docs or the skill,
  is added here, so the schema holds no party that no game can offer.

**Out (do not do):**
- Real sockets: the relay's tokio server, its WebSocket and TLS, and
  `shuyi-server`'s binary and listen mode. They stay 1141's and
  1146's tests; the answer's limits say they aren't tried.
- The LAN (1143), Steam's relay and lobbies (1144) and the browser
  (1142) as hubs. `Via` is `#[non_exhaustive]`, so one can join later
  if a game asks.
- Lobbies in the run: the peers start in the room already. The lobby
  screen and `SessionBackend` are 1145's, tested there.
- Accounts, matchmaking and ratings (1148). The referee's result
  record is neither rated nor checked here; 1146's checker does that.
- Tick games and rollback (1147), as in 2529.
- A referee in `hello-grid`, which has nothing hidden. The referee is
  shown by this ticket's test game.
- More than 8 peers, several rooms at once, and load on the relay:
  1150's research.
- Recording or rewinding each peer's session: 2529's other follow-up.

## Implementation steps

1. Read 2529's `crates/cli/src/contract/peers.rs`, `contract/fake.rs`
   (`FakePeers`), `GameTools::peers` and `shuyi_net::tools::MemoryPeers`
   as built; 1140's `MemoryNetwork` (how it links transports, its
   step); 1141's relay (the room table, the protocol's messages,
   `WsTransport`, `RelaySession`) and `docs/net-relay.md`; 1146's
   `PlayerView`, `RefereeSession` and how its tests inspect every
   message; `docs/contract.md`'s *Peers*; ADR-0006, ADR-0009 §4,
   ADR-0022 and ADR-0039.
2. Write the contract's part for the relay: `Via` (`Direct`,
   `Relay`), `Hub`, the new fields, `Peers::via` and `Peers::hub`,
   `GameTools::peers_via`, `--via`, the text and the limits, with unit
   tests over `FakePeers`.
3. In `shuyi-net`: link chosen pairs, move the relay's pure pieces if
   they need it, add `RelayLink` if 1141 has none, and write
   `MemoryPeers::through_relay` with its integration test.
4. Offer `--via relay` in `hello-grid`, with its CI test.
5. Do the MCP lines (or add them to 2529's MCP follow-up ticket if it
   is still open).
6. Measure the diff, fixtures and snapshots aside (*Size and the
   split*).
7. Under 450 changed lines: add the referee: `Via::Referee`,
   `Peers::expected_hashes`, `compare`'s branch and its limit,
   `MemoryPeers::under_referee`, the test-only referee game and their
   tests. Otherwise write the referee's follow-up ticket and name it
   in the completion notes.
8. Regenerate the schemas, raise `contract::VERSION`, and add its line
   to `versions.json`.
9. Write the docs, and the `shuyi-net` skill's part.
10. Run the gates.

## Acceptance criteria

- [ ] `shuyi-cli` has no dependency on `shuyi-net`, and `shuyi-net`
      none on the relay's crate (`cargo metadata`).
- [ ] An input without `via` reads as `direct`, and 2529's tests pass
      with only `via: "direct"` and no hub added to the answers they
      compare.
- [ ] Two peers of 2529's test game through the in-memory relay, with
      `loss=20, delay=3, reorder=10, seed=7`, end with equal turn
      hashes. The answer is `ok`, names `via: "relay"`, has the hub's
      `NetStats`, and a second run gives the same JSON.
- [ ] Through the relay, each peer's transport has one link, the
      hub's, and the hub's stats count every message the peers sent
      each other.
- [ ] The desync 2529's test option forces on seat 1 at turn 3 is
      found through the relay too, naming peer 1 and turn 3.
- [ ] `--via relay` on a game whose tools offer only direct peers
      answers "the game's tools don't offer peers through a relay
      (GameTools::peers_via)" with code `SH3001`; an unknown party
      answers `SH0001`.
- [ ] The limits name the party that ran and what it didn't try.
- [ ] `hello-grid`'s CI test of `cargo shuyi peers 2 --via relay --net
      seed=1,loss=10,delay=2` answers `ok`.
- [ ] `peers.input.json` and `peers.output.json` are checked in,
      `contract::VERSION` is raised, and `versions.json` has its line.
- [ ] Either the referee is here, or step 7 moved it out and the
      follow-up ticket is in `tickets/open/`, parked, and named in the
      completion notes. If it is here:
  - [ ] two players of the test referee game under the in-memory
        referee, with `loss=30, delay=5, reorder=10, duplicate=10,
        seed=3`, answer `ok`, and every peer's view hashes equal the
        referee's view for it at every turn both have;
  - [ ] no message the network delivered to a player holds the other
        player's secret;
  - [ ] a client forced to apply its view wrongly at turn 3 answers
        `ok: false`, with `desync` naming peer 1, turn 3, the referee's
        hash and the peer's.
- [ ] The test games' screens react to actions and name no `Key`,
      `Chord` or `Button`; only the tests' Harness scripts name keys.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-cli`, over `FakePeers`; the referee's only if step 7
  keeps it):
  - `--via`: each party, an unknown one, and the default;
  - `peers_via` left at its default: `Direct` calls `peers`, and
    `Relay` (and `Referee`) answers `SH3001`;
  - `text` and the limits of a relay answer (and of a referee answer);
  - `compare` with expected hashes: equal, differing at turn 3, a peer
    behind the referee.
- Property: for any `NetConditions`, two peers through the in-memory
  relay agree at every turn both have applied. If the referee is here,
  under it each peer's views equal the referee's at every turn the
  peer has.
- Integration (`crates/net/tests/`, with the `tools` feature):
  - 2529's test lockstep game through the relay: agreement, the
    forced desync, and one link per peer;
  - if the referee is here, a test-only game with something hidden: a
    small `PlayerView` whose two players each hold a secret number, a
    command guesses the other's, and a player's view shows its own
    number only. Reuse 1146's toy game instead if `crates/net/tests/`
    can reach it. The test reads every message the network delivered
    to each player, through the check 1146's tests use or through the
    test client keeping the bytes it polled. A test-only start option
    makes seat 1's client apply its view wrongly at turn 3.
- Snapshot: the schemas test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
