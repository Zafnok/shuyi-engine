---
id: "2050"
title: "The in-house web backend still loses the start of a recording in CI, after 3088"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2050 — The in-house web backend still loses the start of a recording in CI

## Context

Ticket 3088 found that `crates/cli/web/sound.js` threw away the frames mixed before
the `AudioContext` ran. It made the mix wait for a context that is starting.
Ticket 2048 then ran the `sound-parity` job twenty times on one revision (CI run
42389680314, branch `t2048-calibration`, never merged). Two of the twenty failed on
`browser-in-house` alone, the way 3088's did:

- job 9: `browser-in-house scenario:loop: failed: loop.wav differs by 13711 at frame
  0 of 66150, over 2 (pass 1)`;
- job 20: `browser-in-house scenario:ogg_44k` and `repeat:ogg_44k: failed: missing
  marker: its best match has correlation 0.943 on the left and 0.947 on the right,
  under 0.95`. By 3088's arithmetic, √(1 − x/4410) ≈ 0.943 is about 490 frames
  missing, two of the page's 256-frame blocks.

Their artifacts are `sound-parity-run-9` and `sound-parity-run-20` of that run,
while they last. quad-snd's web backend (`browser-chromium`) passed in all twenty.

## Nick input

None.

## Scope

**In:**
- Find what each failing recording lost, and where: the start of the marker, as in
  3088, or something else.
- Fix the cause in the in-house web backend (`sound.js` and its worklet), or in the
  capture if it is the capture's (`capture.py browser`).
- Run the `sound-parity` job twenty times on one revision and keep the result in
  the notes.

**Out (do not do):**
- a lower `marker_correlation`, or any limit raised to pass;
- quad-snd's recordings (2048, 2049) and the in-house Windows backend (3089).

## Implementation steps

1. Read 3088's completion notes and its fix in `crates/cli/web/sound.js`.
2. Open jobs 9's and 20's `browser-in-house` recordings and compare them with a
   passing job's, window by window, as 3088 did.
3. Fix the side that loses them, and test it as 3088's tests do.
4. Run the twenty jobs.

## Acceptance criteria

- [ ] The notes say what jobs 9 and 20 lost and why.
- [ ] Twenty `sound-parity` jobs on one revision pass on `browser-in-house`, or
      each failure is named in the notes with its cause.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: whatever 3088's tests are for the part changed.
- Integration: the twenty runs above, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
