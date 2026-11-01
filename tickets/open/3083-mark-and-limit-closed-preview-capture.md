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

An authenticated preview can add a visible per-recipient mark and platform capture controls. Windows WDA_EXCLUDEFROMCAPTURE and Android FLAG_SECURE impede selected software recorders; Microsoft says the Windows flag is not DRM and cannot stop a camera. A public Steam trailer is transcoded and cannot carry the game's window policy. **From 2915 (done):** [its report](../../docs/research/2915-capture-controls-marks-and-ai-signals.md) sets the scope. A faint per-recipient mark named the right 1 of 1,000 recipients on YouTube's encode and a real phone film, but a person could see it on flat backgrounds and a short cleanup erased it, so call it faint and removable, never invisible. Windows' exclusion blocked all five software paths tested and a phone filmed every frame. A tiled visible code stayed readable but missed the readability screen, so a visible mark here must be drawn away from or behind the game's text and pass that screen. macOS and Android flags were read about, not run: this ticket's matrix runs them, and Apple documents `sharingType` as not to be used for this. Test on real art, in a live session, whether the detector still works without a clean frame. Magnifiers with the flag on are also untested. The report's `verify_public.py` checks are the starting point for the public-build test.

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
