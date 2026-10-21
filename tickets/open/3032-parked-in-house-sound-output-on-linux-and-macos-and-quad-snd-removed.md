---
id: "3032"
title: "Parked: In-house sound output, part 2: Linux through ALSA, macOS through Core Audio, and quad-snd removed"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3031"]
nick_input: sign-off
completed:
---

# 3032 — In-house sound output: Linux and macOS, and quad-snd removed

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the sound research (2902), which may keep, change or drop it.

## Context

The second half of sound output. 3031 made `shuyi-sound-out` with Windows
and the web, and the `in-house-sound` feature in `shuyi-app`. This ticket
adds Linux (the Steam Deck included) and macOS. Then it makes the new
sound the only sound and removes quad-snd, as
[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§5 says.

Written from the ALSA library's API reference (`snd_pcm_open`,
`snd_pcm_hw_params_*`, `snd_pcm_writei`, recovering from `-EPIPE`) and
Apple's Core Audio documentation (Audio Unit `kAudioUnitSubType_DefaultOutput`,
the render callback). Never from the source of quad-snd, cpal or any
other output library (ADR-0021 §2). Bindings: `alsa-sys` (MIT) and
`objc2-audio-toolbox` / `objc2-core-audio` (Zlib OR Apache-2.0 OR MIT).
Re-check both on the day (ADR-0021 §3).

On the Steam Deck and current desktop Linux, PipeWire serves ALSA clients,
so ALSA reaches every Linux sound system with one backend.

## Nick input

**Sign-off:** play the game on the Steam Deck (or another Linux machine)
and on a Mac if one is at hand. Say if anything sounds wrong. If no Mac is
at hand, say so, and the ticket notes that macOS was tested only in CI.

## Scope

**In:**
- **Linux**: ALSA through `alsa-sys`, on its own thread, the `default`
  device, 16-bit at the mixer's rate (or the device's, converted),
  underrun recovery. `libasound` is loaded at run time with
  `dlopen`, so a machine without it runs silent instead of failing to
  start. If `alsa-sys` can only link at build time, write the few
  function declarations ourselves, from the reference, and `dlopen`
  them. When `libasound` is loaded at run time, doctor's `lib-alsa`
  row (`crates/cli/src/contract/doctor/linux.rs`, 2513) changes from
  `pkg-config --modversion alsa` to the soname `libasound.so.2`. The
  row's packages, and the package lines of every
  `scripts/setup-linux.sh`, follow if the package changes.
- **macOS**: a default-output Audio Unit with a render callback, through
  the `objc2-*` crates.
- **The switch**: `in-house-sound` becomes the only path:
  - remove `quad-snd` from `crates/app/Cargo.toml`;
  - turn macroquad's `audio` feature off in the workspace `Cargo.toml`;
  - delete the quad-snd backend in `crates/app/src/audio.rs` (`mod
    device`) and `crates/app/src/audio/native_music.rs`;
  - remove the feature flag, since there's nothing left to choose.
- **Check**: `cargo tree --workspace` shows none of `quad-snd`, `audrey`,
  `lewton`, `hound`, `dasp_*`, `smallvec 0.6` or `maybe-uninit`.
- **For the game:** `shuyi-app`'s audio API is unchanged, so the game's
  next pin bump needs no code change. Say so in the PR.

**Out (do not do):**
- PulseAudio or JACK backends (PipeWire covers both through ALSA).
- iOS and Android (later platforms; ADR-0008).
- Changing the audio player's rules.

## Implementation steps

1. `crates/sound-out/src/alsa.rs` and `src/coreaudio.rs`.
2. CI builds `shuyi-sound-out` for `x86_64-unknown-linux-gnu` and
   `aarch64-apple-darwin`, and runs its tests with the null output.
3. Make the switch (scope above), and fix whatever in `shuyi-app` used
   macroquad's audio.
4. Build the game against this working copy (`run-gates`, *The game at the
   next pin*).
5. Test on the Steam Deck or Linux, and on macOS if possible.
6. In the completion notes: the documents used, the devices tested, and
   what Nick said at sign-off.

## Acceptance criteria

- [ ] Sound and music play on Linux and macOS as on Windows.
- [ ] A Linux machine without `libasound` starts and runs silent.
- [ ] doctor's `lib-alsa` check looks for the soname, and its tests say so.
- [ ] `cargo tree` shows none of the crates listed above.
- [ ] The game builds and plays at this revision with no code change.
- [ ] The PR body says the code was written from the documents listed in
      the completion notes.
- [ ] Nick signs off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: ALSA parameter choice (the rate and format fallbacks), with the
  device calls behind a small trait and a fake.
- Integration: the same null-output golden test as 3031, on Linux and
  macOS runners.
- Unit: doctor's `lib-alsa` row (2513's tests on fakes) passes when
  `libasound.so.2` is found by soname and fails without it.
- Manual: step 5.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
