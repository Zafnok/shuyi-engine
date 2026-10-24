---
id: "2365"
title: "Parked: A beat and bar clock for music: an integer tempo per track, beats from the music clock or a sim's steps, switches on the next beat or bar, and shots on a beat"
type: feature
milestone: "Engine: audio"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2013", "2341"]
nick_input: none
completed:
---

# 2365 — A beat and bar clock for music

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8:

- Godot's Ogg and MP3 streams carry `bpm`, `beat_count` and
  `bar_beats`, and `AudioStreamInteractive` switches clips on the next
  beat or bar, with fades counted in beats
  (https://docs.godotengine.org/en/stable/classes/class_audiostreaminteractive.html);
- Unreal's Quartz plays sounds on a beat or a bar and sends an event on
  each one;
- Unity's `AudioSource.PlayScheduled` starts a clip at an exact time on
  the audio clock, and games work out the beats from it;
- Bevy has none of this.

What Shuyi has today:

- `crates/assets/src/audio.rs`: a `MusicCue` is `{ file, volume,
  looped, length_ms, credit }`, with no tempo. `RawMusic` reads it from
  `audio.ron` (`deny_unknown_fields`) and `Check::music` checks it.
- `crates/ui/src/audio.rs`: `MusicClock { cue, position, length }`,
  built by `MusicClock::from_elapsed` from the time the platform
  reports. The position and the length are `f32` seconds, and the
  position wraps at the length. Fine for drawing; no beats, and no use
  to a simulation (`AGENTS.md` rule 5: no clock and no floats in the
  core).
- `MusicState` switches tracks when asked. 2341 adds how a switch fades
  (`MusicSwitch`, which 2013's ADR makes `#[non_exhaustive]` so a field
  can join it) and `MusicState::heard(clock)`, which this ticket reads
  beats from.
- `crates/assets/src/cinematic.rs`: a cinematic can last as long as a
  music cue (`length: Music(cue)`), but each shot's `TimedShot::at` is
  `f32` seconds, which `check` turns into `ShotDef::at_ms` with
  `millis`. 2364 (parked) adds keyed tracks, sound cues and markers to
  the file and leaves the beat clock here. Whichever of the two lands
  second makes their times take beats.
- Fixed steps: `shuyi_platform::tick::STEPS_PER_SECOND` is 60 (2115
  moves it into `shuyi_core::timer`), and a screen reads the steps due
  in a frame from `FrameInput::ticks()`.
- 2205 (parked MIDI) has a tempo map for sequences only; recorded music
  has nothing.

Why it matters:

- **The game** doesn't need it: its battles take turns, and its music
  sets the mood. That is why this is parked.
- **Other 2D games:** a rhythm game; a dungeon game whose monsters step
  on the beat; lights that pulse with the music; a menu that changes
  music on the next bar so the change sounds written. Without this, each
  game redoes the tempo maths.
- **Determinism:** in a game whose rules run in steps, a beat that
  changes the rules must come from the step count, never from the audio
  device, or a replay would differ. So the maths is integer and lives in
  `shuyi-core`, and the view and the sim each feed it their own time.

Nick has not named music-synced or rhythm games as a kind of game the
engine serves (ADR-0009). If he does, this is the ticket to unpark.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::beat`** (`crates/core/src/beat.rs`, tests in
  `crates/core/src/beat/tests.rs`), `no_std`, integers only:
  - `Tempo { bpm_milli: u32, beats_per_bar: u8, first_beat_ms: u32 }`
    (`bpm_milli`: beats a minute, in thousandths, so 92.5 bpm is
    92 500). `#[non_exhaustive]` with `const fn new(bpm_milli,
    beats_per_bar, first_beat_ms)`, since tempo changes may join it
    later. Serialize and Deserialize, for the pack.
  - `BeatPos { beat: u32, bar: u32, beat_in_bar: u8 }`, a plain value
    (2013's kind 1), counted from 0 at the first beat (`beat` counts
    every beat, `bar` the bars).
  - `Tempo::at(ms: u32) -> Option<BeatPos>` (`None` before the first
    beat).
  - `Tempo::ms_of(beat: u32) -> Option<u32>`: when a beat starts,
    rounded up to a whole millisecond so `at(ms_of(n))` is beat `n`
    (`None` past `u32::MAX`).
  - `Tempo::beat_ms() -> u32`: one beat, rounded to the nearest
    millisecond.
  - `Tempo::beats_crossed(from_ms, to_ms) -> u32` and
    `Tempo::bars_crossed(from_ms, to_ms) -> u32`: the beat starts, or
    the bar starts, in `from_ms < t <= to_ms`. That is the rule of
    2364's `shuyi_core::timer::crossed`; beats are counted by
    arithmetic, not as a list of marks.
  - `Tempo::next_beat(ms) -> Option<u32>` and `Tempo::next_bar(ms) ->
    Option<u32>`: the first beat or bar start after `ms`.
  - `Tempo::at_steps(steps: u64, steps_per_second: u32) ->
    Option<BeatPos>`: the beat after that many fixed steps, computed
    exactly (no step length rounded to whole milliseconds), so a sim at
    60 steps a second never drifts from the tempo. `None` before the
    first beat or when `steps_per_second` is 0. A caller passes
    `STEPS_PER_SECOND`.
  - `u64` inside, and `u128` in `at_steps`, so no `u32` time, no step
    count and no `bpm_milli` the manifest allows overflows. No floats
    (2015's float ban in `shuyi-core`).
- **A tempo on a music cue:** `MusicCue` gains `tempo: Option<Tempo>`.
  In `audio.ron` it is optional (`RawMusic` gets `#[serde(default)]
  tempo: Option<RawTempo>`, a private `deny_unknown_fields` struct):

  ```ron
  (id: "battle", file: "battle.ogg", volume: 80, length_ms: 96000, credit: Own,
   tempo: Some((bpm: 120.0, beats_per_bar: 4, first_beat_ms: 250))),
  ```

  `bpm` is written as a number and kept in thousandths, rounded, as the
  cinematic file keeps seconds as milliseconds. Checked in
  `Check::music`, one message each, like the others: `bpm` from 20 to
  400; `beats_per_bar` from 1 to 16; `first_beat_ms` below
  `length_ms`.
- **The music clock's beats:** `MusicClock` gains `tempo:
  Option<Tempo>` and `length_ms: u32`, both copied from the cue by
  `MusicClock::from_elapsed`, and:
  - `position_ms() -> u32` (the position, rounded down);
  - `beat() -> Option<BeatPos>`;
  - `beats_since(&self, before: &MusicClock) -> u32` and
    `bars_since(&self, before: &MusicClock) -> u32`: the beat or bar
    starts crossed since an earlier clock of the same cue (0 for
    another cue or no tempo). When the position went down, the track
    wrapped: count the starts in `before < t < length_ms`, then in `0
    <= t <= now`. A start at or past `length_ms` is the next loop's, so
    the one at the loop point is counted once.
  A screen keeps the clock it saw last frame and calls `beats_since`.
- **Switches on the next beat or bar:** `SwitchAt` (`Now`, the default,
  `NextBeat`, `NextBar`; `#[non_exhaustive]`) in `shuyi_assets::audio`
  beside `MusicSwitch`, re-exported from `shuyi_ui::audio`.
  `MusicSwitch` (2341) gains `at: SwitchAt`. Its three constructors set
  `Now`, and `#[must_use] const fn with_at(self, at: SwitchAt) -> Self`
  sets it (2013's form for an optional field), so a screen writes
  `MusicSwitch::crossfade(500).with_at(SwitchAt::NextBar)`. The
  manifest's default switch stays `Now`.
  - The new track gets `Load` when it is asked for, so it is ready,
    unless two tracks are already loaded (an overlap under way). Then
    it gets `Load` with the switch, so 2341's rule 5 (at most two
    tracks) holds.
  - `MusicState` holds the switch until `heard` gives a clock of the
    playing track whose `beats_since` (or `bars_since`) the clock
    before is above 0. Then the switch begins, as 2341's rules say.
  - If the playing track has no tempo, or nothing sounds, it begins at
    once. A newer request replaces one that waits, and a track loaded
    for the old one gets `Stop`.
  - The switch begins in the first frame at or past the beat: up to one
    frame late, plus the device's own delay. The docs say so.
