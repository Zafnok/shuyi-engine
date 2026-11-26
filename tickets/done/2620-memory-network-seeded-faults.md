---
id: "2620"
title: "MemoryNetwork's seeded faults: loss, duplicates and reordering of Unreliable messages, and a delay of every delivery, split out of 2613"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2613"]
nick_input: none
completed: 2026-11-26
---

# 2620 — MemoryNetwork's seeded faults

## Context

- 2613 built `shuyi-net` (`crates/net`): the `Transport` trait and
  `MemoryNetwork`, which delivers everything sent at the next `step`, in
  the order sent, on both channels. It was split there at 1,936 counted
  lines: this ticket takes the faults
  ([ADR-0155](../../docs/adr/0155-real-time-rooms-a-server-broadcasts-each-steps-inputs.md)
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

- [x] The same `Faults` seed and the same sends give the same delivery
      order, event for event, in two runs, and a session's events are
      pinned (count and hash) by a test CI's wasm32 `determinism` job
      runs (`-p shuyi-net` is on its line since 2613).
- [x] Reliable messages are never lost, duplicated or reordered under
      any fault setting (a property test over at least 200 seeds with
      loss, duplicates and reordering each at 500 per-mille).
- [x] Over 10,000 Unreliable messages at 100 per-mille each, the
      measured loss, duplicate and reorder rates are each within 70–130
      per-mille.
- [x] Every delivery lands between `delay_min` and `delay_max` steps
      after its send (Reliable may wait longer only behind an earlier
      message).
- [x] A disconnect under delays gives `Disconnected { reason: Closed }`
      on both ends, after the Reliable messages sent before it; nothing
      arrives at an end after it was told.
- [x] A chance over 1,000 counts as 1,000; a delay of 0 as 1; a
      `delay_max` below `delay_min` as `delay_min` (a case each).
- [x] `set_faults` with the same seed keeps the draws going, and with a
      new seed starts them afresh.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not (2854 writes the online chapter).
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534; 2855 rewrites `skills/shuyi-net`).
- [x] All gates in the `run-gates` skill pass.

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

**What was done:**

- **`Faults`** (`crates/net/src/faults.rs`, exported from `lib.rs`):
  `#[non_exhaustive]`, `Copy`, `Faults::none()` and `Default`, `const fn
  with_seed`, `with_loss`, `with_duplicate`, `with_reorder`,
  `with_delay_min`, `with_delay_max`, and a getter for each. Chances over
  1,000 are clamped at the setter; the getters give the delays as set,
  and the network uses them with 0 as 1 and a `delay_max` below
  `delay_min` as `delay_min`.
- **`MemoryNetwork::with_faults`**, `set_faults` and `faults`; `new()` is
  `with_faults(Faults::none())` and delivers as before. The draws come
  from one `SimRng::new(seed)`: five `roll_below` draws for each
  `Unreliable` send (loss, duplicate, reordering, delay, the copy's
  delay), one for a `Reliable` send and one for a disconnect; `connect`
  draws none, and each end is told at the next step. Deliveries are
  ordered by step, then held back last, then send order. Each direction
  of a link keeps the due step of its last `Reliable` item (a later one
  is never earlier) and whether its receiver was told the sender is
  gone (what else arrives is thrown away).
- **Tests:** unit tests of the clamps (a case each), `set_faults`' seed
  rule, the draws per send, reordering at 1,000, and `stats()`'s
  `dropped` and `duplicated` at 1,000 (`crates/net/src/memory/tests.rs`);
  `crates/net/tests/faults.rs`: the Reliable property and the delay
  bounds over 256 cases each (loss, duplicates and reordering at 500
  per mille, delays of 1 to 12 steps, random send scripts), the
  Unreliable rates over 10,000 messages for three seeds (measured 95 to
  109 per mille), two runs of a session equal, and that session pinned
  (206 events and their hash), which CI's wasm32 `determinism` job runs;
  `crates/net/tests/links.rs`: a disconnect under delays, loss,
  duplicates and reordering over 200 seeds, and the closing end told at
  the next step under a delay of 5. They pass on wasm32-wasip1 under
  wasmtime too, the pinned session with them (the two `should_panic`
  tests are skipped there, as WASI can't unwind).
- **Docs:** the crate's rustdoc (a faults example, checked as a
  doctest), `MemoryNetwork`'s *Faults* section, the README's opening,
  cost and how-to-start sections, and the crate's line in
  `site/data/api.toml`.
- **Size** measured on 2026-11-26, Hello, Grid's web release build
  (`cargo build -p hello-grid --release --target wasm32-unknown-unknown`)
  with a live `MemoryNetwork` using faults, against without the `net`
  feature: **+29,301 bytes** (3,928,020 → 3,957,321). The same use of
  2613's fault-free network is 3,939,257, so the faults add 18,064, and
  `MemoryNetwork::new()` builds them too (3,957,193). The README says
  so.

**Deviations:**

- **Size:** 1,100 counted lines against the 800 estimated: the tests
  are 564 of them (the draft's property, statistics and pinned session,
  and 2613's unit tests kept). It would split cleanly between the code
  with its unit tests and `tests/faults.rs`; under 1,500 it stays one.

- `crates/net/Cargo.toml` takes `shuyi-core = { workspace = true }`, not
  the ticket's `path` line: `cargo xtask check-publish` wants engine
  crates from `[workspace.dependencies]` since 3510, and the workspace's
  entry is already `default-features = false`.
- The draft on `t2620-memory-network-faults` was taken. It replaced
  `new()` with `new(faults)` and dropped five of 2613's unit tests; here
  `new()` stays and every test of 2613 is kept.

Behaviour: none. `MemoryNetwork::new()` delivers as it did, everything
at the next step, in the order sent.

No break: every item is new. Searches of the game (`gh search code
--repo Zafnok/visions-of-shuyi` for `MemoryNetwork` and `shuyi_net`)
found nothing.

Game: nothing to adopt, the crate is behind the front door's `net`
feature, off by default, and the game builds none of it.

Book: none, 2854 writes the online chapter, over this crate.

Skills: none, no schema, template, format or problem code changed; 2855
rewrites `skills/shuyi-net`.
