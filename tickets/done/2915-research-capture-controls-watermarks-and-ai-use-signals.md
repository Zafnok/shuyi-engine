---
id: "2915"
title: "Research capture controls, watermarks and AI-use signals for previews"
type: research
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-01
---

# 2915 — Research capture controls, watermarks and AI-use signals

## Context

A human-readable clip is not known to be reliably AI-inaccessible. Windows capture exclusion and protected media block some software recorders, while a camera or human description bypasses them. Watermarks, C2PA and crawler/AI-training preferences serve attribution or training choices, not universal model refusal. Study the controlled-preview boundary before a production feature. **From 2913 (done):** [no-go on clip filters](../../docs/research/2913-human-readable-ai-disruptive-clips.md): a viewer's written description alone rebuilt the shown loop, and hidden low-contrast text was read and disbelieved or compressed away. Its real phone recording was not run, so this ticket's camera arm is the first real one. Its capture crate and encode runner, in `tools/research/2913/` while that folder exists, can serve as the fixtures here.

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

- [x] The report distinguishes software capture friction from secrecy and leak attribution.
- [x] Public platform transcode and phone recapture are tested.
- [x] Rights and training signals are not described as forcing AI refusal.
- [x] All gates in the run-gates skill pass.

## Tests required

- Reproducible capture and transcode fixtures; manual device results recorded.
- Verification that public builds have no capture restrictions.

## Completion notes

**The [report](../../docs/research/2915-capture-controls-marks-and-ai-signals.md)
has the rules, fixed before the first measurement, and every result.** Capture
friction, secrecy and leak attribution are answered apart.

- **Capture:** on Windows, the exclusion flag blocked all five software paths tested
  in the same run that showed each one working with the flag off. A phone filmed all
  14 segments with the flag on. Web and public playback have no such control.
- **Marks:** a faint per-recipient mark named the right 1 of 1,000 recipients on
  YouTube's real 720p encode and a real 4K phone film. Up to 5 averaged copies still
  named a colluder. A short cleanup, written knowing the mark, erased it on these
  flat fixtures. The owner could see it from the weakest strength.
- **Signals:** sorted into training preference and provenance. No signal found
  supports inference refusal.
- **Public builds:** no restriction in the source, the backends or a built game, and
  a real game window and a WebGL page were captured.

**Recommendation for 3083:** a mark described as faint and removable, and Windows'
flag as friction with its bypass stated. A tiled visible code stayed readable but
missed the readability screen by 0.2 of a point, so it is not recommended as drawn.
The handoff is written in 3083.

**Deviations, all stated in the report.** The Print Screen path was dropped after it
opened Snipping Tool. Two arms were added after the first results. Phone alignment
and flash matching changed after seeing the film, checked on its unmarked segment.
macOS, Android, iOS, Steam, TikTok, X and Discord were not run: documents only, and
3083's per-target matrix runs the devices. "Human legibility" is a tool-free reader
model and one non-blind person. Gates: the repository's checks, fmt, clippy, tests,
docs, deny and machete ran locally. The web, no-std, template and skill jobs were
left to CI, since no Rust, template or skill changed.

**Nick:** the two real recordings were yours, thank you. Nothing here is a product
decision yet: 3083 stays parked until you unpark it. The phone film shows your room
and a reflection, so it and its frames stay on your machine in the ignored `target/`.

Nothing in the engine changed. No public API changed, so there is no `Breaking:`
line. The report is the docs home; no other docs change, except one sentence in 2913's
report saying 2915 re-uses its capture crate. `git rm -r tools/research/2915` removes
the study.
