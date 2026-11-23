---
id: "2345"
title: "Cue rules as data: variants without repeats, random volume, copy caps, minimum gaps and priority, decided once for every backend"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-23
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
  - the variant RNG: `VariantRng` moves here from the shared player
    (`crates/platform/src/audio/player.rs`, where 3029 had moved it from
    `shuyi-app`) with its tests. `Cues::new` seeds it with `splitmix64(seed ^ SALT)`
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
  - `proptest` already left `shuyi-app`'s dev-dependencies with 3029,
    and stays a dev-dependency of `shuyi-platform`;
  - `Player::load` (and the Playdate's `audio::player`) lose their
    `seed`: the player picks nothing.
- **Seen in tests and by agents:**
  - the Harness records `App::dropped_sounds` every frame and gives
    them out with `Harness::dropped_sounds(&self) -> Vec<(u32,
    &DroppedSound)>`, each with its frame; `clear_audio` clears them
    with the rest, and `sounds()` still lists the cues asked for;
  - **Moved to 3419** (the PR passed 1,500 counted lines): `run`'s answer (`RunOutput`) gets `dropped: Vec<Dropped>`, a new
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
  `SoundCue::new` since 2024). `SoundRequest::Sound` has new fields,
  which break nothing either: **from 2084 (done)** it is a
  `#[non_exhaustive]` variant built with `SoundRequest::sound`, and the
  new fields start at their defaults there. `RunOutput` has `dropped`, and the pack's
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
  - **From 2342 (done):** the shared player
    (`crates/platform/src/audio/player.rs`) plays a sound at its cue's
    volume times the request's volume times its gain
    (`factor(gains.sound(cue))`), and keeps a handled sound's gain
    (`Handled::gain`). This ticket puts `percent(cue_volume)` in place
    of the cue's own volume there. **From 2258 (done):** `SoundCue` has
    `group`, which `SoundCue::new` leaves `None` beside this ticket's
    fields, and `pack::VERSION` is 11: this ticket bumps it again.
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

- [x] Every manifest in the repository's tests loads with the default
      rules; a test loads `FULL` and finds `CueRules::default()` on
      each cue.
- [x] A cue with `pick: NoRepeat, volume_min: 45, min_gap_ms: 60`,
      written bare as above, loads to those values; `volume_min` over
      `volume` is an error naming the cue, at its line.
- [x] With the default rules, `Cues` passes every request in order,
      voice requests among them, each sound at its cue's volume, and
      drops nothing.
- [x] `NoRepeat` never plays the same file twice in a row, and
      `Shuffle` plays each of n files once in every n plays and never
      the same file twice in a row across a reshuffle (property tests,
      n from 2 to 8). `InOrder` plays the files in turn, and a cue of
      one file plays it each time (unit tests).
- [x] The volume picked is always within `volume_min..=volume`
      (property test).
- [x] Two plays of a cue that both sound are at least `min_gap_ms`
      apart (property test), and two plays of a cue with a gap in one
      frame play once (unit test).
- [x] A dropped play draws nothing: the same requests with the dropped
      ones taken out give the same files and volumes (property test).
- [x] The same seed and the same requests give the same plays and
      drops.
- [x] In the Harness, with the toy game's `click` given
      `min_gap_ms: 40` (`Toy::audio` is public), Confirm on the menu
      overlay (`crates/ui/src/toy.rs`) twice less than 40 ms apart
      lists the second click in `dropped_sounds()` as `Gap`, and `sounds()`
      lists both (`run` answering it under `dropped` moved to 3419).
