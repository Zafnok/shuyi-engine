---
id: "2606"
title: "Parked: Online features share one consent record per purpose with an age gate, and services keep game versions apart during a rollout"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3527", "2601", "1140"]
nick_input: none
completed:
---

# 2606 — One consent record per purpose, and builds kept apart during a rollout

> **Parked** (the blind-spot audit of 2026-11-23; networking is parked by Nick's word of 2026-10-14, "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): it may start when Nick unparks it, through his answers in 3527 (platform and service scope: web HTTP, analytics and the online services), after 2601 (HTTP requests, parked), whose `purpose` it keys on, and after 1140 (the `Transport` trait and the lockstep handshake, parked), whose `Made` check it extends. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the engine blind-spot audit of 2026-11-23 (its findings 16
and 17). Both are about online features, and both are cheapest to
design in before any protocol or service is deployed.

**Consent is designed once per feature today.** Checked on 2026-11-23:

- 3047 (crash reports, parked) designs its own answer, `CrashSending`
  (`Ask`, `Always`, `Never`), and says in its Out list: "Consent for
  other data: accounts (1148) and chat (1149) have their own".
- 1148 (accounts) brings the first stored personal data and its own
  privacy policy. 1149 (chat) lists COPPA for mixed audiences among its
  duties. 2910 (voice, research) lists voice for players under 13 or 16
  (COPPA, the GDPR's age of consent, the UK's Age Appropriate Design
  Code).
- 2601 (HTTP) tags every request with a `purpose: &'static str`, which
  the log shows and a game's privacy notice lists (its ADR, rule 7).
  Nothing stores the player's answer per purpose, and nothing refuses a
  request for a purpose the player said no to.
- `grep -rni "age gate\|child mode" tickets/open` finds nothing. No
  ticket plans one switch that turns chat, voice, accounts and analytics
  off together for a player under the age of consent.

Four or five consent flows drift apart, and that is a legal risk for a
game sold in the EU, the UK or to mixed ages. Steam and the consoles
also expect one place where a player sees and withdraws consent.

**Nothing keeps versions apart during a rollout.** Checked on
2026-11-23 by `grep -n -i "version\|fingerprint\|refus"` over 1140,
1141, 1145, 1146 and 1148:

- 1140's lockstep handshake compares each peer's `Made` with
  `Made::check` (`crates/core/src/made.rs:134`), so a refusal names
  what differs. That is right between two peers.
- 1141's relay protocol has no version of its own. A relay upgraded
  under connected players can't tell an old client from a broken one.
- 1148's matchmaking queue pairs by rating only. After a patch it pairs
  an old build with a new one, and 1140's handshake then refuses the
  match. Builds come up in 1148 only for refusing a `TickRecord` of
  another build.
- 1146's `shuyi-server` isn't said to run more than one build at once.
- Nothing tells a player "an update is required"; they see a refusal.

`Made` is `Serialize` and `#[non_exhaustive]`; `Mismatch` and `Differs`
are not `Serialize` (`crates/core/src/made.rs:188-207`), so a service
sends its accepted `Made`s and the client computes the `Mismatch`
itself.

ADR-0006 (networking in layers) and 2601's ADR are the ones this
extends. The game builds none of this: every ticket named here is
parked, and the game's 0053 has not picked an online mode.

**Since Nick's decision of 2026-11-25** (`docs/decisions.md`, the
online example trails, ADR-0153), the first version handshake exists
before this ticket: 2615's quick-play director opens every connection
with `Hello { protocol, made }`, checks `protocol` against 2614's
`room::wire::PROTOCOL` and `made` with `Made::check`, and answers
`Refused { why: Refusal::OtherBuild { theirs } }` or
`Refusal::Protocol { ours }` (`Refusal` is `#[non_exhaustive]`, in
`shuyi-net`). This ticket generalizes it: `RELAY_PROTOCOL` and the
other services' versions reuse `room::wire::PROTOCOL` (one protocol
number for every `shuyi-net` wire), never a second number, and
`Refused` takes in 2615's `Refusal` rather than standing beside it.
Trails needs no age gate and no consent record: it has no chat, no
names and no accounts, and its server logs no addresses (2616).

## Nick input

None for this ticket. 3527 asks Nick whether web HTTP and the online
services are unparked; this ticket starts only after his answer. A game
that ships consent screens still has its own privacy policy and legal
review; the engine's docs say so and give no legal advice.

## Scope

