---
id: "2345"
title: "Cue rules as data: variants without repeats, random volume, copy caps, minimum gaps and priority, decided once for every backend"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2345 — Cue rules as data

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and GameMaker. Godot, Unity, Unreal and GameMaker each
say, per sound, how its variants are picked or what happens when too
many copies play at once; Bevy doesn't:

- Godot's `AudioStreamRandomizer` picks at random, at random without
  repeating the last one, or in order, with a random volume and pitch
  (docs.godotengine.org/en/stable/classes/class_audiostreamrandomizer.html).
  Its players have `max_polyphony`.
- Unity's Audio Random Container avoids repeats and varies volume and
  pitch (docs.unity3d.com/Manual/AudioRandomContainer.html), and an
  `AudioSource` has a priority.
- Unreal's Sound Concurrency has a maximum count, a resolution rule
  (prevent new, stop oldest, stop quietest, stop lowest priority) and a
  retrigger time
  (dev.epicgames.com/documentation/en-us/unreal-engine/sound-concurrency-reference-guide-in-unreal-engine).
- GameMaker's `audio_play_sound` takes a priority, and a game sets how
  many sounds play at once.
- Bevy's `bevy_audio` has none of this.

What Shuyi has today:

- `SoundCue` (`crates/assets/src/audio.rs`) is `files`, `volume` and
  `credit`. Its doc says one file "is picked at random each time".
  `from_source` reads the manifest with `parse_ron`, so an `Option`
  field would have to be written `Some(…)`.
- `Audio::play_sound` in `shuyi-app` (`crates/app/src/audio.rs`) picks
  `variants[self.rng.below(variants.len())]` with `VariantRng`, a
  SplitMix64 kept apart from the simulation's RNG. The same file can
  play twice in a row, and nothing counts the copies playing.
  `Audio::load` leaves out a file that fails to decode, so a cue's
  decoded variants don't always line up with its `files`.
- That player's `Backend::play(sound, volume, looped)` returns nothing,
  and `Backend::stop(sound)` stops every copy of a sound. quad-snd
  0.2.8's `Sound::play` returns a `Playback` that stops one copy, but
  macroquad's `play_sound` throws it away. quad-snd's `PlaySoundParams`
  is `looped` and `volume` only, so no backend can play a pitch today.
- The `Sound` kind (`crates/assets/src/audio/formats.rs`) is `channels`
  and `sample_rate`, with no length. `ogg_length_ms` already measures a
  whole OGG file, for music.
- `AudioQueue::cursor_tick` (`crates/ui/src/audio.rs`) plays the cursor
  cue on every step, and hello-grid's stage calls it once per cell
  (`examples/hello-grid/src/screens/stage.rs`), so a cursor that moves
  several cells in one frame plays several ticks on top of each other.
- `App`'s `step` (`crates/platform/src/app.rs`) turns a frame's
  requests into `SoundRequest`s and calls
  `platform.audio().play(&frame, elapsed.now)`, with time in whole
  milliseconds (ADR-0024 §3). The headless `Speaker` records them, the
  Harness records the screens' requests per frame, and `run` answers
  with the cues asked for (`RunOutput::sounds`,
  `crates/cli/src/contract/run.rs`).

Plans: 3029 moves the player's rules ("picking a variant, how many
voices") to a place every backend shares, and adds none. 3030 (parked)
gives the in-house mixer "a maximum number of voices, set at creation"
with no rule for which sound loses. 3027 (parked) gives the GBA mixer a
fixed number of voices with no rule either. 3017 (parked) gives each
target an audio channel count. Nobody plans the rules themselves.

Why it matters:

- **The game** doesn't need it and doesn't change: every new field is
  optional, and the defaults are today's behaviour. Its manifest loads
  as it is.
- **Other 2D games** hear the gaps first: footsteps and hits sound
  mechanical when the same file repeats, twenty hits in one frame clip,
  and a cursor ticks several times at once.
- **Ports.** The GBA (3027) and the in-house mixer (3030) have a fixed
  number of voices. If each backend picks which sound loses, a game
  sounds different on each port. The end goal (2026-10-19) is "a wide
  variety of ports" from one game, so the rule is decided once, before
  any backend.

Choices, checked against the code:

