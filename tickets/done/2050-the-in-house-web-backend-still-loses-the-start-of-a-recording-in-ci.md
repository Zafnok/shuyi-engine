---
id: "2050"
title: "The in-house web backend still loses the start of a recording in CI, after 3088"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-08
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

- [x] The notes say what jobs 9 and 20 lost and why.
- [x] Twenty `sound-parity` jobs on one revision pass on `browser-in-house`, or
      each failure is named in the notes with its cause.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: whatever 3088's tests are for the part changed.
- Integration: the twenty runs above, in CI.

## Completion notes

**What jobs 9 and 20 lost: nothing.** Not the start of the marker, as in 3088.
In both, the marker's first 256-frame block is where it should be. Then come
exactly 256 frames of digital zero on both channels, then the rest of the marker,
whole, 256 frames late. Job 9 is `run-1/loop.pcm`, the hole 256 frames after the
first sound. Job 20 is `run-0/ogg_44k.pcm`, the same. Every 128-frame window after
the hole matches at 1.00 (0.97 for the Ogg file, as when whole). The ticket's
"about 490 frames missing" read 0.943 as a whole marker cut. The Ogg marker matches
at 0.975 at best, and 0.975 × √(1 − 256/4410) ≈ 0.946 is one block. That is the
worklet running dry for one block, an underrun. In those two jobs, and in jobs 1 and
2 beside them, no other `browser-in-house` recording has a hole inside a sound.

**Why.** A traced build (scratch branch `t2050-trace`, CI run 42502605217: 8 jobs,
every in-house web scenario three times, 408 recordings) logged each worklet call
and each word the page answered. On CI's Chromium the device takes 512 frames a
callback (`baseLatency` 11.6 ms), rendered as four 128-frame quanta at once. The
worklet's first call comes about 13 ms before the page sees `statechange`. The lead
was 4 blocks (1,024 frames), so it rode out a page that missed only about one
callback. About 100 ms in, the page answered nothing for 25 ms, around the click's
release. In 18 of 360 traced recordings that left the ring holding 128 frames,
one quantum from a hole. In jobs 9 and 20 the late page came a few milliseconds
after the first sound. The lead grows a block after the worklet runs dry, but by
then the first sound already has its hole.

**The fix**, in `crates/cli/web/sound.js`: until the context has played
`SETTLE_MS` (1 second) after it starts, or after a hold lets go, the lead is at
least `START_LEAD_BLOCKS` (8 blocks, 46 ms). That rides out a page late by about
four callbacks. From connect the worklet is 8 blocks ahead, so a sound asked for
on the gesture waits as long as the sounds after it. Every sound in that first
second is 23 ms later than before, all by the same amount, and their timing against
each other doesn't move. Then the lead comes back down to its own (4, or more if
the worklet ran dry), and sounds asked for in the next 23 ms come that much sooner.
The perf workload measures latency 1.5 s after the gesture, after the lead is back,
so `latency_ms` and `old_new` don't move. A permanently larger lead would have
pushed `old_new`'s latency ratio (about 1.8 of quad-snd's, limit 2.46) over its
limit. The contract's comments say this in `sound.js`, `crates/sound-out/src/web.rs`
and `crates/cli/web/README.md`.

**The test.** `tools/sound-parity/ring_test.js` now drives the plugin with a
device like CI's: 512-frame callbacks, four quanta at once, a word every block, and
a page that answers after each callback unless it is busy. A page busy for 35 ms
just after the device starts runs `main`'s plugin dry for 1,024 frames, and the
fixed plugin not at all. It also checks that the worklet is 8 blocks ahead before
the gesture, that the lead comes back to 4 after a second without a frame of
silence, and that a hold let go gets the start's lead again. It runs under node in
CI's `sound-parity` job. Locally it ran under the node Zed ships.

**Twenty runs.** CI run 42507044770, scratch branch `t2050-calibration` (72b446cf:
this change plus `ci.yml`'s two sound-parity jobs twenty times, never merged). The
copies skip one test, `giving_up_covers_all_the_chain_added`, because the scratch
workflow names the sound replacement. Run 42504058170 before it failed only that
test in every job, before recording anything. **All 1,120 judged
`browser-in-house` cells passed in all twenty jobs** (60 `unsupported`, as always).
No in-house web recording of the 680 has a hole beyond the scenarios' own gaps. 14
of 20 jobs passed whole. All 20 `sound-parity-windows` jobs passed. The six failures
are on other platforms:

- job 6: `windows-in-house perf:underruns`, and jobs 10 and 17:
  `windows-in-house old-new:underruns`, one gap while everything played: 3089's;
- job 13: `windows-in-house old-new:cpu_ms`, median ratio 4.093 against 4.0;
- job 2: `browser-chromium perf:cpu_ms`, median ratio 1.051 against 1.05;
- job 15: `browser-chromium perf:file_io_ms`, median ratio 1.333 against 1.25.

No open ticket owned the last three, so **2068** is written for them. 2064's ledger
note says which cells are 3089's and which 2068's.

**A mistake on the way.** The scratch trace commit (c24db844) went to `main`, not
to `t2050-trace`. The branch tracked `origin/main`, `push.default` is `upstream`,
and the owner's token bypassed the ruleset. It changed only the traced `sound.js`, a
scratch workflow and `trace.py`. The game's engine watcher ran on it and only
updated its `engine-pin` issue. Zafnok/shuyi-engine#319 reverts it exactly, back to
26ae83bb. This branch starts from 26ae83bb.

**Follow-ups:** 2068 (above).

**Docs:** no public item, format or contract operation changed. The facts moved
are the web output's own contract, kept where it already lives: the comments of
`sound.js` and `web.rs`, `crates/cli/web/README.md` and the README's table in
`tools/sound-parity/`.

Book: none, no public API changed. A game's sound on the web keeps its calls;
only the first second after the device starts gets a longer lead.

No break: no public API, format or golden value changed. Nick input: none.
