---
id: "2620"
title: "MemoryNetwork's seeded faults: loss, duplicates and reordering of Unreliable messages, and a delay of every delivery, split out of 2613"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2613"]
nick_input: none
completed:
---

# 2620 — MemoryNetwork's seeded faults

## Context

- 2613 built `shuyi-net` (`crates/net`): the `Transport` trait and
  `MemoryNetwork`, which delivers everything sent at the next `step`, in
  the order sent, on both channels. It was split there at 1,936 counted
  lines: this ticket takes the faults
  ([ADR-0152](../../docs/adr/0152-real-time-rooms-a-server-broadcasts-each-steps-inputs.md)
  §4: "its faults draw from one `SimRng` in a fixed order per message").
- A first draft of this ticket's code and tests was written with 2613, on
  the local branch `t2620-memory-network-faults` of its session (commit
  `a55617a9`, "[2613] shuyi-net: …"). It is a draft over the old API
  (`MemoryNetwork::new(faults)`); take it if it is at hand, and change it
  to this ticket's API.
- Who needs the faults: 2614's and 2615's tests (rooms and the director
  under loss and delay), 1140's lockstep property, 2529's `shuyi peers`
  (`NetConditions` becomes `Faults`), 2550 and 2720. Each now names this
  ticket.
- `shuyi_core::SimRng` (`crates/core/src/rng.rs`: `new`, `next_u32`,
  `roll_below`) gives the draws.

## Nick input

None.

## Scope

**In:**
- **`Faults`**, `#[non_exhaustive]`, `Copy`, with `Faults::none()`,
  `Default` (= `none()`) and `const fn with_*` setters (ADR-0039; no
  constructor takes two neighbouring numbers, ADR-0151), and a getter for
  each: `seed: u64`; `loss`, `duplicate` and `reorder` in per-mille
  (`u16`, 0 to 1,000; more counts as 1,000); `delay_min` and `delay_max`
  in steps (`u32`).
  - A delay counts steps from a send to its delivery: 1 is the next
    step, the soonest there is. 0 counts as 1, and a `delay_max` below
    `delay_min` as `delay_min`. `Faults::none()` has both at 1, so it
    delivers as `MemoryNetwork::new()` does today.
- **`MemoryNetwork::with_faults(faults: Faults)`**; `new()` stays, the
  same as `with_faults(Faults::none())`. `set_faults(&mut self, Faults)`
  replaces them between steps (2529): a new seed starts the draws afresh,
  the same seed keeps drawing where it was. `faults(&self) -> Faults`.
- **The draws** come from one `SimRng::new(seed)`, at each send, in a
  fixed order: an `Unreliable` message draws its loss, its duplicate,
  its reordering, its delay and its copy's delay, five draws whatever
  comes up; a `Reliable` message and a disconnect draw one delay. So the
  same seed and the same sends give the same deliveries on every target.
- **Unreliable** messages may be lost (`dropped` on the sender's
  `NetStats`), arrive twice (the copy with its own delay; `duplicated`
  on the sender's `NetStats`), or be held behind the other deliveries of
  their step (the copy too), so later messages that arrive at that step
  come first. A delay that varies reorders them on its own too.
- **Reliable** stays ordered and lossless, as a WebSocket over TCP: each
  is delayed, but no earlier than the one before it on the same link
  (direction), and never lost, duplicated or reordered.
- **Disconnect:** the end that closes is told at the next step; the other
  end after the `Reliable` messages sent before the close, at a drawn
  delay. Once an end is told, what else arrives on that link is thrown
  away: messages still in flight from a closed peer.
- `crates/net/Cargo.toml` gains `shuyi-core = { path = "../core",
  default-features = false }` (and `proptest` as a dev-dependency);
  `tools/layers.ron`'s `"shuyi-net"` entry gets `engine: ["shuyi-core"]`.
- The README's opening, cost (measure the WASM size again, as 2613 did:
  Hello, Grid's web release with a live `MemoryNetwork` using faults,
  against without) and how-to-start sections, and the crate's rustdoc,
  show the faults.

**Out (do not do):**
- Faults on a real backend (2616, 2617): those see a real network's.
- 2529's `NetConditions` and `shuyi peers`.

**Estimated size:** about 800 lines (code 320, tests 420, docs 60): the
draft measured about 600.

## Implementation steps

1. Add `Faults` to `crates/net/src/memory.rs` (or a `faults.rs` beside
   it), exported from `lib.rs`.
2. Give `Shared` a `SimRng`, the faults, a per-direction state (the due
   step of its last `Reliable` item, and whether its receiver has been
   told the sender is gone), and order deliveries by step, then held
   back last, then send order.
3. `with_faults`, `set_faults`, `faults`; the draws in `send` and
   `disconnect`; the throw-away in `deliver`.
4. The tests below; the `Cargo.toml` and `layers.ron` changes; the
   README and rustdoc; the size measured.

## Acceptance criteria

- [ ] The same `Faults` seed and the same sends give the same delivery
      order, event for event, in two runs, and a session's events are
      pinned (count and hash) by a test CI's wasm32 `determinism` job
      runs (`-p shuyi-net` is on its line since 2613).
- [ ] Reliable messages are never lost, duplicated or reordered under
      any fault setting (a property test over at least 200 seeds with
      loss, duplicates and reordering each at 500 per-mille).
- [ ] Over 10,000 Unreliable messages at 100 per-mille each, the
      measured loss, duplicate and reorder rates are each within 70–130
      per-mille.
- [ ] Every delivery lands between `delay_min` and `delay_max` steps
      after its send (Reliable may wait longer only behind an earlier
      message).
- [ ] A disconnect under delays gives `Disconnected { reason: Closed }`
      on both ends, after the Reliable messages sent before it; nothing
      arrives at an end after it was told.
- [ ] A chance over 1,000 counts as 1,000; a delay of 0 as 1; a
      `delay_max` below `delay_min` as `delay_min` (a case each).
- [ ] `set_faults` with the same seed keeps the draws going, and with a
      new seed starts them afresh.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not (2854 writes the online chapter).
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534; 2855 rewrites `skills/shuyi-net`).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the clamps, `set_faults`' seed rule, `stats()`'s `dropped` and
  `duplicated` at chances of 1,000.
- Property: Reliable order and no loss across seeds; delays within
  bounds.
- Statistics: Unreliable loss, duplicate and reorder rates over 10,000
  messages, for a few seeds.
- Determinism: two runs, same seed, equal event lists; a pinned session,
  also on wasm32.
- Integration: a disconnect under delays and loss, over many seeds.

## Completion notes
