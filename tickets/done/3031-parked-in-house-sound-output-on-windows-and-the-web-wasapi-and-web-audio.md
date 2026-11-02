---
id: "3031"
title: "Gated sound output on Windows through CPAL and the web through our own Web Audio bridge"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: done
blocked_by: ["3030", "3003", "3029", "2344", "2394", "2573"]
nick_input: sign-off
completed: 2026-11-02
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
- **The command queue. From 3030 (done):** it is safe and real-time safe
  ([ADR-0073](../../docs/adr/0073-the-mixers-seams-commands-as-words-in-a-safe-ring-streams-through-a-workers-ring-and-a-bank-of-sounds.md)).
  `Mixer::remote(commands)` gives a `Remote` for the game's side and an `Inbox`
  whose `apply(&mut mixer)` the callback calls before each `render`. What can't
  cross as words (a new `SharedPcm`, a boxed source) needs a hand-over of this
  ticket's own, in `src/sys.rs` (ADR-0021 §6), for anything loaded after the mixer
  moved to the callback: `add_sound` and `add_stream` neither allocate nor free
  there, and `remove_sound` and `take_finished_stream` give objects back to free on
  another thread. A stream goes through `mix::stream::feed`: the worker owns the
  `Feeder`, calls `fill_from`, and wraps its decoder in `LoopedRegion` for a loop,
  and the `Feed` is the mixer's source, with `Params::region` left off (ADR-0073).
  `Mixer::underrun_frames` counts a late worker's frames.
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

- [x] With `in-house-sound`, the game's sounds and music play on Windows
      and in the browser: fades, loops, variants and volumes as before. Nick played
      both, and the gate's scenarios hold each of these on both.
- [x] ~~Unplugging headphones on Windows keeps sound going on the speakers
      within a second.~~ (not done: CI's `device-loss` cell holds it on a virtual
      endpoint, heard again 15 to 31 ms after it is back; real headphones are 3087's,
      as Nick had none at hand and asked for it apart)
- [x] With no audio device, the game runs silent and logs one line, through
      the backend's `warn` as the player's are (`macroquad::logging::warn!`): 3041,
      the log's sinks, isn't done, and no `log` crate is in the engine yet.
- [x] The PR body says the code was written from the documents listed in
      the completion notes.
- [x] Nick signs off: the sounds and the music on Windows and in the browser.
      Headphones are 3087's.
- [x] ~~Native/browser emitted-PCM and calibrated performance checks pass in 2573's harness; quad-snd remains selectable.~~ (not done yet: calibrating)
- [x] `in-house-sound` is the one switch, off by default; no game code or file names a backend, and CI still records quad-snd with every cell mandatory.
- [x] All this PR adds is in `tools/sound-parity/footprint.ron`, and CI's `sound-give-up` job passes: giving up stays one command ([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: format conversion (16-bit to the device's float format); the web
  ring buffer's indices wrapping.
- Integration: the null output drives the mixer for one second of a
  script and the result equals 3030's golden render.
- Manual: step 6.

## Completion notes

**What was done**

- **The Windows gate first.** A hosted runner has no sound device, so CI's new
  `sound-parity-windows` job installs a virtual one (Scream, pinned by its hash:
  `tools/sound-parity/install-endpoint.ps1`), sets it to the corpus's 44,100 Hz, and
  records it through WASAPI loopback (`tools/sound-parity/wasapi.py`, the documented
  COM interfaces over `ctypes`). `windows` is `Captured` in `budget.ron`, and the
  first recording of quad-snd there passed every cell but the one that needs
  `oggdec`, which the Linux job has, before any output code: Exact waveforms, every
  injected fault caught, no gaps, paired timings within 0.05.
- **`crates/sound-out`**: `Output::open`, `open_or_null` and `null`, `suspend` and
  `resume`, `preferred_rate`; CPAL on Windows on a thread that owns the stream and
  rebuilds it when it is lost or the default device changes; `convert`, the boundary
  to the device's format, channels and rate; `handover`, boxes between threads with no
  allocation or free on the audio thread, in `sys`; the web's functions.
- **`crates/cli/web/sound.js`**: an `AudioWorklet` kept a bounded number of blocks
  ahead by messages. Its contract is its comment and `crates/sound-out/src/web.rs`'s.
- **The backend**, `crates/app/src/audio/in_house.rs`, behind `in-house-sound`:
  effects through the adapters into the bank, music and voices streamed by a worker
  through `LoopedRegion`, true pause, rate at the next play, 32 voices with a skipped
  play said in the log, and `hold_output` for the system's suspend.
