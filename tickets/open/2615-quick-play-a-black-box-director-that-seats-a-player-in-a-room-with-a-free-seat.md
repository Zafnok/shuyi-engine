---
id: "2615"
title: "Quick play: a black-box director that seats a player in a room with a free seat, up to a cap of rooms"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2614"]
nick_input: none
completed:
---

# 2615 — Quick play: a black-box director over rooms

Unparked by Nick's decision of 2026-11-25 (`docs/decisions.md`): the
next example game, trails, is played online, with matchmaking the
player never sees.

## Context

Nick, 2026-11-25, wants black-box matchmaking. The player says "play",
and the server picks a room that isn't full, up to about 10 rooms. Empty
seats are ridden by bots, and players drop in and out of them (2614).

- **ADR-0006** (networking in layers) puts "who is in a match" in its
  Session layer and matchmaking in its Services layer. Quick play sits
  between them: it is a session the server picks, with no lobby.
- **ADR-0150** (written by 2613) is the sync model this rests on: an
  authoritative server runs each room's `TickSim` headless and
  broadcasts the resolved inputs.
- **2614** builds the room this ticket directs: `RoomGame`, the sans-IO
  `Room` with seats that are `Human(peer)` or `Bot`, `RoomClient`, the
  `room::wire` codec and its `PROTOCOL`, and the test game
  `room::testing::Dots`.
- **2613** builds `Transport`, `PeerId`, `NetEvent` and `MemoryNetwork`
  with seeded faults.
- **2616** (blocked by this ticket) puts a director behind WebSocket,
  TLS, `/healthz` and `/stats`. Everything here stays sans-IO: no
  socket, no clock, no thread.
- The build check reuses `Made` and `Made::check`
  (`crates/core/src/made.rs:111` and `:134`), the stamp that records and
  saves already carry. 1140's lockstep handshake compares builds the same
  way.

What quick play is not:

- It has no accounts, ratings or queues by skill. Those stay with
  1148, which is parked; once Nick unparks it, its ranked queue hands
  matches to this director.
- It has no lobby, ready step, room code or invite. 1145, parked, may
  later wrap this director as a quick-play `SessionBackend`, and keep
  its explicit lobby for invites and friends.
- It has no names, chat or room browser. Free text from strangers would
  bring moderation duties (ADR-0006, Context), and trails keeps no
  personal data (decision of 2026-11-25).

The game (`Zafnok/visions-of-shuyi`, pinned at engine revision
`b69585eee640ba3ba4963bc62ca5f6ec3db189d7`) uses no `shuyi-net` code:
`gh search code --repo Zafnok/visions-of-shuyi 'shuyi_net'` finds
nothing, and `'shuyi-net'` finds only the copied skill pack and its
tickets about copying it. Nothing in the game changes.

## Nick input

None. The room cap of 10 and the seat count of 8 are Nick's numbers from
2026-11-25; they are config, so a game picks its own.

## Scope

**In:** a module `shuyi_net::director` in `crates/net`, pure (no I/O, no
clock, no floats), as `tools/layers.ron` already holds `shuyi-net` after
2613.

- **`DirectorConfig`**, `#[non_exhaustive]` with `DirectorConfig::new()`
  and `with_*` setters (ADR-0039; a new field later is additive):
  - `rooms_max: u16`, default 10;
  - `rooms_warm: u16`, default 1: rooms kept running with bots only, so a
    new player lands in a round already under way. Clamped to
    `rooms_max`;
  - `idle_close_s: u16`, default 60: a room with no human, past the warm
    ones, closes after this long;
  - `msgs_per_second: u16`, default 30: a peer's message budget. Past it
    the peer is disconnected. A client sends `Input` on change and
    `Alive` once a second (ADR-0150), so 30 is far above honest play at
    15 steps a second;
  - `hello_timeout_s: u16`, default 5: a peer that sends no `Hello` in
    this long is dropped;
  - `retry_after_s: u16`, default 10, sent in `Busy`.

  Durations are written in seconds and turned into steps with the room
  game's step rate as 2614 defines it on `RoomGame` (trails runs 15
  steps a second; `shuyi_core::STEPS_PER_SECOND` is 60). The director
  never reads a clock: one `step` is one room step.
