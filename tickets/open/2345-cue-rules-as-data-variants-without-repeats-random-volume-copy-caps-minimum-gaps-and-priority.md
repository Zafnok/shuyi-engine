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
  comes with the in-house mixer (2379, parked). A field that does
  nothing on the desktop and the web would mislead.
- **Not blocked by 3029.** 3029 waits for the Playdate backend (3010).
  Either order works (below).
- **Two PRs.** All of it is about 1 400 changed lines. This ticket is
  part one: the place, the data, variants, volume and gaps, the drops
  seen in tests and by agents, and an ADR that decides every rule,
  copy caps and the voice rule too. Part two is 2378 (blocked by this
  one): file lengths, copy caps, priority, the voice limit and one copy
  stopped alone.

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
  - **From 2024 (done):** `SoundCue` is `#[non_exhaustive]`, and every
    caller outside `crates/assets` builds it with
    `SoundCue::new(files, volume, credit)`, a `const fn`. `new` now
    sets the default rules and stays `const`, so it takes them from a
    `const` (`CueRules::DEFAULT`, which a test checks equals
    `CueRules::default()`), since `CueRules::default()` can't be
    called in a `const fn`. The literals left are in `crates/assets`
    (`Check::sound`, the test of `new` in `audio.rs`, and the tests in
    `credits.rs`) and set `rules`;
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
    `#[non_exhaustive]` with one variant, `Gap`; 2378 adds `Copies`,
    `Voices` and `Cut`.
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
  - the cue rules are `App`'s, and a backend plays what it is told.
- **The shared player plays what it is told**
  (`crates/platform/src/audio/player.rs`, since 3029, ADR-0063):
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
    `gap` (2378 adds `copies`, `voices` and `cut` with no change of
    shape). A new `Session::dropped_sounds(&self) -> Vec<Dropped>` is
    implemented by `Headless` and `FakeSession`
    (`crates/cli/src/contract/fake.rs`). The text answer gets a
    `dropped:` line when there are any;
  - this changes the contract's answer, so `contract::VERSION`
    (`crates/cli/src/contract.rs`) goes up by one, the schemas
    (`run.*`, and `session.step.*`, which answers a `RunOutput`) are
    regenerated, and `crates/cli/schemas/versions.json` gets its line
    (ADR-0022 §5). **From 1133 (done):** the MCP server's `step` tool
    answers through `run::run_output` (`crates/cli/src/contract/run.rs`)
    as `run` does, so filling `dropped` there covers both.
- **The cursor:** `examples/hello-grid/src/sound.rs` gives its cursor
  cue and its move cue `min_gap_ms: 40`, so several steps in one frame
  tick once. `AudioConfig::cursor_cue`'s doc says to give the cue a
  `min_gap_ms`.
- **Docs:** `docs/formats.md` (the cue rules in the audio manifest),
  `docs/platform-backends.md` (the `audio` section: what a
  `SoundRequest::Sound` now carries, and that the cue rules are
  `App`'s), `docs/contract.md` (`run`'s `dropped`).
- **An ADR** (the `write-adr` skill, the next free number), extending
  ADR-0004 §6 and ADR-0024 §2. It decides every cue rule now, so 2378
  (part two) and the backends to come (3027, 3029, 3030) follow one
  text:
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
  - pitch waits for a backend that can play it; 2379 (parked) adds a
    pitch range to these rules once every desktop and web build plays
    through the in-house mixer.
- **For the game:** the API changes. `SoundCue` has a new field, which
  breaks nothing (it is `#[non_exhaustive]` and built with
  `SoundCue::new` since 2024). `SoundRequest::Sound` has new fields
  (match it with `..`), `RunOutput` has `dropped`, and the pack's
  version goes up, so a pack baked before this is refused and rebaked.
  A backend of the game's own, such as its Playdate front end
  (game#1170), plays `variant` at `cue_volume` instead of picking. The
  PR says so, and the game's PR that moves the pin follows it.
- **Either order:**
  - **From 3029 (done):** the shared player plays the given file and
    volume as above, on every backend, the Playdate's among them.
  - **From 2344 (done):** `SoundCue` has `looped` (`SoundCue::new`
    sets it to `false`, `with_looped` changes it), and `pack::VERSION`
    is 5. This ticket rebases onto that `SoundCue` and bumps the
    version again. `Cues` gives `SoundRequest::Start` (a
    `#[non_exhaustive]` variant, built with `SoundRequest::start`) a
    file and a cue volume as it does `Sound`, with no gap: the game
    stops a handled sound itself. The player's `Audio::start_sound`
    in `crates/app/src/audio.rs` is where it plays.
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
- **Part two** is 2378 (blocked by this one): file lengths, copy caps,
  priority, the voice limit and one copy stopped alone.

**Out (do not do):**
- file lengths, copy caps, priority, the voice limit and one copy
  stopped alone: 2378 (part two);
- pitch or rate variation: no backend can play it today. A pitch range
  in these rules is 2379's (parked), played through 3029's `set_rate`
  (3030's resampler, and the Playdate SDK);
- limits per owner (a cue's copies per thing in the world), culling by
  distance, and positional sound (2363, parked; ADR-0004 §6);
- sound groups, buses and limits per group (2342), and ducking (2343);
- looped sounds and handles a screen holds (2344);
- a voice count per target (3017, parked), and the in-house mixer's and
  the GBA's own voice maximum (3030, 3027);
- a "stop quietest" rule: write a ticket if a game shows it needs one.

## Implementation steps

1. Read 2378 (part two, blocked by this one): the ADR this ticket
   writes decides its copy cap and voice rule too, so the two must
   agree.
2. Read `crates/assets/src/audio.rs` (`from_source`, `Check::sound`,
   the tests' `FULL`), `crates/app/src/audio.rs` and its tests,
   `crates/app/src/backend.rs`, `crates/platform/src/app.rs` (`step`,
   `Lists`, `load`, `running`), `crates/platform/src/audio.rs`,
   `crates/platform/src/harness.rs`, and `crates/cli/src/contract/run.rs`,
   `session.rs` and `fake.rs`. Check whether 3029, 2344, 2342, 2363
   and 2218 are done (the either-order clauses).
3. The data: `CueRules`, `Pick`, the RON fields, `parse_ron_implicit_some`,
   the check, `rules` in `SoundCue::new` (still `const`) and the
   literals left in `crates/assets`, the pack version and its
   snapshot. `FULL` must load to the same values, with the default
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

- Unit: the RON fields, written bare, and the check; `SoundCue::new`
  gives the default rules and still builds in a `const` item; each
  `Pick` on a small cue; the gap, in one frame and across frames;
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