- [x] The shared player (`shuyi-app`'s since 3029), against its fake backend, plays the file
      `variant` names at `cue_volume`; when a cue's second file fails
      to load, `variant: 2` still plays its third file and `variant: 1`
      plays nothing.
- [x] `pack::VERSION` is one higher, with the pack snapshot updated
      (`contract::VERSION`, the schemas and `versions.json` moved to 3419).
- [x] `shuyi-platform` still builds for `thumbv7em-none-eabihf`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the RON fields, written bare, and the check; `SoundCue::new`
  gives the default rules and still builds in a `const` item; each
  `Pick` on a small cue; the gap, in one frame and across frames;
  a cue the manifest lacks; the moved `VariantRng` tests;
  `SoundRequest::from_request` for the requests that skip the rules.
- Property: `NoRepeat` and `Shuffle`; the volume range; the gap; a
  dropped play draws nothing; the moved variant-picking test.
- Integration: the Harness test above (`run` with drops moved to 3419);
  the shared player against its
  fake backend (the file, the volume, a failed file's slot).
- Snapshot: the pack's pinned binary forms.

## Completion notes

Done:

- **The data** (`crates/assets/src/audio.rs`): `CueRules` (`pick`,
  `volume_min`, `min_gap_ms`) on `SoundCue::rules`, with `const
  CueRules::DEFAULT` and `const` `with_pick`, `with_volume_min`,
  `with_min_gap_ms` (ADR-0039 kind 2), `SoundCue::with_rules`, and
  `Pick` (`Random`, `NoRepeat`, `Shuffle`, `InOrder`). The RON is flat on
  the sound entry; `from_source` already read with
  `parse_ron_implicit_some`, so `volume_min: 45` is written bare. The
  check refuses `volume_min` over `volume` at the cue's line.
  `pack::VERSION` is 21.
- **`Cues`** (`crates/platform/src/audio/cues.rs`, `no_std`, integers):
  the gap, then the file, then the volume; a dropped play draws nothing;
  a started sound is picked, never dropped and starts no gap.
  `VariantRng` moved in with its tests, seeded `splitmix64(seed ^ SALT)`.
- **`App`** keeps a `Cues` from `ctx.music_seed` (`running`, and the
  load's end), runs each frame's requests through it, and gives the
  drops with `App::dropped_sounds`; the Harness records them with their
  frame (`Harness::dropped_sounds`), cleared by `clear_audio`.
- **`SoundRequest::Sound` and `Start`** carry `variant` and `cue_volume`
  (`SoundRequest::sound` and `start` give 0 and 100, `with_pick` sets
  them). The shared player keeps one slot per file and plays the slot
  named at `percent(cue_volume) × volume × gain`; it has no picker, and
  `Player::load` and the Playdate's `audio::player` lose their seed.
- **hello-grid** gives its cursor and move cues `min_gap_ms: 40`
  (`STEP_GAP_MS`); `AudioConfig::cursor_cue`'s doc says to.
- **ADR-0139** decides every cue rule, part two's copy cap and voice rule
  too; ADR-0004 §6, ADR-0024 §2 and ADR-0063 name it.
- Docs: `docs/formats.md`, `docs/platform-backends.md`, the book's
  chapter on saves and settings (`site/content/learn/book/saves-and-settings.md`).

Deviations:

- **The ticket was a month old** (2026-10-21) and named code 3029 had
  moved: `VariantRng` was in the shared player
  (`crates/platform/src/audio/player.rs`), not `shuyi-app`, and
  `proptest` had already left `shuyi-app`. Fixed in the ticket's Scope.
  The acceptance test of "`shuyi-app`'s player" is the shared player's,
  against its fake backend
  (`the_file_a_request_names_plays_at_its_volume_and_a_failed_file_plays_nothing`).
- **`run`'s `dropped` moved to 3419.** With it the PR was 1,661 counted
  lines; the contract's part (`Dropped`, `Session::dropped_sounds`,
  `RunOutput::dropped`, the contract's version and schemas,
  `docs/contract.md`) lands apart, so it is ticket 3419, and 2378's
  contract clause waits on it. `contract::VERSION` doesn't change here.
- **`Player::load` loses its `seed` parameter** (and
  `shuyi_playdate::audio::player` too): the ticket said the player's
  `rng` field goes, and a seed nothing reads would mislead.
- `SoundRequest::sound` and `start` give `cue_volume: 100` (full), so a
  request built by hand plays its first file at the file's own
  loudness; `App` always sends the picked values.
- A frame's gains come from `game.gains()` after the update, not from
  the frame's output, which borrows the game while the manifest is read
  (the same value, `Game::frame` builds it so).
- **Not checked by ear** on the desktop or the web: this session can't
  hear. `cargo shuyi run --script "Down Down Up Up Enter"` on hello-grid
  lists every `ui_move` and the `ui_select` under its sounds, and the
  two presses two frames (33 ms) apart dropped as `gap` (the engine's
  hold repeat is 55 ms, so a held key still ticks each step); the
  player's tests play the file and volume through the fake backend, and
  the Playdate's through its fake SDK.

Size: 1,542 counted lines, the rest can't land apart: the player plays the file and volume `Cues` picks, so without `Cues` and `App`'s use of it every sound would play its first file at full volume, and the data without `Cues` would read rules that do nothing; the contract's part, which can land apart, is 3419.

Follow-up: 3419 (`run`'s answer lists the drops). 2378 (part two) is
unblocked.

Breaking: shuyi-platform: `audio::player::Player::load`: it takes no `seed`; a backend drops the last argument.

```migration
since: 0.1.0
what: The shared audio player picks no file, so Player::load takes no seed.
old: "Player::load(backend, manifest, files, music_dir, voice_dir, seed)"
new: "Player::load(backend, manifest, files, music_dir, voice_dir)"
before: |
  let player = Player::load(backend, manifest, &files, music, voice, seed).await;
after: |
  let player = Player::load(backend, manifest, &files, music, voice).await;
find: ["voice, seed)"]
```

Breaking: shuyi-playdate: `audio::player`: it takes no `seed`; a front end drops the last argument.

```migration
since: 0.1.0
what: The Playdate's audio::player takes no seed.
old: "shuyi_playdate::audio::player(speaker, manifest, seed)"
new: "shuyi_playdate::audio::player(speaker, manifest)"
before: |
  let player = audio::player(speaker, manifest, seed);
after: |
  let player = audio::player(speaker, manifest);
find: ["manifest, seed)"]
```

Breaking: shuyi-assets: `pack::VERSION` and `formats.ron` "audio manifest: audio-manifest": a sound cue's binary form has its rules, and a baked pack is of version 21; a pack baked before is refused, so a game bakes again with `shuyi bake`.

```migration
since: 0.1.0
what: The pack format is version 21, its sound cues with their rules; a pack of version 20 is refused.
old: "pack::VERSION == 20"
new: "pack::VERSION == 21"
before: |
  cargo shuyi bake   # with the engine before 2345
after: |
  cargo shuyi bake   # again, with this engine
```

Breaking: shuyi-platform: a session record's frame hashes: each frame's hash covers the sounds sent with their file and volume, so a record kept before plays differently from its first sound; a game blesses its kept records again (hello-grid's `tests/sessions/stage.session.ron` is).

```migration
since: 0.1.0
what: A session record's frame hashes hold each sound's file and volume, so a kept record is blessed again.
old: "a frame hash over SoundRequest::Sound { cue, volume }"
new: "a frame hash over SoundRequest::Sound { cue, volume, variant, cue_volume }"
before: |
  assert_session(start(), kept("run.session.ron"));   # passes on the old engine
after: |
  SHUYI_BLESS=1 cargo test   # write the record again, then assert_session as before
```

Behaviour: shuyi-platform: which file of a cue plays is picked in `App` from `ctx.music_seed` (salted), no longer from the backend's clock, so the same seed and requests play the same files on every backend; a file that failed to load no longer shifts the others, it plays nothing when picked.

Behaviour: shuyi-platform: `SoundRequest::Sound` and `Start` carry `variant` and `cue_volume`, and a backend plays them as told; a request built with `SoundRequest::sound` or `start` alone plays the cue's first file at full volume, not at the cue's `volume`.

Behaviour: shuyi-assets: the audio manifest reads `pick`, `volume_min` and `min_gap_ms` on a sound, and refuses `volume_min` over the cue's `volume` at its line.

Behaviour: shuyi-platform: a play of a cue with `min_gap_ms` less than that after its last play that sounded is dropped: `App::dropped_sounds` and `Harness::dropped_sounds` list it, and the Harness's `sounds()` still does. No cue has a gap unless its manifest gives one.

Book: the chapter on saves and settings says how a cue picks its files and volume and spaces its plays (`site/content/learn/book/saves-and-settings.md`).

Game: nothing to adopt, the game builds its cues with `SoundCue::new` (the rules start at their defaults), implements no audio `Backend` and calls neither `Player::load` nor `shuyi_playdate::audio::player` (game#1170's front end will call the latter without a seed); the pin move blesses its kept Quick Battle session again, as each frame's hash now holds a sound's file and volume. `gh search code --repo Zafnok/visions-of-shuyi` for `SoundCue`, `SoundRequest`, `Player::load`, `audio::player`, `impl Backend for` and `assert_session` found its `SoundCue::new` calls, the pins log, and `crates/ui/tests/it/sessions.rs` with `quick_battle.session.ron`.

**Nick:** nothing changes in how the game sounds until its manifest names
rules. A game can now keep footsteps from repeating the same file, vary
a sound's loudness, and stop a cursor from ticking several times at once.
