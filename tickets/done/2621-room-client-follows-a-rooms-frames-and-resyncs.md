---
id: "2621"
title: "Room client: RoomClient follows a room's frames behind a jitter buffer, checks its hash and resyncs"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2614"]
nick_input: none
completed: 2026-11-27
---

# 2621 — Room client: RoomClient follows a room's frames and resyncs

## Context

- Ticket 2614 built the server's side of a tick room in
  `crates/net/src/room/` (`shuyi_net::room`, ADR-0155): `RoomGame`,
  `Room`, `Outbox`, `RoomStats`, the `room::wire` codec and
  `room::testing::Dots` (behind the `testing` feature). It came to about
  2,330 counted lines without the client, so its own split rule moved
  `RoomClient` and the tests over `MemoryNetwork` here.
- What the room sends, which this client reads (`room::wire::Frame`,
  `crates/net/src/room/wire.rs`; `Room::step`,
  `crates/net/src/room/host.rs`):
  - `Step { step, inputs, hash }`: `step` counts from 1 in each round;
    `hash` is `Some` every `room::HASH_EVERY` (15) steps, the room's
    `TickSim::hash()` after that step.
  - `Joined { place, roster, round_start, steps }`: the catch-up, sent at
    the room's next step after a join or a `Resync`, before that step's
    `Step` frame. `steps` holds every step's inputs of the round so far
    (between rounds, the finished round's), at most `MAX_ROUND_STEPS`.
    `round_start.wins` are the wins so far.
  - `Roster { place, roster }`: who rides each seat changed, from the
    next step. It carries the receiver's own `place`, so a watcher learns
    its seat when a round starts.
  - `RoundOver { winner, wins }` right after a round's last `Step`; then
    `PAUSE_STEPS` steps with no frame; then `RoundStart(RoundStart)`,
    then a `Roster` when the round seated watchers, then `Step { step: 1 }`.
  - `Closing`: the server shuts down (2616 sends it).
  - Frames go on the `Reliable` channel: in order, never lost.
- What the room reads (`room::wire::ClientMsg`): `Input(SeatInput)`;
  `Alive`; `Resync`; `Leave`. A seated or watching peer that sends
  nothing for `5 * STEPS_PER_SECOND` steps is dropped. The room latches
  an `Input`'s held set and queues its presses and releases (two at
  most), one per step.
