---
id: "2915"
title: "Research capture controls, watermarks and AI-use signals for previews"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2915 — Research capture controls, watermarks and AI-use signals

## Context

A human-readable clip is not known to be reliably AI-inaccessible. Windows capture exclusion and protected media block some software recorders, while a camera or human description bypasses them. Watermarks, C2PA and crawler/AI-training preferences serve attribution or training choices, not universal model refusal. Study the controlled-preview boundary before a production feature.

## Nick input

None.

## Scope

**In:**
- A target matrix for Windows, web, macOS, Android and public store/social playback: capture API behavior, accessibility and recapture bypasses.
- Per-recipient visible/invisible mark survival under crop, transcode, collusion and phone recording.
- C2PA, text marks and AI-use signals separated into training preference, provenance and inference refusal claims.

**Out (do not do):**
- Proprietary CDM integration or a claim that public trailers cannot be recorded.
- Prompt-injection notices intended to force an agent to refuse.

## Implementation steps

1. Build a permissioned preview capture matrix with ordinary OS tools and a camera.
2. Measure mark survival and human legibility on original and transcoded captures.
3. Publish a precise recommended opt-in scope for closed previews and reject unsupported protection claims.

## Acceptance criteria

- [ ] The report distinguishes software capture friction from secrecy and leak attribution.
- [ ] Public platform transcode and phone recapture are tested.
- [ ] Rights and training signals are not described as forcing AI refusal.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Reproducible capture and transcode fixtures; manual device results recorded.
- Verification that public builds have no capture restrictions.

## Completion notes

*(Filled in by the session that completes the ticket.)*