**In:**
- **The consent record** (`shuyi_ui::consent`, `crates/ui/src/consent.rs`,
  `no_std` + `alloc` through `crate::prelude`):
  - `Purpose`, declared by the game: `Purpose::new(id: &'static str,
    label_key: &'static str)`, with `.adults_only()` for a purpose a
    player under the age of consent never gets (chat, voice, accounts,
    and `analytics` for 2608's game events are the docs' examples; the
    game decides). `id` is the
    same string 2601's requests carry as `purpose`;
  - `Answer`: `NotAsked` (the default), `Yes`, `No`;
  - `AgeAnswer`: `NotAsked` (the default), `Under`, `AtLeast`. The
    engine keeps only which side of the game's line the player is on,
    never a birth date or an age;
  - `Consent`: the answers by purpose id, the `AgeAnswer`, and
    `allows(purpose) -> bool`: true only when the purpose is declared,
    its answer is `Yes`, and it isn't `adults_only` while the age gate
    is set and the answer is not `AtLeast`. A purpose the game never
    declared is never allowed;
  - the saved form (RON) under the storage key `consent`, written by
    `App` itself each time an answer changes, never through
    `GameSettings::to_ron`, so a game can't forget to save it.
    `Settings::restored()` ("Restore defaults") leaves it alone, as it
    leaves the layout: an answer is not a preference with a default.
    An unreadable file reads as every answer `NotAsked`, with one `Warn`
    line in `App`'s log (category `Platform`);
  - public types (ADR-0039): `Purpose`, `Answer`, `AgeAnswer` and
    `Consent` are `#[non_exhaustive]` with constructors; additions only.
- **The game's declaration:** `Start::purposes(&'static [Purpose])` and
  `Start::age_gate(years: u8)`, builders beside 3046's
  `Start::crash_record`. Without `age_gate`, `AgeAnswer` is never asked
  and `adults_only` changes nothing.
- **The screens' side:** `Ctx::consent() -> &Consent`,
  `Ctx::answer(purpose_id, Answer)` and `Ctx::answer_age(AgeAnswer)`.
  An answer `Under` turns every `adults_only` answer to `No` in the
  same frame.
- **The gate on HTTP:** 2601's `HttpQueue::send` refuses a request
  whose `purpose` `Consent::allows` doesn't allow, with a new
  `HttpError::NoConsent { purpose }` (2601's enum is
  `#[non_exhaustive]`, so the variant is additive). `App` hands the
  queue the allowed purposes before each frame, as it hands it its
  presence.
- **The ready-made rows and screen** (`shuyi-ui`):
  `OptionRow::consent(purpose_id)`, a `RowValue::Choice` row of `Yes`
  and `No` labelled by the purpose's `label_key`, made as
  `OptionRow::text_speed` is (`crates/ui/src/screens/options.rs:279`);
  and `screens::age_gate`, a neutral screen of two choices whose words
  are the game's keys (`AGENTS.md` rule 4), asked once at first start
  when `age_gate` is set. Both use `UiAction`s only; `check-keys` and
  `check-text` pass.
- **Builds kept apart** (`shuyi-net`, which 2613 creates):
  - `Refused`, `#[non_exhaustive]`, `Serialize`: `UpdateRequired {
    accepted: Vec<Made> }` (a service accepts none of this build's
    stamps), `OtherBuild { theirs: Made }` (a peer runs another build;
    neither side knows which is newer), `Protocol { ours: u16, theirs:
    u16 }` and `NoConsent { purpose: String }`. The client turns
    `UpdateRequired` and `OtherBuild` into a `Mismatch` with
    `Made::check`;
  - 1140's lockstep handshake refuses with `Refused::OtherBuild`;
  - the relay's handshake (1141) opens with a protocol version,
    `RELAY_PROTOCOL: u16` (the value of `room::wire::PROTOCOL`, not a
    number of its own), and a room keeps its creator's `Made`: a
    join from another build is refused with `OtherBuild` before
    anything is forwarded, and a client of another protocol with
    `Protocol`;
  - a service's accepted builds: `Accepts`, a list of `Made` the relay,
    the referee and the queue are started with, so a rollout runs the
    old and the new build side by side for as long as the operator
    keeps both on the list; a client on none of them gets
    `UpdateRequired`;
  - the queue (1148) pools players by their `Made`, so it never pairs
    two builds;
  - the referee (1146): one `shuyi-server` process per build, each
    started with its `Accepts`, and the queue hands a match to the
    process of its build. Documented, not a multi-build server;
  - the lobby screen (1145) shows `UpdateRequired` and `OtherBuild`
    with keys the game names.
- **The gate on sessions:** a session backend (1145) is opened with a
  purpose id, as a request is, and refuses with `Refused::NoConsent`
  when `Consent::allows` doesn't allow it.
- **An ADR** (the next free number, the `write-adr` skill), extending
  ADR-0006 and 2601's ADR (accepted ADRs are not edited): one consent
  record per player, keyed by purpose; what the age gate stores and
  what it turns off; every online feature asks it; services accept a
  list of builds and refuse the rest by a typed reason.
- **Docs**, where the docs map (`docs/README.md`, the `document`
  skill) places networking and privacy: declaring purposes, the age
  gate, a paragraph a game adapts for its privacy notice (not legal
  advice), and a rollout runbook (run both builds, keep both on
  `Accepts`, drop the old one when its queue is empty).
- `tools/layers.ron` needs no change: `shuyi-ui` and `shuyi-net` gain
  no outside crate.
- **What changes for the game:** additions only. The game builds none
  of this today (every ticket named is parked).

