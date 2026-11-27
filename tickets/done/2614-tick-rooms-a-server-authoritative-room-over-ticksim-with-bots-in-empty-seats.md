---
id: "2614"
title: "Tick rooms: a server-authoritative room over TickSim, with bots in empty seats, drop-in and drop-out, and a client that follows it"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: done
blocked_by: ["2612", "2613", "2620"]
nick_input: none
completed: 2026-11-27
---

# 2614 — Tick rooms: a server-authoritative room with bots in empty seats

## Context

- Nick's decision of 2026-11-25 (`docs/decisions.md`): trails, an
  online light-trail arena with capped rooms, bots in empty seats, and
  players dropping in and out.
- ADR-0155 (2613) chose an authoritative server that broadcasts each
  step's resolved inputs, with a hash every 15 steps and a catch-up from
  the round's start. This ticket builds that room, sans-IO, in
  `shuyi-net`. The server (2616) and the clients (2790) both use it, and
  so do the director's tests (2615).
- It builds on:
  - `shuyi_core::TickSim` (`crates/core/src/tick.rs`: `type Event`,
    `step(&mut self, &TickInput) -> Vec<Event>`, `hash(&self) -> u64`),
    `TickInput` and `SeatInput` (2137, done);
  - 2612 (done): `MAX_SEATS` is 8, and `SeatInput` and `TickInput` have a
    fixed little-endian byte form (`encode`, `decode`, in
    `crates/core/src/tick/bytes.rs`);
  - 2613: `PeerId`, `NetEvent`, `Transport` and `MemoryNetwork`, and
    2620: its seeded faults;
  - `shuyi_core::SimRng` and `shuyi_core::Made`.
- `grep -rnw` over `tickets/open` for `RoomGame`, `RoomClient`, `Room`
  and `Outbox`: only the new tickets of this set (2615 writes into this
  ticket's `Outbox` and adds its messages to this ticket's `wire`; 2789
  implements `RoomGame` for trails; 2790 drives `RoomClient`). Nothing
  older plans them. 1147's snapshots and rollback are a different sync
  model and stay there.

## Nick input

None.

## Scope

**In:** a module `shuyi_net::room`, pure (no I/O, no clock, no floats).

