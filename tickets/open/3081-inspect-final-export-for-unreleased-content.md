---
id: "3081"
title: "Inspect each final export against a game's disclosure manifest"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["3080"]
nick_input: none
completed:
---

# 3081 — Inspect the final export for unreleased content

## Context

Shuyi's asset baker keeps unplanned bundle files as raw bytes (crates/assets/src/pack/bake.rs), and a game may embed files into its program. A pack equality test proves fidelity, not release safety. Build an auditable list of the bytes distributed to a player, beginning after web staging is clean (3080).

## Nick input

None.

## Scope

**In:**
- A game-authored disclosure manifest of allowed paths and forbidden paths or canary strings for an export.
- A machine-readable exposure report for final desktop, web and Playdate packages: files, pack entries, loose assets, feature set and debug companions.
- A failing export when declared forbidden material or undeclared package files are found.

**Out (do not do):**
- A claim that arbitrary game semantics are absent from machine code.
- Per-recipient watermarking, DRM, or changes to game rules.

## Implementation steps

1. Define the manifest under shuyi.ron with exact path matching and clear errors; keep it independent of target profiles.
2. Inspect the final package and pack index after all target-specific copies, and scan bounded binary/Wasm strings for author-declared canaries.
3. Write an exposure report with hashes and the resolved build inputs; make a failed gate leave no publishable archive.

## Acceptance criteria

- [ ] A harmless future-content file in a pack, loose folder or adjacent download fails with its source and package path.
- [ ] A declared canary embedded in a binary or Wasm fails the gate.
- [ ] An allowed fixture game exports on every supported target and has a reproducible report.
- [ ] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Integration: canaries in raw packs, loose files, embedded strings, debug sidecars and final archives.
- Property: report ordering and hashing are deterministic for identical inputs.

## Completion notes

*(Filled in by the session that completes the ticket.)*
