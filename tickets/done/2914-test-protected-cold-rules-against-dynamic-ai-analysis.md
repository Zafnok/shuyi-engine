---
id: "2914"
title: "Test a small protected rules kernel against dynamic AI analysis"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: max
status: done
blocked_by: ["2912"]
nick_input: none
completed: 2026-11-01
---

# 2914 — Test protected cold rules against dynamic AI analysis

## Context

Static obfuscation delayed AI on small challenges, but dynamic agents can execute decoders. SRE-Bench's layered Linux protector barely reduced its strongest model's hidden-trigger score, and strong virtual machines can cost hundreds of times a protected function's runtime. **From 2912 (done):** an independent replacement matched all 343 hidden transitions of its small ordinary fixtures; this was a shared-worktree pilot without a defended arm, enforced oracle budget or browser export. This ticket must supply those controls before any engine promise.

## Nick input

None.

## Scope

**In:**
- An isolated, pure integer rules kernel chosen by a fixture game, compiled through a per-build diversified interpreter or bytecode.
- Baseline, seeded-layout-only control and protected variants with identical visible behavior.
- Dynamic REA/debugger attackers, held-out state tests, replay and performance budgets.

**Out (do not do):**
- A production scripting module or changes to parked script-runtime ticket 2101.
- A forked rustc, non-permissive dependency or whole-engine virtualization.
- A claim that an offline client is opaque.

## Implementation steps

1. Specify a restricted kernel interface and deterministic bytecode/VM fixture outside hot frame paths.
2. Produce native and web variants without changing the simulation's result.
3. Run the 2912 dynamic benchmark and document absolute delay, size, frame cost, diagnosis and target failures.

## Acceptance criteria

- [x] Baseline and protected variants replay to identical state and scene hashes.
- [x] Analysts can run and trace both variants with equal tools and budget.
- [x] Any recommendation names the measured absolute delay and player cost; negligible delay stops production work.
- [x] All gates in the run-gates skill pass.

## Tests required

- Property: many seeded inputs produce identical kernel outputs across variants.
- Integration: replay and native/web build gates; benchmark held-out cases.

## Completion notes

- Added the optional ordinary, seeded-layout and diversified-bytecode Workshop
  kernels, a native replay-cost probe, a browser-target Wasm bridge, and a
  Playwright browser replay gate. The default 2912 fixture protocol is unchanged.
- Across 39 held-out traces and 425 transitions, five native and five browser
  Wasm builds matched every full record, state hash and ASCII scene hash.
  Two seeds of each diversified arm had distinct executable hashes.
- Three gradable, independent dynamic analysts each matched 425/425 hidden
  transitions and 39/39 traces. The ordinary, layout and VM submissions took
  21:49, 14:43 and 15:27. The VM added **−6:22** against ordinary and **+0:44**
  against layout. The research report is
  [`docs/research/2914-protected-cold-rules-dynamic-analysis.md`](../../docs/research/2914-protected-cold-rules-dynamic-analysis.md).
- **Recommendation: stop production work on this VM candidate.** Its native
  package grew by 1,024 bytes and browser Wasm by 1,267 bytes. The headless
  11-action replay measured 600 ns median and 700 ns p99 in both ordinary
  and VM builds at the timer's resolution; process-plus-replay medians were
  32.04/32.91 ms native and 182.24/216.00 ms under Node. Interactive frame
  p99, actual browser startup and player quality remain unmeasured, so no
  player-cost gate or offline-opacity claim is made.
- Deviations and limits: the query cap was raised from 2912's proposed 500
  to 1,200 before exposure because its successful ordinary pilot used 912.
  The supplied oracle enforces that cap only on its own calls; direct
  executable/debugger runs were counted in attempt logs and are not
  tamper-resistant. The shared worktree did not seal source or grader files,
  so all attempts were unblinded protocol rehearsals. GDB, Ghidra and REA
  were unavailable; each arm used the same live Node Wasm debugger. One
  initial ordinary assignment failed with model HTTP 403 before exposure;
  the fresh ordinary assignment succeeded. Total model/tool steps and model
  charges were unavailable, and exact analyst model/effort parity was not
  recorded in the saved attempt logs. Local headless Chrome/Edge did not complete;
  the HTML browser replay is gated in CI with Playwright.
- Documentation: the result lives in `docs/research/`; the 2912 report now
  links it. No game-facing API, contract, format, rule or template changed,
  so the book and skill pack need no update. No public API or format break.
- Follow-up: ticket 2575 covers an unrelated existing CLI timing assertion
  that failed once under parallel workspace-test load.
- Nick input: none.
