---
id: "2612"
title: "TickInput holds up to eight seats, and a seat's input has a byte form, for rooms of riders"
type: feature
milestone: "Engine: networking"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2612 — TickInput holds up to eight seats, and a seat's input has a byte form

## Context

Nick's decision of 2026-11-25 (`docs/decisions.md`) makes the next
example game, trails, an online light-trail arena: rooms of up to eight
riders, humans and bots, served by a server that steps the real
simulation and sends every client each step's inputs
([ADR-0150](../../docs/adr/), written by 2613). Two things in
`shuyi-core` stand in the way:

- **Four seats.** `crates/core/src/tick.rs` has `pub const MAX_SEATS:
  usize = 4` (line 48, "four players at one machine"), and
  `TickInput::new` truncates to it (line 135). A trails room (2788,
  2614) has eight seats.
- **No byte form.** The room protocol (2614) sends each step's resolved
  `TickInput` down to every client, about 15 times a second. Today
  `SeatInput` (its `pressed`, `held` and `released` lists of
  `ActionId(u16)`) and `TickInput` have only their serde form, used by
  `TickRecord`'s RON. A wire format needs a fixed, compact,
  little-endian form that never depends on serde's choices.

ADR-0039 lets both types grow: they are `#[non_exhaustive]` and built
with `new` and `one`. Raising a public constant is still a change a game
can see, so it gets a `Breaking:` line (ADR-0038). A search of the game
(`gh search code --repo Zafnok/visions-of-shuyi 'MAX_SEATS'`, game pin
`b69585eee640ba3ba4963bc62ca5f6ec3db189d7`) found no use of
`MAX_SEATS`; `TickInput` appears only in its skills' text.

What else reads the seat count, checked on this commit:
`crates/core/src/tick/tests.rs` (line 58, the truncation test) and the
doc comments of `tick.rs`. `crates/core/src/toy.rs`'s `[_; 4]` is its
four directions, not seats. `crates/ui/src/tick.rs`'s `TickDriver`
builds seat 0 alone (`TickInput::one`). `TickRecord`'s runs hold a
`Vec` of seats, so existing records (`examples/snake/tests/records/`,
`examples/hello-grid/tests/records/`) don't depend on the count.

## Nick input

None.

## Scope

**In:**
- `MAX_SEATS` becomes 8, its doc says why ("eight riders in a room: a
  couch of four, or a room of humans and bots online"), and every doc
  comment that says four is fixed.
- **A record never holds more seats than `MAX_SEATS`.** `TickInput`'s
  `Deserialize` bypasses `new` today, so a RON record with nine seats
  loads. Make it refuse one (`#[serde(try_from = …)]` over a private
  shadow struct, or a check in `TickRecord`'s load), with an error that
  names the limit. Write a `Behaviour:` line for it.
- **The byte form**, in `crates/core/src/tick/bytes.rs` (a module of
  `tick`), `no_std` + `alloc`, no serde, no floats:
  - `SeatInput::encode(&self, out: &mut Vec<u8>) -> Result<(),
    TooManyActions>`: one header byte whose bits 0, 1 and 2 say whether
    `pressed`, `held` and `released` are non-empty (the other bits 0),
    then for each non-empty list in that order a `u8` count and the
    `ActionId`s as `u16` little-endian. A list of more than 255 actions
    is `Err(TooManyActions)` and writes nothing.
  - `SeatInput::decode(bytes: &[u8]) -> Option<(SeatInput, usize)>`:
    the seat and how many bytes it read. `None` on a short input, a set
    reserved bit, or a list whose count is 0 (the header says it is
    empty: one input has one form).
  - `TickInput::encode(&self, out: &mut Vec<u8>) -> Result<(),
    TooManyActions>`: a `u8` seat count, then each seat.
    `TickInput::decode(bytes: &[u8]) -> Option<(TickInput, usize)>`,
    `None` on a count over `MAX_SEATS` or a bad seat.
  - `TooManyActions` is a public error type with `Display` and
    `core::error::Error`, its fields private (ADR-0039), so it can later
    say which list was too long without a break.
  - An idle step of eight seats is 9 bytes; one turn press adds 3. This
    is what keeps 2614's `Step` frame near 12 bytes of payload.
- Doc examples for both, and the module's doc names its one user so
  far (2614's wire).
- `Book:` and `Skills:` lines in the completion notes: the byte form is
  taught by 2854's chapter and 2855's skill, which come later.

**Out (do not do):**
- Any network code: the transport (2613), the room and its wire (2614).
- Changing what a seat holds (axes and the pointer are 1121's).
- A seat count chosen per game (a const generic on `TickInput`): it
  would spread a parameter through `TickSim`, `TickRecord` and
  `TickDriver` for no game that needs more than eight. If considered,
  say so in the completion notes as rejected.
- A byte form of `TickRecord` (records stay RON, ADR-0007's text first).

**Estimated size:** about 250 lines (code 110, tests 120, docs 20).

## Implementation steps

1. Raise `MAX_SEATS` in `crates/core/src/tick.rs`; fix its doc and
   `TickInput`'s, and the test at `tick/tests.rs:58` (it already reads
   the constant).
2. Refuse a deserialized `TickInput` over `MAX_SEATS`; check that every
   record under `examples/*/tests/records/` and the golden tables still
   load and replay unchanged.
3. Write `tick/bytes.rs`: the two `encode`s and `decode`s and
   `TooManyActions`; re-export `TooManyActions` from `tick`.
4. Tests (below), then `cargo xtask gates`.
5. Completion notes: the `Breaking:` line with its `migration` block
   (`tickets/README.md`, *Breaking changes*), the `Behaviour:` line, and
   `Book:` and `Skills:` lines.

## Acceptance criteria

- [ ] `MAX_SEATS` is 8; an 8-seat `TickInput` steps a `TickSim`, and
      `TickInput::new` with nine seats keeps the first eight, as
      documented.
- [ ] A RON record with a step of nine seats fails to load with an error
      naming the limit of eight.
- [ ] Every existing golden test and record replays bit for bit.
- [ ] `TickInput::decode(&encode(x)) == Some((x, len))` for every input
      the tests generate, and the encoded length of an idle 8-seat step
      is 9.
- [ ] `decode` never panics on any bytes, and refuses a seat count of 9,
      a reserved header bit, a zero count and a short input.
- [ ] The no_std build (`cargo xtask check-no-std` and CI's no_std leg)
      passes.
- [ ] The completion notes have a `Breaking:` line with a `migration`
      block, and a `Behaviour:` line.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`tick/bytes` tests): round trips of hand-picked inputs (no
  seats, one empty seat, eight seats, a seat with all three lists);
  exact bytes of one known input, as a fixture written by hand from the
  layout above, not pasted from the code's output.
- Property (proptest, seeded): 0–8 seats, 0–16 actions a list, round
  trip equal; random bytes up to 256 long never panic, and a decode that
  succeeds re-encodes to the bytes it read.
- Integration: an 8-seat `TickRecord` recorded over a toy sim replays to
  the same hash; the snake and Hello, Grid records still replay (their
  existing golden tests).
- Unit: a nine-seat RON record is refused.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
