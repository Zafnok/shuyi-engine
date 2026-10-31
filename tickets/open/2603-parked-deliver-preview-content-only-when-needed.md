---
id: "2603"
title: "Parked: Deliver preview content only when the visitor reaches it"
type: feature
milestone: "Parked: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2602"]
nick_input: none
completed:
---

# 2603 — Deliver preview content only when needed

> **Parked** This follows the hosted preview experiment (2602) and stays parked with networking until Nick unblocks it.

## Context

Server-held content can keep future chapters unavailable to a current preview visitor. This is a time boundary: a delivered chapter can be captured. The hosted action-to-pixels preview (2602) gives a generic server boundary without shipping the whole pack.

## Nick input

None.

## Scope

**In:**
- An author-declared content schedule for a preview session, with server-only files never sent to the visitor.
- A report of which content was rendered or transmitted and when.
- Tests that future-content canaries stay server-side until their declared reveal step.

**Out (do not do):**
- A local encrypted pack with its key in the client.
- A guarantee that delivered or visible content cannot be copied.
- Changes to game-specific narrative or unlock rules.

## Implementation steps

1. Extend the preview service's content loader with author-declared groups and server-side reveal conditions.
2. Record content access without exposing file names in the public protocol.
3. Verify no future group enters downloadable artifacts or responses before reveal.

## Acceptance criteria

- [ ] An unauthenticated visitor cannot fetch future-content canaries before their reveal.
- [ ] A revealed group plays normally and its post-reveal exposure is documented.
- [ ] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Integration: enumerate public responses before and after reveal with harmless canaries.
- Unit: invalid group, missing group and expired session errors reveal no private path.

## Completion notes

*(Filled in by the session that completes the ticket.)*