- **Shots on a beat:** in a cinematic whose `length` is `Music(cue)`
  and whose cue has a tempo, a shot's `at` may be `Beat(bar, beat)`
  (counted from 0) instead of seconds.
  - `TimedShot::at` becomes `cinematic::At` (`Seconds(f32)` or `Beat {
    bar: u32, beat: u8 }`, `#[non_exhaustive]`).
  - `check` turns a `Beat` into milliseconds with
    `Tempo::ms_of(bar × beats_per_bar + beat)`, with checked
    arithmetic, then the existing checks of order and end apply. An
    index that overflows, or a `ms_of` of `None`, is a shot past the
    end.
  - Problems, one message each: a `Beat` in a cinematic with no music
    or a cue with no tempo; a `beat` not below `beats_per_bar`.
  - Reading both forms: `At`'s `Deserialize` and `Serialize` branch on
    `is_human_readable()`. In RON, read through a private
    `#[serde(untagged)]` helper of `Seconds(f32)` and a one-variant
    enum `Beat(u32, u8)`. RON hands untagged content an enum as a
    one-key map, so `Beat(2, 1)` reads as a beat and `Foo(2, 1)` is
    refused. In the pack (postcard has no `deserialize_any`), use the
    tagged form, so the pack's round trip in
    `crates/assets/src/pack/tests.rs` holds. If RON can't read both
    forms in one field, make `at` optional and add an optional `beat:
    (bar, beat)` beside it, exactly one of the two per shot, and say so
    in the PR.
  - The module docs at the top of `cinematic.rs` show a `Beat` shot.
