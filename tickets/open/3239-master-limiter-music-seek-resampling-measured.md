---
id: "3239"
title: The master bus never hard-clips, music can start partway through a track, and the resampler is held to a measured image limit
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3239 — The master bus never hard-clips, music can start partway through a track, and the resampler is held to a measured image limit

## Context

A blind-spot audit of the engine (2026-11-23) found three gaps in the
in-house mixer (`shuyi-sound`, 3030, done;
[ADR-0073](../../docs/adr/0073-the-mixers-seams-commands-as-words-in-a-safe-ring-streams-through-a-workers-ring-and-a-bank-of-sounds.md)).
Each is checked against the code below.

**1. The master bus hard-clips.** `Mixer::render`
(`crates/sound/src/mix.rs`, line 585) adds every voice into an `i32`
block, then calls `clamp` (line 616) on each sample: anything past
±32,767 is cut flat. A volume may be over `UNITY` ("Over UNITY it
amplifies", line 118). The in-house backend plays up to 32 voices
(`crates/app/src/audio/in_house.rs`, line 47, `VOICES`). An explosion, a
music bed and a few UI sounds together go past full scale, and players
hear hard clipping as harsh distortion. Nothing counts it either:
`Mixer::underrun_frames` (line 563) is the only count, and the perf trace
(`shuyi_ui::perf::FrameRecord`, `crates/ui/src/perf.rs`, line 339) holds
no audio at all. Ticket 3030's Out deferred "a limiter" with the effects
"when a game asks". No open ticket plans one: a search of `tickets/open`
for "limiter", "soft clip" and "headroom" finds no audio ticket.

**2. Music can't start partway through a track.** The decoder side is
mostly there already. 2397 (done) added `VorbisSource::seek_exact` and
`VorbisSource::new_at` (`crates/sound/src/vorbis.rs`, lines 208 and 118)
to go round a loop region far into a file, and the `Source` trait has
`rewind_to` (`crates/sound/src/source.rs`, line 35). But:

- `WavSource` (`crates/sound/src/wav.rs`) has no `rewind_to` and no way
  to open at a frame.
- `mix::Params` (`mix.rs`, line 117) has no start frame, and
  `track_source` (`in_house.rs`, line 126) always builds a track from its
  first frame.
- A game can't ask for it. `MusicCommand::Start`
  (`crates/ui/src/audio.rs`, line 563) says "from the start", and the
  player's `Backend::play` (`crates/platform/src/audio/player.rs`,
  line 107) "Plays `sound` from the start".

Resuming a save with the music where it was, a rhythm game's practice
section, and a cutscene skipped to its middle all need a start position.
Godot (`play(from_position)`), Unity (`AudioSource.time`), LÖVE
(`Source:seek`) and GameMaker (`audio_sound_set_track_position`) have
one. The audit asked for a new `Source::seek`. It isn't needed: a track
starts at a frame by opening its source there, on the worker, as 2397's
`new_at` does.

**3. The resampler is linear, and nothing measures it.** The mixer
interpolates in a straight line between frames
(`crates/sound/src/mix/resample.rs`, `between`; `mix.rs`, line 7).
ADR-0073 rejected a sinc or polyphase resampler "until a game hears the
difference" (line 97). But no gate would ever show a difference in
numbers. The sound-parity mixer cells
(`crates/xtask/src/sound_parity/mixer.rs`, `at_rate`, line 177) check
that the mixer matches a linear reference (`stepped`, line 151), not how
good linear is. Linear interpolation leaves strong images. Worked from its
frequency response (sinc²), a 5.5 kHz tone in a 22.05 kHz file played
into a 48 kHz mix has an image at 16.5 kHz only about 10 dB below the
tone. A 22.05 kHz sound into a 48 kHz mix is a common case: 2272 lets such
files through the manifest check, and the in-house backend mixes at the
device's rate, often 48 kHz (`in_house.rs`, line 449). Changing the
interpolation is cheapest now, before more golden rows and parity
baselines pin the linear output.

The three share one crate, one ADR and one set of parity cells, so they
are one ticket with a split valve.

The audio thread's rules hold throughout: `Mixer::render` takes no lock,
allocates nothing and does no I/O, and `shuyi-sound` has no float
(`#![deny(clippy::float_arithmetic)]`).

