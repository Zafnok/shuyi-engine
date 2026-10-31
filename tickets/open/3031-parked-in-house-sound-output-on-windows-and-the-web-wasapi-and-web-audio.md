---
id: "3031"
title: "Parked: In-house sound output, part 1: the shuyi-sound-out crate, Windows through WASAPI, and the web through our own Web Audio plugin"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["3030", "3003", "3029", "2344", "2394"]
nick_input: sign-off
completed:
---

# 3031 — In-house sound output: Windows and the web

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the sound research (2902), which may keep, change or drop it.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): sound first, replacing quad-snd.
After 2210–2214 (reading files) and 3030 (mixing), what's left is handing
the mix to the system. That's this ticket for Windows and the web, and
3032 for Linux and macOS.

- [ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md):
  a crate of its own, `shuyi-sound-out`, over raw bindings only (§3). It is
  written from Microsoft Learn (WASAPI: `IMMDeviceEnumerator`,
  `IAudioClient`, `IAudioRenderClient`, event-driven shared mode) and MDN
  and the W3C *Web Audio API* (`AudioContext`, `AudioWorklet`). Never from
  the source of quad-snd, cpal or any other output library (§2).
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
- **Windows**: WASAPI shared mode, event-driven, on its own thread, through
  the `windows` crate (re-check its license on the day; ADR-0021 §3).
  Format negotiation: ask for 16-bit at the mixer's rate; if the device
  refuses, take its mix format (often 32-bit float at 48 kHz) and convert.
  Device changes (headphones plugged in): reopen the default device when
  WASAPI reports it invalidated.
- **The web**: `crates/cli/web/sound.js`, our own, with an `AudioWorklet`
  that pulls blocks the Rust side mixed into a shared ring in the wasm
  memory. The `AudioContext` starts on the first user gesture, as
  browsers require. Registered as a miniquad plugin like `gamepad.js`, and
  `cargo shuyi web` copies it.
- **The engine side**: the backend from 3029 implemented over `Mixer`
  (3030) and `Output`, in `shuyi-app` behind a cargo feature
  `in-house-sound`, off by default. Sound effects decode with
  `decode_all`, and music streams with `vorbis::Decoder`, so music no
  longer needs the worker thread of `native_music.rs` on this path.
  It fills in the provided methods of 3029's trait that the mixer
  plays: `pause` and `resume` over the mixer's, and `set_rate` as
  `Params::rate` at the sound's next `play`.
- **Either order with 2234 (FLAC and MP3 played):** if 2234 is done,
  sound effects already reach this backend as WAV (`Audio::load` runs
  `audio::decode_to_wav`), and this backend's `load_music` runs a FLAC
  or MP3 track through `audio::decode_to_wav` too and plays the WAV
  whole, not streamed, on Windows and on the web alike (its web path
  doesn't use the browser's decoder). If not, nothing to do (2234
  does it on this path).
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
- Removing quad-snd or macroquad's `audio` feature (3032).
- Exclusive mode, or choosing a device other than the default (write a
  ticket if a game needs a device picker).
- Recording or microphones.

## Implementation steps

1. `crates/sound-out` with `src/lib.rs`, `src/windows.rs`, `src/web.rs`,
   `src/null.rs` (an output that calls the callback on a timer and
   discards, for tests and machines without sound).
2. Windows: open, negotiate, the event loop on a thread, conversion,
   device loss.
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
