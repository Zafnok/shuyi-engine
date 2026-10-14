# ADR-0001: Record architecture decisions

- **Status:** Accepted
- **Date:** 2026-10-14
- **Related tickets:** 2001

## Context

The engine's technical choices are made by Claude, and its owner (Nick)
doesn't review code. Agents and people who arrive later need to know what
was decided and why, without reading every pull request. The game this
engine came from has kept ADRs since its first day (game ADR-0001), and
they are a large part of why its tickets can be worked without
questions.

## Decision

- Significant technical decisions are recorded as ADRs in `docs/adr/`,
  numbered from 0001, with the `write-adr` skill and the template
  `0000-template.md`.
- An ADR is never edited to change its meaning after it is Accepted. To
  change a decision, write a new ADR that supersedes it and change only
  the old one's `Status:` line.
- Product decisions (name, license, going public, contributions, what is
  parked) are Nick's and live in `docs/decisions.md`, not in ADRs.
- In this repository "ADR-NNNN" means this repository's ADRs; "game
  ADR-NNNN" means the game repository's
  (`github.com/Zafnok/visions-of-shuyi/tree/main/docs/adr`). The game's
  ADRs remain the background for most of the rules here (ADR-0002).
- ADR-0003 to ADR-0006 were first proposed in the game repository as its
  ADR-0064 to ADR-0067 (game ticket 1101) and moved here when this
  repository was made. The game's ADR-0063 (the split itself) stays
  there.

## Consequences

- Every ticket can link the decisions it follows, so a weaker model can
  work it.
- Two ADR series exist; the "game ADR" prefix keeps them apart.

## Alternatives considered

- **One shared ADR series across both repositories** — the engine must
  stand alone for its users, who never see the game repository's
  numbering.
- **No ADRs, decisions in pull requests** — they get lost.