## Nick input

None.

## Scope

**In:**

- **A master limiter** in `Mixer::render`, between the sum and the
  clamp. It works in integers, on whole frames, with no look-ahead (so no
  added latency):
  - The gain is in 1/65,536, starting at 65,536 (unity).
  - On each frame, if the loudest channel's sum times the gain would pass
    32,767, the gain drops at once to the largest value that keeps it at
    or under 32,767 (`32_767 * 65_536 / peak`, rounded down).
  - Otherwise the gain rises back toward unity in a straight line, by a
    fixed step per frame, so that a full recovery takes the release time.
    `Limiter::DEFAULT_RELEASE_MS` is 100. The step is worked out once in
    `Mixer::new` from the rate, with nothing left to drift (as
    `resample::Position` carries its remainder).
  - Each sample is `sum * gain >> 16`, rounded toward zero, then the
    clamp as today, as a last guard.
  - A frame whose sum never passes full scale, while the gain is at
    unity, comes out exactly as today. So a mix that never overloads is
    bit-identical to the plain sum.
- **The limiter is on by default and can be turned off.** `mix::Config`
  is `#[non_exhaustive]` with a constructor (ADR-0039), so add a field
  `limiter: Option<Limiter>` with `Config::with_limiter(Limiter)` and
  `Config::without_limiter()`. `Limiter` is a new `#[non_exhaustive]`
  public type with `Limiter::new(release_ms: u32)` and `Default`. Off
  gives today's hard clamp, for a game that wants that sound.
- **Counts.** `Mixer::limited_frames()` (frames the limiter turned down)
  and `Mixer::clipped_frames()` (frames where the final clamp cut any
  sample: always 0 with the limiter on, the hard-clip count with it off),
  both `u64` totals since `Mixer::new`, as `underrun_frames` is.
- **The counts in the perf trace** (ADR-0046). The in-house backend's
  audio callback stores the three totals in `AtomicU64`s after each
  `render`. `FrameRecord` gains
  `audio: Option<AudioCounts>` (`#[serde(default, skip_serializing_if =
  "Option::is_none")]`, as `heap` is), where `AudioCounts` is a new
  `#[non_exhaustive]` type with `limited_frames`, `clipped_frames` and
  `underrun_frames`. `shuyi-app` fills it each frame with the in-house
  backend, and leaves it `None` with quad-snd. Both additions are
  additive: `FrameRecord` is `#[non_exhaustive]`.
- **A start position.**
  - `WavSource` implements `Source::rewind_to` (a seek of `hound`'s
    reader over its in-memory cursor, no waiting) and gains
    `WavSource::new_at(bytes, limits, frame)`, as `VorbisSource::new_at`
    is.
  - `mix::Params` gains `start: u64` (a frame of the sound) and
    `Params::with_start(frame)`. A bank sound starts there; a stream's
    source must already be at it.
  - `track_source` takes a start frame. It opens the track there
    (`WavSource::new_at`, `VorbisSource::new_at`). A `LoopedRegion` starts
    from that frame too: add `LoopedRegion::starting_at(source, region, at,
    restart)` (or give `landing` the frame), so a start inside or after
    the intro still goes round the region.
  - A start at or past a track's end: a track that doesn't loop plays
    nothing and ends; a looping track starts where the loop has got to
    (`region.start() + (frame - region.start()) % (region.end() - region.start())`, the
    whole file being the region when it has none).
- **A game asks for it** in milliseconds, as music switches already are
  (ADR-0054):
  - `MusicCommand::Start` gains `from_ms: u32` (0 by default), and
    `MusicCommand::start_from(cue, gain, from_ms)`. `Start` is
    `#[non_exhaustive]`, so the new field is additive.
  - `AudioRequest::PlayMusic` gains `from_ms: u32`, and the queue gains
    `AudioQueue::play_music_from(cue, from_ms)` and
    `play_music_from_with(cue, from_ms, switch)`. A request for the track
    already playing still does nothing, whatever its `from_ms`: to move
    inside it, stop it and play it again.
  - The milliseconds become frames with `PcmFormat::frames_of_millis`
    (`crates/sound/src/pcm.rs`, line 108).
  - `player::Backend` gains a provided method `play_from(&mut self,
    sound, volume, looped, from_ms) -> Result<Self::Playing,
    Unsupported>`, whose default says `Unsupported`, as `pause` does
    (line 137). The in-house backend implements it. On `Unsupported` (the
    quad-snd backend: `quad_snd::PlaySoundParams` has no start offset)
    the player plays from the top and says once in the log, naming the
    cue, that this backend starts music from its start.
  - The music clock (`Audio::music_playing`) says where in the track it
    is, so it includes `from_ms`. The headless `Speaker`
    (`crates/platform/src/headless.rs`, line 331) keeps the `from_ms` of
    the track it starts and adds it in `music_playing` (line 345).
