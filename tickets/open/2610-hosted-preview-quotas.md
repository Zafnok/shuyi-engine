---
id: "2610"
title: "The hosted preview's quotas: per address, actions and their rate, the game's time, memory and bytes sent"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2609"]
nick_input: none
completed:
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

**Out (do not do):**
- Accounts, invitations or expiring preview IDs: 3083's.
- Reading a worker's memory on Windows or macOS (platform calls the
  engine's `forbid(unsafe)` rules out): said, not kept.
- The page, the load test and the cost: 2611.

**Estimated size:** about 550 lines (code 300, tests 220, docs 30),
measured on the draft.

## Implementation steps

1. Bring the draft's quota code into `crates/cli/src/preview/server.rs`:
   `Bucket`, `Live::spent` and its fields, `resident` and the memory
   check in `Live::answered`, the start's per-address and per-minute
   checks, the session's log line.
2. Add the flags to `preview.rs`'s `Options`, `Limits`, `NUMBERS` and
   usage, each with its range.
3. Their tests, from the draft's `server/tests.rs`.
4. ADR-0141 §5 changes only if this ticket decides otherwise.
5. With 2609 and this ticket merged, the draft holds nothing more: delete
   the branch (`git push origin --delete draft/2602-preview-server`).

## Acceptance criteria

- [ ] Each limit refuses past its bound with its status, and ends the
      session where the session has used it up; the per-address and
      per-minute limits refuse one address and not another.
- [ ] A session's rate allows a second's burst and refuses past it with
      `Retry-After`.
- [ ] On Linux (CI) a worker holding more than `--memory` ends its
      session; elsewhere the server says it isn't kept.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration (workers on threads running the toy): one test per limit.
- Unit: the bucket's refill and wait; the memory read on Linux, and `None`
  elsewhere.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
