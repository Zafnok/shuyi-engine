---
id: "2272"
title: Sounds at 22.05, 32 and 48 kHz pass the manifest check and play, resampled in integers where the backend can't
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2272 — Sounds at 22.05, 32 and 48 kHz load and are resampled deterministically

## Context

The engine refuses every sound file that isn't at 44.1 kHz.
`shuyi_assets::audio::SAMPLE_RATE` is `44_100` (`crates/assets/src/audio.rs:540`),
and `playable_problem` (same file, line 563) answers
"48000 Hz; must be 44100 Hz" for anything else. The manifest check
(`format_problem`), the `Sound` formats of 2203
(`crates/assets/src/audio/formats.rs`, `sound`, line 78) and the Playdate
exporter (`crates/cli/src/export/playdate/audio.rs`, `decode`, line 113) all
apply that rule. The tests pin it: `audio.rs` lines 1329–1336 and
`crates/assets/src/format/builtin_tests.rs` lines 488, 493 and 516.

The rule came from the game (game ADR-0026, game ticket 0212): quad-snd, the
default backend, plays at 44.1 kHz and resamples anything else by nearest
neighbour, in floats (`quad-snd` 0.2.8, `src/mixer.rs`,
`load_samples_from_file`). That aliases audibly, so the engine refused the
files instead.

48 kHz is the default rate of most audio editors, sound libraries and asset
stores, and 22.05 kHz is common for small retro sound effects. So a user's
first imported sound often fails the check, and they must re-export it by
hand. Godot, Unity, Defold and Bevy accept any common rate. (A blind-spot
audit of 2026-11-23 found this, as a low-severity first-use problem.)

Most of the work is already done:

- The in-house mixer (3030, done) resamples any source rate to the device's
  rate, in integers, with a carried remainder so nothing drifts
  (`crates/sound/src/mix/resample.rs`, `Position` and `between`). The
  `in-house-sound` backend mixes at `Output::preferred_rate()`, often
  48 kHz (`crates/app/src/audio/in_house.rs`, line 584). It already plays a
  44.1 kHz file at another rate. Only the manifest check stops it.
- On the web, quad-snd hands the bytes to the browser's `decodeAudioData`,
  which resamples every file to the audio context's rate, 44.1 kHz files
  included. Nothing there depends on the file's rate.
- `shuyi_ui::audio::Samples` (`crates/ui/src/audio/samples.rs`) already
  takes any rate from 1 to `Samples::MAX_RATE`, and `Samples::to_wav`
  writes it as a 16-bit WAV.

What is missing is a deterministic integer conversion where the backend
can't resample well: the native quad-snd path, and the Playdate bake. The
resampler goes in `shuyi-assets`, not `shuyi-sound`.
`tools/sound-parity/footprint.ron` removes `crates/sound` whole if the sound
replacement is given up (ADR-0068). The quad-snd path must keep working
after that.