- **Either order with 2393:** if 2393 is done, each track its `Speaker`
  keeps carries its own `from_ms`, and its crossfade test gains a case
  where the new track starts at 2,000 ms; if not, the `Speaker`'s one
  `started` track carries it, and 2393 keeps it on each of its tracks.
- **Measuring the resampler.** A new sound-parity mixer cell,
  `mixer:images`, in `crates/xtask/src/sound_parity/mixer.rs` (the xtask
  may use floats). It plays sine tones of 1,000, 2,000, 4,000 and
  5,512 Hz (up to a quarter of the source rate) from a 22,050 Hz mono
  sound into a 48,000 Hz stereo mix at rate 1, and measures, over one
  second after a 10 ms settle, the energy outside 50 Hz either side of
  the tone, in dB below the tone (a DFT in `f64`). The limit, in
  `tools/sound-parity/budget.ron` as `mixer_images_db: 40.0`, is that
  every image is at least 40 dB below its tone. The same cell reports,
  without a limit, the alias level of the 4,000 Hz tone played at rate 2
  (pitch up an octave), for 2379's sake.
- **If linear fails the limit** (the worked numbers say it will), the
  mixer's interpolation changes, in this order, to the first that passes
  and keeps the mixer's speed cells within `budget.ron`'s `mixer`
  `max_share`:
  1. a 4-point cubic Hermite (Catmull-Rom) in integers, over the frame
     before and the two after;
  2. an 8-tap windowed-sinc, from a fixed table of 256 phases of `i16`
     coefficients (Q15) worked out by a test and pinned, not at run time.
  - The sound's first and last frames are held for the taps past its
    ends, as `between` holds the last frame today. A loop region's taps
    read across its join from the region's own frames.
  - `stepped`, the cells' reference (line 151), becomes the same
    interpolation written again in `f64`, sharing no code with the mixer,
    as the module's rule says (line 1).
  - `shuyi_assets::audio::resample` (2272), the offline linear resampler
    that the native quad-snd path and the Playdate bake use, follows the
    mixer's arithmetic. If this ticket moves the mixer to cubic or sinc,
    change that resampler with it, or say in the completion notes why
    not.
- **Either order with 2272:** if 2272 is done, its offline
  `shuyi_assets::audio::resample` changes to the same interpolation, and
  its agreement test with the mixer still passes sample for sample; if
  not, nothing to do (2272 copies the interpolation the mixer has when it
  lands).
- **Either order with 3238:** if 3238 is done, put the limiter on the
  master bus after the buses add into it and before the clamp, and count
  clipped frames on that final sum; if not, put it after the single sum
  in `Mixer::render`, where 3238's buses will later add into master ahead
  of it.
- **An ADR** (the `write-adr` skill): the master limiter (instant attack,
  linear release, off as an option) and the resampler held to a measured
  image limit. It amends ADR-0073's rejected alternative "a sinc or
  polyphase resampler" (line 97), with the measured numbers.
