---
id: "2914"
title: "Test a small protected rules kernel against dynamic AI analysis"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: max
status: blocked
blocked_by: ["2912"]
nick_input: none
completed:
---

# 2914 — Test protected cold rules against dynamic AI analysis

## Context

Static obfuscation delayed AI on small challenges, but dynamic agents can execute decoders. SRE-Bench's layered Linux protector barely reduced its strongest model's hidden-trigger score, and strong virtual machines can cost hundreds of times a protected function's runtime. A Shuyi trial needs the behavioral benchmark (2912) before any engine promise.

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

- [ ] Baseline and protected variants replay to identical state and scene hashes.
- [ ] Analysts can run and trace both variants with equal tools and budget.
- [ ] Any recommendation names the measured absolute delay and player cost; negligible delay stops production work.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Property: many seeded inputs produce identical kernel outputs across variants.
- Integration: replay and native/web build gates; benchmark held-out cases.

## Completion notes

*(Filled in by the session that completes the ticket.)*
