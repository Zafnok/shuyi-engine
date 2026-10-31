---
id: "2378"
title: "Cue rules, part two: file lengths, copy caps, priority and a voice limit, with one copy stopped alone"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2345"]
nick_input: none
completed:
---

# 2378 — Cue rules, part two: copy caps, priority and a voice limit

## Context

2345 (cue rules as data) was about 1 400 changed lines, so it split in
two. 2345 is part one: the cue rules' place in `shuyi-platform`'s
`App` (`Cues`, in `crates/platform/src/audio/cues.rs`), `CueRules`,
variants without repeats, a random volume, minimum gaps, the drops seen
in tests and by agents, and an ADR that decides every cue rule, the copy
cap and the voice rule included. This ticket is part two: it makes the
rest of that ADR run.

The 2026-10-20 comparison in 2345's Context found the same rules in
other engines. Unreal's Sound Concurrency has a maximum count, a
resolution rule (prevent new, stop oldest, stop quietest, stop lowest
priority) and a retrigger time. Godot's players have `max_polyphony`.
Unity's `AudioSource` and GameMaker's `audio_play_sound` take a
priority, and a GameMaker game sets how many sounds play at once.

What the code has once 2345 is done:

- `Cues::apply` turns a frame's requests into `SoundRequest`s, picks
  each sound's file and volume, and drops a play inside its cue's gap as
  `DroppedSound { cue, why: DropWhy::Gap }`. `DropWhy` is
  `#[non_exhaustive]`. The Harness lists the drops
  (`Harness::dropped_sounds`), and `run` answers them under `dropped`
  (`Dropped { frame, cue, why }`, `why` being `gap`).
- `SoundRequest::Sound` is `{ cue, volume, variant, cue_volume }`.
  Nothing counts the copies playing.
- `shuyi-app`'s `Backend::play` (`crates/app/src/audio.rs`) returns
  nothing, and `Backend::stop(sound)` stops every copy of a sound.
  quad-snd 0.2.8's `Sound::play` returns a `Playback` that stops one
  copy, but macroquad's `play_sound` throws it away.
- The `Sound` kind (`crates/assets/src/audio/formats.rs`) is `channels`
  and `sample_rate`, with no length. `ogg_length_ms`
  (`crates/assets/src/audio.rs`) measures a whole OGG file, and
  `wav_format` reads a WAV's `fmt ` chunk. The tests' `wav` helper
  writes no `data` chunk.

Why it matters:

- **Other 2D games:** twenty hits in one frame clip, and a footstep that
  should restart plays over itself. A cap per cue and a priority fix
  both.
- **Ports.** The GBA mixer and the in-house mixer have a fixed number of
  voices, and neither picks which sound loses. The end goal
  (2026-10-19) is "a wide variety of ports" from one game. So the rule
  is decided once, in `App`, before any backend, and the game sounds the
  same on every port.
- **The game** doesn't need it. Every new field is optional, and the
  defaults are today's behaviour, so its manifest loads as it is. Its
  code changes where the API breaks (Scope, *For the game*).

The rules, as 2345's ADR decides them:

- a copy ends at its start plus its file's length, read from the file's
  header;
- the copy cap, with `Skip` or `StopOldest`;
- the voice rule: when `max_voices` copies sound, a new play takes the
  voice of the copy with the lowest priority (the oldest of those) if
  that priority is at most its own, and is dropped otherwise;
- a backend's own voice maximum is a hard limit under these rules and
  never drops or steals a sound on its own.

## Nick input

None.

## Scope

