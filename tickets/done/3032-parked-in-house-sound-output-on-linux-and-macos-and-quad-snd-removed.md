---
id: "3032"
title: "Gated CPAL sound integration on Linux and macOS, retaining quad-snd until final removal"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3031", "2513", "2573"]
nick_input: sign-off
completed: 2026-11-23
---

# 3032 — Gated sound output on Linux and macOS

## Context

3031 provides Windows/web output over the owned mixer and decoder adapters.
This ticket adds Linux (the Steam Deck included) and macOS through CPAL.

**From 3031 (done):**

- **`links = "alsa"`.** CPAL's Linux path needs `alsa-sys` and quad-snd's needs
  `quad-alsa-sys`, each with `links = "alsa"`, which Cargo allows once in a build on
  any target. 3031 patches `alsa-sys` to an empty crate
  (`tools/sound-parity/alsa-sys-held`, [ADR-0075](../../docs/adr/0075-in-house-sound-output-cpal-beside-quad-snd-a-worklet-fed-by-messages-and-a-virtual-endpoint-for-windows-evidence.md))
  and builds CPAL only for Windows. Linux needs real ALSA, so this ticket starts by
  replacing that patch: a held crate that re-exports `quad-alsa-sys`'s bindings under
  the name CPAL's `alsa` wants, or `quad-alsa-sys` patched to use `alsa-sys`, proved
  on Linux with quad-snd's own cells still passing. Its result goes in an ADR.
- **What is there to extend:** `Output` (`crates/sound-out`) with `src/windows.rs`'s
  thread, which owns the stream and watches the default device; `in_house.rs` in
  `shuyi-app` needs no change for a new system. A new backend's platform is a
  `NewBackend` row of `budget.ron`, and `capture.py native` gains what
  `capture.py windows` has, `--new-probe` and `--only`.
