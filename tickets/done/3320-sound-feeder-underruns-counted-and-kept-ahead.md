---
id: "3320"
title: "The in-house sound counts each track's underruns, and its music feeder keeps ahead on a busy machine"
type: bug
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-24
---

# 3320 — The in-house sound counts each track's underruns, and its music feeder keeps ahead on a busy machine

## Context

From the retro of PRs 379 to 484 (T24). 3089 (#380) found the gap in the in-house
Windows backend's output: its render stream ran dry while CPAL's thread waited at
normal priority for a core. It fixed that side only. `shuyi-sound-out` raises the
output's thread at its first callback (`crates/sound-out/src/sys.rs`, `raised`:
the "Pro Audio" task through `AvSetMmThreadCharacteristicsW`, or the time-critical
priority), and counts the callbacks that found the device's buffer empty
(`Output::dry_callbacks`). The callback itself is clean: it takes the mix's lock
with `try_lock` and fills, or plays silence and counts it
(`crates/sound-out/src/windows.rs`, `build`).

A track has a second thread that 3089 left as it was. Music and voice clips are
streams: the `sound-feed` worker (`crates/app/src/audio/in_house.rs`,
`Worker::start`) decodes each track into its ring (`stream::feed`, `FEED_FRAMES` =
4,096 frames, about 93 ms at 44.1 kHz and 85 ms at 48 kHz), then sleeps 10 ms. It
runs at normal priority, and `Feedings::step` skips a turn when the game's thread
holds the list. On a busy machine it can wait as long as the output's thread did
before #380. Unlike the output's thread, it has 85 ms of margin, not 20.

When it falls that far behind, nothing is told:

- The mix reads an empty ring as `Error::Underrun`, and the voice stays where it
  is and sounds nothing until frames come (`crates/sound/src/mix/stream.rs`, its
  contract). A starved track **stalls, it doesn't skip**. Its music is then late
  against the music clock, which `Player::music_playing` counts in wall time from
  the track's start (`crates/platform/src/audio/player.rs`;
  [ADR-0054](../../docs/adr/0054-music-switches-are-data-todays-fade-as-the-default-a-crossfade-as-two-tracks-with-gains-and-the-clock-on-the-newest-that-sounds.md)
  §5 and §6). A fade, or a beat clock (2365), then follows a clock the music has
  fallen behind, for the rest of the track.
- `Mixer::underrun_frames` counts the stalled frames, but as one sum over every
  stream, and only xtask's offline mixer reads it
  (`crates/xtask/src/sound_parity/mixer.rs`, which fails on any). Neither the
  backend nor `sound_probe` reads it.
- The gate can't see a stall. `perf:underruns` counts runs of quiet in the
  recording (`sound_facts` in `tools/sound-parity/capture.py`). The perf workload
  plays 32 voices beside two streams, so a stalled stream leaves the others
  sounding and makes no quiet run. And a gap that is there can be blamed only on
  the output: `one_perf` in `capture.py` adds the output's `dry_callbacks`, and
  `gaps_detail` in `crates/xtask/src/sound_parity.rs` says either "ran dry N
  times" or "not the output's: the loopback, or what the mix was given". Nothing
  names the feeder.