- `grep -rnw RoomClient tickets/open`: 2615 (its 80-client test), 2789
  (its smoke test), 2790 (trails' online screen) and 1147, 2529, 2550
  (parked) use it; none builds it.

## Nick input

None.

## Scope

**In:** `crates/net/src/room/client.rs` (with `client/tests.rs`),
pure (no I/O, no clock, no floats), re-exported from `shuyi_net::room`.

- **`RoomClient<G: RoomGame>`**, built with `RoomClient::new(game: G)`
  (private fields, ADR-0039 kind 8):
  - `on_frame(&mut self, bytes: &[u8]) -> Result<(), WireError>`
    decodes with `Frame::decode::<G>`. A `Joined` rebuilds at once: the
    sim from `game.new_round(seed, round)`, every step of the catch-up
    applied (their events dropped: they were played before), its
    place, roster and wins taken, the buffer emptied, a stop after a
    mismatch lifted. `Step`, `Roster`, `RoundOver` and `RoundStart`
    queue in arrival order. `Closing` marks the client closed. Frames
    before the first `Joined` are dropped.
  - `advance(&mut self) -> Advance`, called once per step by the game's
    tick driver: it applies every queued frame that isn't a `Step` at
    the head of the queue (a `RoundStart` builds the round's sim), then
    at most one `Step`, or two when more than 2 wait (it catches up),
    then the frames that aren't steps after them; none when no step
    waits. The jitter buffer: it starts applying once 2 steps wait, and
    again after it ran dry.
  - `Advance`, `#[non_exhaustive]`: `Stepped(u32)` (how many),
    `Waiting`, `Resync(Vec<u8>)` (the bytes of a `ClientMsg::Resync` for
    the game to send), `Closed`.
  - The check: a `Step` whose `hash` differs from the client's sim's
    after it, or whose `step` isn't the next, stops applying; that
    `advance` returns `Advance::Resync`, and the ones after it `Waiting`
    until the catch-up arrives.
  - `sim() -> Option<&G::Sim>`, `place() -> Option<Joined>`,
    `my_seat() -> Option<usize>`, `roster() -> &Roster`,
    `wins() -> &[u32]`, `round() -> u32`, `steps_behind() -> usize`
    (the `Step`s waiting), and `events() -> &[<G::Sim as TickSim>::Event]`:
    what the steps the last `advance` applied made, so a screen plays
    each sound once (1119's rule).
  - `input(&mut self, seat: &SeatInput) -> Option<Vec<u8>>`: an
    `Input`'s bytes when `seat` has a press or a release, or a held set
    other than the last one sent; else `None`. The same press on two
    steps running is sent twice.
  - `alive(&mut self) -> Option<Vec<u8>>`: an `Alive`'s bytes when
    nothing was sent for `STEPS_PER_SECOND` advances; else `None`.
  - `leave(&mut self) -> Vec<u8>`: a `Leave`'s bytes.
- **Integration tests** in `crates/net/tests/rooms.rs` over
  `MemoryNetwork`, with a host loop written there: a `Connected` joins
  (`Room::join`), a `Message` is decoded (`ClientMsg::decode`) and given
  to `Room::receive`, a `Disconnected` leaves; each step the host calls
  `Room::step`, sends each frame's bytes on `Channel::Reliable`, and
  disconnects `Outbox::drop`'s peers.
- The crate's README (`crates/net/README.md`): the client in *How to
  start*, and its size in *What it costs*, measured with Hello, Grid
  building a `RoomClient<Dots>` in a release build (rule 13: the module
  is already behind the front door's `net` feature).

**Out (do not do):**
- Predicting the player's own rider, rollback and interpolation (1147).
- Sockets and the tick loop of a real client (2617, 1142, 2790).
- The handshake, `Hello` and quick play (2615).
- Changing `Room` or `room::wire`'s frames: a change they need is its
  own ticket (the `write-ticket` skill), unless it is a bug, fixed here
  with a test.

**Estimated size:** about 1,100 lines (code 400, tests 650, docs 50).

**Split:** past 1,000 lines, split off `input`, `alive` and `leave` (the
sending side) and their tests into a new ticket blocked by this one;
the integration tests' clients then send `ClientMsg::Alive.encode()`
themselves.

## Implementation steps

1. `RoomClient::new`, `on_frame` and the queue; the accessors.
2. `advance`: the jitter buffer, catching up, the frames that aren't
   steps, the hash check and `Advance::Resync`.
3. `input`, `alive`, `leave`.
4. Unit tests in `client/tests.rs`, feeding frames a `Room<Dots>` wrote
   (no network).
5. The integration tests over `MemoryNetwork`.
6. The README; the `Book:` (2854) and `Skills:` (2855) lines in the
   completion notes.

## Acceptance criteria

- [x] Three `RoomClient`s and one `Room` of `Dots` over a
      `MemoryNetwork` with delay, and loss, duplicates and reordering on
      (frames on the `Reliable` channel): every client's hash equals the
      room's at every check, across 10 rounds.
- [x] A client that joins mid-round reaches the room's state (equal
      `hash()`) after applying its catch-up, and its next check passes.
- [x] A client whose sim is deliberately corrupted (in a unit test,
      through its private field) answers `Advance::Resync` at the next
      hash, sends it, and is equal again after the catch-up.
- [x] With 1 step waiting before the start, `advance` is `Waiting`; with
      2, `Stepped(1)`; with 3, `Stepped(2)`; with none after the start,
      `Waiting`, and it starts again only at 2.
- [x] `events()` holds each applied step's events once, and a catch-up's
      none.
- [x] `input` is `None` for an unchanged held set with no press or
      release, and `Some` for each press, the same press on two steps
      running included; `alive` is `Some` after `STEPS_PER_SECOND`
      advances with nothing sent, and not before.
- [x] A watcher's `my_seat()` is `None`, and its seat once the next
      round's `Roster` is applied.
- [x] No float arithmetic in `shuyi-net`; `cargo xtask check-layers`
      passes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the jitter buffer's cases; catching up; frames that aren't
  steps applied in order; the step-number check; `input` and `alive`.
- Integration: the equal-hash test with three clients over 10 rounds;
  the mid-round joiner; the corrupted client's `Resync`.

## Completion notes

**What was done.** `shuyi_net::room::RoomClient` and `Advance`, pure,
re-exported from `shuyi_net::room`:

- `crates/net/src/room/client.rs`: `RoomClient::new(game)` (private
  fields, ADR-0039 kind 8), `on_frame`, `advance`, the accessors
  (`sim`, `place`, `my_seat`, `roster`, `wins`, `round`,
  `steps_behind`, `events`) and the sending side (`input`, `alive`,
  `leave`), as the Scope says: the catch-up rebuilds at once with its
  steps' events dropped; the buffer starts at 2 steps and again after it
  ran dry, steps 2 while more than 2 wait, and applies the frames that
  aren't steps in their place; a step out of order or a hash that
  differs is `Advance::Resync`, then `Waiting` until the catch-up.
- `room/client/tests.rs`: 10 unit tests over frames a `Room<Dots>`
  wrote: the buffer's cases, a catch-up mid-round and its next check,
  frames that aren't steps applied after their step, a step missing, a
  sim corrupted through its private field, `events()` against a mirror
  sim, `input`, `alive`, a watcher's seat, and frames before the
  catch-up and `Closing`.
- `crates/net/tests/rooms.rs`: a host loop over `MemoryNetwork` (join on
  `Connected`, `ClientMsg::decode` and `Room::receive` on a `Message`,
  leave on `Disconnected`, frames on `Channel::Reliable`, `Outbox::drop`
  disconnected), with delays of 1 to 4 steps and loss, duplicates and
  reordering on. Three clients follow 10 rounds with no `Resync` and
  their sims equal to the room's after every advance (more than 500
  comparisons each); a joiner at step 37 is the room's state right after
  its catch-up and passes its next check; a client whose first catch-up
  had a byte of its seed flipped on the way resyncs once and is equal at
  every step after.
- `crates/net/README.md`: the client in the intro and *How to start*, and
  its size in *What it costs*. The crate's doc names the client.

**Size measured** on 2026-11-27: Hello, Grid's web release build
(`cargo build -p hello-grid --release --target wasm32-unknown-unknown`)
with `shuyi-net` (and its `testing` feature, for `Dots`) and a live use
of every method of a `RoomClient<Dots>` is 19,299 bytes bigger
(4,038,862 → 4,058,161), `Dots`' rules included. Rule 13 holds without a
new toggle: the client is in `shuyi-net`, behind the front door's `net`
feature, off by default.

**Deviations.**

- A `Closing` closes the client before its first catch-up too, so a
  client waiting for one learns the server is going.
- While the client waits for the catch-up it asked for, it drops every
  frame but the catch-up and `Closing`: the catch-up holds all they
  carry. `steps_behind()` is then 0.
- `Advance::Closed` answers every `advance` after a `Closing`, steps
  waiting or not.
- `alive`'s second restarts on anything the client gave to send: an
  `Input`, an `Alive`, a `Resync` and a `Leave`.
- The integration test can't reach the client's private field, so its
  corrupted client gets a catch-up whose seed was spoiled on the way;
  the unit test corrupts the sim through its field, as the criterion
  says.
- No accessor names a round's winner (the Scope lists none): `wins()`
  gains one for the winner's seat at `RoundOver`. Written into 2790's
  **From 2621 (done)**, the screen that would show it.

**Handovers.** 2790 gets a **From 2621 (done)** list of what a screen
calls on the client; 2615's clause on 2621 now says it is done, and
points its 80-client test at `tests/rooms.rs`'s host loop. 1147
(prediction, parked) names the client already.

**The game.** No break and no `Behaviour:` line: `shuyi-net` only gains
`RoomClient` and `Advance`. `gh search code --repo
Zafnok/visions-of-shuyi 'RoomClient'` and `'shuyi_net'` find nothing.

Book: none, 2854 writes the book's chapter on online play (rooms, bots,
quick play) once trails plays online; this adds the client, which no
chapter teaches yet.

Skills: none, 2855 rewrites the `shuyi-net` skill for rooms and quick
play; this changes no schema, template, format or problem code.

Size: 1,010 counted lines, past the Split's 1,000 by 10 and under the
ticket's own estimate of 1,100, so the sending side stayed: split off,
its 100 lines would come back as a second PR whose integration tests
rewrite this one's players' sends.
