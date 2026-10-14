---
name: write-adr
description: Record a technical/architecture decision as an ADR in docs/adr/. Use when a ticket introduces a new pattern, file format, widely-used dependency, crate, public API shape, or changes/supersedes an existing ADR.
---

# Write an ADR

1. Next number: highest `docs/adr/NNNN-*.md` + 1. Check open PRs for
   numbers already taken (`gh pr list --state open`).
2. Copy `docs/adr/0000-template.md` to `docs/adr/NNNN-kebab-title.md`.
3. Fill in four sections:
   - **Context**: the facts and forces, with sources;
   - **Decision**: concrete enough to follow without guessing, naming
     crates, types and files;
   - **Consequences**: good and bad;
   - **Alternatives considered**: each with why it lost.
4. Add a row to the table in `docs/adr/README.md`.
5. **Superseding:** never rewrite an accepted ADR's decision. Write the new
   ADR, then change only the old one's `Status:` line to `Superseded by
   ADR-NNNN`, and update both rows in the index.
6. Include the ADR in the same PR as the ticket that needed it.
7. Refer to the game's ADRs as "game ADR-NNNN" (ADR-0001).

**Product decisions are not ADRs.** These are Nick's and go in
`docs/decisions.md` through the `ask-nick` skill:

- the name;
- the license;
- going public;
- contributions;
- what is parked;
- what the game gets.
