# ADR-0064: Portable agent workflows and capability routing

- **Status:** Accepted; model routing amended by ADR-0065
- **Date:** 2026-10-31
- **Related tickets:** 2037

## Context

Nick uses multiple providers and requested shared workflows for both
Claude Code and Codex. Ticket recommendations must describe the work's
needs without requiring a model or provider. Existing skill paths are
already linked by tickets and tooling; the game copies engine skills at
its pin and must keep those copies exact.

## Decision

Use the four capabilities documented in tickets/README.md in the existing
model field, preserving effort. Migrate open, parked and done tickets;
keep historical test evidence and real integration identifiers meaningful.
The lint validates capabilities rather than model ids. Example model
families are optional owner preferences, never equivalence guarantees.

Codex's discovery layout follows the [official skills documentation](https://learn.chatgpt.com/docs/build-skills).

AGENTS.md is the common instruction source; CLAUDE.md imports it. Skills
remain in .claude/skills for existing discovery, links and pin tooling.
The generated .agents/skills manifests share their metadata and direct
the reader to the same source and resources. They contain no second copy
of a workflow. Loader descriptions omit angle brackets for Codex manifest
validation; the shared source stays intact. scripts/sync-agent-skills.ps1 writes the loaders; its
-Check mode runs locally and in CI. Regenerate after any shared skill
metadata change or pinned engine skill-pack update. Keep harness-specific
configuration in each harness's own files. Required checks run explicitly
even when a harness does not support the other's hooks.

## Consequences

Either harness can use the workflows without Windows symlink privileges.
Changing a shared skill's body takes effect for both. Discovery metadata
changes need regeneration, and CI catches drift. This does not change the
engine pin or add Codex support to the engine's public game generator;
it covers these two development repositories.

## Alternatives considered

- Per-provider ticket recommendations: duplicate routing and age quickly.
- Independent skill copies: workflow drift, especially after pin updates.
- Moving every skill: breaks existing links and the game's pin updater.
- Symlinked skill directories: inconsistent Windows checkout support.
