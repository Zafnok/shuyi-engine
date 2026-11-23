---
id: "2608"
title: "Parked: A game fetches signed remote data at start, and, if the player agrees, sends its own analytics events to an address it sets"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2601", "2606", "3527"]
nick_input: decision
completed:
---

# 2608 — Signed remote data, and a game's own analytics events

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). It may start when Nick unparks it, after his answer on analytics in decision ticket 3527, and after 2601 (HTTP requests, parked), which it sends through, and 2606 (the shared consent record, parked), which the events half asks. A game that uses it from a real build also needs 2601's native and web backend tickets (written by 2601's step 2). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-11-23 blind-spot audit of the engine (networking and
online services: "no live-ops layer"). Other engines give a shipped game
two small online parts:

- **Remote data:** Unity has Remote Config; Godot and Defold games add a
  plugin or an extension. A game changes a balance number, turns on a
  weekend event, shows a message on its title screen or says "a new
  version is out", without a patch.
- **A game's own analytics:** Unity has Analytics; Defold games use
  GameAnalytics through an extension. A game learns where players stop
  (session length, how far players get, where they quit).

What Shuyi has and plans (checked 2026-11-23):

- **Nothing for either.** `grep -rliE 'remote config|feature flag'`
  over `tickets/` and `docs/adr/` finds only unrelated tickets; `grep
  -rni analytics tickets/open` finds 3047's Out line ("Analytics or
  telemetry of any kind") and the website's tickets. No ticket plans a
  `RemoteData` or a `GameEvents` (`grep -rnw` in `tickets/open`).
- **2601 (parked)** makes HTTP requests a platform capability: a screen
  or `App` queues an `HttpRequest` in `Ctx::http` (an `HttpQueue`), and
  `App`'s `step` (`crates/platform/src/app.rs`) brings the `HttpReply`
  back at a later frame. Every request names a `purpose`; `ScriptedHttp`
  scripts replies on `HeadlessPlatform`. Its ADR says a reply reaches a
  game's rules only as a command a screen issues (or a tick game's input
  for a step), and that the engine never sends anything by itself. Its
  Out leaves "a game's remote settings" to a later ticket, because
  content that changes after release changes what a record replays:
  this is that ticket.
- **3047 (parked)** is the pattern for sending something about the
  player: `Start::send_crashes_to(url)`, nothing sent without the
  player's answer, three tries at most, a report kept in the computer's
  storage until it is sent, and a `docs/crash-reports.md` section for
  the receiving side.
- **2606 (parked, this batch)** gives online features one consent record
  per purpose, with an age gate. The events half asks it, instead of
  adding a setting of its own as 3047's `CrashSending` does.
- **A record replays what the game was handed.** `SessionRecord`
  (`crates/platform/src/session.rs:60`) copies every key and value of the
  storage the platform hands over (`storage`, filled by `handed_over` at
  line 484). So data the game reads from that storage at start is in the
  record, and replays without the network.
- **Signing.** `Cargo.lock` has `sha2` (through other crates) and no
  signature crate. `deny.toml` allows BSD-3-Clause, MIT, Apache-2.0 and
  ISC.
- **The game:** Visions of Shuyi doesn't ask for either. Searched at its
  pin `50472bbcd3084555a61728a5d07518f720bd6514`:
  `gh search code --repo Zafnok/visions-of-shuyi 'analytics'` and
  `'remote config'` find nothing. Nothing here changes it.

Why it matters: without an engine path each game writes its own fetch,
cache and consent code, and a web game can't at all (2601). A static
signed file on free hosting costs nothing, which fits ADR-0002 §8's
no-cost rule.

## Nick input

**Decision** (asked in 3527, recorded with the `ask-nick` skill in
`docs/decisions.md`): should the engine offer games an opt-in,
consent-gated channel for their own analytics events at all?
**Recommended:** yes, off unless the game sets an address and the
player agrees; the engine itself never fills it. If Nick says no, the
events half (steps 6–8 and their criteria) is dropped, 2606 leaves
`blocked_by`, and this ticket is remote data only.

Unparking is Nick's (the note above). A game that sends events takes on
the privacy duties for them; the engine's docs give a paragraph to
adapt, which is not legal advice.

## Scope

**In — remote data:**
- **An ADR** (the next free number, `write-adr` skill), extending
  2601's ADR (accepted ADRs are not edited): a game may fetch one signed
  data file at start; the copy the game reads is the last verified copy
  in storage, so a record replays it; a fresh copy reaches a running
  game only as a command a screen issues; the engine checks the
  signature and never sends anything else.