Builds on 3030 (the mixer's resampler), 2203 (the `Sound` formats), ADR-0063
(audio baked for the Playdate) and ADR-0066 (library codecs, an owned
mixer).

## Nick input

None.

## Scope

**In:**

- **The rule.** `shuyi_assets::audio` accepts the rates in a new
  `pub const SAMPLE_RATES: [u32; 4] = [22_050, 32_000, 44_100, 48_000]`.
  `playable_problem` answers "{rate} Hz; must be 22050, 32000, 44100 or
  48000 Hz" for any other rate. `SAMPLE_RATE` stays, with the same value.
  It now means the rate the quad-snd path and the Playdate play at, and
  the rate synthesized sounds use (2225). The game re-exports it, so it
  must not move.
- **The resampler.** Add `pub fn resample(samples: &[i16], channels: u16,
  from: u32, to: u32) -> Vec<i16>` in a new
  `crates/assets/src/audio/resample.rs`. It works on interleaved samples
  and uses integers only. The step is `from / to` in 1/65536 of a frame,
  and its remainder is carried in whole units of `to`, as
  `Position` does in `crates/sound/src/mix/resample.rs` (a private
  module). Interpolation is linear,
  rounded to nearest, as `between` does. A copy is fine, because
  `shuyi-assets` may not depend on `shuyi-sound` (`tools/layers.ron`).
  - The output has `frames × to / from` frames, rounded down, and at least
    one if the input has any.
  - It never reads past the last frame: the last frames hold it.
  - When `from == to`, the samples come back unchanged.
  - `channels` must be 1 or 2. Anything else returns empty, as an
    unplayable sound does.
- **The native quad-snd path** (`crates/app/src/audio.rs`, the `Macroquad`
  backend, native only) converts before quad-snd sees the bytes:
  - Add a private `fn at_quad_snd_rate(bytes: &[u8]) -> Cow<'_, [u8]>`.
    It reads the header with `format_problem`'s readers. A file at
    `SAMPLE_RATE` passes through borrowed. Any other file is decoded
    (`lewton` 0.9 for OGG, `hound` 3.5 for WAV), resampled with
    `resample`, and written as a WAV with `Samples::to_wav`.
  - Apply it in three places: `Macroquad::effect` (sound effects),
    `native_music::Music::load` (music and voice clips, on its worker
    thread), and `add_sound` for `Samples` whose rate isn't `SAMPLE_RATE`
    (resample the samples; no decode).
  - `lewton` and `hound` become direct native dependencies of
    `shuyi-app`. They are the versions quad-snd's `audrey` already builds
    (`Cargo.lock`: lewton 0.9.4, hound 3.5.1), so no new crate enters the
    build. `lewton` is MIT OR Apache-2.0 and `hound` is Apache-2.0. Check
    both on the day and run `cargo deny check`. `shuyi-app`'s entry in
    `tools/layers.ron` has `outside: Any` and no `readers` list, so
    `cargo xtask check-layers` should pass as it is. If it doesn't, add
    the two to a `readers` list there, as `shuyi-cli`'s entry has them.
    `Samples::to_wav` comes from `shuyi-ui`, which `shuyi-app` already
    depends on. None of this goes in `tools/sound-parity/footprint.ron`:
    it serves quad-snd, so a give-up keeps it.
  - The web quad-snd path is unchanged: the browser resamples every file.
- **The in-house backend** needs no code change. Add a test that a 48 kHz
  and a 22.05 kHz file load and play to the right length.
- **The Playdate bake** (`crates/cli/src/export/playdate/audio.rs`,
  `decode`) resamples a file at another accepted rate to `SAMPLE_RATE`,
  before it averages the file to mono. It still refuses a rate outside
  `SAMPLE_RATES`, with the manifest check's message. The export's output
  names each file it resampled, with both rates, in the same place it
  reports its other per-file work.
  - **Either order with 3334:** if 3334 is done, a baked track's loop
    region from the file's own tags is scaled to the baked rate:
    `frame × SAMPLE_RATE / rate`, rounded down, for the start and the
    length. One test covers a 48 kHz `intro_loop`-shaped track. If not,
    nothing to do here (3334 scales the region when it bakes one).
- **Either order with 2234:** if 2234 is done, the 16-bit WAV that its
  FLAC or MP3 decode hands to quad-snd also goes through
  `at_quad_snd_rate`, and the manifest's FLAC and MP3 messages name
  `SAMPLE_RATES` as the others do. If not, nothing to do (2234 routes its
  WAV through `at_quad_snd_rate` when it lands).
- **Either order with 3239:** if 3239 is done, `resample` uses the
  interpolation 3239 shipped in the mixer
  (`crates/sound/src/mix/resample.rs`), not linear, and the agreement
  test with `Mixer` still matches sample for sample; if not, nothing to
  do (3239 changes `resample` to its interpolation when it lands).
- **Docs.**
  - `docs/formats.md` (line 193, WAV and OGG; line 415, the 2203
    formats): the accepted rates, and where each backend converts.
  - `docs/targets/playdate.md` (line 195): the bake resamples to
    44,100 Hz.
  - `resample` and `SAMPLE_RATES` get rustdoc.
- **Public API.** `SAMPLE_RATES` and `resample` are additions: a const and
  a function, not types. Nothing breaks.
- **For the game.** It pins `50472bbcd3084555a61728a5d07518f720bd6514`.
  It re-exports `SAMPLE_RATE` and `format_problem`
  (`crates/content/src/audio.rs`). It uses `SAMPLE_RATE` in its tools
  (`crates/tools/src/sfx.rs`, `playdate_audio.rs`, `voice_test.rs`). Both
  keep their meaning. Every one of its files is 44.1 kHz:
  `gh search code --repo Zafnok/visions-of-shuyi "44.1 kHz"` finds that
  rule in `assets/audio/README.md`, `docs/voice.md` and game ADR-0084.
  That rule is now the game's own convention, and the engine no longer
  enforces it. `gh search code --repo Zafnok/visions-of-shuyi "must be
  44100"` finds nothing, so no game test reads the old message. The
  game's next pin bump needs nothing. The completion notes carry a
  `Behaviour:` line: the manifest check accepts 22.05, 32 and 48 kHz
  sounds, and its message for another rate changes.

**Out (do not do):**

- Any other rate: 8, 11.025, 16, 24 or 96 kHz, and so on. A later ticket
  can add one to `SAMPLE_RATES` when a game asks.
- More than two channels.
- Any measure of how good the resampler is. That is 3239's job: it
  measures the mixer's resampler.
- Converting at bake time for the native or web packages. The pack keeps
  a file's own bytes (ADR-0007; 2234's Out).