- **The rules run in `shuyi-platform`'s `App`, before the device.**
  `App` already has the game's manifest (`ctx.game.audio_manifest()`),
  the frame's time in `Millis`, and the requests as
  `Game::collect_audio` hands them out. So shuyi-app, the headless
  backend and the Playdate backend later all get the same decisions,
  the Harness sees them, and the times are integers that give the same
  result on every port. The variant pick moves there with them: a
  request reaches the device with its file and its volume already
  chosen.
- **A copy's end comes from its file's length.** The manifest's check
  reads every sound file already (`Check::sound`), so it can measure
  each one then: a WAV's data chunk over its block align, an OGG's last
  granule (`ogg_length_ms`).
- **Pitch is left out.** No backend can play it. 3030's resampler can,
  and the Playdate SDK's sample player can change its rate, so pitch
  comes with the in-house mixer. A field that does nothing on the
  desktop and the web would mislead.
- **Not blocked by 3029.** 3029 waits for the Playdate backend (3010).
  Either order works (below).
- **Two PRs.** All of it is about 1 400 changed lines. This ticket is
  part one: the place, the data, variants, volume and gaps, the drops
  seen in tests and by agents, and an ADR that decides every rule,
  copy caps and the voice rule too. Part two, a ticket this one writes
  in step 1, adds file lengths, copy caps, priority, the voice limit and
  one copy stopped alone.

## Nick input

None.

## Scope