- **Either order with 2364:** if 2364 is done, its keys, cues and
  markers read `at` through `At` too, so they take beats (a few lines).
  If not, 2364 does that when it is worked, as its scope says.
- **Either order with 2205:** if 2205 is done, add
  `Sequence::beat_at(tick) -> shuyi_core::beat::BeatPos`: beats from its
  ticks per quarter note, bars from its time signature (4/4 if it keeps
  none), so a screen keeps time with MIDI and recorded music the same
  way. If not, 2205 adds it when it is worked (it gets an edit).
- **Either order with 2218:** if 2218 is done, its `AudioManifestRon`
  writer writes a music cue's `tempo` when it has one, in the file's
  form (`bpm` as a number, so `bpm_milli` 92 500 is `bpm: 92.5`), and
  its round trip covers a cue with a tempo. If not, 2218 does it (its
  clause says so).
- **Either order with the loop-region ticket 2902 writes** (it has no
  id yet; that ticket puts its id here): if `MusicCue` already has a
  loop region (that ticket is done), `beats_since` and `bars_since`
  count across its wrap, from `loop_end_ms` back to `loop_start_ms`, as
  `MusicClock` wraps, with the beat at the loop point counted once (a
  test). If not, nothing to do: that ticket wraps them (2902 says so).
- **The pack:** `MusicCue`'s and `CinematicFile`'s binary forms change,
  so `shuyi_assets::pack::VERSION` (`crates/assets/src/pack.rs`) goes
  to the next number. One music cue in the pack test's sources gets a
  tempo, so the snapshot of
  `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
  pins `Tempo`'s form, and the snapshot is accepted again.
- **Either order with 2366 (parked, sound captions):** both add a field
  to `MusicCue`, and whichever lands first moves it under 2013's ADR
  if it isn't already. If 2366 is done, its `new` leaves the new
  `tempo` at `None` too, and `pack::VERSION` goes one past 2366's. If
  not, nothing more to do here: 2366 adds its `caption` to this `new`.
- **An ADR** (the `write-adr` skill) that extends 2341's ADR on music
  switches: beats are integers in `shuyi-core`; a view reads them from
  the music clock, a sim from its steps (`at_steps`), never from the
  device; switches land on frames. It says why this is in the default
  crates and not a module (ADR-0009 §1): the music cue, the music clock
  and the cinematic file hold it, no default crate depends on a module,
  and it adds no dependency and one optional field per cue. Its row
  goes in `docs/adr/README.md`.
- **For the game:** the API breaks. `MusicCue`, `MusicClock` and
  `TimedShot` gain or change fields, and `MusicCue` and `MusicClock`
  move under 2013's ADR here if they aren't already. Code that builds
  them with struct literals, or reads `TimedShot::at` as `f32`,
  changes. The completion notes carry a `Breaking:` line in 2010's form
  (`Breaking: shuyi-assets, shuyi-ui: ...`), the PR says so, and the
  game's PR that moves its pin follows it. Its files need no change.

**Out (do not do):**
- tempo changes inside a track (one tempo per track; a follow-up when a
  game asks), and time signatures other than a beat count per bar;
- loop regions (2902 writes that ticket); the beat clock wrapping at
  one is In, above (either order);
- starting a track part-way in so its beats line up, stems in sync,
  scheduling to the sample, and calibrating the device's delay: a
  follow-up when a game asks, on the stack 2902 picks (2902 notes them
  among its needs);
- fade lengths in beats, which 2341 sends here: a game writes
  `tempo.beat_ms() * n` as the switch's milliseconds;
- stopping the music on a beat (a follow-up when a game asks);
- reading the tempo from the file's own tags: a follow-up when a game
  asks (it would read the file's comments, as the loop-region ticket
  2902 writes does);
- MIDI playback (2205, 2206), and game speed (2123);
- a rhythm example or template (a follow-up if Nick names rhythm games
  as a kind of game the engine serves).

## Implementation steps

1. Read 2341's `MusicSwitch`, `MusicState::heard` and its rules,
   `crates/ui/src/audio.rs` (`MusicClock`, `MusicState`),
   `crates/assets/src/audio.rs` (`MusicCue`, `RawMusic`,
   `Check::music`), `crates/assets/src/cinematic.rs` (`TimedShot`,
   `check`, `millis`) and 2013's ADR. Check whether 2364 and 2205 are
   done.
2. `shuyi_core::beat` with its tests, first, and its re-exports in
   `crates/core/src/lib.rs`.
3. The cue's tempo (`RawTempo`, `MusicCue::tempo`), its checks, and the
   pack version. Update every literal (`grep -rn "MusicCue {" crates
   templates examples`).
4. `MusicClock`'s tempo, length and beats.
5. `SwitchAt` and `with_at` in `MusicSwitch`, then the waiting switch
   in `MusicState`, then the Harness test.
6. `cinematic::At`, its two forms, the checks of a `Beat`, and the
   module docs.
7. The ADR, then the gates.

Size: expect about 800 changed lines. If the diff runs well past 600,
shots on a beat (step 6 and the 2364 part) go to a follow-up ticket
written in this PR, which then owns `cinematic::At` for 2364.

## Acceptance criteria

- [ ] At 120 bpm with the first beat at 250 ms, `Tempo::at(249)` is
      `None`, `at(250)` is beat 0, `at(749)` beat 0, `at(750)` beat 1,
      and `at(2250)` is bar 1, beat 0 in 4/4.
- [ ] At 60 steps a second and 90 bpm, `at_steps` matches a reference
      written with exact fractions (`u128` numerator and denominator in
      the test) for every step of 10 minutes: no drift.
- [ ] `shuyi-core` still builds for `thumbv7em-none-eabihf`.
- [ ] Through the Harness, `play_music_with(cue,
      MusicSwitch::crossfade(500).with_at(SwitchAt::NextBar))` gives
      `Load` in the frame of the request and `Start` in the first frame
      whose music clock is at or past the next bar's first beat, never
      before. A `NextBeat` switch away from a track with no tempo starts
      at once.
- [ ] A `NextBar` request during an overlap never has more than two
      tracks between `Load` and `Stop`.
- [ ] `beats_since` over a loop wrap counts the beat at the loop point
      once.
- [ ] A cinematic with `at: Beat(1, 2)` on a 120 bpm 4/4 cue whose
      first beat is at 0 ms puts that shot at 3000 ms. One whose cue has
      no tempo is refused with a message, and so is `Foo(2, 1)`.
- [ ] Every cinematic file that reads today reads the same, and the
      existing cinematic tests pass unchanged.
- [ ] An `audio.ron` with no tempo reads as before; bad tempos are
      refused, one message each.
- [ ] `pack::VERSION` is one higher and the pack snapshot is accepted.
- [ ] The ADR is written, with its row in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Tempo`'s functions on known values, `ms_of` and `next_bar`
  near `u32::MAX`; the manifest's checks; `MusicClock::beat`,
  `beats_since` and `bars_since`; `SwitchAt` in `MusicState` (waiting,
  replaced, no tempo, silence, during an overlap); `Beat` shots and
  their problems; `At` read from `6.0`, `6` and `Beat(2, 1)`.
- Property: `at(ms_of(n))` is beat `n`; `beats_crossed(a, b) +
  beats_crossed(b, c) == beats_crossed(a, c)`, and the same for
  `bars_crossed`; `at_steps` equals `at` wherever a step falls on a
  whole millisecond; 2341's property (at most two tracks between `Load`
  and `Stop`, every gain in 0 to 1) with `SwitchAt` and tempos in the
  random requests.
- Integration: a beat switch through the Harness
  (`crates/ui/tests/it/audio.rs`).
- Snapshot: the pack's binary forms, accepted with the new version.

## Completion notes

*(Filled in by the session that completes the ticket.)*
