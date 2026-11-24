---
id: "3350"
title: "The in-house sound keeps playing on Linux when PULSE_LATENCY_MSEC is set, or a stalled stream is treated as lost"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3032"]
nick_input: none
completed:
---

# 3350 — A CPAL stream that stalls under `PULSE_LATENCY_MSEC`

## Context

3032 found, in CI run 45163539957, that with `PULSE_LATENCY_MSEC=20` in the
environment, CPAL 0.18.2's ALSA stream on ALSA's PulseAudio plugin (`default` routed
to a PulseAudio server) calls the in-house output's callback once, then never again.
Nothing is heard and no error comes: the stream opened, CPAL reports nothing, and the
output's thread sees a stream it thinks plays. Without the variable it runs: 169
callbacks in the same scenario, 39.8 ms ahead of playing, float or 16-bit alike.
quad-snd, which writes to ALSA with blocking calls, plays either way.

The sound gates set the variable for quad-snd's sake (`tools/sound-parity/capture.py`
runs the in-house probe without it, `IN_HOUSE_WITHOUT`), and a player's system
normally doesn't. But players set it by hand: it is common advice against crackling
under Wine and Proton. On a Linux system whose ALSA `default` is PulseAudio's plugin
(PulseAudio itself, not PipeWire's ALSA route), such a player would hear nothing from a
game with the in-house sound.

[ADR-0142](../../docs/adr/0142-in-house-sound-on-linux-and-macos-quad-snds-alsa-bindings-are-alsa-syss.md)
records 3032's side.

## Nick input

None.

## Scope

**In:**
- Measure which values stall it (20 did; try 30, 60 and 100, the values the advice
  gives), and whether PipeWire's ALSA route does the same, in CI on the null sink.
- The output's thread treats a playing stream that has made no callback for a second
  as lost (`Stats::callbacks` already counts them), so it is rebuilt, and if each
  rebuild stalls too, the output goes on as having no device (`Quiet`) and says why
  once, rather than holding a stream that plays nothing.
- If a stream configuration avoids the stall (the buffer asked for, or the plugin's
  own choice), use it on Linux.
- A CI cell that plays the in-house probe with the variable set and holds it to being
  heard, or to saying once that it can't be.

**Out (do not do):**
- A fork of CPAL, or an upstream report (`AGENTS.md`, rule 12).

**Estimated size:** about 400 lines (code 150, tests and CI 200, docs 50).

## Implementation steps

1. Reproduce in CI with the values above, PulseAudio's plugin and PipeWire's.
2. The stalled-stream rule in `crates/sound-out/src/system.rs`, tested on a fake clock.
3. The CI cell.

## Acceptance criteria

- [ ] With `PULSE_LATENCY_MSEC` set, the in-house probe is heard on the null sink, or
      runs silent and says why once, never silent with nothing said.
- [ ] A stream with no callback for a second is rebuilt, tested.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the stall rule over a counter that stops.
- Integration: the probe under the variable, in CI.

## Completion notes

*(Filled in by the session that completes the ticket.)*
