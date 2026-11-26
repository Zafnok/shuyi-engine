---
id: "2613"
title: "shuyi-net: the Transport trait and an in-memory network, split out of 1140, and ADR-0152 for real-time rooms"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-26
---

# 2613 — shuyi-net: Transport, an in-memory network, and ADR-0152

**Split on 2026-11-26:** at 1,936 counted lines this ticket's PR was
split. The seeded faults (`Faults`, their draws, and the tests and
acceptance cases of loss, duplicates, reordering and delay) are 2620's;
this ticket keeps the crate, `Transport` and its types, a
`MemoryNetwork` that delivers at the next step in send order, the
module duties, the CI line and ADR-0152. The scope below is as
written; the Out list and the struck criteria say what moved.

## Context

- Nick's decision of 2026-11-25 (`docs/decisions.md`) makes the next
  example game, trails, an online light-trail arena: rooms on a server,
  bots in empty seats, drop-in and drop-out, quick play. It unparks the
  networking work that example needs.
- 1140 (still parked) planned the `Transport` trait and `MemoryNetwork`
  together with lockstep over `TurnRules`. Trails needs the first two and
  not lockstep. This ticket takes the transport half; 1140 is amended to
  keep only lockstep and commit-reveal, built on this crate.
- It also writes ADR-0152, the architecture of the whole set
  (2613–2619, 2788–2791, 2854–2856), and amends ADR-0006, whose §6 left
  real time in the browser to 1147's research.
- What exists today, checked on 2026-11-25:
  - no `crates/net`; `ls crates` lists `alloc-count app assets cli core
    embed embed-macros platform playdate script shuyi sound sound-out ui
    xtask`;
  - `shuyi_core::SimRng` (`crates/core/src/rng.rs:218`, `new`, `stream`,
    `fork`, `next_u32`, `roll_below`) gives the seeded faults;
  - `docs/modules.ron`'s concern `online` has `none` (ready) and
    `lockstep`, `rollback`, `referee` (parked, `by: "shuyi-net"`);
    `docs/modules.md` has one `shuyi-net` row, parked, 1140–1150;
  - `cargo xtask check-modules` (`crates/xtask/src/modules.rs`) fails a
    module with no `README.md`, no `[package.metadata.shuyi]`, no catalog
    row, or an answer not in `docs/modules.ron`; and a default crate
    (`DEFAULT_CRATES`, `shuyi-app` among them) that depends on a module;
  - the crate names held on crates.io (decisions of 2026-10-21,
    2026-10-22 and 2026-11-24) don't include `shuyi-net` or
    `shuyi-server`; the rule is that a name is held when its crate is
    real. 3510 (open) plans `cargo xtask check-publish --index` in CI's
    `layers` job, which fails a crate whose name isn't on crates.io.
- `grep -rnw` over `tickets/open` for the names this ticket builds:
  `Transport` and `MemoryNetwork` are planned by 1140 (moved here) and
  used by 1141, 1144, 1146, 1147, 2123, 2529, 2550, 2601, 2605, 2606,
  2720, 2910 and 2920, all parked; they now take them from this ticket.
  `PeerId` and `NetEvent` were 1140's only. `NetStats` is also the name
  of 2529's contract type, which copies this crate's `NetStats` field by
  field; the two live in different crates (`shuyi_net` and the tool
  contract), as 2529 already plans.

## Nick input