- Pitch (2345, 2379): the mixer's playback rate is separate from this.
- The sound-parity corpus (`tools/sound-parity/corpus.ron`) stays at
  44.1 kHz. A file at another rate would make the two backends differ by
  design (nearest neighbour against linear before this ticket).
- Making the in-house backend the default (3084), or removing quad-snd
  (3079).

**Estimated size:** about 700 lines (code 250, tests 350, docs 100).

## Implementation steps

1. `crates/assets/src/audio/resample.rs`: `resample`, re-exported from
   `shuyi_assets::audio`. Add `SAMPLE_RATES` beside `SAMPLE_RATE`, and
   change `playable_problem` and its doc comment.
2. Update the rate tests in `crates/assets/src/audio.rs` (lines
   1329–1336) and `crates/assets/src/format/builtin_tests.rs` (lines 488,
   493 and 516): 22.05 kHz and 48 kHz now pass, and 96 kHz and 8 kHz are
   refused with the new message.
3. `crates/app`: add `at_quad_snd_rate` and apply it in `effect`,
   `Music::load` and `add_sound`. Add the native `lewton` and `hound`
   dependencies.
4. `crates/app/src/audio/in_house/tests.rs`: a 48 kHz WAV and a 22.05 kHz
   OGG play to the length their header gives.
5. `crates/cli/src/export/playdate/audio.rs`: resample in `decode`, and
   report each converted file. Settle the 3334 clause.
6. Settle the 2234 and 3239 clauses, then write the docs.

## Acceptance criteria

- [ ] `format_problem("a.wav", <48 kHz stereo PCM WAV>)` and
      `format_problem("a.ogg", <22.05 kHz mono OGG>)` are `None`.
      `format_problem` at 96 kHz answers "96000 Hz; must be 22050, 32000,
      44100 or 48000 Hz".
- [ ] `resample(&[0, 100, 200, 300], 1, 22_050, 44_100)` is 8 samples,
      starting `[0, 50, 100, 150, 200, 250, 300, 300]`: half steps, rounded
      to nearest, with the last frame held. `resample` of any input with
      `from == to` returns the input.
- [ ] One second of samples resampled from 48,000 to 44,100 Hz has exactly
      44,100 frames, and from 22,050 to 44,100 Hz exactly 88,200 frames.
- [ ] On native quad-snd, a 48 kHz file reaches quad-snd as a 44.1 kHz WAV,
      and a 44.1 kHz file reaches it as its own bytes, with no copy. A unit
      test calls `at_quad_snd_rate` directly.
- [ ] `shuyi export playdate` bakes a 48 kHz OGG to an ADPCM WAV whose
      header says 44,100 Hz, with `frames × 44_100 / 48_000` frames, and
      the export's output names the file with both rates.
- [ ] The resampler uses no float: `resample.rs` starts with
      `#![deny(clippy::float_arithmetic)]`, as `shuyi-sound` does. A unit
      test pins `shuyi_core::state_hash` of its output for a fixed sweep
      (48 kHz to 44.1 kHz, stereo, 1,000 frames).
- [ ] Each *Either order* item (2234, 3239, 3334) is settled: done here, or
      noted in the other ticket.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `resample` lengths, rounding, the held last frame, mono and
  stereo, `from == to`; `playable_problem` for every rate in
  `SAMPLE_RATES` and two outside; `at_quad_snd_rate` borrows at 44.1 kHz
  and converts at 48 kHz; the Playdate `decode` at 22.05 and 48 kHz.
- Property: for any `from` and `to` in `SAMPLE_RATES` and up to 10,000
  frames, the output has `frames × to / from` frames rounded down (at
  least one), and every output sample lies between the two input samples
  it interpolates.
- Agreement: in `shuyi-app`'s in-house tests (which have `shuyi-sound` as
  a dev-dependency), one mono voice at full volume played through
  `shuyi_sound::mix::Mixer` at 44.1 kHz from a 22.05 kHz source matches
  `resample` sample for sample. If the mixer's balance or gain changes a
  sample, the test says where, and the completion notes say why.
- Integration: the in-house backend plays a 48 kHz and a 22.05 kHz file
  to their header's length; the manifest check over a bundle with a
  48 kHz sound passes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
