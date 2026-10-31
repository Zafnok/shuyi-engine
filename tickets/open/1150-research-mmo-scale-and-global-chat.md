---
id: "1150"
title: "Parked: Research: what MMO-scale play and global chat would take — zones, interest management, persistence, running costs, legal duties"
type: research
milestone: "Engine: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["1147", "1149"]
nick_input: none
completed:
---

# 1150 — Research: MMO scale and global chat

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Unparked early only if the game's ticket 0053 picks an online mode. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-14, asked for support "for MMO, and global chat",
phased from easy to hard. ADR-0006 §8–§9 put both last. Each is a
commitment more than a library:

- **An MMO** needs:
  - authoritative zone servers;
  - interest management (each player only hears about what is near);
  - sharding or zoning;
  - persistence;
  - accounts;
  - an economy that can't be duplicated;
  - operations.

  The research estimated 40–80+ tickets and hundreds to thousands of
  dollars a month. Project Gorgon once put it at up to $2,000 a month
  per shard of about 800 players.
- **Global chat** needs moderators and a legal process (1149's list).

This ticket gives Nick a costed answer before any of it is built.

## Nick input

None to do the research. The report ends with a go or no-go question
for him.

## Scope

**In:**
- Build on 1147's snapshot model. Spike interest management (a grid of
  cells, each client subscribed to nearby cells) with 500 simulated
  clients on one machine. Measure CPU, memory and bandwidth.
- Design sketches for:
  - zones and moving between them;
  - persistence (what is saved, how often, crash safety);
  - an item and economy model that can't be duplicated (server-side
    only, transactions);
  - accounts (from 1148);
  - deployment (a VPS per zone, or Agones on Kubernetes, Apache-2.0;
    Edgegap as a commercial option).
- Global chat:
  - moderation staffing per number of players;
  - the legal checklist with sources;
  - what Steam and the age-rating boards require;
  - vendor options and their terms.
- A report in `docs/research/mmo-and-global-chat.md`. It covers costs
  per month at 100, 1,000 and 10,000 concurrent players, a ticket
  estimate, the risks, and a recommendation.

**Out (do not do):**
- building any of it;
- choosing a vendor without Nick.

## Implementation steps

1. Write the spike (not committed) and its measurements.
2. Write the sketches and the legal research.
3. Write the report and present it to Nick in plain words.

## Acceptance criteria

- [ ] The report exists, with sources and measured numbers.
- [ ] Nick's answer (go or no-go, and for what) is recorded in it.
- [ ] If go: tickets exist, and `cargo xtask ticket-lint` passes.

## Tests required

None (research).

## Completion notes