**In:**
- **The data** (`crates/assets/src/audio.rs`), all optional in the
  manifest, so every manifest written today loads unchanged:
  - `CueRules` (derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`,
    `Default`, `Serialize`, `Deserialize`), a new field `rules` of
    `SoundCue`:
    - `pick: Pick`: `Random` (the default, today's), `NoRepeat` (never
      the last file again), `Shuffle` (each file once in a random
      order, then a new order, never the same file twice in a row
      across the seam), `InOrder` (in the order of `files`, then from
      the first again). A cue of one file plays it every time;
    - `volume_min: Option<u8>`: each play picks a whole percent from
      `volume_min` to the cue's `volume`. `None` plays at `volume`, as
      today;
    - `min_gap_ms: u32` (0): a play less than this after the cue's last
      play that sounded is dropped;
  - `SoundCue::new(files, volume, credit)`, with the default rules, and
    the four literals outside the manifest's check moved onto it
    (`crates/app/src/audio.rs` tests, `crates/assets/src/credits.rs`
    tests, `crates/ui/src/toy.rs`, `examples/hello-grid/src/sound.rs`);
  - the RON, flat on the sound entry:
    ```ron
    (id: "step", files: ["sfx/step_1.ogg", "sfx/step_2.ogg", "sfx/step_3.ogg"],
     volume: 60, credit: Own, pick: NoRepeat, volume_min: 45, min_gap_ms: 60),
    ```
    `RawSound` takes each field on its own with `#[serde(default)]`
    (`#[serde(flatten)]` doesn't work with its `deny_unknown_fields`);
  - `from_source` reads with `parse_ron_implicit_some`
    (`crates/assets/src/ron.rs`, as `crates/assets/src/target.rs` does)
    instead of `parse_ron`, so `volume_min: 45` is written bare. No
    field of today's manifest is an `Option`, so nothing written today
    reads differently;
  - `Check::sound`: `volume_min` over `volume` is an error naming the
    cue, at its line;
  - the manifest's binary form changes, so `pack::VERSION`
    (`crates/assets/src/pack.rs`, 2 today) becomes the next number and
    the pinned snapshot in `crates/assets/src/pack/tests.rs` is
    accepted.
- **The rules at run time:** `shuyi_platform::audio::cues`
  (`crates/platform/src/audio/cues.rs`, a child of `audio.rs`),
  `no_std`, integers only:
  - `Cues::new(seed: u64)` and `Cues::apply<V: ClipVariant>(&mut self,
    manifest: &AudioManifest, now: Millis, asked: &[AudioRequest<V>],
    out: &mut Vec<SoundRequest>, dropped: &mut Vec<DroppedSound>)`. It
    turns the frame's requests, in order, into `SoundRequest`s: each
    `PlaySound` through the rules below, every other request through
    `SoundRequest::from_request` as today;
  - the variant RNG: `VariantRng` moves here from `shuyi-app` with its
    tests. `Cues::new` seeds it with `splitmix64(seed ^ SALT)`
    (`shuyi_ui::audio::splitmix64`, a fixed `SALT`), so it doesn't
    repeat the music pool's picks. It never touches the simulation's
    RNG;
  - per cue, in a `BTreeMap` by id: the last file played, the shuffle
    order and where it is, the next file in order, and the time of the
    last play that sounded;
  - each `PlaySound`: first the gap (`DropWhy::Gap`). A dropped play
    draws nothing from the RNG. A play that sounds draws its file, then
    its volume, and becomes a `SoundRequest::Sound` with them;
  - a cue the manifest lacks, or one with no files, goes through with
    `variant: 0` and `cue_volume: 0`, and the player warns of it as
    today. A manifest built in code with `volume_min` over `volume`
    plays at `volume`;
  - `DroppedSound { cue: String, why: DropWhy }`. `DropWhy` is
    `#[non_exhaustive]` with one variant, `Gap`; part two adds
    `Copies`, `Voices` and `Cut`.
- **What reaches the device** (`crates/platform/src/audio.rs`):
  - `SoundRequest::Sound` gains `variant: usize` (an index into the
    cue's `files`) and `cue_volume: u8` (the percent picked). Its doc
    says `volume` multiplies `cue_volume`;
  - `App` keeps a `Cues`, made from `ctx.music_seed` when the game
    starts running (in `load`, and in `running`). `step` builds the
    frame's sound requests with `Cues::apply` in place of today's
    `filter_map` over `SoundRequest::from_request`, keeps the frame's
    drops in `Lists`, and gives them out with
    `App::dropped_sounds(&self) -> &[DroppedSound]`;
  - the module's doc no longer says variants wait for 3029: the cue
    rules are `App`'s, and a backend plays what it is told.
- **`shuyi-app` plays what it is told** (`crates/app/src/audio.rs`):
  - `Audio::sounds` keeps one slot per file (`Vec<Option<B::Sound>>`),
    so `variant` indexes the manifest's `files` even when a file failed
    to decode (warned at load, as now). An empty slot plays nothing;
  - `play_sound` plays the slot `variant` names at
    `percent(cue_volume) * volume * sound_volume`. Its `rng` field goes;
  - `proptest` leaves `shuyi-app`'s dev-dependencies: its one use,
    `VariantRng`'s test, moves (`cargo machete` would flag it).
- **Seen in tests and by agents:**
  - the Harness records `App::dropped_sounds` every frame and gives
    them out with `Harness::dropped_sounds(&self) -> Vec<(u32,
    &DroppedSound)>`, each with its frame; `clear_audio` clears them
    with the rest, and `sounds()` still lists the cues asked for;
  - `run`'s answer (`RunOutput`) gets `dropped: Vec<Dropped>`, a new
    type beside `Sound` in `crates/cli/src/contract/session.rs`:
    `Dropped { frame: u32, cue: String, why: String }`, `why` being
    `gap` (part two adds `copies`, `voices` and `cut` with no change of
    shape). A new `Session::dropped_sounds(&self) -> Vec<Dropped>` is
    implemented by `Headless` and `FakeSession`
    (`crates/cli/src/contract/fake.rs`). The text answer gets a
    `dropped:` line when there are any;
  - this changes the contract's answer, so `contract::VERSION`
    (`crates/cli/src/contract.rs`, 4 today) goes up by one, the schemas
    are regenerated, and `crates/cli/schemas/versions.json` gets its
    line (ADR-0022 §5). The MCP server (1133) answers the same when it
    lands.
- **The cursor:** `examples/hello-grid/src/sound.rs` gives its cursor
  cue and its move cue `min_gap_ms: 40`, so several steps in one frame
  tick once. `AudioConfig::cursor_cue`'s doc says to give the cue a
  `min_gap_ms`.
- **Docs:** `docs/formats.md` (the cue rules in the audio manifest),
  `docs/platform-backends.md` (the `audio` section: what a
  `SoundRequest::Sound` now carries, and that the cue rules are
  `App`'s), `docs/contract.md` (`run`'s `dropped`).
- **An ADR** (the `write-adr` skill, the next free number), extending
  ADR-0004 §6 and ADR-0024 §2. It decides every cue rule now, so part
  two and the backends to come (3027, 3029, 3030) follow one text:
  - the cue rules run in `App`, after `Game::collect_audio` and before
    the device, in whole milliseconds, with an RNG of their own;
  - a backend plays the file and the volume it is told;
  - a copy ends at its start plus its file's length, read from the
    file's header;
  - the copy cap, with `Skip` or `StopOldest`, and the voice rule: when
    `max_voices` copies sound, a new play takes the voice of the copy
    with the lowest priority (the oldest of those) if that priority is
    at most its own, and is dropped otherwise;
  - a backend's own voice maximum (3030, 3027) is a hard limit under
    these rules and never drops or steals a sound on its own;
  - pitch waits for a backend that can play it.
- **For the game:** the API changes. `SoundCue` has a new field (build
  it with `SoundCue::new`), `SoundRequest::Sound` has new fields (match
  it with `..`), `RunOutput` has `dropped`, and the pack's version goes
  up, so a pack baked before this is refused and rebaked. A backend of
  the game's own, such as its Playdate front end (game#1170), plays
  `variant` at `cue_volume` instead of picking. The PR says so, and the
  game's PR that moves the pin follows it.
- **Either order:**
  - **3029:** if it is done, the shared player it made plays the given
    file and volume as above; if not, `shuyi-app`'s player does, and
    3029 moves it with these changes in.
  - **2344** (looped sounds with handles): if it is done,
    `SoundCue::new` sets `looped: false`, and `Cues` gives its
    `SoundRequest::Start` a file and a cue volume as it does `Sound`,
    with no gap (the game stops a handled sound itself). If not, 2344
    does that. The second to land rebases onto the other's `SoundCue`
    and bumps `pack::VERSION` again.
  - **2342** (sound groups): if it is done, `SoundCue::new` leaves
    `group` empty, and the player multiplies `percent(cue_volume)` by
    the gains 2342 gives in place of `sound_volume`. If not, 2342 does
    that. The second to land rebases and bumps `pack::VERSION` again.
  - **2363** (parked, positional sound): `Cues` runs in `App` on what
    `Game::collect_audio` hands out, so a sound 2363 drops past its
    `silent` distance never reaches it and counts toward no gap, cap or
    priority. If 2363 is done, `Cues` copies `pan` from `PlaySound` into
    `SoundRequest::Sound`, and a Harness test shows that a sound past
    `silent` starts no gap. If not, nothing to do here.
  - **2218** (writers of the text formats): if it is done, its
    `AudioManifestRon` writer writes the rule fields that aren't the
    default, and its round trip covers a cue with rules. If not, 2218
    picks them up.
  - **Public types (ADR-0039):** `CueRules`, `Pick`, `DroppedSound` and
    `DropWhy` follow it; `DropWhy` is `#[non_exhaustive]` as above.
- **Part two**, written as a ticket in step 1 and not done in this PR:
  - the data: `max_copies: Option<u8>` (at most this many copies of the
    cue sound at once), `when_full: WhenFull` (`Skip`, the default: the
    new play is dropped; `StopOldest`: the cue's oldest copy is cut and
    the new one plays), `priority: u8` (128 when left out, so
    `CueRules`'s `Default` is then written by hand), and
    `AudioManifest::max_voices: Option<u16>` (`None` is no limit),
    read with `#[serde(default)]` on `RawSound` and `RawFile`; the
    checks: `max_copies: 0`, `max_voices: 0`, and a file that is read
    but whose length can't be, when the cue has `max_copies` or the
    manifest has `max_voices`;
  - lengths: `sound_length_ms(name, bytes) -> Option<u32>`, public, to
    the nearest millisecond (a WAV's `data` chunk size over the block
    align of its `fmt ` chunk, over its rate, and `None` with no `data`
    chunk or a block align of 0; an OGG's `ogg_length_ms`);
    `SoundCue::lengths_ms: Vec<Option<u32>>`, one per file, filled by
    `Check::sound` (`None` for a file `sound_left_out` leaves out) and
    never written in the RON; `AudioManifest::measure_sounds(&mut self,
    read: &dyn Fn(&str) -> Bytes)` for a manifest built in code; the
    `Sound` kind's `length_ms: Option<u32>` from the same function
    (fix the literal in `crates/assets/src/format/builtin_tests.rs`);
    `pack::VERSION` up again;
  - a `.sfx.ron` sound (2225): `sound_length_ms` gives the length it
    renders to (`n × 1000 / SAMPLE_RATE` from its `SfxDef`), whether
    the bytes are its RON text (as `AudioManifestRon` reads the bundle)
    or the WAV a game's hook rendered, and `SfxRon` fills the `Sound`
    kind's `length_ms`. Either order: if 2225 is not done, nothing to
    do; if part two lands first, 2225 does it;
  - in `Cues`: the copies sounding, `(CopyId, cue, priority, started,
    ends)`, `ends` being `started` plus the file's length. A copy of
    unknown length ends at once and never holds a copy or a voice.
    Copies whose `ends` is at or before `now` are forgotten first. Each
    play checks the gap, then the cue's copies (`Skip` drops it as
    `Copies`; `StopOldest` cuts the oldest, reported as `Cut`), then
    the voices by the ADR's rule (`Voices`, or `Cut` for the copy that
    loses its voice). Music and voice clips have their own players and
    don't count;
  - `SoundRequest::Sound` gains `copy: CopyId`, and a new
    `SoundRequest::StopCopy { copy }`;
  - `shuyi-app` stops one copy alone. Either order with 2344: if it is
    done, use the per-playback handle it gave `Backend::play`; if not,
    add it as 2344 describes (`Backend::play` returns `Self::Playing`,
    `Backend::stop_playing(playing)`, sound effects on a quad-snd
    context of `shuyi-app`'s own, natively and on the web, where it
    waits for `Sound::is_loaded`; quad-snd 0.2.8, MIT or Apache-2.0,
    allowed by `deny.toml`, is already in the tree through macroquad's
    `audio` feature and is added to the wasm dependencies at the same
    version). The player keeps each copy's handle until `StopCopy`
    names it or its length has passed; a copy of unknown length keeps
    none;
  - 3017 (parked): if it is done, `App` caps `max_voices` with the
    target's `capabilities.audio.channels`; if not, 3017 does it;
  - its criteria: at no time do more than `max_copies` copies of a cue,
    or `max_voices` in all, sound (a property test over random rules
    and timed plays, checked by a model that rebuilds the copies
    sounding from what was played and cut); the voice rule in unit
    tests (a lower priority is dropped when the voices are full of
    higher ones; an equal or higher one cuts the oldest of the lowest);
    `sound_length_ms` on fixture WAV and OGG files to the millisecond,
    and `None` for a WAV with no `data` chunk; `shuyi-app`'s fake
    backend stops exactly the copy `StopCopy` names and no other copy
    of the same file. If part two passes about 600 lines, its lengths
    go first, as a ticket of their own.

**Out (do not do):**
- part two above: its own ticket;
- pitch or rate variation: no backend can play it today (3030's
  resampler, and the Playdate SDK, later);
- limits per owner (a cue's copies per thing in the world), culling by
  distance, and positional sound (2363, parked; ADR-0004 §6);
- sound groups, buses and limits per group (2342), and ducking (2343);
- looped sounds and handles a screen holds (2344);
- a voice count per target (3017, parked), and the in-house mixer's and
  the GBA's own voice maximum (3030, 3027);
- a "stop quietest" rule: write a ticket if a game shows it needs one.

## Implementation steps

1. Write part two as a ticket (the `write-ticket` skill: the next free
   number in the 23xx block, After MVP, `blocked_by: ["2345"]`), from
   **Part two** above. Then search `tickets/open/` for `2345` and point
   each clause about file lengths, copy caps, priority, `max_voices`,
   `StopCopy` or the per-playback handle at the new ticket (2225, 2344,
   2902, 3017, 3027, 3029 and 3030 have such clauses). Add the new
   ticket's row to `docs/ROADMAP.md` beside 2345's.
2. Read `crates/assets/src/audio.rs` (`from_source`, `Check::sound`,
   the tests' `FULL`), `crates/app/src/audio.rs` and its tests,
   `crates/app/src/backend.rs`, `crates/platform/src/app.rs` (`step`,
   `Lists`, `load`, `running`), `crates/platform/src/audio.rs`,
   `crates/platform/src/harness.rs`, and `crates/cli/src/contract/run.rs`,
   `session.rs` and `fake.rs`. Check whether 3029, 2344, 2342, 2363
   and 2218 are done (the either-order clauses).
3. The data: `CueRules`, `Pick`, the RON fields, `parse_ron_implicit_some`,
   the check, `SoundCue::new` and the literals, the pack version and
   its snapshot. `FULL` must load to the same values, with the default
   rules.
4. `Cues` in `crates/platform/src/audio/cues.rs`, with `VariantRng`
   moved in, and its unit and property tests. `proptest` (a workspace
   dependency) becomes a dev-dependency of `shuyi-platform` and leaves
   `shuyi-app`'s.
5. `SoundRequest::Sound`'s new fields; `App`'s `Cues`, `step` through
   it, `App::dropped_sounds`; the Harness's record. Fix the
   `SoundRequest::Sound` literals in `crates/platform/src/audio.rs`'s
   tests, `crates/platform/src/app/tests.rs` and
   `crates/platform/src/headless/tests.rs`.
6. `shuyi-app`: a slot per file, and the given file and volume. Check
   by ear on the desktop and in the web build (the `run` skill) that
   hello-grid's menu and cursor sounds still play.
7. The contract: `Dropped`, `Session::dropped_sounds`, `run`'s answer,
   the version, the schemas.
8. hello-grid's gaps, `AudioConfig::cursor_cue`'s doc, the docs and the
   ADR. Run the gates.

## Acceptance criteria

- [ ] The part-two ticket is in `tickets/open/`, blocked by 2345, with
      the scope and criteria above, and no clause in `tickets/open/`
      names 2345 for file lengths, copy caps, priority, `max_voices`,
      `StopCopy` or the per-playback handle.
- [ ] Every manifest in the repository's tests loads with the default
      rules; a test loads `FULL` and finds `CueRules::default()` on
      each cue.
- [ ] A cue with `pick: NoRepeat, volume_min: 45, min_gap_ms: 60`,
      written bare as above, loads to those values; `volume_min` over
      `volume` is an error naming the cue, at its line.
- [ ] With the default rules, `Cues` passes every request in order,
      voice requests among them, each sound at its cue's volume, and
      drops nothing.
- [ ] `NoRepeat` never plays the same file twice in a row, and
      `Shuffle` plays each of n files once in every n plays and never
      the same file twice in a row across a reshuffle (property tests,
      n from 2 to 8). `InOrder` plays the files in turn, and a cue of
      one file plays it each time (unit tests).
- [ ] The volume picked is always within `volume_min..=volume`
      (property test).
- [ ] Two plays of a cue that both sound are at least `min_gap_ms`
      apart (property test), and two plays of a cue with a gap in one
      frame play once (unit test).
- [ ] A dropped play draws nothing: the same requests with the dropped
      ones taken out give the same files and volumes (property test).
- [ ] The same seed and the same requests give the same plays and
      drops.
- [ ] In the Harness, with the toy game's `click` given
      `min_gap_ms: 40` (`Toy::audio` is public), Confirm on the menu
      overlay (`crates/ui/src/toy.rs`) twice less than 40 ms apart
      lists the second click in `dropped_sounds()` as `Gap`, `sounds()`
      lists both, and `run` answers it under `dropped`.
- [ ] `shuyi-app`'s player, against its fake backend, plays the file
      `variant` names at `cue_volume`; when a cue's second file fails
      to load, `variant: 2` still plays its third file and `variant: 1`
      plays nothing.
- [ ] `contract::VERSION` and `pack::VERSION` are each one higher, with
      the schemas, `versions.json` and the pack snapshot updated.
- [ ] `shuyi-platform` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the RON fields, written bare, and the check; `SoundCue::new`;
  each `Pick` on a small cue; the gap, in one frame and across frames;
  a cue the manifest lacks; the moved `VariantRng` tests;
  `SoundRequest::from_request` for the requests that skip the rules.
- Property: `NoRepeat` and `Shuffle`; the volume range; the gap; a
  dropped play draws nothing; the moved variant-picking test.
- Integration: the Harness test above; `run` with drops through the fake
  session and through the toy game; `shuyi-app`'s player against its
  fake backend (the file, the volume, a failed file's slot).
- Snapshot: the pack's pinned binary forms; the contract's schemas.

## Completion notes

*(Filled in by the session that completes the ticket.)*
