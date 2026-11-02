---
id: "3031"
title: "Gated sound output on Windows through CPAL and the web through our own Web Audio bridge"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["3030", "3003", "3029", "2344", "2394", "2573"]
nick_input: sign-off
completed:
---

# 3031 — In-house sound output: Windows and the web

## Context

Nick, 2026-10-19 (`docs/decisions.md`): sound first, replacing quad-snd.
After 2210 (decoder adapters) and 3030 (mixing), what's left is handing
the mix to the system. That's this ticket for Windows and the web, and
3032 for Linux and macOS.

- [ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
  selects CPAL for native output and an original Web Audio bridge, preserving
  ADR-0021's clean-room rule for our code. Call CPAL's documented API; do
  not reimplement WASAPI or inspect another output implementation to copy it.
- 3003 puts audio behind the `Platform` trait, and 3029 moves the audio
  player's rules out of `shuyi-app`, behind a small backend trait. This
  ticket adds a second implementation of that backend trait, over
  `shuyi-sound` and `shuyi-sound-out`, beside the quad-snd one.
- The web build uses macroquad's loader, not `wasm-bindgen` (game
  ADR-0034). The web side is a JS plugin of our own, registered the way
  `crates/cli/web/gamepad.js` is.

## Nick input

**Sign-off:** play the game on Windows and in the browser with the new
sound turned on (the `in-house-sound` feature). Say if any sound or track
sounds wrong, cuts out, or starts late.

## Scope

**In:**
- Before new platform output code, require 2573's actual automated Windows
  reference capture/performance baseline and original-loader browser baseline.
  **From 2573 (done):** the browser baseline and the harness are there, and
  Windows is yours to establish first: add its capture to
  `tools/sound-parity/capture.py` (WASAPI loopback on a real render endpoint),
  mark it `Captured` in `budget.ron` with calibrated limits, and pass `cargo
  xtask sound-parity port-ready windows`. A missing/silent capture is
  blocking, never a passed/ignored cell. Extend its corpus with both paths,
  including bounded streams, sample-aligned loop joins, crossfades, pause,
  rate, independent copies and no hidden stealing. Keep quad-snd available
  and default until each consumer's correctness/speed gates and sign-off pass.
  Internal region mechanics can be tested through mixer/source parameters;
  2397 later adds the cue/metadata/clock contract and is not a blocker here.
- **The crate** `crates/sound-out` (`shuyi-sound-out`), MIT OR Apache-2.0,
  no `shuyi-*` dependency except `shuyi-sound`:
  - `Output::open(OutputConfig { rate, channels, frames }) -> Result<Output,
    OutputError>`, which calls a `FnMut(&mut [i16]) + Send` callback from
    the system's audio thread;
  - `Output::suspend` and `resume`, for 3003's `Suspend` and `Resume`
    events (the web build sends them when its tab is hidden and shown,
    since 3043), reached through `Audio::suspend` and `resume` (2394),
    which `App` calls on those events; while suspended the backend's
    music clock holds, as 2394's rule for a backend that can pause says;
  - errors when there's no device, and a quiet fallback the caller picks
    (no sound rather than a crash).
- **Windows**: CPAL, defaults off (0.18.2 researched as Apache-2.0; recheck
  manifest/repository and full graph). Negotiate the device's format/rate,
  convert the integer mix at the boundary, and handle default-device loss.
  Keep ASIO off. Prove GNU and CI builds; 2902's silent construction probe
  is not a playback or recovery test. Add its device-crate edge to layers.
- **The web**: `crates/cli/web/sound.js`, our own, with an `AudioWorklet`
  that consumes bounded PCM blocks, with the queue/message or shared-memory
  contract specified and tested. State secure-context, isolation/header and
  atomics requirements for the chosen design; do not assume wasm-bindgen.
  The `AudioContext` starts on the first user gesture, as
  browsers require. Registered as a miniquad plugin like `gamepad.js`, and
  `cargo shuyi web` copies it.
