---
id: "3083"
title: "Parked: Mark each closed preview and limit supported software capture"
type: feature
milestone: "Parked: networking"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2602", "2915"]
nick_input: none
completed:
---

# 3083 — Mark closed previews and limit supported capture

> **Parked** This follows the hosted preview service (2602). Nick's request puts the work on the ticket track, not into the current MVP. Nobody starts it until Nick unparks it.

## Context

An authenticated preview can add a visible per-recipient mark and platform capture controls. Windows WDA_EXCLUDEFROMCAPTURE and Android FLAG_SECURE impede selected software recorders; Microsoft says the Windows flag is not DRM and cannot stop a camera. A public Steam trailer is transcoded and cannot carry the game's window policy.

## Nick input

None.

## Scope

**In:**
- Expiring preview IDs with a visible mark bound to the invited session.
- Optional capture flags only on backends that support them, with a plain statement of the bypass.
- A record of which preview artifact or session received each ID, kept private by the operator.

**Out (do not do):**
- A promise to prevent filming, cropping, collusion or AI analysis.
- Capture restrictions on ordinary public builds, trailers or accessibility tools.
- Proprietary CDM/FairPlay integration.

## Implementation steps

1. Add an opt-in mark to the preview renderer, outside the game simulation.
2. Apply supported platform capture flags to controlled preview windows only; report unsupported targets.
3. Document privacy, expiration and operator-side ID retention.

## Acceptance criteria

- [ ] Two invitees receive distinguishable marked previews with identical game rules.
- [ ] Supported software capture behavior is checked per target; phone recapture is documented as a bypass.
- [ ] Ordinary public builds retain capture and accessibility behavior.
- [ ] The book's chapter is updated (2802), or the completion notes have a Book: line saying why not.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Integration: preview ID display and expiration, no ID in public edition.
- Manual matrix: supported OS software capture, crop and phone recapture.

## Completion notes

*(Filled in by the session that completes the ticket.)*
