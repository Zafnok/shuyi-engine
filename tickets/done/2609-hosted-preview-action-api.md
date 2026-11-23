---
id: "2609"
title: "The hosted preview's action API: sessions over HTTP, one frame an action, and deadlines"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2602"]
nick_input: none
completed: 2026-11-23
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
  [--scale N] [--deadline SECONDS] [--until-stdin-closes] [the game's
  options]`, run in the game's tool program (`shuyi` runs it there, as
  `frame-png`), on 127.0.0.1:8000 by default. It checks at start that the
  game offers a preview.
- Each session's worker is `std::env::current_exe()` run as
  `preview-worker`, after the project's `game.args` as `shuyi` runs the
  tool program, in the project's root, with the server's options: stdin
  and stdout piped, reply lines read after their mark, a reply at most 32
  MiB, the worker killed past `--deadline` and when its session ends.
- HTTP/1.1 of our own: one request a connection, a head of at most 8 KiB,
  a body of at most 1 KiB, a request's deadline, no chunked body, no two
  lengths, no bytes past the body; every answer `no-store`, `nosniff`,
  `no-referrer`, `Connection: close`.
- `POST /session` (a session, its id 128 bits from `getrandom`, and its
  first frame), `POST /session/<id>` with `{"action": "<name>"}`, `DELETE
  /session/<id>`; `405` for another method there, `404` for every other
  path. An answer is the frame's `session`, `png`, `text`, `actions` and
  `ended`, or `{"error": "…"}`.
- At most `--sessions` sessions at once, those starting among them.

**Out (do not do):**
- The per-address limits, the actions' count and rate, the game's time, a
  worker's memory and the bytes sent: 2610.
- The idle and lifetime expiry (`--idle`, `--lifetime`) and the cap on
  connections served at once: 2610, split off for size (the completion
  notes).
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
4. ~~The reaper: idle and lifetime expiry, and its test.~~ Split off into
   2610 with the cap on connections, for size.
5. The end-to-end test: `shuyi preview --port 0 --until-stdin-closes
   --stage` in `examples/hello-grid`, over HTTP: a start, the actions
   Confirm, Up, Right and Confirm (Turn 2), Debug and an unknown action
   refused, paths not served, a delete, and the stop.
6. The README's `shuyi-cli` row and the preview's module docs name the
   command; ADR-0141 changes only if this ticket decides otherwise.

## Acceptance criteria

- [x] `cargo shuyi preview --stage` in `examples/hello-grid` serves Hello,
      Grid on 127.0.0.1: a session starts on its first frame, an allowed
      action answers the frame after it, and an unknown action or a body
      that isn't one is refused with `400` while the session goes on.
- [x] Every path but the three answers `404`, and an answer holds only a
      frame's five fields or an error: no script, frame count, scene,
      snapshot, log or file is served.
- [x] A worker that doesn't answer within `--deadline` is killed and its
      session ended; `--sessions` bounds the sessions at once. (A session
      idle past `--idle` or older than `--lifetime` is 2610's now.)
- [x] With `--until-stdin-closes`, closing its standard input stops it and
      every worker.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration (workers on threads running the toy): an allowed action, a
  refused action and body, a worker past its deadline, the session cap,
  and every unserved path over a real connection. (An expired session is
  2610's now.)
- Unit: the HTTP reader's refusals, an answer's headers.
- End-to-end: Hello, Grid through `shuyi preview`, its workers real
  processes.

## Completion notes

**What was done.** `shuyi preview`, in a game's tool program, serves the
hosted preview over HTTP ([ADR-0141](../../docs/adr/0141-the-hosted-preview-a-worker-per-visitor-actions-in-pictures-out.md)
§5), started from the draft on `draft/2602-preview-server` (`0ea6a442`)
without its quotas and re-checked against the worker on `main`.
`crates/cli/src/preview/http.rs` reads one request a connection within
bounds (a head of at most 8 KiB and its blank line, a body of at most
1 KiB, a deadline; no chunked body, no two lengths) and writes one
answer, every one `no-store`, `nosniff`, `no-referrer` and
`Connection: close`. `server.rs` answers `POST /session` (a worker
started, a 128-bit id from `getrandom`, its first frame), `POST
/session/<id>` with `{"action": …}` and `DELETE /session/<id>`; `405`
with `Allow` for another method there, `404` for every other path. A
session's worker is the tool program again as `preview-worker`, killed
past `--deadline` and when its session ends; `--sessions` bounds the
sessions at once, those starting among them; a second action while one
runs is `409`; with `--until-stdin-closes`, a closed standard input ends
every session and its worker, then the server. `preview` checks at start
that the game offers a preview, and says so with the game's code if not.

**Deviations.**
- **Split again, at the ticket's Split line.** With the expiry it
  measured 1,603 counted lines: the ticket's 1,300 was measured on the
  draft, whose tests left mutants alive that CI's mutation gate would
  fail. The idle and lifetime expiry (step 4) and the cap on connections
  served at once went to 2610, which now says what 2609's tests learnt
  about both. This PR: 1,485 counted lines; past 1,000, it would split
  cleanly between the HTTP reader (`http.rs` and its tests) and the
  server.
- **A worker starts as `shuyi` starts the tool program.** The ticket said
  `current_exe()` run as `preview-worker`. The game runs its tool program
  as `trpg-tools shuyi <command>` (its `shuyi.ron`: `game: (package:
  "trpg-tools", args: ["shuyi"])`), whose own commands come first: a bare
  `preview-worker` is an unknown command there, and every session would
  fail. The worker runs with the project's `game.args` before
  `preview-worker`, and the ticket's scope says so now.
- **The worker reads its own arguments.** The draft had the worker read
  the server's options; `main`'s worker (2602) reads its own
  (`worker::parse_args`), so the server writes them
  (`Options::worker_args`) and a test reads them back with the worker's
  parser.
- **Beyond the draft:** `405` carries `Allow`; a game that doesn't start
  says so with its own code (`code_of`: `SH3002`, not `SH3001`); a closed
  standard input ends the workers before the process exits; `reason` is a
  table, and the head's bound is exact. Each comparison of the reader and
  the start has a test at its edge, and a worker past its deadline is a
  real process too (`sort`, which answers nothing): killed, and on Linux
  waited for, no zombie left.
- **The title** said "deadlines and expiry"; the expiry is 2610's now,
  and the ROADMAP's row follows.

**Manual check**, on `7e2fe44f` (the branch with `main`'s tip,
`223d751a`, merged): in `examples/hello-grid`, `cargo shuyi preview
--stage --until-stdin-closes`, its standard input a `sleep`, played over
HTTP from Python's `http.client`. `POST /session` answered `200` with the
five fields, the six player's actions and the four headers, the stage at
Turn 1 (a 3,922-byte PNG); `Debug`, `Jump`, `{"script": "F2"}` and `not
json` were `400`; `/`, `/scene`, `/files/stage.map` and
`/session/<id>/snapshot` were `404`; `GET /session/<id>` was `405`,
`Allow: POST, DELETE`; Confirm, Up, Right and Confirm answered Turn 2,
your piece moved and the foe stepped; `DELETE` was `204`, then `404`. Its
input closed, it ended the two sessions left ("the server stopped") and
stopped, and no `tools.exe` was left running.

**Follow-up tickets:** 2610 (now also the expiry and the connections'
cap; `status: todo`, 2609 being done) and 2611, both 2602's work and
unparked with it.

Book: none, `preview` is a command of the game's tool program, not a
public item; the book's passage on the hosted preview comes with the page
a visitor plays (2611).

Behaviour: shuyi-cli: `preview` is a new command of a game's tool
program; `COMMANDS` and `shuyi`'s usage name it, so `check-skills` takes a
skill that names it.

No break: `preview` is a private module, and `COMMANDS` keeps its type.
Nothing in the game must change: `gh search code --repo
Zafnok/visions-of-shuyi` finds no `shuyi_cli::COMMANDS`, no
`shuyi_cli::usage` and no `preview-worker`; its `trpg-tools` has an `icon
preview` of its own, a subcommand of `icon`, which `preview` doesn't
reach. When it moves its pin, `cargo shuyi preview` serves it, its
workers started as `trpg-tools shuyi preview-worker`.
