---
id: "2816"
title: "Prepare a verifiable public reveal kit from game captures and releases"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: blocked
blocked_by: ["2811", "3050"]
nick_input: none
completed:
---

# 2816 — Prepare a verifiable public reveal kit

## Context

A public gameplay hook can be imitated from footage without a binary. A developer benefits from a canonical store link, approved captures and dated evidence before the first reveal. Existing preview capture (2811) and tag release attestations (3050) supply the inputs. A timestamp helps attribution, not ownership of an idea.

## Nick input

None.

## Scope

**In:**
- A local, human-reviewed reveal bundle containing approved captures, canonical store URL, game name, artifact hashes and provenance receipts.
- Optional private salted commitments to unreleased source/content; opening them later requires the retained salt and source.
- A copyable incident evidence index tying a public clip to its source build.

**Out (do not do):**
- Automatic Steam publication, legal conclusions or guaranteed takedowns.
- A requirement to ship a demo before a store page.

## Implementation steps

1. Compose existing frame capture and tagged-release outputs without duplicating their pipelines.
2. Define a stable manifest and hash/signature verification command; preserve originals and platform encodes.
3. Add a shuyi-ship workflow for checking the canonical page and approved media before reveal.

## Acceptance criteria

- [ ] A fixture reveal bundle verifies after copying to another machine.
- [ ] Changing a captured frame or manifest field invalidates verification.
- [ ] A creator can prepare a store-only reveal with no demo artifact.
- [ ] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Integration: produce, copy and verify a fixture reveal bundle; tampering fails.
- Unit: salted commitment opens with original bytes and salt, not changed bytes.

## Completion notes

*(Filled in by the session that completes the ticket.)*
