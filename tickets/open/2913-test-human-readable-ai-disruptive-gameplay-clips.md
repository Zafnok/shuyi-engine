---
id: "2913"
title: "Test gameplay clips that remain readable while disrupting video AI"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2912"]
nick_input: none
completed:
---

# 2913 — Test human-readable AI-disruptive gameplay clips

## Context

CVPR and AAAI research has degraded selected video models with subtle perturbations, but no cited study established resistance to adaptive video-to-game cloning after social transcoding or recapture. **From 2912 (done):** the synthetic footage runner scores shown and held-out frames and winning paths, but its shared-worktree pilot had no hard blind boundary or human gameplay rubric. This ticket must add those before any Shuyi capture feature is promised.

## Nick input

None.

## Scope

**In:**
- A bounded capture experiment with ordinary and perturbed clips from permissioned Shuyi games.
- Re-encoded, cropped, extracted-frame and phone-recorded variants, multiple model families and a human-assisted attacker.
- Human viewer comprehension, comfort and ability to identify the original game.

**Out (do not do):**
- A production video filter or a claim that AI cannot view public footage.
- Glaze/Nightshade-style training poisoning as a substitute for inference testing.

## Implementation steps

1. Implement candidate perturbations outside the engine's release path using its deterministic frame captures.
2. Run blinded footage-to-playable-game trials with the 2912 grader.
3. Report transfer, capture quality, maintenance cost and a stop/go recommendation.

## Acceptance criteria

- [ ] The report measures delivered playable imitation quality, not only caption accuracy.
- [ ] Ordinary transcode and human-description bypasses are included.
- [ ] A failed robustness or viewer-quality test ends the feature proposal.
- [ ] All gates in the run-gates skill pass.

## Tests required

- Reproducible capture fixture and hash of each evaluated encode.
- Blind human viewing and gameplay-scoring protocol.

## Completion notes

*(Filled in by the session that completes the ticket.)*
