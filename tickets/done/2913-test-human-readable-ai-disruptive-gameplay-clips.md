---
id: "2913"
title: "Test gameplay clips that remain readable while disrupting video AI"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2912"]
nick_input: none
completed: 2026-11-01
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

- [x] The report measures delivered playable imitation quality, not only caption accuracy.
- [x] Ordinary transcode and human-description bypasses are included.
- [x] A failed robustness or viewer-quality test ends the feature proposal.
- [x] All gates in the run-gates skill pass.

## Tests required

- Reproducible capture fixture and hash of each evaluated encode.
- Blind human viewing and gameplay-scoring protocol.

## Completion notes

**Recommendation: no-go; the feature proposal ends.** The
[report](../../docs/research/2913-human-readable-ai-disruptive-clips.md) has the
rules, fixed before the first attempt, and every result.

- Added the `clip-capture` crate (engine raster captures of the 2912 fixtures),
  six candidate filters outside the release path, a tool-free attacker whose
  programs are scored in a sealed container by the 2912 grader, and a blind
  human kit with its scoring. 98 attempts and 18 describer turns cost about
  US$30, offered no tools and used none.
- All six candidates failed robustness. Five were bypassed by Sonnet and Opus on
  untouched frames and after a transcode. Motion-defined form, added during the
  study at the owner's suggestion, stopped every stills attacker (0/3 lossless,
  0/3 transcoded), and its short decoder undid it (3/3 lossless, 2/3 transcoded).
- A builder given only a written description passed the floor (Opus, 1/3 on the
  ordinary clip). Nothing done to pixels closes that route.
- Hidden low-contrast false rules were read by all six models' turns that saw
  them losslessly, named as suspicious, and not acted on.

**Deviations, all stated in the report.** These were not run, because each could
only add a bypass and the stop rules had already ended the proposal: human
viewers, gameplay scoring by people, a real phone recording (`recapture` is a
simulation), a person as the describer or helper, an attacker with tools, and
any non-Anthropic model. The scope names phones, several model families and a
human-assisted attacker; a future *go* would need them all. The decoder was
written by this study and handed to the attackers, so its result does not
measure how fast an attacker would find it.

**Nick:** one thing here is yours if you want it. Motion-defined clips do stop
someone who only pastes frames into a chatbot, at a cost of 93 times the video
bytes, no colour and a screen of moving static whose comfort nobody has tested.
Under the rules that is a no-go as protection, so no ticket follows. Nothing in
the engine changed. No public API changed, so there is no `Breaking:` line.

Handed on: ticket 2915 now records the result and the first real phone test.
The report is the docs home; no other docs change.
