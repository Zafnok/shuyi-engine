---
id: "2610"
title: "The hosted preview's quotas: per address, actions and their rate, the game's time, memory and bytes sent"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2609"]
nick_input: none
completed: 2026-11-23
---

# 2610 — The hosted preview's quotas

## Context

The hosted preview's server (2609) runs each visitor's game in a worker
process and answers actions with frames
([ADR-0141](../../docs/adr/0141-the-hosted-preview-a-worker-per-visitor-actions-in-pictures-out.md)).
This ticket bounds what one visitor, or one address, may cost: ADR-0141
§5's limits, split from 2602 for its size. It is 2602's work, unparked
with it: local and CI only
([decisions](../../docs/decisions.md#2026-11-23-the-hosted-preview-2602-unparked-built-and-tested-locally-only)).
2611 measures the cost these limits bound.

A draft of it, written and passing with 2602, is on the branch
`draft/2602-preview-server` (commit `0ea6a442`), in
`crates/cli/src/preview/server.rs` (`Bucket`, `Live::spent`, `resident`,
the per-address and starts-a-minute checks in `Server::start`) and
`preview.rs` (the flags), with their tests. Start from it.

**From 2609 (done):** the server is built ([ADR-0141](../../docs/adr/0141-the-hosted-preview-a-worker-per-visitor-actions-in-pictures-out.md)
§5): `preview` in the game's tool program, its HTTP reader
(`crates/cli/src/preview/http.rs`) and `server.rs` (`Server`, its `Table`
of sessions, a `Live` each, `serve`), with `--sessions` and `--deadline`.
2609 left two parts to this ticket, split off for its size: its step 4,
the idle and lifetime expiry, and the cap on connections served at once.
The draft has both: `Live::born` and `last`, `Server::reap` and the
reaper thread in `serve`, and `Server::connections`. What 2609's tests
learnt: give `reap` the time it reaps at (`reap(&self, now: Instant)`),
so its test reaps at `last + idle` less a nanosecond and at `last + idle`,
with no sleep, which also catches the comparisons' mutants; and hold one
connection that has sent nothing to fill the cap, then wait for the
count to fall to 0 before the next request.

## Nick input

None.

## Scope

**In:**
- Flags of `preview`, each with its default: `--per-address N` sessions of
  one address at once (2), `--starts N` sessions one address may start a
  minute (10), `--actions N` a session may press (1,000), `--rate N`
  actions a second (8, a bucket of tokens that holds a second's worth),
  `--cpu SECONDS` of the game's time a session may use in all, its calls'
  time summed (120), `--memory MB` a worker may hold (512) and `--egress
  MB` a session's answers may send (64).
- Past a limit: `429` (or `503` for memory), with `Retry-After` where
  waiting helps, and the session ended when it has used up its actions,
  time, memory or bytes.
- A worker's memory read after each answer from Linux's
  `/proc/<pid>/status` (`VmRSS`); on a system that doesn't say it, the
  server says at start that `--memory` isn't kept there.
- One line in the server's log for each session that ends: why, its
  actions, its game's time, the bytes it sent and the most memory it held.
- From 2609: `--idle SECONDS` (120) and `--lifetime SECONDS` (1,800), a
  thread that ends each session idle that long since its last action or
  that old, at most a second late, and never one busy with an action; and
  at most 64 connections served at once, past it `503` with
  `Retry-After`.

**Out (do not do):**
- Accounts, invitations or expiring preview IDs: 3083's.
- Reading a worker's memory on Windows or macOS (platform calls the
  engine's `forbid(unsafe)` rules out): said, not kept.
- The page, the load test and the cost: 2611.

**Estimated size:** about 750 lines (code 360, tests 360, docs 30),
measured on the draft, with 2609's two parts.

## Implementation steps

1. Bring the draft's quota code into `crates/cli/src/preview/server.rs`:
   `Bucket`, `Live::spent` and its fields, `resident` and the memory
   check in `Live::answered`, the start's per-address and per-minute
   checks, the session's log line.
2. Add the flags to `preview.rs`'s `Options`, `Limits`, `NUMBERS` and
   usage, each with its range.
3. 2609's two parts: the expiry (`Live::born` and `last`, `Server::reap`
   and the reaper thread in `serve`, `--idle` and `--lifetime`) and the
   connections' cap (`Limits::connections`, the count in `serve`).
4. Their tests, from the draft's `server/tests.rs`.
5. ADR-0141 §5 changes only if this ticket decides otherwise.
6. With 2609 and this ticket merged, the draft holds nothing more: delete
   the branch (`git push origin --delete draft/2602-preview-server`).

## Acceptance criteria

- [x] Each limit refuses past its bound with its status, and ends the
      session where the session has used it up; the per-address and
      per-minute limits refuse one address and not another.
- [x] A session's rate allows a second's burst and refuses past it with
      `Retry-After`.
- [x] On Linux (CI) a worker holding more than `--memory` ends its
      session; elsewhere the server says it isn't kept.
- [x] A session idle past `--idle` or older than `--lifetime` is ended,
      at its limit and not before, and a session busy with an action is
      neither; past 64 connections at once, a connection is answered
      `503` with `Retry-After`.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration (workers on threads running the toy): one test per limit;
  the expiry at its edges with the time given, and the reaper thread
  ending an idle session by itself; the connections' cap over real
  connections.
- Unit: the bucket's refill and wait; the memory read on Linux, and `None`
  elsewhere.

## Completion notes

**What was done.** `preview`'s server keeps ADR-0141 §5's quotas,
brought from the draft on `draft/2602-preview-server` (`0ea6a442`) onto
`main`'s server (2609) and re-checked against it. Each is a flag of
`preview` with its range and default: `--per-address` (2) and `--starts`
(10 a minute, a bucket per address) refuse a start with `429`, the
minute's with `Retry-After`; `--actions` (1,000), `--cpu` (120 s of its
calls' time summed) and `--egress` (64 MB, each answer's bytes head and
all) are checked before a session's next action, which is refused `429`
and the session ended once one is used up; `--rate` (8 a second, a bucket
holding a second's worth) refuses `429` with `Retry-After` and the
session goes on; `--memory` (512 MB) is read after each answer from
`/proc/<pid>/status` (`VmRSS`), and past it the answer is `503` and the
session ended, its start included. Off Linux the server says at start
that `--memory` isn't kept there. 2609's two parts: `--idle` (120 s since
the last action) and `--lifetime` (1,800 s), a thread that reaps every
quarter of the shorter, between 20 ms and a second, never a session busy
with an action; and at most 64 connections at once, past it `503` with
`Retry-After: 5`. Each session that ends logs one line: why, its actions,
its game's time, the bytes it sent and the most memory it held (or "its
memory unread"), and the sessions still open.

**Tests.** One per limit in `server/tests/limits.rs`, over workers on
threads running the toy, each at its edge where it has one (the game's
time at its bound less a nanosecond and at it, the bytes at theirs less
one and at it, the actions at their count); the per-address and
per-minute limits refuse one address and not another; the expiry with
the time given (`reap(now)`) at `last + idle` less a nanosecond and at
it, the lifetime likewise, and a held session neither; the reaper's
thread ending an idle session by itself; 64 held connections that send
nothing, the 65th answered `503`, and the count back to 0 before the
next request. Unit: the bucket's refill, its cap and its waits, rounded
up to whole seconds; `VmRSS` read from a status text, `resident` on
Linux and `None` elsewhere. The memory test gives a worker on a thread
the memory of a real idle process (`sort`), so it ends the session on
Linux and is not kept on Windows. The integration test over HTTP reads
the server's log: the session's line, and the memory notice off Linux.

**Deviations.**
- **The draft's regressions against `main` were left out**: it lacked
  2609's `Allow` on `405`, the worker's arguments after the project's
  (`worker_args(before)`), `code_of`, and ending the sessions as the
  standard input closes. Its quota code went onto `main`'s server
  instead of replacing it.
- **A refused start uses none of the minute's starts.** The draft took a
  start before the sessions' and the address's checks, so a start
  refused `503` or `429` still counted; it is taken last now, and a test
  says so.
- **An IPv6 address counts as its /64**, and an IPv4 one written in IPv6
  (`::ffff:a.b.c.d`) as itself: a host is given a /64, so per address
  would otherwise be per host's whim. ADR-0141 §5 says "per address"; the
  /64 is how an address is read, and `--help` says so, so the ADR stands.
- **The reaper ends a session within the table's lock**, holding the
  session's: the draft let go of it between choosing and ending, where an
  action could start.
- **`429` has its reason phrase** ("Too Many Requests") in the HTTP
  reader's table: without it, a `429` went out as `429 Internal Server
  Error`.
- **`Retry-After` rounds up**: the draft's `as_secs` sent 0 for a wait
  under a second (`retry_after` raised it to 1, but 1.5 s was 1).
- **The connections' cap is not a flag**, as the ticket's scope lists:
  `Limits::connections`, 64.
- **Its size**: 1,037 counted lines against the 750 estimated, the tests
  at each limit's edge being the most of the difference (515 lines). Past
  1,000 it would split cleanly between the quotas and 2609's two parts
  (the expiry and the connections' cap), which share only `Limits`.
- **The draft's branch is not deleted here** (step 6): it holds nothing
  more once this merges, and deleting it is for after the merge,
  `git push origin --delete draft/2602-preview-server`.

**Follow-up tickets:** none new; 2611 (now `status: todo`) has a
**From 2610 (done)** paragraph: the limits, their statuses, the log line
it can read for a session's cost, and that a load test from one machine
is one address and raises `--per-address`, `--starts` and `--sessions`.

Book: none, `preview` is a command of the game's tool program, not a
public item, and its quotas are its `--help`; the book's passage on the
hosted preview comes with the page a visitor plays (2611).

Behaviour: shuyi-cli: `preview` takes `--per-address`, `--starts`,
`--actions`, `--rate`, `--idle`, `--lifetime`, `--cpu`, `--memory` and
`--egress`, each on by default: a session is refused `429` or `503`, or
ended, past one; at most 64 connections are served at once; each ended
session's log line names what it spent.

No break: `preview` is a private module. Nothing in the game must change:
`gh search code --repo Zafnok/visions-of-shuyi 'shuyi preview'` and
`'preview-worker'` find nothing; the game serves no preview yet.

Game: nothing to adopt, the game runs no `preview`.