- **`Director<G: RoomGame>`**, built with `Director::new(config, game,
  made: Made, seed: u64)`. Each room it opens takes its seed from a
  `SimRng` seeded from `seed`, so a run is reproducible.
  - `on_event(&mut self, event: NetEvent)`: a connect, a message or a
    disconnect from 2613's `NetEvent`.
  - `step(&mut self, out: &mut Outbox)`: steps every open room once, in
    room id order, and collects what each peer is sent. The outbox is
    the one 2614's `Room::step` writes into; if 2614 names it
    differently, use 2614's.
  - `stats(&self) -> DirectorStats`: `rooms`, `humans`, `bots`, `peers`
    (connected, seated or not), `refused` (a count each of
    `OtherBuild`, `Busy`, `Protocol` and flood disconnects since the
    start). `#[non_exhaustive]`, plain counts, `Copy`; 2616 serves it as
    `/stats`.
- **The handshake**, per peer: `Connected` → `Greeted` → `Seated(room)`
  or `Watching(room)`.
  - The first message must be `Hello { protocol: u16, made: Made }`.
    Anything else first, or a `Hello` that doesn't decode, is
    `Refused { why: Refusal::Protocol { ours } }`, then a disconnect.
  - `protocol` is checked against `room::wire::PROTOCOL`; a mismatch is
    `Refusal::Protocol { ours }`.
  - `made` is checked with `Made::check` against the director's `Made`;
    a mismatch is `Refusal::OtherBuild { theirs: Made }` (the director's
    own stamp), so the client can show `Mismatch`'s text with its own
    `Made::check`.
  - Otherwise the answer is `Welcome`.
- **`Play`**, after `Welcome`, seats the peer:
  1. the open room with the most humans that still has a bot seat a
     human may take (2614's join rule: the lowest bot seat whose rider
     is alive, or a watching place until the next round), ties to the
     lowest room id;
  2. otherwise a new room, while `rooms < rooms_max`;
  3. otherwise `Refused { why: Refusal::Busy { retry_after_s } }`. The
     peer stays connected and may send `Play` again.

  A full room (every seat `Human`) is never picked. A peer already in a
  room who sends `Play` again is ignored.
- **`Leave`** frees the seat: 2614's room turns it back into a `Bot` at
  the next step, and the peer is back at `Greeted`. A disconnect, or
  2614's no-`Alive` time-out, does the same and forgets the peer.
- **Room lifecycle:**
  - at `Director::new`, `rooms_warm` rooms open with bots only;
  - a room with no human counts its idle steps; past `idle_close_s` it
    closes, unless closing it would leave fewer than `rooms_warm` open
    rooms;
  - room ids are `u16`, never reused while the director runs, so a log
    line names one room.
- **A message budget:** each peer has a counter per second of steps.
  Past `msgs_per_second` the director stops reading that peer, sends it
  nothing more, frees its seat, and asks the transport to close it (the
  disconnect 2613's `Transport` offers). The count goes in `refused`.
- **Wire:** `Hello`, `Welcome`, `Play`, `Leave` and `Refused` are added
  to 2614's `room::wire` codec, in its byte form (no serde, as 2612
  gives `SeatInput` one), and `PROTOCOL` goes up by one. `Made` is
  encoded as three length-prefixed fields (the engine's version and the
  game's word as UTF-8, at most 255 bytes each, then the content hash's
  8 bytes, little-endian). The messages join 2614's fuzz and round-trip
  tests.
- **`Refusal`**, `#[non_exhaustive]` (a new reason later is additive):
  `OtherBuild { theirs: Made }`, `Busy { retry_after_s: u16 }`,
  `Protocol { ours: u16 }`.
- `room::testing::Dots` (2614) gets what these tests need, and nothing
  game-like.

**Out (do not do):**
- sockets, TLS, HTTP, `/stats` as a page, and the real clock (2616);
- the client's screen for quick play and its text for each refusal
  (2790);
- accounts, ratings, leaderboards, queues by skill (1148, parked);
- room codes, invites, a lobby and the `SessionBackend` wrapping this
  director (1145, parked); the relay (1141, parked);