- **The engine side**: the backend from 3029 implemented over `Mixer`
  (3030) and `Output`, in `shuyi-app` behind a cargo feature
  `in-house-sound`, off by default. **From 2210 (done):** the adapters' guard
  turns a library's panic on a corrupt file into `Error::Malformed` where the
  target unwinds. `wasm32-unknown-unknown` doesn't, so on the web a panic stops
  the program: choose what the web hands the adapters knowing that, and say so.
  `decoder-share` in CI's `sound-parity` job measures their speed in Chromium.
  Sound effects use 2210's adapters;
  backend workers incrementally decode music into bounded queues, seeking/
  restarting, pre-rolling and trimming ahead of loop boundaries. No file
  I/O, decoder work, allocation or blocking lock in the audio callback.
  It fills in the provided methods of 3029's trait that the mixer
  plays: `pause` and `resume` over the mixer's, and `set_rate` as
  `Params::rate` at the sound's next `play`.
- **Either order with 2234 (FLAC and MP3 played):** if 2234 is done,
  sound effects already reach this backend as WAV (`Audio::load` runs
  `audio::decode_to_wav`), and this backend's `load_music` uses the permitted
  claxon/nanomp3 adapter in bounded chunks. Whole-song WAV transcoding
  remains only the legacy path's behavior. If not, nothing to do (2234
  adds this replacement path).
- **Voices.** The backend makes the `Mixer` with at least as many
  voices as the cue rules let sound at once (the audio manifest's
  `max_voices`, which 2378 adds; 32 when there is none), so the mixer
  never refuses a play that 2345's rule let through. If `play` still
  returns `Full`, the sound is skipped with one
  `log::warn!(target: "audio", …)` line. **Either order with 2378:**
  if 2378 is done, read `max_voices` here; if not, make 32 voices, and
  2378 sizes the mixer from `max_voices` when it lands.
- **The command queue.** If 3030's completion notes say its queue
  couldn't be made real-time safe without `unsafe`, write it here, in
  `src/sys.rs` (ADR-0021 §6), between the backend and `render`;
  otherwise use 3030's.
- **Either order with 2363 (parked, positional sound):** if 2363 is
  done, the in-house backend pans: it says it can (`can_pan`, or the
  provided method 3029's ADR set for the pan) and implements the pan
  2363 put in the trait as 3030's `Params::balance`, set from 2363's
  `pan_gains(pan)` divided by 16 (1/65536 to 1/4096), with a test on
  the null output that a sound panned all the way right sounds only on
  the right. If not, nothing to do: 2363 adds it here when it lands, as
  its own clause says.
- **Latency.** Measure the time from a `play` to the sound reaching the
  device's buffer, and record it in the completion notes. Aim for under
  30 ms on Windows.

**Out (do not do):**
- Linux and macOS (3032).
- Removing quad-snd or macroquad's `audio` feature (3079 only).
- Exclusive mode, or choosing a device other than the default (write a
  ticket if a game needs a device picker).
- Recording or microphones.

## Implementation steps

1. `crates/sound-out` with `src/lib.rs`, `src/windows.rs`, `src/web.rs`,
   `src/null.rs` (an output that calls the callback on a timer and
   discards, for tests and machines without sound).
2. Establish the platform's reference gate, then CPAL open/negotiation,
   callback conversion and device-loss recovery.
3. The web: the ring buffer layout (documented in `sound.js` and
   `web.rs`), the worklet, the gesture start.
4. The backend in `shuyi-app` behind `in-house-sound`.
5. CI: build `shuyi-sound-out` for `x86_64-pc-windows-msvc` and
   `wasm32-unknown-unknown`, and run its tests with the null output.
6. Play the game on Windows and in the browser with the feature on.
7. In the completion notes: the documents used, the latency measured, and
   what Nick said at sign-off.

## Acceptance criteria

- [ ] With `in-house-sound`, the game's sounds and music play on Windows
      and in the browser: fades, loops, variants and volumes as before.
- [ ] Unplugging headphones on Windows keeps sound going on the speakers
      within a second.
- [ ] With no audio device, the game runs silent and logs one
      `log::warn!(target: "audio", …)` line (3041's sinks).
- [ ] The PR body says the code was written from the documents listed in
      the completion notes.
- [ ] Nick signs off.
- [ ] Native/browser emitted-PCM and calibrated performance checks pass in 2573's harness; quad-snd remains selectable.
- [ ] `in-house-sound` is the one switch, off by default; no game code or file names a backend, and CI still records quad-snd with every cell mandatory.
- [ ] All this PR adds is in `tools/sound-parity/footprint.ron`, and CI's `sound-give-up` job passes: giving up stays one command ([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: format conversion (16-bit to the device's float format); the web
  ring buffer's indices wrapping.
- Integration: the null output drives the mixer for one second of a
  script and the result equals 3030's golden render.
- Manual: step 6.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
