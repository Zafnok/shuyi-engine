---
id: "1148"
title: "Parked: Networking, step 9: accounts, a matchmaking queue, Glicko-2 ratings and verified results"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["1146", "3054"]
nick_input: setup
completed:
---

# 1148 — Accounts, matchmaking, ratings, verified results

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0006 §1, §4, §7. Ranked play needs four things:

- accounts, to know who is who;
- a queue that pairs players of similar rating;
- a rating system;
- results nobody can fake. The referee (1146) records them, or a checker
  re-simulates them.

Glicko-2 is public domain and is implemented by `skillratings`
(MIT/Apache, no dependencies). Never TrueSkill: it is patented
(estimated expiry 2029) and trademarked.

This brings the first stored personal data: account ids, names and match
history. That means a privacy policy and a decision on where the data
lives.

## Nick input

**Setup**, before anything runs publicly. Nothing here is turned on in
our game without 0053.

- Choose where the database runs: a small VPS with Postgres, or a managed
  Postgres. The agent lists the prices on the day.
- Approve a privacy policy and terms of service written for this. A
  lawyer's look is recommended, as for `LICENSE`.

## Scope

**In:**
- `shuyi-services` is a server crate: tokio, Postgres through `sqlx`
  with `default-features = false` and rustls with ring, its root
  certificates from the system's store (sqlx 0.8's
  `tls-rustls-ring-native-roots` feature; check the name in the version
  used), never `webpki-roots`, whose CDLA-Permissive-2.0 is not on
  `deny.toml`'s allow list (2601 has the same rule), checked with
  `cargo deny`.
  - **Accounts.** A device id, or a Steam auth ticket when the `steam`
    feature is on. The client asks for the ticket through the one Steam
    client (the `steam` module in `shuyi-app`, 3037 or 1144) and never
    starts Steam itself. No passwords. A display name with the word
    filter, `rustrict` (MIT/Apache), which this ticket adds and 1149's
    chat reuses.
    **Either order with 1144 and 3037:** if either is done, the client
    asks its `steam` module's handle for the auth ticket, and
    `shuyi-services` checks it with Steam. If neither is, accounts are
    device ids only, the completion notes say so, and 1144 adds the
    Steam auth ticket (its clause on this ticket).
  - **Matchmaking queue.** Rating ranges that widen while waiting.
    Matches are handed to a referee.
  - **Ratings.** Glicko-2 via `skillratings` for one-against-one, and
    Weng-Lin for teams. Ratings update only from referee-recorded or
    re-simulated results.
  - **Leaderboards.** Read from the same verified results. A score
    submitted through 3054's `Leaderboards` counts only when the record
    it carries (`Submission::record`) re-simulates, like 1146's result
    record, to the score it claims. On the client, `shuyi-net` gets a
    backend implementing 3054's `Leaderboards` over this server.
    **Either order with 2137:** a turn game's record re-simulates with
    `History`. If 2137 is done, a tick game's record (its `TickRecord`,
    through 2137's `replay`) re-simulates too, and the tests refuse a
    forged tick score. If not, a tick game's score doesn't count yet,
    and this PR writes a parked follow-up ticket (the `write-ticket`
    skill), blocked by 2137, that adds the tick check to this server.
- A client API in `shuyi-net` for queueing, cancelling, getting matched,
  and reading a rating. It may reach the services over HTTP through
  2601's capability (`Ctx::http`), which works on native and in the web
  build once 2601's two backend tickets are done.
- Data rules, written in `docs/net-services.md`:
  - what is stored and for how long;
  - account deletion;
  - export of a player's own data (GDPR).
- Tests with a Postgres container in CI:
  - queue pairing;
  - rating updates against the published Glicko-2 worked example;
  - a forged result is refused;
  - a forged score (a record that doesn't replay to it) is refused;
  - deletion removes everything.

**Out (do not do):**
- the `Leaderboards` trait, local high-score tables and Steam's boards
  (3054);
- chat (1149);
- running a public instance;
- anything in our game.

## Implementation steps

1. Write the schema, the services, the client API and its
   `Leaderboards` backend.
2. Write the tests.
3. Write the docs and the data rules, plus a draft privacy policy for
   Nick in `docs/`.

## Acceptance criteria

- [ ] The tests above pass in CI.
- [ ] The Glicko-2 example matches the paper's numbers.
- [ ] `cargo deny check` passes.
- [ ] All gates pass.

## Tests required

- Integration: services with Postgres; ratings; forged results; a
  forged score (a record that doesn't replay to it) is refused;
  deletion.

## Completion notes