**Either order with 3047:** if 3047 is done, its sender also checks
`Consent::allows("crash report")`, and its `CrashSending::Always` and
`Never` set that purpose's answer to `Yes` and `No`; `Ask` stays its
per-report question. If not, nothing to do (3047 reads and writes the
`"crash report"` purpose of this record).

**Either order with 1141:** if 1141 is done, add `RELAY_PROTOCOL`, the
room's `Made`, `Accepts` and the three refusals to its relay and
`RelaySession`, with tests. If not, nothing to do (1141 builds them
from this ticket's types).

**Either order with 1145:** if 1145 is done, its `SessionBackend` gains
the purpose id at opening and its lobby screen shows the refusals. If
not, nothing to do (1145 builds both).

**Either order with 1146:** if 1146 is done, `shuyi-server` takes its
`Accepts` at start and its docs gain the one-process-per-build rule. If
not, nothing to do (1146 builds it).

**Either order with 1148:** if 1148 is done, its queue pools by `Made`
and takes its `Accepts`, and accounts declare their purpose
`adults_only` in its example. If not, nothing to do (1148 builds it).

**Either order with 2910:** if 2910 is done, read its ADR and give voice
its purpose, `adults_only`, in the docs' examples. If not, nothing to do
(2910's research names this record as voice's consent).

**Either order with 2605:** if 2605 is done, put the relay's protocol
version into its UDP connect handshake too, refusing an old one as the
WebSocket does. If not, nothing to do (2605 carries the version into
its UDP handshake).

**Either order with 2920:** if 2920 is done, let the consent record
take the platform online check and any age rule its console online
model names. If not, nothing to do (2920 names what it needs and adds a
line here).

**Out (do not do):**
- A privacy policy, terms of service or legal advice: each game's own.
- A server-side record of consent with dates (a service that must prove
  consent keeps its own).
- Asking for or storing a birth date or an age.
- Analytics or remote data: 2608.
- A server that hosts several builds in one process.
- Chat's own moderation: not this ticket's job.

**Estimated size:** about 1,300 lines (code 600, tests 500, docs 200),
counted from the steps and doubled.

**Split:** past 900 lines, split off steps 5–7 (builds kept apart:
`Refused`, `Accepts`, the relay protocol version and the rollout
runbook) into a new parked ticket.

## Implementation steps

1. Write the ADR.
2. Add `crates/ui/src/consent.rs`: `Purpose`, `Answer`, `AgeAnswer`,
   `Consent` and `allows`, the RON form and its reader, with unit tests.
3. Add `Start::purposes` and `Start::age_gate`; make `App` read the
   `consent` key at start and write it on each change; add
   `Ctx::consent`, `Ctx::answer` and `Ctx::answer_age`.
4. Gate 2601's `HttpQueue::send` with `HttpError::NoConsent`; add
   `OptionRow::consent` and `screens::age_gate`, and wire both into the
   toy (`crates/ui/src/toy.rs`) with its English keys.
5. In `shuyi-net`, add `Refused` and `Accepts`; make 1140's handshake
   refuse with `OtherBuild`.
6. Apply the clauses above for whichever of 1141, 1145, 1146 and 1148
   are done.
7. Write the docs and the rollout runbook.

## Acceptance criteria

- [ ] A request whose purpose is undeclared, `NotAsked` or `No` is
      refused with `HttpError::NoConsent`, and one answered `Yes` is
      sent (scripted replies on `HeadlessPlatform`).
- [ ] With `age_gate(16)` and `AgeAnswer::Under`, an `adults_only`
      purpose answered `Yes` before is `No` in the same frame and its
      request is refused; a purpose that isn't `adults_only` is sent.
- [ ] With an age gate set and `AgeAnswer::NotAsked`, an `adults_only`
      purpose is refused.
- [ ] The `consent` key survives a restart and "Restore defaults", and
      a game whose `GameSettings::to_ron` writes nothing still keeps it.
- [ ] An unreadable `consent` file reads as all `NotAsked` with one
      `Warn` line.
- [ ] Two peers of different `Made` are refused with `OtherBuild`, and
      the client's `Mismatch` names each differing part.
- [ ] A client whose `Made` is on no service's `Accepts` gets
      `UpdateRequired`; with two builds on `Accepts`, each is served and
      never paired with the other.
- [ ] A relay client of another `RELAY_PROTOCOL` is refused with
      `Protocol` (when 1141 is done).
- [ ] The toy's options show one consent row per declared purpose, and
      `check-keys` and `check-text` pass.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Consent::allows` over every `Answer`, `AgeAnswer` and
  `adults_only` combination; the RON form round-trips; an unknown
  purpose id in the file is kept, not dropped.
- Property: for any sequence of answers, `allows` is false for every
  `adults_only` purpose after `Under`, and the saved form reads back
  equal.
- Snapshot / integration: the toy's options screen with two purposes
  and the age-gate screen (scene snapshots); an HTTP request refused
  and one sent on `HeadlessPlatform`; two `MemoryTransport` peers of
  different `Made` refused at the handshake; a queue with two builds
  on `Accepts` pairing only within a build.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
