---
id: "2622"
title: "Rooms keep a seat's input to its game's actions: ids past them and repeats are dropped before they are stepped and sent"
type: feature
milestone: "Engine: networking"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["2614"]
nick_input: none
completed:
---

# 2622 — Rooms keep a seat's input to its game's actions

## Context

- 2614 built `shuyi_net::room` (ADR-0155). A client's
  `ClientMsg::Input(SeatInput)` may hold up to 255 action ids in each of
  `pressed`, `held` and `released` (2612's byte form). The room latches
  the last `held` set and puts it in **every** step's `Frame::Step`, to
  every peer in the room, until the next `Input` (`Latch` in
  `crates/net/src/room/host.rs`). It keeps each list to 255 ids
  (`wire::fit`) and nothing tighter.
- So one `Input` holding a long `held` list costs every peer of the room
  that list's bytes 15 times a second, for as long as the player stays.
  2616's `max_msg_bytes` (512) caps one message at about 250 ids, and
  2615's budget counts messages, not their size: neither stops it.
  ADR-0155 §3 counts on about 36 bytes a step frame; eight seats each
  holding 250 ids make it about 4 KB.
- A game knows how many actions it has (`ActionSet::ALL` in
  `shuyi-assets`, `shuyi_ui::tick::ActionIds`); the room doesn't.

## Nick input

None.

## Scope

**In:**
- `RoomGame` gains `const ACTIONS: u16`, the number of the game's
  actions (ids `0..ACTIONS`), with a default of 16 so an implementation
  without it still builds (ADR-0039 kind 7: a new item on a trait games
  implement has a default).
- The room keeps each list of a seat's input, a human's and a bot's, to
  ids below `ACTIONS`, each once, in its order, before it steps the sim
  and sends the step (where `wire::fit` runs today), so the room's sim
  and the clients' agree.
- `Dots` sets `ACTIONS` to 5.

**Out (do not do):**
- A cap on messages by size or rate: 2616's `max_msg_bytes` and 2615's
  budget.
- Refusing such an `Input` or dropping its peer: the room keeps what is
  allowed and plays on.

**Estimated size:** about 200 lines (code 60, tests 120, docs 20).

## Implementation steps

1. Add `ACTIONS` to `RoomGame` (`crates/net/src/room.rs`), its rustdoc
   saying what the room drops.
2. Replace `wire::fit`'s use in `Room` (`Latch::latch` and the bot's
   input in `Room::play`) with a keep-to-the-game's-actions function;
   `Frame::encode` keeps its own cut to 255.
3. Tests in `crates/net/src/room/host/tests.rs`.

## Acceptance criteria

- [ ] An `Input` whose `held` holds 250 ids, of which 3 are below
      `ACTIONS`, makes step frames whose seat holds those 3, each once,
      in the order sent.
- [ ] A bot's input with an id at `ACTIONS` is stepped and sent without
      it.
- [ ] Ids `0..ACTIONS` pass unchanged, repeats aside.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the three cases above, through `Room::receive` and `Room::step`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
