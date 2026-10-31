---
id: "2037"
title: Make tickets and agent workflows provider neutral
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-31
---

# 2037 — Make tickets and agent workflows provider neutral

## Context

Nick requested this migration across the engine and game on 2026-10-31.
Existing ticket recommendations and agent roles assume one provider.
The companion migration is game#0136; both repositories can adopt the
workflow independently without an engine pin move.

## Nick input

None. Nick authorized all ticket rewrites and shared harness support.

## Scope

**In:** all open, parked and done ticket recommendations and agent roles;
ticket templates, lint and routing guidance; shared instructions; portable
skills and discovery in Claude Code and Codex; checks for loader drift.

**Out (do not do):** game design, engine APIs, pin changes, new model
benchmarks, provider account settings, starting any parked implementation.
Historical comparisons, quotations and real integration paths keep their
meaning. The companion migration needs no output from this ticket.

## Implementation steps

1. Map old recommendations to mechanical, implementation, reasoning and
   creative capabilities, preserving effort and lifecycle fields.
2. Update the template and ticket lint; test supported capabilities and
   rejection of provider model ids.
3. Use AGENTS.md for shared instructions and a CLAUDE.md import. Remove
   provider-specific actor and execution requirements from owned skills.
4. Generate Codex discovery loaders pointing at the existing shared skill
   sources; check drift locally and in CI. Preserve pinned engine skills.
5. Document the decision and verify the migrated backlog and loaders.

## Acceptance criteria

- [x] Every ticket uses a provider-neutral capability recommendation.
- [x] Existing ticket scope, lifecycle fields and blockers are preserved.
- [x] Both harnesses discover the same workflows and shared instructions.
- [x] Ticket lint accepts capabilities and rejects provider model ids.
- [x] Shared skill loader checks detect missing or outdated loaders.
- [x] Relevant checks pass; any environment limitations are recorded.

## Tests required

- Unit: routing schema accepts all capabilities and rejects model ids.
- Integration: lint the complete migrated backlog and validate skill loaders.
- Docs: spelling and links (and the engine's docs map).

## Completion notes

Historical routing below is amended by 2038 and ADR-0065;
current recommendations use the explicit pairs in tickets/README.md.

Migrated all 508 existing tickets, plus the template and backlog guide.
An independent YAML audit compared every lifecycle field, effort and blocker
with main. Recommendations now use mechanical, implementation, reasoning
or creative; the lint rejects provider model ids. Examples are optional
owner preferences in tickets/README.md. Historical evidence and actual
harness integrations retain their names. Ticket 2525 now lets Nick choose
the first eval adapter, with Claude-specific command examples marked as such.

AGENTS.md is shared; CLAUDE.md imports it. Eleven Codex skill loaders read
the existing portable workflows and resources. The generator and CI check
catch missing/stale loaders and metadata drift, and refuse to overwrite an
unmanaged skill. Every loader and owned source passed skill validation.
Generator failure modes were exercised in an isolated fixture checkout.
ADR-0064 and the docs map record the arrangement. No public API break.

Validation: native workspace tests (one fixture-build check rerun successfully
without the inherited CARGO_TARGET_DIR setting); all 407 xtask tests;
fmt, Clippy including capture, rustdoc with warnings denied, dependency and
workflow audits, unused dependencies, spelling, links, ticket lint against
the local game checkout, layers, modules, pins, toolchain and no-std checks.
Platform builds and no-std tests were also run before pushing.

This is one bulk migration requested by Nick. The size comes from updating
archived recommendations together with the schema; splitting those would
leave a backlog that fails lint. No parked implementation was started.
Work is in a fresh worktree from main, branch t2037-portable-agent-workflows;
the original checkout was restored clean to main after a byte-verified move.
The game adopts its independent half in game#0136; no pin move is needed.

Merged the latest main to resolve the ADR index collision: the audio ADR
keeps 0063, portability becomes 0064, and new tickets 3077 and 3078 use
implementation capability while retaining their scope and lifecycle fields.