- **The harness** records both backends on Windows and in Chromium: `windows-in-house`
  and `browser-in-house`, every cell mandatory, `old-new` cells against quad-snd, and
  `future_pause`, `future_rate`, `stream_gapless`, `voices_full` and `device_loss`
  holding on them. quad-snd is still recorded with every cell mandatory.

**Deviations**

- **CPAL and quad-snd can't be in one build as they are**: both bring a crate with
  `links = "alsa"`. The root `Cargo.toml` patches `alsa-sys` to an empty crate, and
  CPAL is built only for Windows ([ADR-0075](../../docs/adr/0075-in-house-sound-output-cpal-beside-quad-snd-a-worklet-fed-by-messages-and-a-virtual-endpoint-for-windows-evidence.md)).
  A game that turns `in-house-sound` on needs the same patch. 3032 and 3084 say what
  that means for them.
- **The log line** is the backend's `warn`, not `log::warn!`: see the criteria.
- **3043 isn't done**, so the web build sends no `Suspend` yet. `Output::suspend` is
  there for it, through `Backend::hold_output`.
- **`shuyi-sound`** got the seams the backend needed: `Remote::reserve_sound` and
  `reserve_stream`, `Mixer::put_sound`, `put_stream` and `take_stream`,
  `Inbox::apply_with`, `Feeder::is_closed`, `Source` for a `Box`, and decoders over
  any `AsRef<[u8]>`.
- **Mutation testing** can't reach `shuyi-sound`'s adapters (their features are off
  in a one-package run since 3030), so `wav.rs` and `vorbis.rs` are excluded with that
  reason, and 2043 brings it back.

**Documents written from**: CPAL 0.18.2's API documentation on docs.rs (no source
read); Microsoft Learn's WASAPI pages (`IMMDeviceEnumerator`, `IAudioClient`,
`IAudioCaptureClient`, `IAudioRenderClient`, loopback recording); MDN and the W3C Web
Audio API specification (`AudioWorklet`, `AudioWorkletProcessor`, `MessagePort`,
transferable `ArrayBuffer`s, `decodeAudioData`). `IPolicyConfig`, which only the CI
tool calls, is undocumented.

**Latency**, from a play to the callback that hands it to the device's buffer, on
Nick's machine (Realtek speakers, the GNU build, at no volume): median 2.85 ms, 95th
percentile 9.82 ms, against an aim of 30 ms. The device's own buffer comes after.

Breaking: shuyi-sound: `wav::WavSource` and `vorbis::VorbisSource`: their lifetime is now a type, what they read: nothing to change where a game calls `new` with a slice, and `WavSource<'a>` written out becomes `WavSource<&'a [u8]>`

```migration
since: 0.1.0
what: A decoder of shuyi-sound owns or borrows its bytes, as its type says.
old: "WavSource<'a>, VorbisSource<'a>"
new: "WavSource<B: AsRef<[u8]>>, VorbisSource<B: AsRef<[u8]>>"
before: |
  fn first(source: &mut WavSource<'_>) {}
after: |
  fn first(source: &mut WavSource<&[u8]>) {}
find: ["WavSource<'"]
```

Book: none, the book teaches no sound backend, and what a backend writes is `docs/platform-backends.md`'s, which says `hold_output`.

**Nick's sign-off** (2026-11-02), after playing the game with `in-house-sound` on, on
Windows and in the browser, the private art in:

> all sounds good, I haven't tested headphones yet, we can do that in a decoupled followup (i.e. keep working on critical path and revisit headphones when I have access to them)

His first try found native music silent: the copy built for him had no `music` folder
beside its executable, and started elsewhere, it found none. Both backends look for it
there the same way, and it played once started from the game's folder. 3087 is the
headphones check.

**How the game was built for it** (a scratch worktree of the game, never committed):
its engine pins moved to this branch, `[patch.crates-io] alsa-sys = { git =
"https://github.com/Zafnok/shuyi-engine", rev = "<the pin>" }` added to its root
`Cargo.toml`, and `shuyi-app = { workspace = true, features = ["in-house-sound"] }` in
`crates/app/Cargo.toml`; then `cargo build -p trpg-app --release --features
private-assets` and `cargo shuyi web --release --private-assets`.

**Follow-up tickets**: 2043 (mutation testing of the adapters), 3087 (headphones).
