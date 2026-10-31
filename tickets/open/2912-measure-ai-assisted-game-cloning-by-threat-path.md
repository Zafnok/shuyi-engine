---
id: "2912"
title: "Measure AI-assisted imitation, extraction and reconstruction separately"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: max
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2912 — Measure AI-assisted game cloning by threat path

## Context

A four-hour video imitation, demo asset extraction and behavioral reconstruction are different tasks. REA, SRE-Bench and video-to-game examples measure different outcomes; Shuyi should not infer protection from byte differences or decompiler appearance. Its deterministic headless tools can grade permissioned fixture games with held-out rules.

## Nick input

None.

## Scope

**In:**
- Four separate tracks: footage-only playable imitation, final-package future-content extraction, binary/Wasm behavioral replacement with a runnable oracle, and canonical-store attribution.
- Private synthetic Shuyi fixtures, hidden tests, equal attacker budgets, dynamic tools and explicit refusal/tool-failure accounting.
- A written baseline and go/no-go thresholds tied to absolute demo-to-launch delay and player cost.

**Out (do not do):**
- Offensive testing of third-party games.
- Treating hidden-trigger success as a whole-game clone.
- A production obfuscator or DRM claim.

## Implementation steps

1. Specify two representative fixture games and their withheld validation cases.
2. Build a capped pilot with ordinary builds and controls, including seeded Wasm diversity as a non-security control.
3. Publish methods, budgets, result denominators and reproducible non-secret fixtures without leaking held-out tests.

## Acceptance criteria

- [ ] Each track has a distinct success measure and no track borrows another's percentage.
- [ ] The baseline is solvable within the allotted budget before a defense is credited.
- [ ] A pilot reports time, tool cost, gameplay quality, player performance and failures separately.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Harness: seeded replay equality and held-out state-transition checks.
- Audit: fixture sources and hidden grader are absent from attacker artifacts.

## Completion notes

*(Filled in by the session that completes the ticket.)*
