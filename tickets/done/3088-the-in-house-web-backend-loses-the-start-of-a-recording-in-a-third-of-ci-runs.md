---
id: "3088"
title: "The in-house web backend loses the start of a recording in a third of CI runs"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-03
---

# 3088 — The in-house web backend loses the start of a recording in a third of CI runs

## Context

Ticket 3031 added the in-house web backend: a worklet fed by messages
([ADR-0075](../../docs/adr/0075-in-house-sound-output-cpal-beside-quad-snd-a-worklet-fed-by-messages-and-a-virtual-endpoint-for-windows-evidence.md)).
The `sound-parity` job records it as platform `browser-in-house`
(`tools/sound-parity/README.md`).

Ticket 2042 ran the job 20 times on one revision with no sound change (CI run
41595469804, on the merge of `main` at 12e1256e into 2042's branch). In 7 of the
20 (jobs 2, 4, 10, 11, 18, 19 and 20), one or two `browser-in-house` scenarios
failed with `missing marker`, and their `repeat:` cell with them. Which scenarios
varied (`wav_44k`, `wav_22k`, `gain`, `onset`, `loop`, `loop_gapless`,
`ogg_48k`, `future_rate`, `voices_full`). The best match's correlation was
nearly always one of two values: 0.804 left and 0.809 right, or 0.837 and
0.843, against `marker_correlation` 0.95. In job 2, the recording `fault:` cells
break was one of those, so they failed too. PR 245's own run (CI run
41595404465) failed the same way on `wav_44k`. In the 47 matrices kept from
2573's merge to 2026-11-02 that 2042 gathered, only `browser-in-house` ever
failed `missing marker`, once, and no other platform did in the 20.

Two exact values in place of noise say the marker's start is cut or delayed by
the same amount each time: likely the worklet's first blocks, before the ring
has the samples, or a gap between the context resuming and the first message.

## Nick input

None.

## Scope

**In:**
- Find why the marker's start goes missing, from the 20 runs' recordings (the
  `sound-parity-run-N` artifacts of run 41595469804, while they last) and new
  ones.
- Fix it in the backend (`crates/app/src/audio/in_house.rs`, the worklet and its
  ring), so that a sound asked for is heard whole from its start.
- A test that would have caught it, below the gate if it can be (the ring's
  test, `tools/sound-parity/ring_test.js`, or the backend's own).

**Out (do not do):**
- lowering `marker_correlation` or any correctness tolerance;
- retrying a scenario that failed;
- the in-house Windows backend's gap (3089);
- timing limits (2044).

## Implementation steps

1. Download a failing job's artifact and compare its recording of a failed
   scenario with the same scenario from a passing job: where the marker starts,
   and what is missing.
2. Find what in the worklet's start leaves those samples out.
3. Fix it, and add the test.
4. Run the `sound-parity` job 20 times on one revision (2042's completion notes
   say how: a scratch branch with numbered copies of the two jobs), and keep the
   numbers in the completion notes.
5. Run the `document` skill, then the gates.

## Acceptance criteria

- [x] The cause is in the completion notes.
- [x] No `browser-in-house` cell fails `missing marker` in 20 runs of the job on
      one revision.
- [x] A test fails on the old code and passes on the fix.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit or integration: the test named above.
- CI: the 20 runs.

## Completion notes

**What was missing.** In the failing recordings of run 41595469804 (job 2's
`onset`, both runs; job 10's `loop`), the marker's first 1,280 or 1,536 frames
are silence. The rest of it is exactly where the whole marker would have put it:
every 128-frame window after the gap matches at 1.00, and every one before it at
0.00. That is 5 or 6 of the 256-frame blocks the page mixes (`BLOCK_FRAMES`), and
the two correlations the ticket saw are those two cuts: √(1 − 1280/4410) ≈ 0.84
and √(1 − 1536/4410) ≈ 0.81. The frames were mixed and then lost, not delayed.
Passing recordings of the same scenarios have the marker whole.

**The cause.** Until the `AudioContext` ran, `sound.js`'s timer mixed the frames
that time passed and discarded them, so the player's clock moved on. The click
only asks for the context to start: `index.html` calls `resume()` on
`mousedown`, and the state stays `suspended` until the device runs, about 30 ms
on CI's runner. The probe plays the marker on the frame after the click (`onset`
has no settle). So whatever the device took to start, the timer threw away from
the marker's start, a block at a time. When the device started quickly enough,
nothing was lost, which is why only some runs failed. The same happened to a
sound played as a hold let go (`shuyi_sound_hold(0)` resumes the context too).

**The fix**, in `crates/cli/web/sound.js`: once the page has had its gesture
(`navigator.userActivation.hasBeenActive`, or where a browser lacks that, a
`keydown`, `mousedown` or `touchend` seen by the plugin), a context that isn't
running yet is starting. The timer then stops discarding, and the mix waits for it.
A sound asked for on the gesture is heard whole, as late as the device is to
start. That is the contract's "a late page delays the sound". After `START_MS`
(2 seconds) of a context that hasn't run, the timer catches up on the clock and
discards as before, so one that never starts doesn't stop the player's clock.
Escape and the modifier keys, on which `index.html` resumes too but no browser
starts a context, don't count where `userActivation` exists. The contract's
comments say the same in `sound.js`, `crates/sound-out/src/web.rs` and
`crates/cli/web/README.md`.

**The test.** `tools/sound-parity/ring_test.js` now runs the whole plugin under
node, on a fake page: a clock moved by hand, a context that runs when told, a
worklet node that records what it gets, and a mix whose blocks carry their
numbers. It checks that before the gesture the mix runs and is discarded. It
checks that every block mixed from the gesture on reaches the worklet, in order,
while the context takes 40 ms to start: with `userActivation`, without it, and as
a hold lets go. And it checks that a context that never starts makes the mix wait
2 seconds, then catch up. On `main`'s `sound.js` it fails, "a block was
discarded". CI's `sound-parity` job already runs this file.

**Twenty runs.** CI run 41689870848, on the scratch branch `t3088-calibration`
(never merged): 2042's workflow, its twenty numbered copies of the
`sound-parity-windows` and `sound-parity` pair, on this fix. **No
`browser-in-house` cell failed or was inconclusive in any of the twenty**, and no
platform missed a marker. 18 of the 20 passed whole. The other two failed on
Windows' timings alone:

- job 15: `windows-in-house perf:decode_wav_ms`, median ratio 1.069, spread
  0.096: a limit from three runs, **2044**'s;
- job 18: `windows perf: no samples`. The Windows job measured `windows` three
  times. The gate found the third inconclusive (`peak_kib`), moved its
  `perf.jsonl` aside to measure it again, and can't measure Windows from Linux.
  That is a bug in the gate step's retry, now **2045**.

**Follow-ups:** 2045 (above). 3089 and 2044 are unchanged by this.

**Docs:** no public item, format or contract operation changed. The facts moved
are the web output's own contract, kept where it already lives: the comments of
`sound.js` and `web.rs`, `crates/cli/web/README.md` and the README's table in
`tools/sound-parity/`.

Book: none, no public API changed. A game's sound on the web keeps its calls,
and only now hears a sound played on the first click whole.

No break: no public API, format or golden value changed. Nick input: none.