- **`shuyi_platform::remote_data`** (`crates/platform/src/remote_data.rs`),
  behind a new `remote-data` feature of `shuyi-platform`, off by
  default:
  - **The signed file** (`RemoteFile`), RON: `(version: 1, serial: u64,
    body: String, signature: String)`. `body` is the game's own text (a
    RON or JSON file it decodes with `Formats`, ADR-0007); `signature`
    is an Ed25519 signature, in base64, over the bytes
    `b"shuyi-remote-data 1\n"`, then `serial` as 8 little-endian bytes,
    then `body`. A body is 64 KiB at most.
  - **The check:** `RemoteFile::verify(&self, key: &PublicKey) ->
    Result<(), RemoteDataError>`. A file passes only if its signature
    checks against the game's key and its `serial` is greater than or
    equal to the cached copy's, so an old signed file can't be served
    again to roll a change back.
  - **The game's side:** `Start::remote_data(url: &'static str, key:
    [u8; 32])`, a builder beside 3046's `Start::crash_record`. `Setup`
    gains `remote_data: Option<RemoteCopy>` (`serial` and `body`), the
    cached copy read from the handed-over storage under the key
    `remote-data`, `None` before the first fetch or when it fails its
    check. At the running game's first frame `App` sends one GET through
    `Ctx::http`, `purpose` `"remote data"`. A file that passes is
    written to storage for the next start, and offered once to the
    running game: `Ctx::take_remote_update() -> Option<RemoteCopy>`. A
    screen that wants it now turns it into a command; otherwise it
    applies at the next start. A reply that fails (a status not `2xx`, a
    network error, a bad signature, an older serial) gives one `Warn`
    line in `App`'s log (3040), category `Platform`, and keeps the
    cached copy.
  - A frame never waits: the cached copy is ready at load, and the fetch
    answers at a later frame. Without HTTP on the platform nothing is
    fetched and the cached copy stays.
  - **Crates:** `ed25519-dalek` 2 (BSD-3-Clause), `default-features =
    false`, verify only, which brings `curve25519-dalek` (BSD-3-Clause),
    `ed25519` and `signature` (MIT OR Apache-2.0), `subtle`
    (BSD-3-Clause) and `sha2` (MIT OR Apache-2.0); and `base64`
    (MIT OR Apache-2.0) with `alloc` only. All optional, under the
    feature, `no_std`. Use the versions current when this starts, check
    each license with `cargo deny check` with the feature on, and list
    every new crate in the PR. Each joins `shuyi-platform`'s `outside`
    list in `tools/layers.ron`, with a comment.
- **The signing tools** in `shuyi-cli`: `shuyi remote-data keygen
  <secret-file>` writes a new key pair (the secret file, and the public
  key printed as the 32 bytes to paste into `Start::remote_data`), and
  `shuyi remote-data sign <body-file> --secret <secret-file> --serial N
  -o <out>` writes the signed RON. The secret never goes in a game's
  repository: `keygen` refuses a path inside a git working tree and
  says why. Key generation uses `getrandom` (MIT OR Apache-2.0), native
  only.
- **Public types (ADR-0039):** `RemoteFile`, `RemoteCopy` and
  `RemoteDataError` are `#[non_exhaustive]` with constructors.

**In — a game's own events** (if Nick says yes in 3527):
- **`shuyi_platform::game_events`** (`crates/platform/src/game_events.rs`),
  behind a `game-events` feature, off by default:
  - `Start::send_events_to(url: &'static str)`. Without it nothing is
    ever queued or sent. A URL that fails 2601's URL rule gives one
    `Warn` line and nothing is sent.
  - `Ctx::record_event(name: &'static str, fields: &[(&'static str,
    EventValue)])`, `EventValue` an `i64`, a `bool` or a short string
    (64 bytes at most). The engine never calls it itself.
  - **Consent:** through 2606's consent record. The game declares a
    purpose `analytics` with 2606's `Start::purposes`, and an event is
    queued and sent only while `Consent::allows("analytics")` allows it.
    The batch goes through 2601's queue with that purpose, so 2606's
    gate on `HttpQueue::send` refuses it too. A player who hasn't said
    yes, or is under 2606's age gate, records nothing: the events are
    dropped, not kept for later. Saying no later clears what waits. A
    game that never declares the purpose sends nothing.
  - **The queue:** events wait in the computer's storage under the key
    `game-events`, 256 at most and 64 KiB at most; past either, the
    oldest are dropped and a counter says how many. A batch is sent at
    the running game's first frame and whenever 50 wait: one POST of
    JSON (`serde_json`, `default-features = false`, `alloc`; already in
    `Cargo.lock`), `purpose` `"analytics"`. A `2xx` clears the batch;
    anything else keeps it for the next try, three tries at most per
    batch, then it is dropped.
  - **What a batch holds:** the events with their frame numbers, the
    game's `Made` stamp (engine and game versions) and the target id. No
    player id, no machine id, no wall time, no header but the content
    type. A game that wants a session id sends one as a field.
- **Docs:** `docs/remote-data.md`, a new page in the docs map
  (`docs/README.md`, the `document` skill): signing and hosting a file
  (a static host is enough), the serial rule, what a replay sees, and
  the events' JSON with an example body, what a receiver does (accept
  the POST, answer `2xx`, allow the web origin through CORS), and the
  paragraph to adapt for a privacy policy.
