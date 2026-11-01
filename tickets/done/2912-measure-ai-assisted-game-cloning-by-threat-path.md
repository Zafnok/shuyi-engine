---
id: "2912"
title: "Measure AI-assisted imitation, extraction and reconstruction separately"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: max
status: done
blocked_by: []
nick_input: none
completed: 2026-10-31
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

- [x] Each track has a distinct success measure and no track borrows another's percentage.
- [x] The baseline is solvable within the allotted budget before a defense is credited.
- [x] A pilot reports time, tool cost, gameplay quality, player performance and failures separately.
- [x] All gates in the run-gates skill pass.

## Tests required

- Harness: seeded replay equality and held-out state-transition checks.
- Audit: fixture sources and hidden grader are absent from attacker artifacts.

## Completion notes

- Added two author-owned `TurnRules` fixtures and a native/WASI oracle,
  with seeded replay, winning-path and atomic-refusal tests. The research
  runner creates audited attacker bundles, a precommitted held-out grader,
  and a seeded Wasm custom-section byte-diversity control.
- The [research report](../../docs/research/2912-game-cloning-threat-paths.md)
  holds the method, proposed decision thresholds, artifact hashes and pilot
  results: footage imitation matched 33/343 hidden frames after matching
  all shown frames; present package content was extracted 4/4; a separate
  behavioral replacement matched 343/343 held-out transitions; the
  synthetic receipt selected the right card 1/1. These denominators are
  separate. No defense was tested or credited.
- The pilot used a shared worktree, so access was procedural rather than
  securely blind. Two initial agents failed before artifact receipt when
  Daybreak was unavailable for their selected model, and their retries
  succeeded. Model billing and exploratory oracle-query counts were not
  available. Player cost was measured as process-plus-replay time and
  bytes, not interactive frame time; human player quality was not measured.
  The report treats the confirmatory gates as unassessable. These limits
  preclude a production protection claim.
- The report is the research home in the docs map; no public engine API,
  game format, skill or template changed. Tickets 2913 and 2914 inherit
  the fixture and grader for their separate experiments. No Nick input.