- 2606's `Accepts`, `UpdateRequired` and consent refusals (2606,
  parked): when 2606 is unparked, it decides whether its `Refused` takes
  in this ticket's `Refusal`;
- regions, a room browser, names, chat, spectating by choice;
- the room itself, its seats, bots, join and leave rules (2614);
- anything in trails' rules (2788, 2789).

**Estimated size:** about 900 lines (code 450, tests 380, docs 70).

## Implementation steps

1. In `crates/net/src/room/wire.rs` (2614's), add `Hello`, `Welcome`,
   `Play`, `Leave` and `Refused` with `Refusal`, the byte form of `Made`,
   and bump `PROTOCOL`. Add them to the round-trip and fuzz tests.
2. Add `crates/net/src/director.rs` (with `director/tests.rs`):
   `DirectorConfig`, `DirectorStats`, the per-peer handshake states and
   `on_event`.
3. Write the seating policy as a pure function over the rooms' seat
   counts (`fn pick(rooms: &[RoomSeats], rooms_max) -> Pick`, `Pick`
   being `Room(id)`, `Open` or `Busy`), so the policy is tested apart
   from the rooms.
4. Write the room lifecycle: warm rooms at start, idle counting, close,
   ids never reused; then `step` over the rooms in id order.
5. Write the message budget and the flood disconnect.
6. Write `stats`.
7. Write the tests below, the churn test over `MemoryNetwork` with
   seeded faults and `room::testing::Dots`.
8. Rustdoc on the module saying what quick play does and doesn't do
   (the Out list, in a game maker's words). Add the completion notes'
   `Book:` line (2854 writes the chapter on online play) and `Skills:`
   line (2855 rewrites the `shuyi-net` skill for rooms and quick play).

## Acceptance criteria

- [ ] 80 peers who each send `Hello` and `Play` fill 10 rooms of 8
      humans; the 81st gets `Busy { retry_after_s: 10 }`, and its next
      `Play` after one human leaves is seated in that freed room.
- [ ] With three open rooms holding 5, 7 and 7 humans out of 8 seats,
      `Play` seats the peer in the lower-id room of 7; with rooms of 8,
      8 and 2, in the room of 2.
- [ ] With `rooms_max` rooms open and every seat human, `Play` is
      `Busy`, and no room is opened.
- [ ] A `Hello` with another `Made` gets `OtherBuild` carrying the
      director's `Made`; one with another `protocol` gets `Protocol`; a
      `Play` before `Hello` gets `Protocol`; each is then disconnected.
- [ ] A peer who sends nothing for `hello_timeout_s` is dropped.
- [ ] With `rooms_warm: 1`, one bots-only room runs from the start and
      never closes; a second room whose last human left closes exactly
      `idle_close_s` seconds of steps later, not one step sooner.
- [ ] A peer sending `msgs_per_second + 1` messages in one second is
      disconnected, its seat is a `Bot` at the next step, and
      `stats().refused` counts it; one sending exactly
      `msgs_per_second` is not.
- [ ] Over 100,000 seeded operations (connect, `Hello`, `Play`,
      `Leave`, disconnect, flood, step) on `MemoryNetwork` with seeded
      loss and delay: a peer is in at most one room; open rooms ≤
      `rooms_max`; in every room, humans + bots = seats; open rooms ≥
      `rooms_warm`; `stats()` matches a count over the rooms.
- [ ] Two directors with the same seed fed the same events write the
      same outbox, byte for byte.
- [ ] `cargo xtask check-layers` passes: `shuyi-net` gains no outside
      crate.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the seating policy's cases (the fullest room that isn't full,
  ties to the lowest id, open a room, cap reached, full rooms skipped).
- Unit: each refusal of the handshake, and the hello time-out.
- Unit: warm rooms, the idle close at its exact step, ids not reused.
- Unit: the flood limit at `msgs_per_second` and one past it.
- Unit: `stats` counts.
- Property: the churn test above, 100,000 seeded operations, with its
  invariants checked after every operation.
- Property: the new wire messages round-trip, and any byte string
  decodes to a message or an error, never a panic.
- Integration: 80 `RoomClient`s (2614) over `MemoryNetwork` follow
  their rooms, and each client's hash equals its room's at every hash
  step.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