**In:**
- **The data** (`crates/assets/src/audio.rs`), all optional in the
  manifest, so every manifest written today loads unchanged:
  - `CueRules` gains `max_copies: Option<u8>` (at most this many copies
    of the cue sound at once; `None` is no cap), `when_full: WhenFull`
    and `priority: u8` (higher wins; 128 when left out);
  - `WhenFull` (the derives of 2345's `Pick`): `Skip`, the default (the
    new play is dropped), and `StopOldest` (the cue's oldest copy is cut
    and the new one plays, so a footstep with `max_copies: 1,
    when_full: StopOldest` restarts);
  - since `priority` doesn't default to 0, `CueRules`'s `Default` is
    written by hand, and 2345's `CueRules::DEFAULT` gets the same
    values (its test that the two are equal still passes);
  - `AudioManifest::max_voices: Option<u16>`: at most this many copies
    of all cues sound at once. `None` is no limit;
  - the RON, flat on the sound entry, and `max_voices` at the top of
    the file, written bare (2345's `parse_ron_implicit_some`):
    ```ron
    (id: "hit", files: ["sfx/hit_1.wav", "sfx/hit_2.wav"], volume: 80,
     credit: Own, max_copies: 3, when_full: StopOldest, priority: 200),
    ```
    `RawSound` and `RawFile` take each field with `#[serde(default)]`
    (`priority` with a function that gives 128);
  - the checks, each an error at its line: `max_copies: 0` (naming the
    cue), `max_voices: 0`, and a file that is read but whose length
    can't be measured, when its cue has `max_copies` or the manifest has
    `max_voices` (naming the cue and the file).
- **Lengths** (`crates/assets/src/audio.rs`):
  - `pub fn sound_length_ms(name: &str, bytes: &[u8]) -> Option<u32>`,
    to the nearest millisecond, in checked `u64` integers. A WAV's
    length is its `data` chunk's size over the block align of its
    `fmt ` chunk, times 1 000, over its sample rate. It is `None` with
    no `data` chunk, or a block align or a rate of 0. An OGG's is
    `ogg_length_ms`;
  - `SoundCue::lengths_ms: Vec<Option<u32>>`, one per file, filled by
    `Check::sound` (`None` for a file `sound_left_out` leaves out) and
    never written in the RON. `SoundCue::new` leaves it empty and stays
    `const`. A list shorter than `files` means the rest are unknown;
  - `AudioManifest::measure_sounds(&mut self, read: &dyn Fn(&str) ->
    Bytes)` fills the lengths of a manifest built in code;
  - the `Sound` kind gains `length_ms: Option<u32>`, from
    `sound_length_ms`. Fix its literal in
    `crates/assets/src/format/builtin_tests.rs` and its pattern in
    `formats.rs`;
  - the manifest's binary form changes, so `pack::VERSION`
    (`crates/assets/src/pack.rs`) becomes the next number, and the
    pinned snapshot in `crates/assets/src/pack/tests.rs` is accepted;
  - **Either order with 2225:** if 2225 is done, `sound_length_ms`
    gives a `.sfx.ron` the length it renders to (`n × 1000 /
    SAMPLE_RATE`, `n` from its `SfxDef`), whether the bytes are its RON
    text (as `AudioManifestRon` reads the bundle) or the WAV a game's
    hook rendered, and `SfxRon` fills the `Sound` kind's `length_ms`; if
    not, nothing to do (2225 does it).
  - **Either order with 2234:** if 2234 is done, `sound_length_ms`
    gives a `.flac` or `.mp3` file the length of 2234's
    `audio::track_length_ms`, so a FLAC sound of a cue with
    `max_copies` has a length; if not, nothing to do (2234 does it).
- **Copies in `Cues`** (`crates/platform/src/audio/cues.rs`, `no_std`,
  integers only):
  - `CopyId`: a `Copy`, `Ord` id around a `u64` counter that `Cues`
    keeps, from 1;
  - the copies sounding: `(CopyId, cue, priority, started, ends)`,
    `ends` being `started` plus the length of the file picked. A copy
    whose file's length is unknown ends at once, so it never holds a
    copy or a voice. Copies whose `ends` is at or before `now` are
    forgotten first, each frame;
  - each `PlaySound`, in order: the gap (2345), then the cue's copies,
    then the voices. At `max_copies`, `Skip` drops the play as
    `DropWhy::Copies`, and `StopOldest` cuts the cue's oldest copy
    (which frees a voice, so the play sounds). At `max_voices`, the play
    cuts the copy with the lowest priority, the oldest of those, if
    that priority is at most its own; otherwise it is dropped as
    `DropWhy::Voices`. Ties in age go to the lower `CopyId`;
  - a cut copy gives a `SoundRequest::StopCopy` before the new play's
    `Sound`, and a `DroppedSound` with `DropWhy::Cut` that names the cut
    copy's cue;
  - a play dropped as `Copies` or `Voices` draws nothing from the
    variant RNG and starts no gap, as 2345's gap drop does. A play that
    sounds draws its file and volume (2345), then gets its `CopyId`;
  - music and voice clips have their own players and don't count;
  - `DropWhy` gains `Copies`, `Voices` and `Cut` (additive: it is
    `#[non_exhaustive]`).
- **What reaches the device** (`crates/platform/src/audio.rs`):
  - `SoundRequest::Sound` gains `copy: CopyId`, and `SoundRequest`
    gains `StopCopy { copy: CopyId }`: stop that one copy, and leave
    the other copies of its file playing. A copy that has already ended
    is no error;
  - the headless `Speaker` (`crates/platform/src/headless.rs`) records
    `StopCopy` with the frame's other requests. `Harness::sounds()`
    doesn't change, and `Harness::dropped_sounds()` lists the new
    reasons;
  - fix the `SoundRequest::Sound` literals and patterns in
    `crates/platform/src/audio.rs`'s tests,
    `crates/platform/src/app/tests.rs` and
    `crates/platform/src/headless/tests.rs`.
- **The contract:** `run`'s `dropped` gives `why` as `copies`,
  `voices` and `cut` for the new reasons, with no change of shape. If
  the schemas change (a doc comment that lists the words, say), raise
  `contract::VERSION` (`crates/cli/src/contract.rs`), regenerate them
  and add the line to `crates/cli/schemas/versions.json` (ADR-0022 §5).
- **`shuyi-app` stops one copy alone** (`crates/app/src/audio.rs`,
  `crates/app/src/backend.rs`):
  - **From 2344 (done):** `Backend::play` returns `Self::Playing`, one
    playback, and `Backend` has `set_playing_volume` and
    `stop_playing(playing)`. Sound effects load into a quad-snd
    `AudioContext` of `shuyi-app`'s own, natively and on the web
    (`device::Macroquad`'s `effects`), and quad-snd is a dependency on
    every target. Use them. A handled sound (`SoundRequest::Start`)
    gets no `copy` and holds no copy or voice in `Cues`: the game stops
    it itself;
  - the player keeps each copy's `Playing` by its `CopyId` until
    `StopCopy` names it or its length (the cue's `lengths_ms`) has
    passed by the frame's time. A copy of unknown length keeps none;
  - the tests' `Fake` backend records each playback's start and stop.
- **From 3029 (done):** the player's rules are in
  `crates/platform/src/audio/player.rs`, so keep each copy's playback
  there, and the Playdate backend (`crates/playdate/src/audio.rs`) keeps
  each copy's sample player until `StopCopy` names it or its length has
  passed, and stops that copy alone (a test against its fake SDK: of two
  copies of one sound, one stops and the other plays on). 3029 left this
  to this ticket, since `CopyId` is this ticket's.
- **Either order with 3017** (parked, target capabilities): if 3017 is
  done, `Cues`' voice limit is the smaller of `max_voices` and the
  running target's `capabilities.audio.channels`, a `None` on either
  side being no limit, as 3017 says; if not, nothing to do (3017 does
  it when it lands).
- **Either order with 3031** (parked, in-house output): if 3031 is
  done, its backend makes the `Mixer` with `max_voices` voices, or 32
  when the manifest has none, so the mixer never refuses a play the cue
  rules let through; if not, nothing to do (3031 reads `max_voices`).
- **Either order with 2218:** if 2218 is done, its `AudioManifestRon`
  writer writes a cue's `max_copies` (bare), `when_full` and `priority`,
  and the manifest's `max_voices` (bare), each only when it isn't the
  default, and never `lengths_ms`; its round trip covers a cue and a
  manifest with them. If not, nothing to do (2218 adds them to this
  writer).
- **Docs:** `docs/formats.md` (the new fields, their checks, and that
  lengths are measured, never written), `docs/platform-backends.md`
  (`copy` and `StopCopy`: a backend stops one copy alone),
  `docs/contract.md` (the new `why` words). 2345's ADR already decides
  these rules; if the code must differ from it, amend it (the
  `write-adr` skill).
- **Public types (ADR-0039):** `WhenFull` may grow (a "stop quietest"
  rule), so it is `#[non_exhaustive]`; `CopyId` is a plain value.
- **For the game:** the API breaks. `SoundRequest::Sound` gains `copy`
  (match it with `..`), `SoundRequest` gains `StopCopy` (an exhaustive
  match breaks), `AudioManifest` gains `max_voices` (a literal without
  `..` breaks), and the `Sound` kind gains `length_ms`. Each break gets
  a `Breaking:` line in the completion notes (ADR-0038). The pack's
  version goes up, so a pack baked before this is refused and rebaked.
  A backend of the game's own, such as its Playdate front end
  (game#1170), stops the one copy `StopCopy` names. The PR says so, and
  the game's PR that moves the pin follows it.

**Size.** If the PR passes about 600 changed lines (fixtures and
snapshots not counted), the lengths go first as a ticket of their own
(`write-ticket`, 23xx, After MVP, blocked by 2345): `sound_length_ms`,
`lengths_ms`, `measure_sounds`, the `Sound` kind's `length_ms`, the pack
version and the 2225 clause. That PR adds it to this ticket's
`blocked_by` and points 2225's clause at it.

**Out (do not do):**
- what 2345 does: the cue rules' place in `App`, variants, the random
  volume, gaps, `Dropped` in the contract, and the ADR;
- pitch or rate variation (2379, parked);
- limits per owner, culling by distance and positional sound (2363,
  parked). A sound 2363 drops never reaches `Cues`, so it holds no copy
  or voice;
- sound groups, buses and limits per group (2342), and ducking (2343);
- looped sounds and handles a screen holds (2344), beyond the
  per-playback handle above. A handled sound counts toward no cap;
- the in-house mixer's and the GBA's own voice maximum (3030, 3027,
  parked): a hard limit under these rules that never drops or steals a
  sound on its own;
- a "stop quietest" rule, or "stop lowest priority" for the copy cap:
  write a ticket if a game shows it needs one.

## Implementation steps

1. Read 2345's ADR and its `Cues`; `crates/assets/src/audio.rs`
   (`Check::sound`, `wav_format`, `ogg_length_ms`, the tests' `wav` and
   `ogg_file`); `crates/assets/src/audio/formats.rs`;
   `crates/platform/src/audio.rs`, `headless.rs` and `harness.rs`;
   `crates/app/src/audio.rs` and `backend.rs`; and
   `crates/cli/src/contract/run.rs`. Check whether 2218, 2225, 2344,
   3017, 3029 and 3031 are done (the either-order clauses).
2. Lengths: `sound_length_ms`, `lengths_ms` filled in `Check::sound`,
   `measure_sounds`, and the `Sound` kind's `length_ms`. Give the tests
   a WAV helper with a `data` chunk of a known number of frames.
3. The data: `max_copies`, `WhenFull`, `priority`, `max_voices`, the
   RON fields, the checks, the hand-written `Default` and
   `CueRules::DEFAULT`, the pack version and its snapshot. Measure the
   diff here: if steps 2 to 6 look like more than about 600 lines, split
   as **Size** says.
4. `Cues`: `CopyId`, the copies sounding, the cap, the voice rule and
   the new `DropWhy`s, with the unit tests and the property test.
5. `SoundRequest::Sound`'s `copy`, `SoundRequest::StopCopy`, the
   `Speaker`, and the platform's tests; the Harness test.
6. `shuyi-app`: the per-playback handle (2344's), a
   handle per copy, and `StopCopy`. Check by ear on the desktop and in
   the web build (the `run` skill) that hello-grid's menu and cursor
   sounds still play.
7. The contract's words, the docs and the `Breaking:` lines. Run the
   gates.

## Acceptance criteria

- [ ] Every manifest in the repository's tests loads unchanged; `FULL`
      loads with `CueRules::default()` (no cap, priority 128) on each
      cue and `max_voices: None`.
- [ ] A cue with `max_copies: 3, when_full: StopOldest, priority: 200`
      and a manifest with `max_voices: 12`, written bare, load to those
      values. `max_copies: 0`, `max_voices: 0`, and a cue with
      `max_copies` whose WAV has no `data` chunk, are errors at their
      line.
- [ ] `sound_length_ms` gives the length to the millisecond of mono and
      stereo WAV bytes with a `data` chunk and of OGG bytes, built in
      the tests; it gives `None` for a WAV with no `data` chunk and for
      a block align of 0. `Check::sound` fills `lengths_ms`, and the
      `Sound` kind's `length_ms` agrees.
- [ ] At no time do more than `max_copies` copies of a cue, or
      `max_voices` in all, sound: a property test over random rules,
      lengths and timed plays, checked by a model that rebuilds the
      copies sounding from the `Sound` and `StopCopy` requests and the
      lengths.
- [ ] The voice rule, in unit tests: with the voices full of higher
      priorities, a lower one is dropped as `Voices`; an equal or higher
      one cuts the oldest of the lowest (a `StopCopy` for it, reported
      as `Cut`) and sounds.
- [ ] `Skip` drops a play over the cap as `Copies`, and `StopOldest`
      cuts the cue's oldest copy and the new one sounds; a copy whose
      length has passed holds nothing; a file of unknown length never
      holds a copy or a voice (unit tests).
- [ ] A play dropped as `Copies` or `Voices` draws nothing: the same
      requests with the dropped ones taken out give the same files and
      volumes (property test). The same seed and the same requests give
      the same plays, cuts and drops.
- [ ] In the Harness, with the toy game's `click` given `max_copies: 1`
      and a length of 500 ms (on its `lengths_ms`), Confirm on the menu
      overlay twice 100 ms apart lists the second click in
      `dropped_sounds()` as `Copies`, and `run` answers it under
      `dropped` with `why: copies`.
- [ ] `shuyi-app`'s player, against its fake backend: of two copies of
      one file, `StopCopy` stops exactly the copy it names and no other
      copy of the same file, and a copy whose length has passed gives
      back its handle.
- [ ] `pack::VERSION` is one higher, with the pack snapshot updated;
      the contract's version and schemas are updated if they changed.
- [ ] `shuyi-platform` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the RON fields, written bare, and the checks; `Default` and
  `CueRules::DEFAULT` equal; `sound_length_ms` on built WAV and OGG
  bytes; `measure_sounds`; the `Sound` kind's `length_ms`; `Skip`,
  `StopOldest`, the voice rule, ends, unknown lengths; the `Speaker`
  records `StopCopy`.
- Property: the caps against the model; a dropped play draws nothing;
  the same seed gives the same result.
- Integration: the Harness test above; `run` with a `copies` drop
  through the toy game; `shuyi-app`'s player against its fake backend
  (one copy stopped alone, a handle given back).
- Snapshot: the pack's pinned binary forms; the contract's schemas if
  they changed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
