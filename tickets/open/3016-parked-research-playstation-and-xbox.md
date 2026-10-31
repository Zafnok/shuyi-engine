---
id: "3016"
title: "Parked: Research: PlayStation and Xbox, their partner programs, costs, toolchains and whether a Rust engine can ship there"
type: research
milestone: "Parked: export targets"
model: reasoning
effort: high
status: blocked
blocked_by: ["3003"]
nick_input: decision
completed:
---

# 3016 — Research: PlayStation and Xbox

> **Parked** (Nick, 2026-10-15: "and maybe even mobile and console"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§2. Like Nintendo's (3014), these consoles' SDKs are under agreements,
so any backend is private. A few things are known in public and need
checking:

- Xbox has the ID@Xbox program, and Microsoft's GDK needs MSVC (CI and a
  build machine only, never Nick's, per `AGENTS.md`).
- PlayStation has its partner program.

How well Rust works on each is not public. There is no profile for
either until this research reports.

## Nick input

**Decision** (the `ask-nick` skill), after the report: whether to apply
to either program, given its costs and duties.

## Scope

**In:**
- **A public report** in `docs/targets/consoles.md`, built only from
  public sources, each with its date:
  - how to apply;
  - what it costs (kits, fees, a company);
  - the toolchain (C/C++ only, or whether Rust is known to have shipped);
  - certification in broad terms;
  - how a build shows its third-party license notices with no folder a
    player can browse (2223 leaves an engine screen of credits and
    notices out until a console needs one);
  - porting partners as an alternative.
- **A recommendation** for each console: apply, use a partner, or skip.
- **The `ask-nick` question.**
- **Public engine tickets, for a yes:** what these consoles need that
  may be said in public, unless an open ticket already has one. The
  first is pausing a one-player game when its pad is removed, which
  2406 leaves to the consoles. If 2406 is done, that ticket builds on
  its pad removals (`PadRemoved`, `seat_events`); if not, it is blocked
  by 2406 and parked like it.

**Out (do not do):** applying, signing, or anything under an agreement.

## Implementation steps

1. Research from public sources only.
2. Write the report and the recommendation.
3. Ask Nick, and record the answer.

## Acceptance criteria

- [ ] `docs/targets/consoles.md` exists, with sources and dates.
- [ ] Nick's answer is recorded, and tickets are written for a yes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
