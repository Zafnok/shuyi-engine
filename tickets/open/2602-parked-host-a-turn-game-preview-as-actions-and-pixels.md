---
id: "2602"
title: "Host a turn-game preview as bounded actions and pixels, built and tested locally"
type: feature
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: max
status: in-progress
blocked_by: []
nick_input: none
completed:
---

# 2602 — Host a turn-game preview as actions and pixels

> **Unparked** by Nick on 2026-11-23 ([`decisions.md`](../../docs/decisions.md#2026-11-23-the-hosted-preview-2602-unparked-built-and-tested-locally-only)): it runs locally and in CI only; no paid hosting until Nick approves its measured cost. The rest of networking stays parked.

## Context

A client-side encrypted pack eventually exposes its decoded content. A hosted preview can keep the executable, source art and structured scene off a visitor's device while still accepting play. Shuyi's headless session can produce a picture, but its CLI/MCP host also exposes snapshots, scenes and logs and accepts scripts; it is not a safe public protocol. See ADR-0006 for the broader networking plan.

**From 3302 (done):** closed builds' copies carry a recipient's code and a signed ledger ([ADR-0118](../../docs/adr/0118-a-closed-builds-recipient-overlay-three-marks-a-signed-ledger-never-proof.md)); a hosted preview's marks are 3083's, after this ticket.

## Nick input

None.

## Scope

**In:**
- A self-hosted turn-game proof of concept with a narrow authenticated or rate-limited action-to-PNG API.
- Server-owned seed and target profile, isolated bounded sessions and explicit CPU, memory, action and egress limits.
- Player-visible accessible text alongside pixels without exposing internal scene/state JSON.

**Out (do not do):**
- Exposing the existing MCP/Session API to visitors.
- Real-time video/audio streaming, matchmaking or a hosted Shuyi business.
- A claim that visible gameplay cannot be copied.
- Paid hosting of any kind: no rented server, hosting account or public
  deployment. The service runs on the developer's machine and in CI
  only, until Nick approves the cost this ticket measures.

**Estimated size:** about 1,600 lines (code 900, tests 500, docs 200).

**Split:** past 1,000 lines, split off step 3 (the browser client, the
load test and the accessibility review) into a new ticket after this
one, and finish here with the worker, its action API and its quotas.

## Implementation steps

1. Wrap a headless game session in an isolated worker; accept only declared player actions.
2. Render one bounded result frame, enforce quotas and expire idle sessions.
3. Make a thin browser client and load-test it; check that no game Wasm, pack, source or structured state endpoint is public.

## Acceptance criteria

- [ ] A visitor can play a fixture turn game without downloading its executable or assets.
- [ ] Arbitrary scripts, frame counts, scene, snapshot and log endpoints are unavailable externally.
- [ ] p95 action-to-image delay, CPU/RAM per session, egress and concurrent-session limits are reported.
- [ ] The completion notes turn those measurements into a plain monthly estimate for hosting a closed preview (machine size, traffic per session), for Nick to decide on; nothing is deployed or paid for.
- [ ] Keyboard and screen-reader paths are usable for the fixture.
- [ ] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Integration: allowed action, rejected unknown action, quotas, expired session and no structured endpoint.
- Load: bounded concurrent sessions with recorded resource use.
- Accessibility: keyboard and screen-reader fixture review.

## Completion notes

*(Filled in by the session that completes the ticket.)*