- **What changes for the game:** additions only, behind features that
  are off. `Start` gains builders, `Setup` a field, `Ctx` methods. Say
  so in the PR.

**Out (do not do):**
- An HTTP client of its own: 2601 and its backend tickets.
- A server, a hosted service or a vendor's SDK (GameAnalytics, Unity
  and others). The engine ships no receiver.
- Fetched assets as a bundle layer: not this ticket. 2207 already loads
  layers handed in at run time, and a game may hand it bytes it fetched.
- Expiry by date: `Platform` gives a timing clock (`PerfClock`), not a
  date, and the serial rule covers rolling back.
- Crash reports: 3047.
- More than one remote file per game, and a file past 64 KiB.
- The Playdate and console backends.

**Estimated size:** about 1,600 lines (code 750, tests 650, docs 200).

**Split:** past 900 lines, split off the events half (steps 6–8, their
criteria and tests) into a new parked ticket, blocked by this one, 2606
and 3527.

## Implementation steps

1. Check that 3527 records Nick's answer on analytics, and that 2601,
   its two backend tickets and 2606 are in `tickets/done/`. If the answer
   is no, drop the events half from this file first.
2. Read 2601's `crates/ui/src/http.rs` and the HTTP part of `step`;
   2606's consent record; 3046's `Start::crash_record` and `Setup` in
   `crates/platform/src/app.rs`; `handed_over` in
   `crates/platform/src/session.rs`. Confirm the handed-over storage is
   the one `App` writes `remote-data` to; if a later write isn't seen by
   the next start's handover, say so in the ADR and fix it here.
3. Write the ADR and add its row to `docs/adr/README.md`.
4. Write `remote_data.rs`: `RemoteFile`, its RON, `verify`, the serial
   rule, `RemoteCopy`, `Start::remote_data`, `Setup::remote_data`, the
   fetch in `App`, `Ctx::take_remote_update`, with unit tests and `App`
   tests on `HeadlessPlatform` and `ScriptedHttp`.
5. Write `shuyi remote-data keygen` and `sign`, with tests that a file
   it signs passes `verify` and a changed byte fails.
6. Write `game_events.rs`: `EventValue`, `Ctx::record_event`, the
   consent check through 2606, the stored queue and its limits.
7. Write the batch, its JSON and the sending in `App`, with `App` tests.
8. Write `docs/remote-data.md` and its row in the docs map; add the
   features to the `shuyi` crate's catalog (`docs/modules.md`, 2003)
   if `cargo xtask check-modules` asks.
9. Run the gates, `cargo xtask check-no-std` and the `layers` job with
   both features on.

## Acceptance criteria

- [ ] A file signed by `shuyi remote-data sign` passes `verify`; a
      changed body, a changed serial or another key's signature fails.
- [ ] `App` tests on `HeadlessPlatform` with `ScriptedHttp`: without
      `Start::remote_data`, no request; with it, one GET at the running
      game's first frame with `purpose` `"remote data"`.
- [ ] A passing reply is written to storage, offered once by
      `take_remote_update`, and is `Setup::remote_data` at a second
      start (a new `App` given the first one's storage).
- [ ] A file with a serial below the cached one, a `500`, a `Network`
      failure and a bad signature each keep the cached copy and give one
      `Warn` line, category `Platform`.
- [ ] A session record of a start that read a cached copy replays with
      `ScriptedHttp` absent and the same frame hashes.
- [ ] `keygen` refuses a path inside a git working tree.
- [ ] Events: nothing is queued without `Start::send_events_to`, without
      the player's yes in 2606's record (`Consent::allows("analytics")`),
      without the declared `analytics` purpose, or under its age gate; a
      no after a yes clears the queue.
- [ ] 50 events send one POST with `purpose` `"analytics"`, whose JSON
      keys match `docs/remote-data.md`'s list and hold no player id or
      machine id; a `2xx` clears it; three failures drop it.
- [ ] The queue keeps 256 events at most, the newest, and counts the
      dropped.
- [ ] `cargo deny check` passes with both features in its graph, and a
      game built without them gains no crate (`cargo tree -p
      shuyi-platform` shows none of the new crates).
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-platform`): `RemoteFile` read and written; `verify`'s
  cases; the serial rule; the queue's limits and dropped count; the
  batch's JSON keys; `EventValue`'s string limit.
- Unit (`shuyi-cli`): `keygen` and `sign` round trip; the refused path.
- Integration (`App` on `HeadlessPlatform` with `ScriptedHttp`): every
  case in the acceptance criteria, a second start included, and the
  replay of a record that read a cached copy.
- wasm32: `verify`'s unit tests also run on `wasm32-unknown-unknown`,
  as CI's `determinism` job runs the golden tables, so a signature
  checks the same in the web build.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
