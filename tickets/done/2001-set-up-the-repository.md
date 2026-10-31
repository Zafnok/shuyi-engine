---
id: "2001"
title: "Set up the repository: plans, tickets, decisions, skills and licenses (no code)"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: decision
completed: 2026-10-14
---

# 2001 — Set up the repository

## Context

Nick, 2026-10-14, after the feasibility study (game ticket 1101): plan B,
a separate repository for the engine, and "Let's do the very initial
setup in this session (new github repo, push up tickets and ADRs etc, push
up useful dev skills like the ones we have in this repo for dev of the
engine). No actual engine work to be done in this session."

## Nick input

**Decision:** the name, public or private, and the license (all answered;
`docs/decisions.md`).

## Scope

**In:** the repository on GitHub; `README.md`, `AGENTS.md`, `CLAUDE.md`,
`CONTRIBUTING.md`, the licenses; `docs/decisions.md`, `docs/ROADMAP.md`;
the ADRs; the tickets moved from the game repository and new ones; the
development skills.

**Out (do not do):** any engine code; CI (2002).

## Implementation steps

1. Ask Nick the name, visibility and license; record his words.
2. Create `Zafnok/shuyi-engine`, public.
3. Write the files; move ADRs 0064–0067 and tickets 1119–1129, 1132–1135
   and 1140–1150 from the game repository.
4. Port the game's development skills.

## Acceptance criteria

- [x] The repository exists, public, with both license files.
- [x] Nick's decisions are recorded verbatim.
- [x] The moved ADRs and tickets are here and gone from the game
      repository (game PR for ticket 1101).
- [x] The skills `work-ticket`, `write-ticket`, `write-adr`, `run-gates`,
      `input-actions` and `ask-nick` exist.

## Tests required

None (no code).

## Completion notes

Done 2026-10-14 in the same session as game ticket 1101.

- **Names.**
  - Nick named the engine **Shuyi Engine**, after two rounds of
    suggestions he turned down.
  - Crates are `shuyi-*`; the binary is `shuyi`.
  - The game's carve tickets were updated to use these names from the
    start, so no rename ticket is needed (the game's 1130 was dropped).
- **Formats.** Nick's message about formats added ADR-0007 and the
  tickets 2201–2204 and 2901.
- **ADR numbering.** The moved ADRs are renumbered here:

  | Game ADR | Here |
  | -------- | ---- |
  | 0064 | 0003 |
  | 0065 | 0004 |
  | 0066 | 0005 |
  | 0067 | 0006 |

- **No CI yet.** The gates come with 2002, and CI for code with game
  ticket 1131.