- **`pub trait RoomGame`**, what a game gives the room:
  - `type Sim: TickSim`;
  - `const SEATS: usize`, at most `shuyi_core::tick::MAX_SEATS` (a
    `const` assertion);
  - `const STEPS_PER_SECOND: u32` (trails: 15);
  - `const MAX_ROUND_STEPS: u32`: a round that reaches it ends as a
    `Draw`, which bounds the catch-up;
  - `const PAUSE_STEPS: u32`: the steps between a round's end and the
    next round;
  - `fn made(&self) -> Made` (2615's handshake);
  - `fn new_round(&self, seed: u64, round: u32) -> Self::Sim`;
  - `fn round_winner(sim: &Self::Sim) -> RoundEnd`, with `RoundEnd {
    Going, Won(usize), Draw }`;
  - `fn seat_alive(sim: &Self::Sim, seat: usize) -> bool`: whether a
    seat still has a living rider, for the join rule;
  - `fn bot_input(sim: &Self::Sim, seat: usize, rng: &mut SimRng) ->
    SeatInput`: the game's bot, pure. The engine gives the hook, never
    the AI.
- **`Room<G: RoomGame>`**, the server side, without I/O:
  - `Room::new(game: G, seed: u64)`: the room's own `SimRng` from
    `seed` draws round seeds and feeds `bot_input`;
  - seats are `Seat::Human(PeerId)` or `Seat::Bot`;
  - `join(&mut self, peer) -> Result<Joined, Full>`:
    `Joined::Seat(n)` takes the lowest `Bot` seat whose rider is alive
    (`seat_alive`), controlled from the next step; if none is alive,
    `Joined::Watching`, and the peer takes the lowest bot seat when the
    next round starts. `Full` when humans and watchers already fill
    `SEATS`. A peer already in the room gets its current place back;
  - `leave(&mut self, peer)`: the seat becomes `Bot` at the next step;
  - **silence:** a seat whose peer sent neither `Input` nor `Alive` for
    `5 * STEPS_PER_SECOND` steps becomes `Bot`, and the peer is listed
    in the outbox's `drop` so the host closes it;
  - **an input latch** per seat: the last `held` set wins; `pressed` and
    `released` events queue, at most 2 per seat, one applied per step,
    so a quick double turn between two steps survives. An `Input` from
    a watcher or an unknown peer is ignored;
  - `step(&mut self, out: &mut Outbox)`: builds the step's `TickInput`
    (each human seat from its latch, each bot seat from `bot_input` with
    the room's rng, in seat order), steps the sim, and writes a `Step`
    frame for every human and watcher. Every 15th step the frame carries
    `hash()`. `Outbox { frames: Vec<(PeerId, Frame)>, drop: Vec<PeerId>
    }`, `#[non_exhaustive]` with `Outbox::new()` and `clear()`;
  - **rounds:** when `round_winner` isn't `Going`, the room sends
    `RoundOver { winner, wins }`, waits `PAUSE_STEPS`, draws a new seed,
    seats the watchers, sends `RoundStart { round, seed, wins }`, and
    clears the round's input log. Rounds go on without end;
  - `Roster { seats }` (each `Human` or `Bot`, without peer ids: a
    client learns only its own seat) on every change;
  - **the catch-up:** on join, and on `Resync`, `Joined { place, roster,
    round_start, steps: Vec<TickInput> }` with every step's inputs since
    the round began (at most `MAX_ROUND_STEPS`);
  - `Room::stats()`: humans, bots, watchers, the step and the round
    (plain counts, `Copy`, `#[non_exhaustive]`), for 2615's
    `DirectorStats`.
- **`RoomClient<G: RoomGame>`**, the client side:
  - `on_frame(&mut self, bytes: &[u8]) -> Result<(), WireError>` takes
    what the transport delivered; `advance(&mut self) -> Advance` is
    called once per step by the game's tick driver and applies at most
    one buffered `Step`, or two when more than 2 are waiting (it
    catches up), or none when the buffer is empty (it waits);
  - a jitter buffer of 1–2 steps: it starts applying once one step is
    buffered past the first;
  - `sim()`, `my_seat() -> Option<usize>`, `roster()`, `wins()`,
    `steps_behind()`, and the events each applied step made, so a
    screen plays sounds once (1119's rule);
  - `input(&mut self, seat_input: &SeatInput) -> Option<Vec<u8>>`: the
    bytes of an `Input` message when the actions changed since the last
    one, else `None`; `alive(&mut self) -> Option<Vec<u8>>` once a
    second of steps when nothing else was sent;
  - on a hash that differs from its own, it stops applying and sends
    `Resync`; the room answers with a fresh catch-up.
- **`room::wire`**, a hand-written fixed little-endian codec, no serde:
  - `pub const PROTOCOL: u16`, starting at 1;
  - each message is a one-byte tag, then its fields; `TickInput` uses
    2612's byte form;
  - down: `Step { step: u32, inputs: TickInput, hash: Option<u64> }`,
    `Joined`, `Roster`, `RoundOver`, `RoundStart`, `Closing` (the
    server is shutting down, 2616);
  - up: `Input(SeatInput)`, `Alive`, `Resync`, `Leave`;
  - `Frame` and `ClientMsg` are `#[non_exhaustive]`, so 2615's `Hello`,
    `Welcome`, `Play` and `Refused` are additive;
  - `decode(bytes) -> Result<_, WireError>` never panics, refuses an
    unknown tag, a short or overlong input, a seat over `SEATS`, and a
    catch-up longer than `MAX_ROUND_STEPS`.
- **`room::testing::Dots`**, behind a `testing` feature: a tiny 16x16
  `RoomGame` (dots that move one cell a step and stop at walls, with
  `seat_alive` and `round_winner` simple enough to script), for 2615's
  and 2616's tests and this ticket's.
- Public types follow ADR-0039: `RoundEnd`, `Joined`, `Frame`,
  `ClientMsg`, `WireError` and `Outbox` are `#[non_exhaustive]`.

**Out (do not do):**
- Choosing a room, the `Hello` handshake and quick play: 2615.
- Sockets, the real clock and the tick loop: 2616 (server), 2617 and
  1142 (clients).
- Client-side prediction of the player's own rider, rollback and
  interpolation: 1147 (and 2604's measured costs).
- Connections that only watch by choice, names and chat (ADR-0155).
- Trails' own rules and bot: 2788, 2789.
- A `cargo fuzz` target for `room::wire`: 2020's harness isn't built
  (2020 is open), so the seeded random-bytes test below covers decoding.

**Estimated size:** about 1,300 lines (code 650, tests 550, docs 100).

**Split:** past 1,000 lines, move `RoomClient` and its tests (steps 4
and part of 5) to a new ticket written then with the `write-ticket`
skill, blocked by this one; this ticket keeps `wire`, `Room`, `Dots`
and the room's tests, and 2790's `blocked_by` gains the new ticket.

## Implementation steps

1. `room::wire`: the messages, `encode`, a never-panic `decode`, and
   their round-trip tests.
2. `Room`: seats, `join`, `leave`, silence, the input latch.
3. `Room::step`, rounds, the pause, the catch-up and `Roster`.
4. `RoomClient`: frames, the jitter buffer, `advance`, hash checks and
   `Resync`.
5. `room::testing::Dots`, and the integration tests over 2613's
   `MemoryNetwork` with 2620's faults on.
6. Write the `Book:` (2854) and `Skills:` (2855) lines in the completion
   notes.

## Acceptance criteria

- [x] ~~Three `RoomClient`s and one `Room` of `Dots` over a `MemoryNetwork`
      with delay, and loss, duplicates and reordering on (Reliable
      channel): every client's hash equals the room's at every check,
      across 10 rounds.~~ (not done: split off to 2621 with `RoomClient`)
- [x] ~~A client that joins mid-round reaches the room's state (equal
      `hash()`) after applying its catch-up, and its next check passes.~~
      (the client's half split off to 2621; the room's half is here: a
      mid-round catch-up, and a `Resync`'s, replayed from its
      `RoundStart` equals the room's sim)
- [x] A `leave`, or `5 * STEPS_PER_SECOND` steps of silence, makes the
      seat `Bot` at the next step; a join takes the lowest bot seat
      whose rider is alive; with none alive it is `Watching` until the
      next round, then seated.
- [x] Joining a room whose seats are all humans or watchers is `Full`.
- [x] Rounds restart after `PAUSE_STEPS` with wins counted; a round at
      `MAX_ROUND_STEPS` ends as a `Draw`.
- [x] ~~A client whose sim is deliberately corrupted sends `Resync` and is
      equal again after the catch-up.~~ (not done: split off to 2621; the
      room's answer to a `Resync` is tested here)
- [x] A catch-up never holds more than `MAX_ROUND_STEPS` inputs.
- [x] Two quick presses between two steps are both applied, on two
      steps.
- [x] No float arithmetic in `shuyi-net`; `cargo xtask check-layers`
      passes.
- [x] The completion notes have a `Book:` line naming 2854 and a
      `Skills:` line naming 2855.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each message's round trip; each `WireError`.
- Fuzz-style: decode 100,000 seeded random byte strings (and mutations
  of valid frames) without a panic.
- Property: join and leave churn from seeded operations keeps the
  invariants (each seat is `Human` or `Bot`; a peer holds at most one
  seat or watching place; humans plus watchers never exceed `SEATS`).
- Integration: the equal-hash test with three clients; catch-up
  equality; `Resync`; bot takeover timing; round restart and wins.

## Completion notes

**What was done.** `shuyi_net::room`, the server's side of a tick room
(ADR-0155), pure:

- `crates/net/src/room.rs`: `RoomGame` (its `SEATS`, checked at compile
  time to be 1 to `MAX_SEATS`, with `STEPS_PER_SECOND` at least 1, where
  `Room::new` and `Frame::decode` name the check), `RoundEnd`, `Joined`,
  `Seat` (`Human(PeerId)` or `Bot`), `SeatKind` (the same without the
  peer, for clients), `Full`, and `HASH_EVERY` (15).
- `room/host.rs`: `Room` (`new`, `join`, `leave`, `receive`, `step`,
  `place`, `seats`, `sim`, `wins`, `stats`, `game`), `Outbox` and
  `RoomStats`. Seats, the input latch, silence, rounds with their pause,
  the catch-up and the roster, as the Scope says.
- `room/wire.rs`: `PROTOCOL` (1), `Frame`, `ClientMsg`, `Roster`,
  `RoundStart`, `WireError`, each message's `encode` and a `decode` that
  never panics. Frames take the first bytes `0x01`–`0x06`, client
  messages `0x80`–`0x83`.
- `room/testing.rs`: `Dots`, behind the new `testing` feature (off by
  default; the crate's tests turn it on through a dev-dependency on
  itself): eight dots on 16 by 16, 15 steps a second, rounds of at most
  600 steps, a pause of 3; a bot that turns and goes out at random.
- Tests: the wire's round trips, each `WireError` and its text, 100,000
  seeded random strings and 100,000 mutations of valid messages, with
  every string that decodes encoding back to the same bytes; 16 room
  tests, among them a churn of 20,000 seeded joins, leaves, messages and
  steps that checks the seats' books after each.
- The crate doc names `room`; `crates/net/README.md` says what the rooms
  are, cost and how to start one.
- The API gate lists `shuyi-net` with its `testing` feature, as it lists
  the other crates' `toy` (`crates/xtask/src/api.rs` and its test's
  table): `Dots` is public to other crates' tests.

**The split.** At about 2,300 counted lines without the client, the
ticket's own split rule moved `RoomClient`, its tests and the
integration tests over `MemoryNetwork` (steps 4 and part of 5, the first
and the client's half of the second and sixth criteria) to **2621**. Its
Context holds what a client reads from the room, as built here. 2615 and
2790 (and 1147, parked) gain 2621 in `blocked_by`; 2789, 2529, 2550,
2854 and 2855 name 2621 for the client.

**Deviations.**

- `Frame::Roster` carries the receiver's own `place` beside the roster:
  with `Roster { seats }` alone, a watcher had no way to learn the seat
  the next round gives it. `wire::Roster` itself holds no peer id.
- A peer's messages reach the room through `Room::receive(peer,
  ClientMsg)`, which the Scope didn't name; it takes them decoded, so
  2615's director can read its own (`Hello`, `Play`) first.
- `join` takes no outbox (the Scope's signature), so the catch-up goes
  out at the room's next `step`, before that step's frame; a `Resync`'s
  too, once however often it is asked.
- A silent watcher is dropped after the same five seconds of steps as a
  seat: otherwise it held a place for ever.
- A third press or release waiting between two steps is dropped (the
  Scope said at most 2, not which go), and those waiting when a round
  ends are dropped when the next one starts; the held set stays.
- `RoundOver.winner` is `Option<usize>` (none for a draw), not a
  `RoundEnd`, which can't be `Going` there. Wins are a `u32` a seat.
  `RoundStart.wins`, in a catch-up too, are the wins so far, so a peer
  that joins between rounds sees the last one counted.
- The room cuts each list of a seat's input to 255 actions before it
  steps the sim with it, so the sim and the clients agree when an input
  holds more than the byte form does. What a game's actions allow is
  2622's.
- `WireError`'s variants are `Short`, `Overlong`, `UnknownTag`,
  `NoSuchSeat`, `TooManySeats`, `CatchUp`, `BadInput` and `BadField`. A
  step's inputs cut short read as `BadInput`: 2612's `TickInput::decode`
  doesn't tell short from malformed.
- `Dots` has eight seats, as 2615's rooms of eight need. Its
  `new_round` reads only the seed (the room draws a new one each
  round), each cell and heading a draw of its own.
- **CI's mutants** (first run, 29 missed) asked for tests of two
  watchers at once, a watcher's silence carried into its seat, a winner
  past the seats, `Outbox::clear`, the round's seed, the seat and roster
  bytes, and `Dots`' own rules; and two lines changed so no mutant of
  them is equivalent: a round's seed adds its halves, and a roster's
  byte sums its humans' bits, where `|` and `^` gave the same.

**Follow-up tickets.** 2621 (the client, above). 2622: a client's
`Input` may hold up to 255 ids a list, and the room repeats a seat's
held set in every step to every peer, so the room should keep a seat's
input to the game's actions.

**Handovers.** 2615 gets a **From 2614 (done)** list of what a director
calls on a room; its wire step no longer adds `Leave`, which is here,
and its outbox clause is settled (`room::Outbox`).

**The game.** No break and no `Behaviour:` line: `shuyi-net` only gains
a module and a feature off by default. At the game's pin (`10cbb98b`),
`gh search code --repo Zafnok/visions-of-shuyi 'shuyi_net'` and
`'RoomGame'` find nothing.

Book: none, 2854 writes the book's chapter on online play (rooms, bots,
quick play) once trails plays online; this adds the room's server side
only, which no chapter teaches yet.

Skills: none, 2855 rewrites the `shuyi-net` skill for rooms and quick
play; this changes no schema, template, format or problem code.

Size: 2,706 counted lines (2,331 before the tests CI's mutation testing
asked for), the ticket's own split point: `RoomClient` and
the tests over `MemoryNetwork` went to 2621. What is left lands
together: the wire's frames are what `Room::step` writes and nothing
else makes them, so the wire alone would be a codec of frames no code
writes, and the room's tests read every frame through the wire's round
trip; `Dots` is what both test with.