None to build it. If CI's `layers` job already runs `cargo xtask
check-publish --index` when this PR is opened (`grep -n "check-publish"
.github/workflows/ci.yml`), the new crate fails CI until its name is
held: ask Nick with the `ask-nick` skill to hold `shuyi-net` and
`shuyi-server` as 0.0.0 placeholders, as on 2026-11-24, before merging.
If it doesn't run yet, write both names in the completion notes for
Nick's next batch of names.

## Scope

**In:**
- **A new crate `crates/net`** (package `shuyi-net`), pure: no I/O, no
  clock, no threads, no floats (`#![cfg_attr(not(test),
  deny(clippy::float_arithmetic))]`, as `shuyi-core` has). It depends on
  `shuyi-core` only, and no outside crate.
- **The types**, each public and documented with an example:
  - `PeerId(u32)`, `Copy`, `Ord`, `Display`;
  - `Channel { Reliable, Unreliable }`;
  - `NetEvent { Connected(PeerId), Message { peer: PeerId, channel:
    Channel, bytes: Vec<u8> }, Disconnected { peer: PeerId, reason:
    DisconnectReason } }`, `#[non_exhaustive]`;
  - `DisconnectReason`, `#[non_exhaustive]`: `Closed` (either side asked),
    `TimedOut`, `Refused`, `Error(String)` (a backend's own words);
  - `SendError`, `#[non_exhaustive]`, with `Display` and
    `core::error::Error`: `UnknownPeer(PeerId)`, `Closed(PeerId)`,
    `TooLarge { len, max }`;
  - `pub trait Transport { fn send(&mut self, peer: PeerId, channel:
    Channel, bytes: &[u8]) -> Result<(), SendError>; fn poll(&mut self)
    -> Vec<NetEvent>; fn disconnect(&mut self, peer: PeerId); fn
    stats(&self) -> NetStats; }`. No `Send` bound: a game's transport
    lives on its main thread, and a backend with a thread of its own
    (2617) keeps the thread inside;
  - `NetStats`, `#[non_exhaustive]`, `Copy`, plain whole numbers with a
    `NetStats::new()` and getters: `bytes_out`, `bytes_in`,
    `messages_out`, `messages_in`, `dropped` (lost by a fault),
    `duplicated`.
- **`MemoryNetwork`**, the in-memory backend for tests:
  - `MemoryNetwork::new(faults: Faults)`; `endpoint(&mut self) ->
    MemoryEndpoint` hands out a new endpoint with its own `PeerId`;
    `connect(a, b)` links two endpoints (each sees `Connected` of the
    other at the next `step`); `MemoryEndpoint` implements `Transport`.
    Endpoints share the network through `Rc<RefCell<…>>` (single
    thread, as a pure crate must be); one endpoint may be linked to many
    (a server and its clients).
  - It moves on only by `MemoryNetwork::step(&mut self)`: one step
    delivers what is due. Nothing happens between steps.
  - **`Faults`**, `#[non_exhaustive]` with `Faults::none()` and `with_*`
    setters (ADR-0039), all whole numbers: `seed: u64`; `loss`,
    `duplicate` and `reorder` in per-mille (0 to 1,000); `delay_min` and
    `delay_max` in steps. `set_faults(&mut self, Faults)` replaces them
    between steps (2529 changes them mid-session). The draws come from
    one `SimRng` seeded from `seed`, in a fixed order per message, so
    the same seed and the same sends give the same deliveries on every
    target.
  - **Reliable stays ordered and lossless**, like a WebSocket over TCP:
    it is delayed (each message no earlier than the one before it on
    the same link), never lost, duplicated or reordered. Loss,
    duplicates and reordering hit `Unreliable` only.
  - **Disconnect:** `disconnect(peer)` on either end gives both ends
    `Disconnected { reason: Closed }`. The other end gets it after the
    Reliable messages sent before it, as a TCP close follows its data.
    Messages sent after it are `Err(Closed)`.
  - A `max_message` limit (default 64 KiB) gives `TooLarge`, so tests
    catch a frame no real backend would carry.
- **Module duties** (ADR-0009 §4):
  - `[package.metadata.shuyi]`: `answers = [{ concern = "online", answer
    = "server" }]`, `serves = ["fixed-ticks"]` (the arcade time of 2739;
    check the id in `docs/modules.ron`'s `time` concern).
  - `docs/modules.ron`: the answer `(id: "server", name: "a game server:
    rooms, bots in empty seats, quick play", by: "shuyi-net", status:
    planned, tickets: ["2614", "2615", "2616"])`, so `check-modules`
    passes. With two answers ready or planned, `shuyi new` starts asking
    the `online` question (`crates/cli/src/contract/new/wizard.rs`,
    `skipped`). Update the wizard's tests and snapshots that change, and
    write a `Behaviour:` line. 2855 sets it `ready` once a template can
    turn it on.
  - `docs/modules.md`: the `shuyi-net` row says "rooms on a server
    (planned); lockstep, rollback and a referee (parked)", tickets
    2613–2617 and 1140–1150.
  - `crates/net/README.md` in ADR-0009 §4's fixed order. Its cost
    section gives the WASM size it adds, measured.
- **The layers:** in `tools/layers.ron`, `"shuyi-net": (engine:
  ["shuyi-core"], outside: Only([]), devices: false, floats: false)`,
  and the front door's `"shuyi"` entry gains the edge `shuyi-net`.
- **The front door:** a feature `net = ["dep:shuyi-net"]` on `crates/shuyi`,
  **off by default**, and `#[cfg(feature = "net")] pub use shuyi_net as
  net;`. The crate adds nothing to a game that doesn't turn it on (rule
  13): no default feature reaches it.
- **wasm32:** add `-p shuyi-net` to the `cargo test --target
  wasm32-wasip1` step of CI's `determinism` job (`.github/workflows/ci.yml`,
  the line that tests `-p shuyi-core … -p shuyi-script`), so delivery
  order is checked on wasm32, where the browser client runs. (1140's
  "From 2116" line moves here.)
