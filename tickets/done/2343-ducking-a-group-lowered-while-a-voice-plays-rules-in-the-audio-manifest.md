---
id: "2343"
title: "Ducking: a sound group lowered while a voice plays, as rules in the audio manifest, stepped in whole milliseconds"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2342"]
nick_input: none
completed: 2026-11-09
---

# 2343 — Ducking the music under voices

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. When a character speaks over music, games lower the
music so the words are heard, and bring it back after:

- Unity: the Duck Volume effect on a mixer group, driven by a send from
  another group (<https://docs.unity3d.com/Manual/AudioMixer.html>);
- Godot: a Compressor effect on a bus, with another bus as its
  sidechain
  (<https://docs.godotengine.org/en/stable/classes/class_audioeffectcompressor.html>);
- Unreal: Audio Modulation and submix effects;
- Bevy has none in its own audio.

What Shuyi has (checked 2026-10-20):

- nothing lowers one sound because another plays: `duck` and
  `sidechain` find nothing about audio in `crates/` (only a toy input
  action named `Duck`);
- the device reports the music it plays (`Audio::music_playing(now)`,
  `crates/platform/src/audio.rs`), and `App` hands it to
  `Game::set_music_playing` before every frame
  (`crates/platform/src/app.rs`, line 600 today);
- nothing reports a voice, though `shuyi-app`'s voice player knows:
  `Voices::playing` in `crates/app/src/audio/voice.rs` holds the clip
  and when it ends. It is set when the clip starts, and cleared by a
  stop, by the next clip, or in `Voices::poll` once its end has passed.
  A clip is always an OGG file (`shuyi_assets::voice::clip_file`), so
  its end is known unless its length can't be read (`ogg_length_ms`);
- the headless `Speaker` (`crates/platform/src/headless.rs`) keeps the
  last frame's requests but no voice;
- **From 2342 (done):** every cue has a gain from its sound group,
  worked out in `shuyi-ui` every frame (`GroupGains`,
  `crates/ui/src/audio/groups.rs`, ADR-0091), which the shared player
  applies. The built-in groups' ids are in
  `crates/assets/src/audio/groups.rs` (`BUILT_IN`, with their parents);
  the manifest's own groups are `AudioManifest.groups` (2258, done).

Why it matters:

- **The game first.** Visions of Shuyi has voiced lines and music. Its
  ticket game#0238
  ([`0238-voice-clip-playback-plumbing.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0238-voice-clip-playback-plumbing.md))
  left ducking the music under a voice to "a tuning ticket if Nick
  asks". When he does, it is one line of the game's audio manifest.
- **Other 2D games:** any game with voiced dialogue, a narrator or a
  voiced tutorial over music.
- **Ports.** The rules are data and the step is a pure function of
  time, so a test hears the same ducking every run. Every backend
  (`shuyi-app`, 3029, and the parked 3027 and 3030) gets it through
  2342's gains, and needs only to say when a voice plays.

**From 2206 (open):** Parked: a sequence cue in `music` plays at the gains this ticket's rules multiply into, so a rule that lowers `music` lowers it too. 2206 does nothing for that when this lands first, and nothing here changes for 2206.

## Nick input

None. Whether Visions of Shuyi lowers its music under voices is the
game's choice (game#0238); a manifest with no rule changes nothing. If
Nick wants it before the MVP, this ticket moves to the MVP with 2342.

The game now asks for it: game#0865 (the music dips under a voiced
line, from a playtest of the Act 1 build, 2026-11-08) is blocked on this
ticket, and the game's Act 1 playtest runs through 0865. The retro of
2026-11-08 found nobody had told this side. Whether that moves 2342 and
2343 to the MVP is Nick's call (`ask-nick`); until he says, they stay
After MVP and the game ships without the dip.

## Scope

**In:**

- **Rules in the audio manifest** (`crates/assets/src/audio.rs`, with
  the checks in `crates/assets/src/audio/groups.rs`, beside the built-in
  ids):
  `AudioManifest.ducks: Vec<DuckRule>`, read from an optional `ducks`
  list on `RawFile` (`#[serde(default)]`), for example
  `(when: "voice", lower: "music", to: 80, attack_ms: 100, release_ms: 500)`.
  `DuckRule` has the public fields `when: String`, `lower: String`,
  `to: u8`, `attack_ms: u32` and `release_ms: u32`:
  - `when`: the group whose playing lowers another. Only `voice` for
    now, since the device reports only voices; another group is an
    error that says so;
  - `lower`: a known group, not `when` and not one of its parents
    (lowering `master` would lower the voice too). Known is one of
    `BUILT_IN` or a group the manifest declares (`AudioManifest.groups`,
    2258, done; checked before the cues, `Check::is_group` in
    `crates/assets/src/audio/groups.rs`), with a test of a declared one;
  - `to`: a position on 2342's loudness curve, 0–100; over 100 is an
    error. While ducked, the lowered group's gain is multiplied by
    `loudness(to)`: 80 lowers it by 12 dB (1029 of 4096), and 100
    changes nothing;
  - `attack_ms` and `release_ms`: how long the gain takes to go down
    and to come back, in whole milliseconds; 0 is at once;
  - a second rule lowering the same group is an error: with `voice` the
    only `when`, the two would always run together. The ticket that
    allows other `when` groups decides how two rules combine;
  - an error names the rule by its place in the list and its `lower`
    group (`ducks[1] (lower: "music")`), at the first line with that
    `lower: "<group>"` (`line_of`). A rule has no `id`, so `Check::err`,
    which looks for `id: "<id>"`, can't find its line;
  - the manifest's binary form changes again: make `pack::VERSION`
    (`crates/assets/src/pack.rs`) the next number, add one rule to the
    `MANIFEST` in `crates/assets/src/pack/tests.rs`, and accept the
    snapshot of
    `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`.
- **The device says when a voice plays:**
  `fn voice_playing(&self, now: Millis) -> bool` on
  `shuyi_platform::Audio`, beside `music_playing`, with a default body
  returning `false`. A backend that can't tell never ducks, and the
  trait change is additive:
  - `shuyi-app`: `Sound` (`crates/app/src/backend.rs`) asks its player,
    which asks `Voices`. A clip plays from its start until its end has
    passed at `now` (checked against `now`, because `poll` clears it
    only later in the frame), it is stopped or it is replaced. A clip
    whose length couldn't be read plays until it is stopped or
    replaced, as `Voices` keeps it today;
  - the headless `Speaker`: from the `now` of the `play` whose requests
    hold a `SoundRequest::Voice`, until a `StopVoice`, `stop_all`, or
    the length set by a new `Speaker::set_voice_length(Duration)` has
    passed (with no length set, until it is stopped).
    `Harness::voice_length(seconds: f32)` sets it, as
    `Harness::music_load_delay` sets the load delay;
  - `Silent`: the default.
- **`Game::set_voice_playing(bool)`**, which `App` calls before every
  frame with `platform.audio().voice_playing(elapsed.now)`, right after
  `set_music_playing` (`crates/platform/src/app.rs`, line 600 today).
- **`DuckState`** (`shuyi_ui::audio::duck`, a new
  `crates/ui/src/audio/duck.rs`; `no_std`, integers only):
  - `Game::with_stack` builds it from the manifest's rules, where it
    reads `music_fade_ms` today, and `Game` keeps it beside
    `MusicState`. So a test that adds a rule to `toy::ctx()`'s
    `game.audio` before making the `Game` sees it;
  - per rule: the multiplier now (a `Gain`), where it is going, and the
    whole milliseconds since it set off from where it was;
  - `update(dt_ms, sounding)`, called in `Game::collect_audio` beside
    `MusicState::update`: the multiplier moves in a straight line to
    `loudness(to)` while `sounding`, and back to 4096 when not. Its
    speed is the full distance (4096 minus `loudness(to)`) over
    `attack_ms` going down and over `release_ms` coming back. A change
    of direction keeps that speed from where it is, so from halfway
    down it is back in half of `release_ms`. Its value is a pure
    function of where it set off, where it goes, the speed and the
    time, so the frame sizes add no rounding. A distance of 0
    (`to: 100`) moves nothing and divides by nothing;
  - `dt_ms` is the frame's `dt` (already through `frame_seconds` in
    `Game::frame`) in whole milliseconds, by
    `shuyi_ui::cinematic::millis`;
  - `sounding` is the device's answer and the `voice` group's gain above
    0: a voice the player turned down to 0 lowers nothing. No rule can
    lower `voice` or its parents, so that gain never depends on the
    duck.
- **Into the gains:** 2342's `GroupGains` multiplies each lowered
  group's own gain by its multiplier (in `u32`, rounded, as 2342
  multiplies), before the group's children multiply by it, so they go
  down with it. The frame's `Gains` carry it to the device with no new
  field, and a track already playing follows it, as 2342 makes it
  follow the player's volume.
- **An ADR** (the `write-adr` skill, the next free number): "Ducking:
  rules in the audio manifest, a voice the device reports, and a
  straight line in whole milliseconds in shuyi-ui". It extends 2342's
  ADR (ducking multiplies into its gains) and ADR-0024 §2
  (`Audio::voice_playing`, a provided method that answers `false`). It
  gets a row in `docs/adr/README.md`, and the rows of ADR-0024 and of
  2342's ADR say "extended by ADR-NNNN".
- **Docs:** `docs/platform-backends.md`: `voice_playing(now)` in the
  `audio` section and in step 6 of "What one frame does". The docs of
  `AudioManifest` and `DuckRule` describe the rules.

**Either order:**

- **From 3029 (done):** `voice_playing` reads the shared voice player
  (`shuyi_platform::audio::player::voice`, ADR-0063).
- **2218** (writers of the engine's text formats): if it is done, its
  `AudioManifestRon` writer writes `ducks` and its round-trip test
  covers a rule; if not, nothing here.
- **From 2258 (done):** a game's own sound groups; see `lower` above.
- **2725** (parked: a sound group per voiced character, under
  `voice`): if it is done, `sounding` also needs the gain of the group
  the playing clip was asked in above 0, not only `voice`'s, so a
  character turned down to 0 lowers nothing, with a test; if not,
  nothing here (2725 does it).

**For the game:** additive. A manifest without `ducks` sounds as
before. A backend of the game's own that implements `Audio` keeps
compiling and never ducks until it says when a voice plays.
`AudioManifest` gains a public field, so a literal of it without
`..AudioManifest::default()` adds `ducks: Vec::new()`. The PR says so,
and the game's PR that moves its pin follows this one.

**Out (do not do):**

- `when` for groups other than `voice`: the device would have to say
  when each sound ends, which no backend does today; a ticket when a
  game asks;
- a sidechain compressor that follows the voice's loudness: it needs a
  DSP follow-up over the integer mixer selected by 2902 (3030,
  parked), not the initial mixer scope;
- curves other than a straight line for attack and release (2109's ease
  curves, if a game asks);
- snapshots: several groups' volumes changed together by name;
- a player setting that turns ducking off;
- a rule per voice clip or per speaker: a ticket when a game asks.
  2725 (parked) gives each voiced character a group under `voice`, and
  a rule with `when: "voice"` ducks for every one of them;
- the groups themselves, the curve and the option rows (2342).

## Implementation steps

1. Read 2342's ADR and ADR-0024 §2; `shuyi_ui::audio::groups`,
   `loudness`, `GroupGains`, `MusicState` and `Game::collect_audio`
   (`crates/ui/src/audio/`, `crates/ui/src/game.rs`);
   `crates/assets/src/audio.rs` and `audio/groups.rs`;
   `crates/app/src/audio/voice.rs`; and
   `crates/platform/src/{audio,app,headless,harness}.rs`.
2. Write the ADR.
3. `DuckRule`, the manifest's `ducks`, their checks, and the pack
   version and snapshot, with tests.
4. `DuckState`, with tests.
5. `Audio::voice_playing`: in `Voices`, `shuyi-app`'s player and
   `Sound`; in the headless `Speaker`, with `Harness::voice_length`.
   Then `Game::set_voice_playing` and the call in `App`.
6. Build `DuckState` in `Game::with_stack`, step it in
   `collect_audio`, and fold its multipliers into `GroupGains`.
7. The Harness test, the docs, and the gates.

## Acceptance criteria

- [x] A manifest with no `ducks` loads as before, and with no rule the
      gains are 2342's, frame for frame.
- [x] Each rule error is reported at its line, a test each: `when` not
      `voice`, an unknown `lower`, a `lower` that is `when` or one of its
      parents, a `to` over 100, and a second rule lowering the same
      group.
- [x] `DuckState` with `to: 80`, `attack_ms: 100` and a voice playing
      is halfway down (to within one unit) at 50 ms, reaches exactly
      `loudness(80)` (1029) once 100 ms have passed in frames of 16 and
      17 ms, and never goes below it. With `attack_ms: 0` it is there
      on the first frame; with `to: 100` it stays at 4096.
- [x] A voice that stops halfway down is back at 4096 after half of
      `release_ms`.
- [x] `shuyi-app`'s voice player says a clip is playing from its start
      until its length has passed, and not after a stop (a test with
      its fake backend).
- [x] The headless `Speaker` says a voice plays after a `Voice` request,
      and not after a `StopVoice`, `stop_all` or the length it was set.
- [x] A Harness test with the toy game and a rule lowering `music`: once
      a voice has played for `attack_ms`, `Harness::gain("music")` is
      the music gain before the voice times `loudness(to)`, rounded; the
      voice ends (`Harness::voice_length`) and the gain is back after
      `release_ms`; the `sfx` gain never changes; with the `voice`
      volume at 0 the music gain doesn't move.
- [x] `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `DuckRule`s and their errors; `DuckState` (down, up, a change
  of direction, zero lengths, `to: 100`); the gains with a multiplier
  (a child of the lowered group goes down too); `Voices` with its fake
  backend; the headless speaker's `voice_playing`.
- Integration: the Harness test above.
- Snapshot: the pack's binary form test
  (`the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`).

## Completion notes

- **Manifest** (`crates/assets/src/audio.rs`, `audio/groups.rs`): an
  optional `ducks` list, kept in `AudioManifest.ducks` as `DuckRule`s
  (`when`, `lower`, `to`, `attack_ms`, `release_ms`; `#[non_exhaustive]`
  with `Default`, ADR-0039). The checks run after the groups: `when` not
  `voice`, an unknown `lower`, a `lower` that is `when` or one of its
  parents, a `to` over 100, and a second rule lowering the same group,
  each named `ducks[N] (lower: "<group>")` at the first line with that
  `lower`. A test covers a declared group lowered and one under `voice`.
  `pack::VERSION` is 14 (3097 took 13 first); the format tests' manifest has one rule, and
  the pack snapshot and the audio manifest's golden value were accepted.
- **`DuckState`** (`crates/ui/src/audio/duck.rs`, `no_std`, integers):
  per rule the multiplier now, where it set off from, where it goes and
  the whole milliseconds since; its value a pure function of those, so
  frames of 33 ms and of 1 ms land on the same value (a test).
  `update(dt_ms: u64, sounding)` takes `cinematic::millis`' `u64`.
- **Into the gains:** `GroupGains::update_ducked(groups, volumes,
  ducks)` multiplies a lowered group's own gain before its children's;
  `update` is it with nothing lowered, so a manifest without rules gives
  2342's gains. `Game` builds the state in `with_stack`, and in
  `collect_audio` works the gains out, reads `voice`'s, and steps the
  state with `sounding` = `Ctx::voice_playing` and `voice` above 0; the
  frame's gains are worked out again with the stepped state.
- **Deviation: the device side was already there.** Ticket 2255 (done)
  added `Audio::voice_playing(now) -> Option<&str>` (provided, `None` by
  default; `PlayerDevice`, the headless `Speaker` and `Harness::
  voice_length` answer it) and `Game::set_voice_playing(Option<&LineId>)`,
  called by `App` after `set_music_playing`. So this ticket adds no
  trait method and no `bool` setter: `DuckState` reads
  `Ctx::voice_playing`. The acceptance criteria on `shuyi-app`'s voice
  player and the headless `Speaker` are met by 2255's tests
  (`crates/platform/src/audio/player/voice.rs`,
  `crates/platform/src/headless/tests.rs`, `audio::a_voice_sounds_from_
  the_next_frame_until_its_length_has_passed`). ADR-0099 extends
  ADR-0091 only; ADR-0024 is unchanged, and its row too.
- **Tests:** the rules and each error; `DuckState` down, up, a change of
  direction both ways, zero lengths, `to: 100` and `to: 0`, frame sizes;
  a lowered group's child goes down and no other group; the Harness tests
  `a_voice_lowers_the_music_as_the_manifests_rule_says` and
  `a_voice_turned_down_to_0_lowers_nothing` (`crates/ui/tests/it/audio.rs`).
- **Docs:** ADR-0099; the module, `AudioManifest` and `DuckRule` docs;
  `docs/platform-backends.md` (step 7 and `voice_playing`); the book's
  *Saves and settings* chapter shows a `ducks` entry.
- **Settled:** 2206, 2218 and 2725 now say what holds (2218 writes
  `ducks`; 2725 adds the clip's own group to `sounding`).
- **The game:** game#0865 (the music dips under a voiced line) moves the
  pin to this change and adds the rule to its manifest. Whether 2342 and
  2343 count for the MVP stays Nick's call; nothing here needs it.

Breaking: shuyi-assets: pack::VERSION and the audio manifest's golden value: a pack is of version 14 (the manifest's binary form has `ducks`); a pack baked before is refused, bake it again with `shuyi bake`.

```migration
since: 0.1.0
what: The pack format is version 14; packs baked before are refused.
old: "pack::VERSION == 13"
new: "pack::VERSION == 14"
before: |
  shuyi bake
after: |
  shuyi bake
```
