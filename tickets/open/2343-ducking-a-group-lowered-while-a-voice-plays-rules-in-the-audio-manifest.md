---
id: "2343"
title: "Ducking: a sound group lowered while a voice plays, as rules in the audio manifest, stepped in whole milliseconds"
type: feature
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: ["2342"]
nick_input: none
completed:
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
- 2342 gives every cue a gain from its sound group, worked out in
  `shuyi-ui` every frame (`GroupGains`), which every backend applies.

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

## Scope

**In:**

- **Rules in the audio manifest** (`crates/assets/src/audio.rs`, with
  the checks beside 2342's in `crates/assets/src/audio/groups.rs`):
  `AudioManifest.ducks: Vec<DuckRule>`, read from an optional `ducks`
  list on `RawFile` (`#[serde(default)]`), for example
  `(when: "voice", lower: "music", to: 80, attack_ms: 100, release_ms: 500)`.
  `DuckRule` has the public fields `when: String`, `lower: String`,
  `to: u8`, `attack_ms: u32` and `release_ms: u32`:
  - `when`: the group whose playing lowers another. Only `voice` for
    now, since the device reports only voices; another group is an
    error that says so;
  - `lower`: a known group (built in or declared), not `when` and not
    one of its parents (lowering `master` would lower the voice too);
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
  mixer of our own (3030, parked) and the sound research (2902,
  parked);
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

- [ ] A manifest with no `ducks` loads as before, and with no rule the
      gains are 2342's, frame for frame.
- [ ] Each rule error is reported at its line, a test each: `when` not
      `voice`, an unknown `lower`, a `lower` that is `when` or one of its
      parents, a `to` over 100, and a second rule lowering the same
      group.
- [ ] `DuckState` with `to: 80`, `attack_ms: 100` and a voice playing
      is halfway down (to within one unit) at 50 ms, reaches exactly
      `loudness(80)` (1029) once 100 ms have passed in frames of 16 and
      17 ms, and never goes below it. With `attack_ms: 0` it is there
      on the first frame; with `to: 100` it stays at 4096.
- [ ] A voice that stops halfway down is back at 4096 after half of
      `release_ms`.
- [ ] `shuyi-app`'s voice player says a clip is playing from its start
      until its length has passed, and not after a stop (a test with
      its fake backend).
- [ ] The headless `Speaker` says a voice plays after a `Voice` request,
      and not after a `StopVoice`, `stop_all` or the length it was set.
- [ ] A Harness test with the toy game and a rule lowering `music`: once
      a voice has played for `attack_ms`, `Harness::gain("music")` is
      the music gain before the voice times `loudness(to)`, rounded; the
      voice ends (`Harness::voice_length`) and the gain is back after
      `release_ms`; the `sfx` gain never changes; with the `voice`
      volume at 0 the music gain doesn't move.
- [ ] `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `DuckRule`s and their errors; `DuckState` (down, up, a change
  of direction, zero lengths, `to: 100`); the gains with a multiplier
  (a child of the lowered group goes down too); `Voices` with its fake
  backend; the headless speaker's `voice_playing`.
- Integration: the Harness test above.
- Snapshot: the pack's binary form test
  (`the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`).

## Completion notes

*(Filled in by the session that completes the ticket.)*