- **Golden values.** The mixer's golden table (`crates/sound/tests/golden/
  mix.ron`) changes where a script overloads (`script` plays a voice at
  `u16::MAX`, 16 times unity, `mix_golden.rs`, line 87) and where the
  interpolation changes. Add rows: `limiter: under full scale` (equal to
  the plain sum), `limiter: overload` and, with the limiter off,
  `limiter: off`. The completion notes carry a `Breaking:` line for each
  golden row that changes (ADR-0038).
- **Docs** (the `document` skill): the `shuyi-sound` `mix` module's
  rustdoc (its first lines say the result clamps and frames are linearly
  interpolated); `docs/contract.md` and `skills/shuyi-perf/SKILL.md` for
  the trace's `audio` counts; the rustdoc of `play_music_from` and
  `Backend::play_from`, saying which backends start partway.
- **For the game.** It pins `50472bbcd3084555a61728a5d07518f720bd6514`.
  `gh search code --repo Zafnok/visions-of-shuyi "MusicCommand"` finds its
  Playdate player (`crates/playdate/src/audio.rs`, line 274) matching
  `MusicCommand::Start { cue, gain, .. }`, with `..`, so the new field
  breaks nothing; that player keeps starting music from the top.
  `gh search code --repo Zafnok/visions-of-shuyi "shuyi_sound"` finds
  nothing, and `"FrameRecord"` finds only docs and a ticket. The game's
  next pin bump needs nothing. The completion notes carry `Behaviour:`
  lines: a loud mix is turned down instead of clipped, and the mixer's
  output for a resampled sound changes.
- **The sound replacement stays reversible** (ADR-0068). Run `cargo xtask
  sound-parity give-up` (a dry run) and add each new line outside
  `crates/sound` that only the in-house path uses to
  `tools/sound-parity/footprint.ron`.

**Out (do not do):**

- Submix buses, filters, reverb and delay: 3238 adds them.
- A compressor, a soft knee, a look-ahead limiter, or a limiter per voice
  or per group.
- The device-boundary conversion in `shuyi-sound-out`
  (`crates/sound-out/src/convert_impl.rs`): it stays linear. The mix runs
  at the device's rate, so it is usually a pass-through.
- Starting music partway on the quad-snd backend or the Playdate: both
  start from the top, with the log line.
- Cue pitch (2379, parked): this ticket only reports its alias level.
- Rewinding a session (2357): it starts the replayed game's track from its
  top and stays so. A later ticket can use `play_music_from` there.
- Cinematic sound tracks scrubbed to a time (2364, parked): it can use
  `play_music_from` when it is unparked.
- Making the in-house backend the default (3084), and the first release
  (2005). Every addition here is additive, so neither is affected.

**Estimated size:** about 2,400 lines (code 1,200, tests 900, docs 300).

**Split:** past 900 lines, split off steps 6–9 (the start position) into
a new ticket; past 900 again, split off steps 10–12 (the resampler) into
another. The limiter and its counts (steps 1–5) land first.

## Implementation steps

1. `crates/sound/src/mix.rs`: `Limiter`, `Config::with_limiter` and
   `without_limiter`, the gain and its release step in `Mixer`, set up in
   `Mixer::new`. Apply it in `render` before `clamp`. Add
   `limited_frames` and `clipped_frames`. Update the module's rustdoc.
2. `crates/sound/src/mix/tests.rs`: the limiter's unit and property tests.
   `crates/sound/tests/mix_alloc.rs`: an overloaded render allocates
   nothing. `crates/sound/tests/mix_golden.rs`: the three new rows, and
   the changed rows regenerated.
3. `crates/xtask/src/sound_parity/mixer.rs`: the `clamp` cell (line 342)
   becomes `limit`, which checks, by its own arithmetic, that no sample
   passes 32,767, nothing clipped, and the gain is back at unity after the
   release time; add `clamp-off` for the limiter off, which keeps today's
   expected samples.
4. `crates/ui/src/perf.rs`: `AudioCounts` and `FrameRecord::audio`.
   Regenerate `crates/cli/schemas/session.perf.output.json` and
   `attach.output.json`.
5. `crates/app/src/audio/in_house.rs`: the atomics after `render`
   (line 324), read into each frame's record. Document the field.
6. `crates/sound/src/wav.rs`: `rewind_to` and `WavSource::new_at`.
   `crates/sound/src/mix.rs`: `Params::start`, `with_start`, and a bank
   voice starting there. `crates/sound/src/mix/stream.rs`: a
   `LoopedRegion` that starts at a frame.
7. `crates/app/src/audio/in_house.rs`: `track_source` takes the start
   frame; `Backend::play_from` for `InHouse`.
8. `crates/ui/src/audio.rs`: `from_ms` on `AudioRequest::PlayMusic` and
   `MusicCommand::Start`, `play_music_from`, `play_music_from_with`,
   `MusicCommand::start_from`, carried through `MusicState` (line 714).
   `crates/platform/src/audio/player.rs`: `Backend::play_from` with its
   `Unsupported` default, the fallback and its one log line, and the clock.
9. `crates/platform/src/headless.rs`: the `Speaker` keeps `from_ms`.
   Settle the 2393 clause.
10. `crates/xtask/src/sound_parity/mixer.rs`: the `mixer:images` cell and
    `mixer_images_db` in `budget.ron`. Run it on linear and write the
    numbers down.
11. If linear fails: `crates/sound/src/mix/resample.rs` gains the cubic,
    then if needed the windowed-sinc table, as the Scope orders them.
    `stepped` follows in `f64`. Run the speed cells. Regenerate the golden
    rows the change moves.
12. Settle the 2272 and 3238 clauses. Write the ADR and the docs.

## Acceptance criteria

- [ ] With the limiter on, two voices of a full-scale square wave
      (±32,767) at `UNITY` render no sample past ±32,767,
      `clipped_frames()` is 0, and `limited_frames()` equals the number of
      frames whose sum passed 32,767.
- [ ] A mix whose sum stays at or under 32,767 renders the same samples
      with the limiter on as with it off (property test over random
      voices, volumes and balances that can't overload).
- [ ] After the last overloaded frame, with no more overload, the output
      is the plain sum again from 100 ms on (4,800 frames at 48 kHz), and
      the gain never rises faster than the release step.
- [ ] With `Config::without_limiter()`, the output is today's hard clamp
      sample for sample, and `clipped_frames()` counts each frame where a
      sample was cut.
- [ ] The perf trace's frame lines carry `audio` with the three counts
      when the in-house backend runs, and leave it out with quad-snd; an
      old line without `audio` still parses.
- [ ] A 10-second WAV track and a 10-second OGG track asked for with
      `play_music_from(cue, 4_000)` give, as their first frame, frame
      `4_000 × rate / 1000` of the decoded file, exactly, on the in-house
      backend.
- [ ] A looped track with a region from 2 s to 6 s, started at 9,000 ms,
      starts at 5 s (2 s + 7 s mod 4 s); a track that doesn't loop,
      started past its end, plays nothing and ends.
- [ ] `music_playing` says 4.0 s plus the time since it sounded, on the
      in-house backend and the headless `Speaker`. On quad-snd the track
      plays from the top, and the log says so once for the cue.
- [ ] The `mixer:images` cell reports the image level for each tone and
      passes only when each is at least 40 dB below its tone. The
      completion notes give the numbers for linear and for what ships.
- [ ] The mixer's speed cells still pass `budget.ron`'s `mixer`
      `max_share` with the interpolation that ships.
- [ ] `shuyi-sound` still has no float, and `Mixer::render` still
      allocates nothing (`mix_alloc.rs`).
- [ ] Each *Either order* item (2272, 2393, 3238) is settled: done here,
      or noted in the other ticket.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the limiter's attack (the gain on the first overloaded frame),
  its release step and its exact return to unity; `limited_frames` and
  `clipped_frames` with it on and off; `WavSource::rewind_to` and
  `new_at` on 8-, 16- and 24-bit files; `Params::with_start` on a bank
  sound; `LoopedRegion` starting before, inside and after its region;
  `play_music_from` through `MusicState` to `MusicCommand::Start`; the
  player's fallback and its one log line; the headless clock with
  `from_ms`.
- Property: no rendered sample passes ±32,767 with the limiter on, for any
  voices, volumes up to `u16::MAX` and balances; a mix that can't overload
  is unchanged by the limiter; the interpolation that ships never reads
  outside the sound's frames.
- Golden: the three `limiter:` rows in `mix.ron`, checked on wasm32 by
  CI's `determinism` job.
- Parity: `mixer:limit`, `mixer:clamp-off` and `mixer:images` in the
  sound-parity mixer cells; `rate-double`, `rate-half`, `resample-mono-up`
  and `resample-down` against the new `stepped`.
- Integration: the in-house backend's tests
  (`crates/app/src/audio/in_house/tests.rs`) start a WAV and an OGG track
  at 4,000 ms, and render an overloaded mix whose counts reach the perf
  record.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