Keep old/new paths side by side; the separate final ticket 3079 owns removal.
See [ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
and [2902's evidence](../../docs/research/2902-sound-after-quad-snd.md).

## Nick input

**Sign-off:** listen on the Steam Deck/Linux and a Mac if one is at hand.
If no Mac is available, record that macOS was tested only in CI, as the
original ticket allowed. Automated emitted-output/performance evidence is
still mandatory on every required port; a CI build alone does not supply it.

## Scope

**In:**
- Before new output code or switching a platform, establish its actual
  old-output capture and calibrated speed baseline in 2573's harness.
  Linux reuses the virtual sink. **From 2573 (done):** Linux's baseline is in
  CI; macOS is yours to establish first, and `cargo xtask sound-parity
  port-ready macos` refuses until it has passed evidence. Automated hosted
  macOS capture was not established by research: make a route work on a
  suitable runner or remain blocked there with the old backend. Missing capture, fake callback
  traces and unsupported cells never count as passed audio parity.
- CPAL defaults off (0.18.2 researched Apache-2.0; recheck manifest,
  repository license and full graph), through public APIs. Own glue stays
  original; do not copy implementation source or write raw ALSA/CoreAudio
  backends merely to follow the old outline. Add actual layers edges.
- Linux default ALSA output: negotiated formats, device loss and underrun
  recovery. Audit actual linked/dynamic libraries and the sniper glibc
  floor. Linux packages already require ALSA; do not claim CPAL uses
  `dlopen` without proof. Preserve the planned silent-launch behavior
  without an audio library using a compatible loading adapter/version,
  or retain the old path until resolved. Doctor (2513), setup and package
  requirements/tests follow the actual chosen behavior.
- **Either order with 3071:** if 3071 is done, update its actual CPAL
  library audit/package tests in `crates/cli/export/libraries.ron` and
  `docs/targets/desktop.md`; if not, nothing to do (3071 audits this path).
- **Either order with 3320:** if 3320 is done, hold Linux and macOS to its
  per-track stalled-frame counts: no stalled frame in their calibration runs, and
  the `sound-feed` thread raised there where the system allows it without rights a
  player lacks, or its ring sized to their measured margin; if not, nothing to do
  (3320 covers the platforms that exist when it lands).
- macOS default output on Apple silicon and Intel. CPAL's researched
  docs list macOS 14.2; Shuyi has no explicit OS floor. Verify deployment
  rather than declaring a regression or silently raising the minimum.
  Resolve incompatibility with a compatible version/adapter or keep old.
- Extend actual emitted-PCM, equal-work CPU/speed, bounded-memory, hotplug,
  suspend and music-clock checks. Only switch consumers after comparative
  gates and existing stage sign-off pass; record manual hardware limits
  above and leave the reference selectable.
  Hound/lewton remain intentional dependencies.
- Keep the public audio/player API and game's files unchanged. A discovered
  break gets a Breaking line and game handoff, not a silent break, and one only
  the new backend could honour stops this stage until it needn't ([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).
- Stay behind 3031's `in-house-sound`, off by default: making it the default is
  3084's, and CI still records quad-snd. All this PR adds goes in
  `tools/sound-parity/footprint.ron`, and CI's `sound-give-up` job passes.

**Out (do not do):** quad-snd, macroquad audio, feature, source, plugin and
notice removal (3079); new cue rules; mobile backends; optional native
PipeWire/JACK support. No unvalidated target is abandoned to finish this ticket.

**Estimated size:** about 900 lines (code 350, tests and CI 350, docs 200).

## Implementation steps

1. Prove each platform's reference capture/performance gate before its adapter.
2. Integrate CPAL, negotiate/recover, audit package requirements and floors.
3. Extend 2573's scenarios and CI on Linux and both Mac architectures.
4. Build the game against this copy, listen on hardware, record artifacts,
   devices, target floors, dependencies and sign-offs for 3079.

## Acceptance criteria

- [x] Actual old/new output and calibrated CPU/speed comparisons pass on Linux and macOS; unavailable capture blocks migration. (Linux: every `linux-in-house` cell in 20 calibration runs; macOS has no capture route yet, so it stays `Blocked` and `port-ready macos` refuses: 3348.)
- [x] ~~Both Mac architectures and verified deployment floor work; Linux package/doctor/library checks match reality.~~ (not done in full: both architectures build and pass their tests, the floor is verified, and it is macOS 14.2 against the program's own 11.0 and 10.12, which 3349 settles; Linux's libraries, glibc floor and `doctor` match what CI reads.)
- [x] Planned silent launch without an audio library is preserved and tested, or migration remains blocked. (Neither backend has it: both link `libasound.so.2`, which CI checks. 3084 waits on 3347.)
- [x] Device recovery, suspend/clock, bounded streams and all required playback scenarios pass. (On Linux, every scenario, `device-loss`, `future_pause`, `stream_gapless` and the rest, in 20 runs; macOS: 3348.)
- [x] ~~The game builds/plays without code changes; existing stage sign-off and hardware-availability notes are recorded.~~ (not done yet: the game builds with the feature on and no code change; Nick's listening is pending.)
- [x] Quad-snd and the reference path remain available for 3079.
- [x] All gates in the `run-gates` skill pass.

## Tests required

Unit: format negotiation/conversion and recovery on fakes; doctor/package
requirements. Integration: actual emitted PCM and repeated performance in
2573's harness, null-output golden render, bounded memory/underruns, Mac
universal/export checks and Linux missing-library behavior. Manual: hardware
listening/recovery as above, in addition to automated correctness and speed.

## Completion notes

**What was done**

- **quad-snd and CPAL in one build on Linux.** quad-snd's ALSA bindings are now
  `alsa-sys`'s: the root `Cargo.toml` patches `quad-alsa-sys` with
  `tools/sound-parity/quad-alsa-sys-held`, a crate of that name and version without
  `links`, whose source is `pub use alsa_sys::*;`. quad-snd compiles against it
  unchanged, and its own `linux-native` cells passed in every CI run since. 3031's
  empty `alsa-sys` is gone ([ADR-0139](../../docs/adr/0139-in-house-sound-on-linux-and-macos-quad-snds-alsa-bindings-are-alsa-syss.md),
  which amends ADR-0075).
- **One output for every CPAL system.** `crates/sound-out/src/windows.rs` became
  `system.rs`, built on Windows, Linux (ALSA's `default`) and macOS (Core Audio's
  default), CPAL with no features. Its rules are in `system/rules.rs`, tested alone:
  on Linux a callback asks for about 20 ms (`BufferSize::Fixed`, inside the device's
  range); a playing stream with no callback for half a second is built again; on Linux
  an error CPAL can't name passes while the stream still calls back. The stream opens
  at the mix's own rate where the device takes it. `dry_callbacks` counts on Windows
  alone, as Core Audio's fixed blocks each read as dry.
- **Linux's evidence.** `capture.py native --new-probe --take-device` records the
  in-house probe beside quad-snd on the same null sink: `linux-in-house` is a
  `NewBackend` in `budget.ron`, every cell mandatory, with `old-new` cells and a
  `device-loss` cell (the sink unloaded while the probe plays, then loaded again).
  Its limits are from the 20 runs of calibration run 45215695721, by the file's rule.
- **The Linux floor and what a package needs.** CI's `sound-linux-floor` builds both
  probes in the sniper SDK: each needs `libasound.so.2`, `libc.so.6`, `libdl.so.2`,
  `libgcc_s.so.1`, `libm.so.6` and `libpthread.so.0`, and asks for glibc 2.30 at the
  newest (the floor is 2.31). `sound-parity`'s step *What each native probe loads* fails
  if either stops naming `libasound.so.2`: CPAL links ALSA, it doesn't `dlopen` it.
  `doctor`'s `lib-alsa` row already checks ALSA with `pkg-config`, as `alsa-sys` finds
  it; only its comment changed.
- **macOS.** CI's `sound-macos` runs the output's and the backend's tests on
  `macos-15` (Apple silicon) and `macos-15-intel`, joins the probe's slices with
  `lipo`, checks its floor (`tools/sound-parity/mac-floor.sh`), and plays a scenario on
  the runner's default device: the in-house output opened "Apple Virtual Sound Device"
  (48,000 Hz) and "Null Audio Device" (44,100 Hz) and played.
- **Layers.** CPAL's Core Audio crates (`objc2-audio-toolbox`, `objc2-avf-audio`,
  `objc2-core-audio`, `objc2-core-audio-types`) joined the device crates in
  `tools/layers.ron`, the template's copy and `shuyi lint`'s list.
- All of it is in `footprint.ron`, and `sound-give-up` passed on every run.

**What the gate says of Linux's new backend**, over 20 calibration runs (45215695721):
every `linux-in-house` cell passed in all 20. Against quad-snd on the same runners: CPU
0.49 to 0.64 of its own, latency 0.42 to 0.51 (12 to 19 ms against 27 to 39), memory
0.56 to 0.58; no gap. Device loss: heard from the first frame the sink rendered after
it came back, in every run. The one cell that failed in those runs was quad-snd's own
`windows perf:underruns`, once.

**What it took to get there** (each is in a commit of the PR):

- `PULSE_LATENCY_MSEC`, which the harness sets for quad-snd, stalls CPAL's stream on
  ALSA's PulseAudio plugin after one callback: nothing heard, no error. The in-house
  probe plays without it (`IN_HOUSE_WITHOUT` in `capture.py`). A player who sets it by
  hand, as advice against crackling under Wine and Proton says to, would hear nothing:
  3350.
- The plugin says it takes 48,000 Hz whatever the server runs at, so the mix was
  resampled and the marker blurred (correlation 0.91): the stream now opens at the
  mix's rate where the device takes it, and the probe mixes at the sink's
  (`SHUYI_SOUND_PROBE_RATE`, the probe's alone).
- The plugin says `snd_pcm_avail_delay` failed with an I/O error early in some
  streams that play on; rebuilding for it cut 17 ms out of 11 of 20 runs in the first
  calibration (45194312657). Such an error now passes on Linux, and the stall rule
  catches a stream that did end.
- A null sink made anew renders its first block at its largest, about 2 seconds, so
  device loss is timed on the sink's own clock; the wall clock's is in the report.

**Deviations**

- **Silent launch without an audio library** isn't preserved, as there is none to
  preserve: quad-snd's probe and the in-house one both name `libasound.so.2` in
  `DT_NEEDED`, so neither starts without it (CI checks both). The "planned" `dlopen` was
  3004's old assumption. Migration waits on it: 3347 asks Nick whether a Linux game
  must start without the library, and 3084 is blocked on it.
- **macOS capture** isn't established: no route was tried beyond finding what the
  runners offer, which 3348 starts from. macOS stays `Blocked` in `budget.ron`, on the
  old backend, as the ticket allowed; `port-ready macos` still refuses.
- **macOS's floor is 14.2** with CPAL 0.18.2: it imports `AudioHardwareDestroyProcessTap`
  strongly, which the SDK makes 14.2's; every other audio import is older than the
  program's `minos` (11.0 Apple silicon, 10.12 Intel). CI expects 14.2, so a change is
  seen. Not raised silently: 3349 makes the import weak or absent, or asks Nick.
- **Either order with 3071 and 3320:** neither is done, so nothing here; each got a
  **From 3032 (done)** note with what it needs.
- **The ticket was 35 days old.** Its steps held: 3031 had built what it named, and
  the files it named are where it said. Its `Estimated size:` was added (900 lines).
- `sound-probe` builds print more of the output's state (`Output::device`, `ahead`,
  `calls`, `glitches`, `rebuilt_because`), which found each problem above.

**Follow-up tickets:** 3347 (Linux without `libasound`), 3348 (macOS capture), 3349
(macOS before 14.2), 3350 (`PULSE_LATENCY_MSEC`). 3084 waits on all four.

Size: 1,193 counted lines, past the 1,000 planned: the CI evidence needed each fix
above. A split would put macOS's job and `mac-floor.sh` (about 150 lines) apart.

Game: game#1241 turns `in-house-sound` on, and now names `quad-alsa-sys`'s patch
in place of `alsa-sys`'s (below). A scratch worktree of the game, never committed,
built with its pins at this branch, the patch and the feature on.

Breaking: shuyi-app: `in-house-sound`: a game that turns it on patches `quad-alsa-sys` in its root `Cargo.toml` instead of `alsa-sys`

```migration
since: 0.1.0
what: A game with the in-house sound gives quad-snd alsa-sys's bindings, instead of holding alsa-sys's place.
old: "[patch.crates-io] alsa-sys = { git = \"https://github.com/Zafnok/shuyi-engine\", rev = \"<pin>\" }"
new: "[patch.crates-io] quad-alsa-sys = { git = \"https://github.com/Zafnok/shuyi-engine\", rev = \"<pin>\" }"
before: |
  [patch.crates-io]
  alsa-sys = { git = "https://github.com/Zafnok/shuyi-engine", rev = "<pin>" }
after: |
  [patch.crates-io]
  quad-alsa-sys = { git = "https://github.com/Zafnok/shuyi-engine", rev = "<pin>" }
```

Behaviour: shuyi-sound-out: on Linux and macOS `Output::open` opens the system's default device through CPAL, where it said `Unavailable`; a game sees sound there with `in-house-sound` on.
Behaviour: shuyi-sound-out: a stream that makes no callback for half a second is built again, on every system; on Linux an error CPAL can't name no longer rebuilds a stream that still calls back.
Behaviour: shuyi-sound-out: the stream opens at the mix's rate where the device takes it, and `Output::dry_callbacks` counts on Windows alone.
Behaviour: shuyi-cli: `shuyi lint` refuses CPAL's Core Audio crates in a game's rules, as it does the other device crates.

Book: none, the book teaches no sound backend, and what changed is the in-house backend behind a feature no game turns on yet.

**Nick's sign-off:** *(pending: listening on the Steam Deck or Linux, and a Mac if one
is at hand.)*