- **ADR-0152**, written with the `write-adr` skill. Title in the style
  of the others, for example "Real-time rooms: an authoritative server
  broadcasts each step's inputs, with bots in empty seats and quick play
  without accounts". It says:
  - **the sync model:** the server runs the real `TickSim` headless and
    decides everything (seats, which input counts at which step, bots,
    rounds and their seeds). Clients send `Input` only when their
    actions change, and `Alive` once a second. Each step the server
    sends a `Step` frame: the step number and the step's whole
    `TickInput` in 2612's byte form (about 12 B of payload), and every
    15 steps `TickSim::hash()`. Clients step their own copy of the same
    deterministic sim with exactly those inputs (ADR-0006 §2 used
    client-server). Joining, or a resync after a hash mismatch, is a
    catch-up: `RoundStart { round, seed, wins }` plus every step's
    inputs since the round began, at most `MAX_ROUND_STEPS` of them
    (about 32 KB). No game needs a state codec. Clients show confirmed
    steps behind a 1–2 step jitter buffer and don't predict;
  - **why not rollback:** rollback suits 2–4 peers with no server; this
    needs a server anyway (bots, drop-in, matchmaking); the browser has
    only WebSocket (TCP), where resend-on-loss gains little; server
    authority settles ADR-0006 §4's cheating concern; a game with no
    hidden information may show every client the whole state. A game
    with hidden information needs 1146's referee, and own-rider
    prediction stays 1147's;
  - **the bandwidth math:** per message, 36 B payload + 2 B WebSocket
    header + 22 B TLS 1.3 record + 52 B TCP/IPv4 with timestamps = 112 B;
    at 15 steps a second, 1,680 B/s a human; 80 humans around the clock
    about 348 GB a month, plus about 7 GB of catch-ups; about 35 GB at a
    realistic 10% human. Sending the whole grid each step would be about
    6.6 TB a month, so input broadcast is required. Bots cost no
    traffic;
  - **the crate split:** `shuyi-net` pure (transport trait, rooms,
    director); `shuyi-server` a tokio library (2616); the client's
    sockets in the backend `shuyi-app` behind a `net` feature with its
    own byte-level API, adapted to `Transport` in the front door, since
    a default crate may not depend on a module (ADR-0009 §4,
    `check-modules`) (2617, 1142);
  - **the server as a headless platform backend** under ADR-0008 §2: it
    touches sockets and the clock, never the rules;
  - **quick play without accounts:** no names, no chat, no room
    browser, no codes; free text from strangers would bring moderation
    duties (ADR-0006 Context and §8);
  - **what stays where:** 1141's relay, 1145's lobbies, 1146's referee,
    1147's rollback and prediction, 1148's accounts.
  - It amends ADR-0006 §1 (the Transport row's order and a
    "server-authoritative input broadcast" sync model for ticks), §5
    (systemd on a free host beside Docker images; 2618's ADR) and
    §6 (real time over WebSocket in the browser is built now, not left to
    1147). Add an "Amended by ADR-0152" line under ADR-0006's status,
    and one under ADR-0008's for the server backend.
- **Public types** follow ADR-0039: `NetEvent`, `DisconnectReason`,
  `SendError`, `Faults` and `NetStats` may grow, so they are
  `#[non_exhaustive]`, with constructors where built by a user.
  `PeerId` and `Channel` are plain values.
- The completion notes get a `Book:` line (2854 writes the online
  chapter) and a `Skills:` line (2855 rewrites `skills/shuyi-net`), for
  `cargo xtask docs-rules`.

**Out (do not do):**
- The seeded faults: `Faults`, `set_faults`, the draws from `SimRng`,
  delays, and their tests (2620, split out of this PR). Until then the
  crate depends on no engine crate (2620 adds `shuyi-core`).
- Lockstep, its handshake, commit-reveal, the messages' fuzz target and
  the `net` debug channel: 1140, built on this crate.
- Rooms, bots, frames and the wire codec: 2614.
- The director and quick play: 2615.
- Sockets: the server 2616, the native client 2617, the web client
  1142.
- `shuyi peers` and the `tools` feature: 2529.
- Rewriting `skills/shuyi-net`: 2855. Its `waits_for` keeps 1140,
  which stays open.

**Estimated size:** about 800 lines (code 350, tests 300, docs and the
ADR 150).

## Implementation steps

1. Add `crates/net` to the workspace (`crates/*` already covers it),
   with `Cargo.toml` (`version.workspace = true`, the metadata, a
   dependency on `shuyi-core` from `[workspace.dependencies]` if 3510
   has made that the rule), `src/lib.rs` with the float deny, and the
   `README.md`.
2. Add the `layers.ron` entry and the front door's edge and `net`
   feature. Run `cargo xtask check-layers` and `check-modules`.
3. Add the `online` answer to `docs/modules.ron` and update the catalog
   row; fix the wizard's tests and snapshots.
4. Write the types and the `Transport` trait, with doc examples.
5. Write `MemoryNetwork`, `MemoryEndpoint` and `Faults`, then the tests
   below.
6. Add `-p shuyi-net` to CI's wasm32 `determinism` step.
7. Write ADR-0152 with the `write-adr` skill, and the back-links in
   ADR-0006 and ADR-0008.
8. Check the crate names (Nick input), and write the `Book:`,
   `Skills:` and `Behaviour:` lines.

## Acceptance criteria

- [x] `cargo xtask check-layers` and `cargo xtask check-modules` pass
      with the new crate, and `shuyi-net` is reached by no default crate.
- [x] ~~The same `Faults` seed and the same sends give the same delivery
      order, event for event, in two runs and on wasm32.~~ (not done: moved to 2620, split out of this PR)
- [x] ~~Reliable messages are never lost, duplicated or reordered under
      any fault setting (a property test over at least 200 seeds with
      loss, duplicates and reordering each at 500 per-mille).~~ (not done: moved to 2620, split out of this PR)
- [x] ~~Over 10,000 Unreliable messages at 100 per-mille each, the
      measured loss, duplicate and reorder rates are each within 70–130
      per-mille.~~ (not done: moved to 2620, split out of this PR)
- [x] ~~Every delivery lands between `delay_min` and `delay_max` steps
      after its send (Reliable may wait longer only behind an earlier
      message).~~ (not done: moved to 2620, split out of this PR)
- [x] A disconnect gives `Disconnected { reason: Closed }` on both ends,
      after the Reliable messages sent before it; a send after it is
      `Err(Closed)`.
- [x] `send` to an unknown peer is `Err(UnknownPeer)`, and an oversized
      message is `Err(TooLarge)`; neither panics.
- [x] A game built without the front door's `net` feature has no
      `shuyi-net` in `cargo tree`.
- [x] ADR-0152 is merged, and ADR-0006 and ADR-0008 name it.
- [x] The completion notes have a `Book:` line naming 2854 and a
      `Skills:` line naming 2855.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each `SendError` path; `Connected` on `connect`; `stats()`
  counts bytes and messages both ways.
- Property: Reliable order and no loss across seeds; delays within
  bounds.
- Statistics: Unreliable loss, duplicate and reorder rates over 10,000
  messages.
- Determinism: two runs, same seed, equal event lists; the same test on
  wasm32 in CI's `determinism` job.
- Integration: one endpoint linked to three, disconnect of one while
  the others keep talking.

## Completion notes

Done on 2026-11-26.

- **The crate** `crates/net` (`shuyi-net`): `PeerId`, `Channel`,
  `NetEvent`, `DisconnectReason`, `SendError`, `NetStats` and the
  `Transport` trait (`src/transport.rs`), and `MemoryNetwork` and
  `MemoryEndpoint` (`src/memory.rs`). Pure: no I/O, no clock, no
  threads, floats denied, and no dependency at all yet. `MemoryNetwork`
  delivers what was sent at the next `step`, in send order, on both
  channels; a disconnect tells both ends at the next step, each after
  the messages sent to it before the close; a send after it is
  `Err(Closed)`; `max_message` (64 KiB) gives `TooLarge`.
- **Module duties:** the metadata (`online` = `server`, serving
  `fixed-ticks`), the `server` answer in `docs/modules.ron` (planned,
  2614–2616), the `shuyi-net` row in `docs/modules.md`, the README in
  ADR-0009 §4's order with the size measured, `tools/layers.ron`'s entry
  and the front door's edge, and the front door's `net` feature, off by
  default.
- **Tests:** 13 unit tests (`src/memory/tests.rs`: each `SendError` path,
  `Connected` on `connect` and before a message sent at once, stats both
  ways, send order on both channels, nothing between steps, a clone
  sharing its inbox, the panics of `connect`), and `tests/links.rs`: a
  server linked to three clients with one leaving while the two others
  keep talking both ways, and a disconnect after the messages sent before
  it, on both ends. They pass on wasm32-wasip1 under wasmtime too (the
  two `should_panic` tests are skipped there, as WASI can't unwind).
- **CI:** `-p shuyi-net` joins the wasm32 `cargo test` step of the
  `determinism` job, and the same line in the `run-gates` skill and in
  `tools/sound-parity/footprint.ron` (which rewrites that line).
- **ADR-0152** (real-time rooms), with "Amended by" lines in ADR-0006 and
  ADR-0008 and both index rows.
- **Size measured** on 2026-11-26: Hello, Grid's web release build with
  a live use of `MemoryNetwork` is 21,159 bytes bigger (3,915,937 →
  3,937,096). Without the `net` feature, `cargo tree -p hello-grid` and
  `cargo tree -p shuyi` have no `shuyi-net`.

**Deviations:**

- **Split: the seeded faults are 2620's.** With them the PR counted
  1,936 lines (code 950, tests 701). The faults stand apart (a
  fault-free `MemoryNetwork` already serves a test of online code), so
  they went to 2620, with their four acceptance cases (struck above).
  Their draft code and tests are on this session's local branch
  `t2620-memory-network-faults`. 1140, 2529, 2550, 2614, 2615, 2720 and
  2854 now name 2620 for the faults; 1140 and 2614 have it in
  `blocked_by` (2615 waits through 2614, 2529 through 1140).
  `shuyi-core` and the layers' `engine: ["shuyi-core"]` come with 2620.
- **The ADR's number is 0152, not 0150.** ADR-0150 (the contract's
  version) and ADR-0151 (constructors) were taken after this ticket was
  written. The net tickets' references to ADR-0150 now say ADR-0152
  (1140, 1141, 1142, 1146, 1147, 2551, 2606, 2613–2618, 2789, 2790,
  2854–2856). The hosting ADR they called ADR-0151 is now "2618's ADR",
  numbered by `cargo xtask next-id adr` when 2618 writes it (2616, 2618,
  2619, 1141, 1146, 2854–2856).
- **`NetStats` gains `record_sent`, `record_received`, `record_dropped`
  and `record_duplicated`.** A `#[non_exhaustive]` type with getters
  alone can't be filled by a backend in another crate (2616, 2617).
- **`MemoryNetwork::new()` takes nothing** (2620 adds `with_faults`), and
  it gains `set_max_message`, `max_message` and `steps()`, and `Default`.
- **`connect` takes two `PeerId`s** and panics on one not of the network
  or on a self-link. A pair is linked once: a peer that comes back is a
  new endpoint, as a new connection has a new id. A send works from
  `connect` on, before `Connected` is polled.
- **The front door's feature is `net = ["std", "dep:shuyi-net"]`**, as
  `script`'s is. 2617's line naming the feature now says so.
- **`cargo xtask api` checks the crate:** `shuyi-net` joins
  `API_CRATES`, and `shuyi`'s checked features gain `net`
  (`crates/xtask/src/api.rs` and its test's table), since `check_catalog`
  fails an engine crate left out. Its catalog tests named `shuyi-net` as
  a crate not yet listed: they name `shuyi-physics` now, and count 15
  crates (the sound give-up's footprint, 15 → 13).
- **The root `README.md`'s crate table** gains a `shuyi-net` row.
- **2617** gains a "From 2613 (done)" note: `shuyi::net` is a re-export
  of the crate today, so its `connect` needs `net` made a module that
  re-exports `shuyi_net::*`. **2550**'s clause on linking chosen pairs
  now says what holds: `connect(a, b)` links chosen pairs.
- **Crate names:** CI's `layers` job doesn't run `cargo xtask
  check-publish --index` yet, so nothing blocks the merge. For Nick's
  next batch of names to hold on crates.io: `shuyi-net` and
  `shuyi-server`.

Behaviour: shuyi-cli: `shuyi new` asks "How do players play online?"
(default `none`), since the `online` concern now has two answers ready
or planned (`none`, and `server`, planned); `shuyi modules` lists
`shuyi-net` (feature `net`) beside `shuyi-script`. A script or a test
that answers `shuyi new`'s questions in order answers one more, or passes
`--online none`.

No break: every item is new. Searches of the game (`gh search code
--repo Zafnok/visions-of-shuyi` for `shuyi_net`, `MemoryNetwork` and
`"--online"`) found nothing: no game code calls `shuyi new`'s wizard or
names the crate.

Game: nothing to adopt, the crate is behind a feature that is off by
default, and the game builds none of it.

Book: none here; 2854 writes the online chapter, over this crate and
ADR-0152.

Skills: none here; 2855 rewrites `skills/shuyi-net`, whose `waits_for`
keeps 1140.

Nick input: none needed. Two crate names for his next batch:
`shuyi-net` and `shuyi-server`.
