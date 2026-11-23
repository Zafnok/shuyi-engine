---
id: "2609"
title: "The hosted preview's action API: sessions over HTTP, one frame an action, deadlines and expiry"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2602"]
nick_input: none
completed:
---

# 2609 — The hosted preview's action API

## Context

**From 2602 (done):** the hosted preview's worker is built: `preview-worker`
in a game's tool program runs one visitor's game in a process of its own,
reads `{"action": "<name>"}` lines and answers each with the frame after it
(a PNG and its text), and `Session::for_visitor` gives the player's actions
with the developer's debug tools off
([ADR-0141](../../docs/adr/0141-the-hosted-preview-a-worker-per-visitor-actions-in-pictures-out.md)
§1–§4). This ticket is the server in front of it, ADR-0141 §5, split from
2602 for its size. It is 2602's work, unparked with it: it runs on Nick's
machine and in CI only, and nothing is deployed
([decisions](../../docs/decisions.md#2026-11-23-the-hosted-preview-2602-unparked-built-and-tested-locally-only)).

A draft of it, written and passing with 2602 before the split, is on the
branch `draft/2602-preview-server` (commit `0ea6a442`):
`crates/cli/src/preview/http.rs` and `server.rs` with their tests,
`preview.rs`'s options, usage and `main`, and an end-to-end test over HTTP
in `crates/cli/tests/it/preview.rs`. It holds 2610's quotas too. Start from
it, re-checked against `main`.

## Nick input

None.

## Scope

**In:**
- `preview [--port N] [--bind ADDRESS] [--sessions N] [--frames N]
  [--scale N] [--idle SECONDS] [--lifetime SECONDS] [--deadline SECONDS]
  [--until-stdin-closes] [the game's options]`, run in the game's tool
  program (`shuyi` runs it there, as `frame-png`), on 127.0.0.1:8000 by
  default. It checks at start that the game offers a preview.
- Each session's worker is `std::env::current_exe()` run as
  `preview-worker` in the project's root, with the server's options:
  stdin and stdout piped, reply lines read after their mark, a reply at
  most 32 MiB, the worker killed past `--deadline` and when its session
  ends.
- HTTP/1.1 of our own: one request a connection, a head of at most 8 KiB,
  a body of at most 1 KiB, a request's deadline, no chunked body, no two
  lengths, no bytes past the body; every answer `no-store`, `nosniff`,
  `no-referrer`, `Connection: close`.
- `POST /session` (a session, its id 128 bits from `getrandom`, and its
  first frame), `POST /session/<id>` with `{"action": "<name>"}`, `DELETE
  /session/<id>`; `405` for another method there, `404` for every other
  path. An answer is the frame's `session`, `png`, `text`, `actions` and
  `ended`, or `{"error": "…"}`.
- Sessions ended when idle past `--idle` or older than `--lifetime`, at
  most `--sessions` at once, and at most 64 connections served at once.

**Out (do not do):**
- The per-address limits, the actions' count and rate, the game's time, a
  worker's memory and the bytes sent: 2610.
- A page for visitors, a load test, the cost: 2611.
- Deploying it, renting a machine or opening a hosting account.

**Estimated size:** about 1,300 lines (code 760, tests 490, docs 50),
measured on the draft without its quotas.

**Split:** past 1,000 lines, split off step 4 (the idle and lifetime
expiry) into 2610, which bounds a session's time too.

## Implementation steps

1. Bring the draft's `crates/cli/src/preview/http.rs` and its tests in
   whole.
2. Bring `crates/cli/src/preview/server.rs` and its tests, without the
   quotas (`Bucket`, `Live::spent`, `resident` and the memory check, the
   per-address and starts-a-minute checks, and their tests), and
   `preview.rs`'s `Options`, `Limits`, `parse_args`, `usage` and `main`
   with only this ticket's flags.
3. Wire `preview` in `crates/cli/src/lib.rs`: `COMMANDS`, the usage, the
   dispatch (the game's tools, else `game::run_in_game_program`).
4. The reaper: idle and lifetime expiry, and its test.
5. The end-to-end test: `shuyi preview --port 0 --until-stdin-closes
   --stage` in `examples/hello-grid`, over HTTP: a start, the actions
   Confirm, Up, Right and Confirm (Turn 2), Debug and an unknown action
   refused, paths not served, a delete, and the stop.
6. The README's `shuyi-cli` row and the preview's module docs name the
   command; ADR-0141 changes only if this ticket decides otherwise.

## Acceptance criteria

- [ ] `cargo shuyi preview --stage` in `examples/hello-grid` serves Hello,
      Grid on 127.0.0.1: a session starts on its first frame, an allowed
      action answers the frame after it, and an unknown action or a body
      that isn't one is refused with `400` while the session goes on.
- [ ] Every path but the three answers `404`, and an answer holds only a
      frame's five fields or an error: no script, frame count, scene,
      snapshot, log or file is served.
- [ ] A worker that doesn't answer within `--deadline` is killed and its
      session ended; a session idle past `--idle` or older than
      `--lifetime` is ended; `--sessions` bounds the sessions at once.
- [ ] With `--until-stdin-closes`, closing its standard input stops it and
      every worker.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration (workers on threads running the toy): an allowed action, a
  refused action and body, a worker past its deadline, an expired session,
  the session cap, and every unserved path over a real connection.
- Unit: the HTTP reader's refusals, an answer's headers.
- End-to-end: Hello, Grid through `shuyi preview`, its workers real
  processes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
