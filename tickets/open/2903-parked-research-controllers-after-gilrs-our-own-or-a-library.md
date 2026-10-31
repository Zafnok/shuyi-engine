---
id: "2903"
title: "Parked: Research: controllers after gilrs: our own code or a library, and the tickets that follow"
type: research
milestone: "Engine: in-house platform"
model: reasoning
effort: high
status: blocked
blocked_by: []
nick_input: decision
completed:
---

# 2903 — Research: controllers after gilrs

> **Parked** (Nick, 2026-10-19: controllers are "a 2 way door decision and we can make it much later in the future after additional research in a dedicated ticket"): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), and after the sound research (2902) in Nick's order. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): "The gilrs seems low stars and
barely maintained so I still plan to skip it... I feel like of
everything, controller is quite easy to add support for but this can be a
2 way door decision". The engine reads pads with gilrs on native
(`crates/app/src/pads/native.rs`) and with our own `gamepad.js` on the web.
`shuyi-ui`'s `Pads` holds all the rules, so the source can change without
touching them.

Facts on 2026-10-19, for whoever weighs this later: gilrs lives on GitLab
(`gitlab.com/gilrs-project/gilrs`), so GitHub's stars don't show it. It
had releases in January and May 2026 and commits into August 2026, and
Bevy uses it (`bevy_gilrs` 0.20). Re-check all of this on the day.

A plan for our own code is already written, parked behind this ticket:
the `shuyi-pads` crate with SDL's mapping database and Windows (2402),
then Linux and macOS (2403).
[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§1 leaves the choice to this ticket.

## Nick input

**Decision:** once the options are weighed, ask Nick (the `ask-nick`
skill) which way to go, with a recommendation.

## Scope

**In:**
- **What the engine needs from pads**: the standard layout, hot-plugging,
  the pad's kind (for button names and the Switch-style swap), sticks and
  triggers as values (1121), rumble (2408, parked: gilrs's force feedback
  on Windows and Linux, the browser's `vibrationActuator` on the web), and
  what later work may want (motion, Steam Deck controls, mobile).
- **The options:**
  - our own code, as 2402 and 2403 plan;
  - keeping gilrs, with its health measured on the day (releases, open
    issues, response times, who maintains it);
  - other libraries maintained on the day (SDL3's gamepad API through
    bindings, under Zlib, would bring a C library; others as found). For
    each: license (on the day), what it pulls in, platforms, force feedback
    per platform, and whether it needs `wasm-bindgen` (the web keeps its
    own path either way).
- **Test on real pads** where possible: an Xbox pad, a PlayStation pad, a
  Switch Pro pad, the Steam Deck.
- **Outputs:**
  - a report, `docs/research/2903-controllers-after-gilrs.md`;
  - Nick's answer in `docs/decisions.md`;
  - an ADR if the choice adds a dependency or changes ADR-0021;
  - 2402 and 2403 kept, rewritten or closed (moved to `tickets/done/` with
    a completion note saying why).

**Out (do not do):**
- Building the replacement.
- Reading the source of gilrs, SDL or any other library to copy it
  (ADR-0021 §2). Reading documentation to judge them is fine.

## Implementation steps

1. List the needs from the code, 1121 and the input tickets.
2. Survey the options and fill a table in the report.
3. Try the shortlist on real pads, in a scratch folder.
4. Ask Nick, with a recommendation, and record his answer.
5. Update or close 2402 and 2403, and `docs/ROADMAP.md`'s order.

## Acceptance criteria

- [ ] The report covers each option, with the facts dated.
- [ ] Nick's answer is in `docs/decisions.md`, verbatim.
- [ ] 2402 and 2403 match the answer, and the ticket lint passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it keeps or writes say their tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
