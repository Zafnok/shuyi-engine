---
id: "2038"
title: Correct model routing with explicit Claude and OpenAI pairs
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-31
---

# 2038 — Correct model routing with explicit Claude and OpenAI pairs

## Context

Nick corrected the merged portability migration on 2026-10-31. Its generic
reasoning category suggested Astra for work originally recommended for Opus.
He requested specific models on every ticket line, with both providers:
Fable/Astra, Opus/Sol, Sonnet/Terra and Haiku/Luna. Game ticket 1008 must
recommend Opus/Sol, not Astra. The independent companion is game#0137;
neither correction needs the other merged or an engine pin move.

## Nick input

None. The model pairs and correction scope are Nick's explicit instruction.

## Scope

**In:** all open, parked and done ticket recommendations; templates,
ticket schema and tests, active guidance, shared instructions, owned skills
and generated issue guidance. Preserve shared skill loading and the game's
exact copies of the pinned engine's skill pack.

**Out (do not do):** gameplay, assets, engine APIs, engine pins, account
settings, new benchmarks, or implementation of a parked ticket. The
companion correction is independent and leaves no work to this ticket.

## Implementation steps

1. Translate each category to its exact model pair; preserve all other
   metadata and ticket scope.
2. Accept only the four approved pairs; reject generic labels, single
   model ids and mismatched pairs.
3. Correct current guides, templates, instructions, skills and generated
   recommendations. Mark old routing decisions as amended.
4. Audit all ticket metadata against the branch base, verify 1008 uses
   Opus/Sol with high effort in the game, and run relevant gates.

## Acceptance criteria

- [x] Every ticket names its correct Claude/OpenAI pair on the model line.
- [x] All other metadata, scope and lifecycle fields stay unchanged.
- [x] High-effort Opus work recommends Sol rather than Astra.
- [x] Lint accepts the four pairs and rejects ambiguous or wrong mappings.
- [x] Templates, skills, instructions and generated guidance agree.
- [x] Skill loaders remain current and relevant gates pass.

## Tests required

- Unit: approved pairs, incorrect pairings, generic labels and single ids.
- Audit: compare all original ticket YAML fields with the branch base.
- Integration: ticket lint, docs, links, skill loaders and affected tooling.

## Completion notes

Corrected all 509 existing ticket recommendations and the template to the
explicit pairs Nick requested. Opus now pairs with GPT-6.1 Sol, Sonnet
with GPT-6.1 Terra, Fable with GPT-6 Astra and Haiku with GPT-6 Luna.
Each model line contains both ids; effort stays independent. All 285
former reasoning tickets recommend Opus/Sol, and all 224 former
implementation tickets recommend Sonnet/Terra. No engine ticket recommends
Fable/Astra. No existing field other than model was changed; the audit
also checked ordinary ticket bodies against the branch base.

Updated the schema and regression test to accept the four complete pairs
and reject categories, single ids and mismatched pairs including Opus/Astra.
The backlog guide, shared AGENTS.md and write-ticket workflow agree.
CLAUDE.md still imports the shared rules and both harnesses use the same
skills. ADR-0065 amends only ADR-0064's routing; its shared layout remains.
Earlier completion notes are marked as historical. No public API break.

Validation: full native workspace tests, all 408 xtask unit/integration
tests, fmt, capture Clippy, warning-denied rustdoc, dependency/unused-crate
gates, ticket lint with the local game checkout, docs map, links, pins,
toolchain, spelling, skill validation and loader drift checks pass.
The correction touches repository tooling and documentation; no template,
public API, simulation, decoder, asset or runtime backend changes need new
worked examples or visual checks. Mutation testing remains in CI.

One bulk schema correction keeps the archived backlog valid with its
validator. Implemented in a fresh worktree from merged main; original
checkouts remain clean. The independent game correction is game#0137;
no engine pin move or merge order is required.
