---
id: "2611"
title: "The hosted preview's page, its load test and accessibility review, and a month's cost to host it"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2610"]
nick_input: none
completed:
---

# 2611 — The hosted preview's page, load test and cost

## Context

2602 planned a hosted preview of a turn game, as actions in and pictures
out, built and tested locally, with its cost measured for Nick to decide
on. Nick unparked it on 2026-11-23: it runs on his machine and in CI only,
and nothing is deployed or paid for until he approves the measured cost
([decisions](../../docs/decisions.md#2026-11-23-the-hosted-preview-2602-unparked-built-and-tested-locally-only)).
2602 built the worker, 2609 the server's action API and 2610 its quotas
([ADR-0141](../../docs/adr/0141-the-hosted-preview-a-worker-per-visitor-actions-in-pictures-out.md)).
This ticket is 2602's step 3, split from it for its size: the page a
visitor plays, the load test that measures it, the accessibility review,
and the estimate Nick decides from.

What 2602 measured on Hello, Grid's stage in a debug build, as a start:
35 to 75 ms an action, about 6 KB a picture at scale 1 (384 × 384).

## Nick input

None. The estimate in the completion notes is for Nick to decide on later,
with the `ask-nick` skill; this ticket asks nothing.

## Scope

**In:**
- `GET /` answers one page of the server's own, its script and style in
  it or at two more fixed paths, with a strict `Content-Security-Policy`
  (`default-src 'none'`, the script and style by hash, images from `data:`
  or `blob:`, requests to its own origin only). No game program, pack,
  file, scene or state is ever on it or behind it.
- The page: the frame as an image with its `alt`, the frame's text in a
  live region (`aria-live="polite"`), a button for each action, playable
  with Tab and Enter or Space alone; one action at a time; a message when
  the session ends or a limit refuses.
- Keys: the worker adds to each frame the player's keys for each action,
  from the game's keymap (`shuyi_ui::input::help_keys`), and the page binds
  those, translating the browser's key names to the engine's. No key is
  written into the page (`AGENTS.md` rule 8).
- A load test of concurrent sessions against a local server (an `xtask`
  command, run by a CI job on Linux): the p50 and p95 time from an action
  to its picture, each worker's CPU time and most memory, the bytes a
  session sends, and how many sessions a machine holds within the limits;
  its report kept as the job's artifact.
- An accessibility review of the fixture: a keyboard alone, and a screen
  reader (NVDA or Narrator on Windows, Orca on Linux), written in the
  completion notes.
- The book's chapter on targets and export, *Releasing*: what the hosted
  preview gives and what it can't, and how to run it on your machine.

**Out (do not do):**
- Deploying it, renting a machine, a domain, TLS or a hosting account.
- Video or sound streaming, matchmaking, accounts.
- A claim that what is seen can't be copied.

**Estimated size:** about 1,400 lines (code 600, tests 300, docs 250,
the CI job and its data 250).

**Split:** past 1,000 lines, split off the load test, its CI job and the
estimate into a new ticket after this one.

## Implementation steps

1. The worker's frame gains each action's keys (`Shown::keys`, from the
   session's keymap); `Session` gains what gives them.
2. The page and its paths in `crates/cli/src/preview/`, its policy, and
   the tests that it holds nothing of the game's.
3. The load test and its CI job; its report.
4. The reviews; the book's passage; the README's and the docs' rows.
5. The completion notes: the measurements, and a month's estimate for a
   closed preview of N concurrent visitors (machine size, traffic a
   session, the price of a small VPS read on the day, with its source).

## Acceptance criteria

- [ ] A visitor plays Hello, Grid from the page in a browser without
      downloading its program or files: the browser's network log of a
      session shows the page, its fixed paths and the session's requests
      alone.
- [ ] No game Wasm, pack, source file or structured state is served at any
      path: a test asks for each kind and gets `404`.
- [ ] The p95 action-to-picture time, a session's CPU and memory, its
      bytes, and the sessions a machine holds are reported.
- [ ] The completion notes turn them into a plain monthly estimate of
      hosting a closed preview (machine size, traffic per session), for
      Nick to decide on; nothing is deployed or paid for.
- [ ] The keyboard and screen-reader paths are usable for the fixture, as
      the review says.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the page's policy and what it holds; the keys a frame
  gives; every kind of game file asked for and refused.
- Load: bounded concurrent sessions, with their resources recorded.
- Accessibility: the keyboard and screen-reader review of the fixture.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
