---
id: "3014"
title: "Parked: Nintendo Switch and the eShop: join Nintendo's developer program, check Rust on its toolchain, and plan a private backend"
type: research
milestone: "Parked: export targets"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3003", "3005", "3008"]
nick_input: decision
completed:
---

# 3014 — Nintendo Switch and the eShop

> **Parked** (Nick, 2026-10-15: "and eventually nintendo eshop"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§1–2. The `switch` profile exists as `planned` (3001), with a
1280 × 720 handheld screen, 1920 × 1080 docked, and 60 fps. Facts known
in public (2026-10-15; re-check them):

- **Registering costs nothing.** Joining Nintendo's developer portal is
  free, and its tools and documents are under a non-disclosure
  agreement.
- **The hardware costs money.** A Switch development kit cost about
  50,000 yen in 2017. Access to Switch 2 kits was limited in 2025, by
  press reports.
- **Rust.** Rust has a tier-3 target, `aarch64-nintendo-switch-freestanding`.
  Whether Nintendo's toolchain and certification accept a Rust game is
  not public, and finding out is this ticket's first question.

The engine's part is public:

- a stable `Platform` trait (3003);
- `no_std`-ready crates (3005);
- budgets (3008).

The backend itself goes in a private repository under Nintendo's
agreement, and never comes here.

## Nick input

**Decision** (the `ask-nick` skill), when unparked:

- Switch 2, original Switch, or both?
- Is it worth the costs? These are the kit, and any company or legal
  setup Nintendo asks for. A lawyer's look at the agreement is worth it.
- Self-publish, or go through a porting or publishing partner? Several
  studios port indie games under their own agreements.

**Setup**, if he says yes: register at Nintendo's developer portal under
his name or company, accept the agreement, and apply for the platform.
The session gives the exact steps then.

## Scope

**In:**
- the decision above;
- after access, a spike in the **private** repository: does `shuyi-core`
  build and run a replay on the kit with Nintendo's toolchain;
- filling in the `switch` profile (inputs: Joy-Con pairs, a single
  sideways Joy-Con, the Pro Controller; the reserved buttons) with only
  what may be said in public;
- the backend tickets, in the private repository;
- a public note in `docs/targets/switch.md` with only public facts.

**Out (do not do):**
- **anything under the agreement in this repository**: no SDK names
  beyond public ones, no API details, no certification rules;
- buying hardware or signing anything for Nick.

## Implementation steps

1. Wait for Nick to unpark it and decide.
2. Re-check the public facts above.
3. After Nick's setup, run the spike in a private repository, and
   record in public only "works" or "doesn't", with no details.
4. Write the private backend tickets, and the public profile changes.

## Acceptance criteria

- [ ] Nick's decision is recorded in `docs/decisions.md`.
- [ ] If yes: the spike's outcome is recorded, the `switch` profile is
      filled in from public facts, and the private tickets exist.
- [ ] Nothing under Nintendo's agreement is in this repository.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None in this repository.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