No feeder underrun has been seen, because nothing counts one. One failure may have
been one: in 2048's job 1 (CI run 42389680314), `windows-in-house
scenario:stream_gapless` failed with `loop.wav differs by 11756 at frame 45844 of
66150`. A stall makes everything after it differ, and 3089's notes explain only
the run of zeros of job 6.

This ticket counts first, then keeps the feeder ahead, and proves it with twenty
runs, as 3089 did for the output. It is part of the sound replacement's chain
([ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)),
and stays reversible
([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).

The game: it pins engine revision e738ffc2. `gh search code --repo
Zafnok/visions-of-shuyi 'in-house-sound'` finds only `docs/engine/pins.md` and
two tickets (the open game#1241, the done game#0912), and no `Cargo.toml`.
`'underrun_frames'` finds nothing. So no game builds this backend yet, and
nothing in the game changes.

## Nick input

None.

## Scope

**In:**
- **Counted per track.** The backend counts each stream's stalled frames, and
  knows which track it was. `Mixer::underrun_frames` stays the sum it is.
- **Measured.** For each track, the fewest frames its ring held when the feeder
  came round: its margin, from `Feeder::room()` before each fill. This is the
  measured worst wait that sizes the ring.
- **Recorded.** `sound_probe` says, after a scenario and after a timing, each
  track's stalled frames and its lowest margin. `capture.py` puts them in each
  timing's metrics, and in a scenario's facts.
- **Judged.** `perf:underruns` and `old-new:underruns` fail on a stalled stream
  frame, as on a gap: a music track that stops for a moment while the effects play
  on is heard. Their detail names whose it is: the output (its dry callbacks), the
  feeder (the track and its frames), or neither (the loopback, or the mix). A
  failed scenario's message says the same when a stream stalled in it.
- **Kept ahead.** The feeder keeps its rings ahead on a busy runner, by either or
  both of these:
  - its thread raised on Windows, as #380 raised the output's, but below the
    output's thread and above the game's;
  - `FEED_FRAMES` sized to the measured margin, plus room, with `mixer_memory`'s
    `max_held_frames` in `tools/sound-parity/budget.ron` moved with it by that
    file's rule (the runs named beside it, and `version` raised).

  Which one, and why, goes in the completion notes, with the numbers.
- Twenty `sound-parity` runs on one revision (`calibrate.yml`) with no stalled
  stream frame and no `windows-in-house` gap.
- **Either order with 3032:** if 3032 is done, the counts, the gate and the
  twenty runs cover its Linux and macOS in-house platforms too, and the feeder
  is raised there where the system allows it without rights a player lacks; if
  not, Windows only, and 3032 holds its platforms to no stalled frame.
- **Either order with 2068:** if 2068 is done, the twenty runs must also pass its
  timing cells. A `cpu_ms` limit that the raise moves follows `budget.ron`'s rule:
  a lean found, or the runs named beside the new limit. If not, a timing cell that
  fails in the twenty runs goes to 2068 as a **From 3320** note in its Context.
- No public type is added. `shuyi-sound` gains methods only: additive, no
  `Breaking:` line. Behaviour that a game sees goes on `Behaviour:` lines (the
  feeder's priority, and the ring's size if it changes).

**Out (do not do):**
- Allowing stalls or gaps in either `underruns` cell, or raising a limit to pass.
- A feeder woken by the audio thread instead of its sleep: the callback never waits
  or signals (its rules in `in_house.rs`, `AudioSide`). Write a ticket if the
  numbers say the sleep can't keep ahead.
- The web. There the page's one thread steps the feeding in the callback, before
  each mix (`AudioSide::fill`), so it can't fall behind the mix. The worklet in
  `crates/cli/web/sound.js` posts a new `{ played, silent }` object every
  256-frame block, about 187 a second. It also makes an `Int16Array` view of each
  block it is sent on the render thread. The view is what a transferred buffer
  costs: avoiding it needs a `SharedArrayBuffer`, which needs a cross-origin
  isolated page. No browser gap has been measured, so no ticket now. A
  `browser-in-house` gap in a later run is the evidence for one: fewer reports, or
  the counts carried on the `back` message.
- The timing cells (2068) and Linux/macOS output (3032), but as the two clauses
  say.
- Making the in-house backend the default (3084).

**Estimated size:** about 900 lines (code 400, tests 350, docs 150).

## Implementation steps

1. **Count per track.** In `shuyi-sound`, keep a stalled-frame count for each
   stream, where `Mixer::underrun_frames` is counted today
   (`Voice::mix`'s `Stop::Underrun`), and make it readable from the worker's side
   without a lock: for example an atomic in what `stream::feed` shares between
   `Feeder` and `Feed`, read by a new `Feeder::underrun_frames()`. The audio thread
   still allocates, frees and waits for nothing. Unit tests in
   `crates/sound/src/mix/tests.rs` beside
   `an_empty_feed_is_an_underrun_that_delays_and_loses_nothing`: two streams, one
   starved, each counted apart, and the sum unchanged.
2. **Measure the margin.** `Feeding::step` (`in_house.rs`) reads `Feeder::room()`
   before `fill_from` and keeps the ring's lowest fill so far. `Feeding` keeps the
   track's name (its cue, as the probe started it) beside both counts.
3. **Hand them to the probe.** Behind `sound-probe`, as `output_dry` is:
   `InHouse::stream_stats()` (or a like name) lists each track since the probe
   last asked, with its stalled frames and lowest margin, those that have ended
   included. `say_output` in `crates/app/examples/sound_probe.rs` adds a line,
   `{"event":"streams","tracks":[{"track":…,"stalled":…,"lowest":…}]}`, after a
   scenario and after a timing.
4. **Record and judge them.** `capture.py`'s `one_perf` puts the sum of
   `stalled`, and the lowest `lowest`, in a timing's metrics (`stalled_frames`,
   `lowest_margin`), and prints the tracks beside a gap as it does
   `dry_callbacks`. A scenario's facts get the same. In
   `crates/xtask/src/sound_parity.rs`, `perf:underruns` and `old-new:underruns`
   fail when `stalled_frames` is above 0, and `gaps_detail` names whose each is.
   Tests in `tools/sound-parity/capture_test.py` and
   `crates/xtask/src/sound_parity/tests.rs` (beside the three `gaps_detail` cases).
5. **Measure before fixing.** Twenty runs of today's feeder with the counts
   (`gh workflow run calibrate.yml --ref <branch> -f job=sound-parity -f
   count=20`). Keep each platform's lowest margin and any stall in the completion
   notes.
6. **Keep it ahead.** Raise the `sound-feed` thread on Windows at its start. The
   Windows calls belong in `shuyi-sound-out`'s `sys.rs` beside `raised`, as an item
   `shuyi-app` calls, so `windows` stays that crate's alone. It goes below the
   output's thread: `THREAD_PRIORITY_HIGHEST`, or the multimedia scheduler's
   "Audio" task, not "Pro Audio" or time-critical, so decoding never preempts the
   mix. And/or size `FEED_FRAMES` from step 5's lowest margin, and move
   `budget.ron`'s `max_held_frames` with it by its rule. Its tests: what the
   thread was raised to, as `a_thread_is_raised_and_let_go_on_its_own` does.
7. **Prove it.** Twenty more runs on one revision: no stalled frame and no
   `windows-in-house` gap in any. Put the table in the completion notes as 3089's
   has it, with each run's id and revision.
8. **Keep giving up one command** (ADR-0068 §0). Every path changed here is
   already owned whole in `tools/sound-parity/footprint.ron` (`crates/sound`,
   `crates/sound-out`, `in_house.rs`, `sound_probe.rs`, `tools/sound-parity`,
   `crates/xtask/src/sound_parity*`). An edit anywhere else gets its footprint
   line in this PR: a new mutant exclusion in `.cargo/mutants.toml` for the
   Windows-only raise changes the `Replace` the footprint holds for that file.
   Add 3320 to the tickets that step 3 of
   `.claude/skills/give-up-sound-replacement/SKILL.md` closes as dropped. Then
   run `pwsh -File scripts/sync-agent-skills.ps1 -Check`. CI's `sound-give-up` job
   passes.
9. Run the `document` skill: `tools/sound-parity/README.md` says what a stalled
   frame and a margin are, and how a gap or a stall says whose it is. Rustdoc for
   the new items. Then the gates.

## Acceptance criteria

- [x] `shuyi-sound` counts each stream's stalled frames apart, and a unit test
      starves one of two streams and sees only its count rise, with
      `Mixer::underrun_frames` still the sum.
- [x] `sound_probe` says each track's stalled frames and lowest margin after a
      scenario and after a timing, and `capture.py` records them.
- [x] `perf:underruns` fails on a timing with a stalled frame and no gap, and its
      detail names the track (a test in `sound_parity/tests.rs`).
- [x] A gap with no dry callback and a stalled stream is said to be the feeder's,
      and one with neither is said to be neither's (tests).
- [x] The completion notes give today's lowest margin per platform, measured over
      twenty runs before the fix, and say which fix was made, and why.
- [x] Twenty `sound-parity` runs on one revision after the fix: no stalled stream
      frame and no `windows-in-house` gap. The table and run ids are in the notes.
- [x] On Windows, the `sound-feed` thread runs above normal and below the output's
      thread, and a test says so; or the notes say why only the ring was resized.
- [x] 3320 is in step 3 of the `give-up-sound-replacement` skill, all this PR adds
      is in `tools/sound-parity/footprint.ron`, and CI's `sound-give-up` job passes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: per-stream stalled frames in `crates/sound/src/mix/tests.rs`; the margin
  kept by `Feeding::step`; the raise in `sys.rs`.
- Gate: `crates/xtask/src/sound_parity/tests.rs` (cells and their detail) and
  `tools/sound-parity/capture_test.py` (metrics from the probe's line).
- Integration: the two twenty-run calibrations, in CI.

## Completion notes

**What was done.**

- **Counted per track.** A mix tells a stream's source the frames it waited on it,
  through a new provided method, `Source::stalled` (nothing by default, forwarded
  by `Box`), called where `Voice::mix` stops on an underrun. A `Feed` adds them to
  an `AtomicU32` it shares with its `Feeder`, which `Feeder::underrun_frames()`
  reads without a lock. Only the `Feed` stores it, so a load and a store make the
  sum: the audio thread waits for nothing and allocates nothing, and the count
  needs no compare-and-swap (the no-std targets). It stops at `u32::MAX`, a day of
  stalls at 48 kHz. `Mixer::underrun_frames` is still the sum.
- **Measured.** `Feeding::step` reads `Feeder::room()` before each fill after the
  first, and keeps the fewest frames the ring held: the track's margin. A
  `Feeding` keeps its track's file name (`Track` now holds it).
- **Recorded.** Behind `sound-probe`, `InHouse::stream_stats()` lists each track
  since the probe last asked, the ended ones first (kept as their feeding retires,
  on the worker's thread, natively only), and `InHouse::feeder_raised()` says
  whether the worker was raised. `say_output` adds
  `{"event":"streams","raised":…,"tracks":[{"track","stalled","lowest"}]}` after a
  scenario and after a timing. `capture.py`'s `read_probe` makes a timing's
  `stalled_frames` and `lowest_margin`, and its row keeps the tracks as `streams`;
  a stall is printed beside a gap, and after a scenario.
- **Judged.** `perf:underruns` and `old-new:underruns` fail on a stalled frame as on
  a gap. `gaps_detail` names whose each is: the output's (dry callbacks), the
  feeder's (each track and its frames), or neither's ("the loopback, or the mix").
  The gate's `parse_log` reads the `streams` line, and a failed scenario's
  message says which tracks stalled in that run.
- **Kept ahead.** The `sound-feed` thread raises itself at its start through
  `shuyi_sound_out::raise_worker()`, on Windows `SetThreadPriority` with
  `THREAD_PRIORITY_HIGHEST` (in `sys.rs`'s `raised`, beside the output's). Not the
  multimedia scheduler's "Audio" task: its realtime range is above the output's
  fallback, `THREAD_PRIORITY_TIME_CRITICAL`, so on a machine that refuses the "Pro
  Audio" task to the output and grants "Audio" to the feeder, decoding would
  preempt the mix. The highest priority of the normal class is above the game's
  thread (normal) and below both of the output's ways, needs no rights, and lasts
  as long as the thread. `FEED_FRAMES` is unchanged: over the forty runs below,
  the worker never left its ring with fewer than 2,560 of its 4,096 frames (58 ms
  of the 93 ms at the runner's 44.1 kHz), raised or not, so the ring has room to
  spare and `budget.ron`'s `max_held_frames` stays where it is.

**Why the raise, when nothing stalled.** Today's feeder never stalled in twenty
runs: the hosted runner isn't busy enough to keep a thread of normal priority
waiting 58 ms. The raise is for the machine the ticket is about, a player's with
other work on its cores, where #380 found the output's thread waiting more than
its 20 ms. It costs nothing measured: the median `cpu_ms` of `windows-in-house`
was 98.78 ms before and 99.06 ms after, its `latency_ms` 18.88 and 18.25 ms (360
timings each). Resizing the ring instead would have bought margin the runs say
isn't needed, in memory every track holds.

**The numbers.** Two calibrations of `sound-parity`, 20 runs each:
before, run [45338103265](https://github.com/Zafnok/shuyi-engine/actions/runs/45338103265)
on `d35fcea9` (the counts, the feeder at normal priority); after, run
[45338123935](https://github.com/Zafnok/shuyi-engine/actions/runs/45338123935) on
`79db7314` (the counts and the raise: this PR's code, without its ticket edits).
Each run counts 80 tracks on `windows-in-house`, in its scenarios and timings. The
platforms in-house: `windows-in-house` counts; `browser-in-house` feeds in the
callback and has no `streams` line; Linux and macOS have no in-house output
(3032). The first two dispatches (45334520942 and 45334926863) built nothing
measured: the probe-only items were dead code in a build without the probe, which
the Windows job denies, and a commit here fixed that.

| Run | Before: lowest margin | Before: stalled | Before: failed cells | After: lowest margin | After: stalled | After: raised | After: failed cells |
|---|---|---|---|---|---|---|---|
| 1 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 2 | 2,816 | 0 | none | 3,072 | 0 | yes | none |
| 3 | 2,560 | 0 | none | 3,072 | 0 | yes | `browser-chromium slowdown:file_io_ms` |
| 4 | 2,816 | 0 | none | 3,072 | 0 | yes | `browser-in-house perf:file_io_ms` |
| 5 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 6 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 7 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 8 | 3,072 | 0 | none | 2,816 | 0 | yes | none |
| 9 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 10 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 11 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 12 | 3,072 | 0 | none | 2,816 | 0 | yes | none |
| 13 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 14 | 3,072 | 0 | none | 2,560 | 0 | yes | none |
| 15 | 2,560 | 0 | `windows-in-house perf:latency_ms` | 3,072 | 0 | yes | none |
| 16 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 17 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 18 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 19 | 3,072 | 0 | none | 3,072 | 0 | yes | none |
| 20 | 3,072 | 0 | none | 3,072 | 0 | yes | none |

Today's lowest margin, over the twenty runs before the fix: `windows-in-house`
2,560 frames. No stalled stream frame and no `windows-in-house` gap in any run of
either calibration: both `underruns` cells passed in all forty, each saying "0 gaps
while everything played; the output's buffer ran dry 0 times; no stream stalled".
The margins move in steps of 256 frames, the mix's block. The cells that failed
are timing cells, 2068's.

**Deviations.**

- A track is named by its file (`music_a.ogg`), not its cue: the backend's
  `load_music` gets a path, never a cue, and in the probe's plan each cue has a
  file of its own.
- The count is an `AtomicU32` read as a `u64`, not a `u64`: the mixer's rings are
  all 32-bit atomics, and an `AtomicU64` isn't there on every target the crate
  builds for.
- The `streams` line also says whether the feeder's thread was raised, as the
  `output` line does for the output's.
- The `in_house` tests' three `assert!(… == expected)` that clippy names when it
  is run with `--features in-house-sound` were there before, and CI lints neither;
  left as they were.

**Handed on.**

- No ticket: the web keeps the feeding in the callback, as the ticket's **Out**
  says, and no `browser-in-house` stall can happen there.
- The timing cells that failed in the forty runs went to 2068, as a **From 3320**
  note in its Context, as the ticket's clause with 2068 says.

Behaviour: shuyi-app: with `in-house-sound` on Windows, the `sound-feed` thread that decodes music and voice tracks runs at the highest priority of its class (`THREAD_PRIORITY_HIGHEST`), above the game's thread and below the output's. The ring stays 4,096 frames.
Behaviour: shuyi-sound: a mix tells a stream's source how many frames it waited on it (`Source::stalled`); a source that doesn't override it sees nothing change.

No break: `Source::stalled` is a provided method, and `Feeder::underrun_frames` and
`shuyi_sound_out::raise_worker` are new. No public type is added.

Book: none, `Source::stalled`, `Feeder::underrun_frames` and `raise_worker` are the in-house backend's own plumbing, which no game calls and no chapter covers: the book's sound chapter is the player's.

Game: nothing to adopt, no game builds the in-house backend: `gh search code --repo
Zafnok/visions-of-shuyi 'in-house-sound'` finds `docs/engine/pins.md` and three
tickets (game#1241, game#1348, game#0912), `'underrun_frames'` and `'shuyi_sound'`
nothing; its `impl Source for` hits are `shuyi-assets`' pack source, a trait of its
own.

Nick input: none.
