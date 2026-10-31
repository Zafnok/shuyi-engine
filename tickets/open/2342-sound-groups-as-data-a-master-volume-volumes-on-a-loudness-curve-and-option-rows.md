---
id: "2342"
title: "Sound groups as data: a master volume, a volume per group on a loudness curve, and option rows"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: todo
blocked_by: ["2511"]
nick_input: none
completed:
---

# 2342 — Sound groups and a master volume

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and GameMaker
([`docs/decisions.md`](../../docs/decisions.md), 2026-10-20, *a second
comparison*: features most 2D games expect come after the MVP). Each of
them lets a game put its sounds in groups, with a volume per group and
one over all of them:

- Godot: audio buses, every one feeding the Master bus
  (<https://docs.godotengine.org/en/stable/tutorials/audio/audio_buses.html>);
- Unity: Audio Mixer groups under a master group
  (<https://docs.unity3d.com/Manual/AudioMixer.html>);
- Unreal: Sound Classes in a tree, and submixes;
- GameMaker: audio groups and buses;
- Bevy: one `GlobalVolume` over every sound
  (<https://docs.rs/bevy/latest/bevy/audio/struct.GlobalVolume.html>).

What Shuyi has (checked 2026-10-21):

- `crates/ui/src/settings.rs`: `Settings` has `music_volume` and
  `sound_volume` (`u8`, 0–100, default `DEFAULT_VOLUME`, 80), made into
  gains by `volume_factor`, which is linear (`v / 100`). There is no
  master volume.
- `crates/ui/src/ctx.rs`: the voice volume is `Ctx::voice_volume`
  (0–10, `MAX_VOICE_VOLUME`, `DEFAULT_VOICE_VOLUME` 8), on the context
  and not in the settings, so the engine doesn't save it.
  `Ctx::voice_gain` makes it 0–1.
- `crates/ui/src/game.rs`: `FrameOutput` carries three `f32`s,
  `music_volume`, `sound_volume` and `voice_volume`.
  `crates/platform/src/audio.rs`: `AudioFrame` carries the same three.
- `crates/assets/src/audio.rs`: `SoundCue` and `MusicCue` have no
  group.
- `crates/ui/src/screens/options.rs`: `OptionRow::music_volume` and
  `OptionRow::sound_volume` are written by hand, one each, as
  `RowValue::Volume` rows over the two fields.
- `crates/app/src/audio.rs`: the player multiplies a sound by
  `sound_volume` and a track by `music_volume` (`set_volumes`), and
  `crates/app/src/audio/voice.rs` a clip by the voice volume.
- [ADR-0004](../../docs/adr/0004-assets-standard-files-in-the-engines-own-data-out-and-a-deterministic-movement-kit.md)
  §6, "Audio stays as it is", foresees only positional sound.

So a game can't give ambience or menu sounds a volume of their own, or
offer a master volume, without changing `shuyi-ui` and
`shuyi-platform`. The three volumes use two scales in two places. And
a linear slider puts what a player hears in its top steps: from 50 to
100 the sound changes by only 6 dB, and everything quieter is crowded
into the bottom half.

Why it matters:

- **The game first.** Visions of Shuyi has music, sound and voice
  volumes, and its open ticket game#0826
  ([`0826-options-voices.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0826-options-voices.md))
  moves the voice volume into its settings. Here the voice volume
  becomes a group like the others, and the game can show a master
  volume when it wants one. A player who never opens the options hears
  what they hear today, and old saves keep their loudness (below).
- **Other 2D games** expect a master volume and a volume per kind of
  sound in their options.
- **Ports.** Every backend gets one gain per cue, worked out before it:
  `shuyi-app` today, 3029's Playdate player, 3027's GBA mixer and
  3030's mixer of our own (both parked). None of them knows the groups.
  That suits the end goal of 2026-10-19 (many ports), and the in-house
  sound that will replace quad-snd.
- **Agents.** A group is one line of the audio manifest, and its row
  four lines in the game's rows function.

Ducking the music under a voice builds on this: 2343.

## Nick input

None. Whether Visions of Shuyi shows a master volume is the game's
choice, made in the game; this ticket makes it possible and changes
nothing a player hears. If Nick wants it before the MVP, this ticket
moves to the MVP.

## Scope

**In:**

- **Groups in the audio manifest** (`crates/assets/src/audio.rs`, and
  a new `crates/assets/src/audio/groups.rs` beside `formats.rs`):
  - five built-in groups, with their ids as `pub const`s: `master`
    (no parent); `music`, `sfx` and `voice` under `master`; `ui` under
    `sfx`. `ui` is under `sfx` so the sound volume keeps lowering menu
    sounds, as it does today;
  - an optional `groups` list in the manifest (`#[serde(default)]` on
    `RawFile`, which denies unknown fields):
    `(id: "ambience", parent: "sfx", volume: 80)`. `parent` is
    optional and means `master`; `volume` is the position a new player
    starts at (0–100), optional;
  - an entry whose id is built in may set only `volume` (the game's own
    default for that group); a `parent` on it is an error;
  - an optional `group` on a sound cue and on a music cue
    (`Option<String>`, `#[serde(default)]` on `RawSound` and
    `RawMusic`);
  - errors at the line that names them (`Check::err`): a duplicate
    group id, an unknown parent, a loop of parents, a volume over 100
    (`Check::volume`), a cue naming an unknown group;
  - `AudioManifest.groups` holds what the file declares and not the
    built-ins, so the asset stays what its file says;
  - the module docs of `crates/assets/src/audio.rs` name the groups;
  - the pack: the manifest's binary form changes, so make
    `pack::VERSION` (`crates/assets/src/pack.rs`, 2 today) the next
    number and accept the new snapshot of
    `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
    (`crates/assets/src/pack/tests.rs`), as that test's comment says.
- **The loudness curve** (`shuyi_ui::audio::loudness`, a new
  `crates/ui/src/audio/loudness.rs`, `no_std`, integers only):
  - `Gain(u16)` in units of 1/4096 (4096 is full), the unit of 3030's
    mixer;
  - `loudness(volume: u8) -> Gain` from a table of 101 entries: 0 is
    silent; from 1 to 100 each step is 0.6 dB, from −59.4 dB up to
    0 dB, so entry `v` is `round(4096 × 10^((v − 100) × 0.03))`. Above
    100 counts as 100. The 0.6 dB is *tunable*;
  - `position_of_linear(percent: u8) -> u8`: the position whose gain is
    nearest `percent / 100`, for old saves (80 gives 97, 35 gives 85);
  - it stays out of `shuyi-core`: a volume isn't simulation.
- **`SoundGroups`** (`shuyi_ui::audio::groups`, a new
  `crates/ui/src/audio/groups.rs`): the tree, from the manifest and
  `AudioConfig`:
  - the built-ins with their defaults (`master` and `ui` 100; `music`,
    `sfx` and `voice` 97, the position nearest today's 0.8, so a new
    player hears what they hear today), then the manifest's groups and
    defaults over them;
  - the group a sound cue plays in: its manifest `group`, else `ui` for
    the cues `AudioConfig` names (the menu sounds and the cursor's
    tick), else `sfx`. A music cue: its `group`, else `music`. Every
    voice clip: `voice`;
  - `Ctx::new` builds it, and `Game::with_stack` builds it again where
    it reads `music_fade_ms` today, so a test that changes the manifest
    before making the `Game` sees the change. `Ctx::sound_groups()`
    reads it.
- **The player's volumes in the settings** (`crates/ui/src/settings.rs`,
  `crates/ui/src/ctx.rs`):
  - `Settings.volumes: Volumes` replaces `music_volume` and
    `sound_volume`: the positions (0–100) the player set, by group id,
    `#[serde(transparent)]` over a `BTreeMap<String, u8>`. A group not
    in it is at its default; a saved group the game no longer has is
    kept and ignored;
  - `Volumes::set_from_linear(group, percent)`: an old linear volume
    put at the position that sounds the same (`position_of_linear`),
    for a game's saved form that reads its old fields;
  - `Settings::restored()` empties it, so the defaults come back.
    `music_factor`, `sound_factor`, `volume_factor` and
    `DEFAULT_VOLUME` go;
  - `Ctx::voice_volume`, `MAX_VOICE_VOLUME`, `DEFAULT_VOICE_VOLUME` and
    `Ctx::voice_gain` go: the voice volume is the `voice` group's, on
    the same 0–100 scale, in the settings, so it is saved.
    `Ctx::voices_on` stays;
  - `Ctx::volume(group) -> u8`, the player's position or the default,
    and `Ctx::preview_volume(group, volume) -> bool`, through
    `preview_settings` so the options screen keeps or throws away the
    change like any other. Setting a group back to its default removes
    it from `Volumes`, so moving a slider away and back is no change.
    In a debug build both panic on a group `SoundGroups` lacks, as a
    cue the manifest lacks does (`Game::collect_audio`);
  - `Ctx::load_settings` drops each loaded volume that equals its
    group's default, so an old save at today's 80 reads as the defaults
    and moving its slider away and back is no change either.
- **A frame's gains:** `GroupGains`, each group's `loudness` times its
  parent's gain, down from `master` (multiplied in `u32`, rounded, back
  to a `Gain`), worked out by `Game` every frame. `FrameOutput` and
  `AudioFrame` replace `music_volume`, `sound_volume` and
  `voice_volume` with `gains: Gains<'a>`, a `Copy` view over
  `SoundGroups` and `GroupGains`. It answers `sound(cue)`,
  `music(cue)`, `voice()` and `group(id)`, each a `Gain`; `each()`,
  every group's id and gain, for a device that records them; and
  `muted()`, the same view with every gain 0, for 3042's mute in the
  background. `crates/platform/src/app.rs` (lines 605–616 today)
  passes it on.
- **The backends:**
  - `shuyi-app` (`crates/app/src/audio.rs`): `Audio::set_gains`
    replaces `set_volumes` and `set_voice_volume`. A sound plays at its
    cue's volume times the request's volume times `gains.sound(cue)`; a
    track at its cue's volume times its fade gain times
    `gains.music(cue)`, set again on a playing track when that gain
    changes; the voice at `gains.voice()`. A `Gain` becomes an `f32`
    here, at the edge, and nowhere before;
  - `shuyi_platform::headless::Speaker`: `volumes()` becomes `gains()`,
    the last frame's gain per group (copied with `Gains::each`).
    `Harness::volumes()` becomes `Harness::gain(group) -> Gain`;
  - `Silent` changes only with the frame's fields;
  - `crates/app/src/backend.rs` (lines 43–44) calls `set_gains`.
- **Option rows** (`crates/ui/src/screens/options.rs`). Each row stays a
  `RowValue::Volume`, whose `get` and `set` are closures over a group
  id constant, `|ctx| ctx.volume(MUSIC)` and
  `|ctx, v| ctx.preview_volume(MUSIC, v)`. They capture nothing, so
  they are `fn` pointers as today; `RowValue`, `ValueView` and the
  painters don't change:
  - `OptionRow::music_volume()` and `OptionRow::sound_volume()` keep
    their names and text keys and read and set `music` and `sfx`;
  - new: `OptionRow::master_volume()`, `voice_volume()` and
    `ui_volume()`, with the text keys `options.row.master_volume`,
    `options.row.voice_volume` and `options.row.ui_volume`;
  - `OptionRow::volumes()`: master, music, sound and voice, in that
    order;
  - the docs of `OptionRow::music_volume` show a game's own group's
    row, written the same way with its id.
- **Saved forms.** `crates/ui/src/toy.rs`,
  `templates/grid/crates/game/src/settings.rs`, the engine's
  `impl GameSettings for ()` (`crates/ui/src/settings.rs`, 2511), which
  the blank template uses, and `examples/hello-grid/src/settings.rs`
  save `volumes`, and still read `music_volume` and `sound_volume` from
  an old save (`Option<u8>` fields, never written) through
  `Volumes::set_from_linear`. Each gets a test that reads a save from
  before this change.
- **An ADR** (`write-adr` skill, the next free number): "Sound groups:
  the player's volumes as a tree of groups on a loudness curve, worked
  out before any backend". It extends ADR-0004 §6 and says: groups are
  data in the audio manifest; the built-ins; the curve; gains in 1/4096,
  with `f32` only at a backend's edge; a backend gets one gain per cue
  and never the tree; ducking (2343) multiplies into the same gains;
  effects on groups wait for a mixer of our own (3030). It gets a row
  in `docs/adr/README.md`, and ADR-0004's row adds "extended by
  ADR-NNNN" after "extended by ADR-0007".
- **Docs:** `docs/platform-backends.md`, the `audio` section: `play`
  gets the frame's `gains` instead of the three volumes.

**Either order:**

- **From 3029 (done):** the player's rules are shared by every backend
  (`shuyi_platform::audio::player`, ADR-0063), so apply the gains there.
- **3027** (GBA audio, parked): if it is done and `shuyi-gba`'s `Audio`
  still reads the frame's `sound_volume` and `music_volume` itself (it
  isn't under the shared player), move it onto `gains` here,
  as `shuyi-app`'s player; if not, nothing to do (3027 reads the gains).
- **3061** (DS music, parked): if it is done and `shuyi-nds`'s `Audio`
  still reads the frame's `music_volume` itself for its songs (they
  aren't under the shared player), move it onto `gains` here,
  with its sound effects if they read the volumes too, as `shuyi-app`'s
  player; if not, nothing to do (3061 reads the gains).
- **3042** (focus and the game's background policy): if it is done, its
  mute while unfocused moves onto `Gains` here, with every group's gain
  at 0 in the frames it mutes (`Gains::muted`); if not, 3042 mutes on
  `Gains`.
- **2110** (save migrations): if it is done, the forms it versions (the
  engine's `()` settings and the grid template's) each get their next
  `VERSION`, a `Migrate::upgrade` step that reads `music_volume` and
  `sound_volume` into `volumes` with `Volumes::set_from_linear`, and
  fixtures of the new version, as its guard asks; the toy's and
  hello-grid's read the `Option` fields. If not, every form reads the
  `Option` fields, and 2110 puts this form in its version 1.
- **2351** (built-in English for the engine's text keys): if it is done,
  the three new row keys get their English there; if not, in
  `crates/ui/src/toy/en.ron` and `examples/hello-grid/files/en.ron`.
- **2408** (parked, rumble): if it is done, `Settings::vibration`
  stays a field of its own and never a group in `Volumes`, its row
  stays a `RowValue::Volume` over it, `FrameOutput::vibration` stays
  beside `gains`, and each saved form changed here keeps saving
  `vibration`. If not, nothing to do (2408 follows this ticket's
  shape, its clause says so).
- **2218** (writers of the engine's text formats): if it is done, its
  `AudioManifestRon` writer writes `groups` and the cues' `group`, and
  its round-trip test covers a manifest with them; if not, 2218 writes
  them with the rest of the kind.
- **From 2344 (done):** a sound started with a handle is one playback
  the player keeps by its `SoundId` (`Handled` in
  `crates/app/src/audio.rs`), at its cue volume times its requests'
  volume times the player's sound volume, set again on the playback
  (`Backend::set_playing_volume`) when the sound volume changes. This
  ticket plays it at `gains.sound(cue)` in place of the player's sound
  volume, and sets it again when that gain changes, as a track's is.
- **2345** (cue rules): if it is done, `SoundCue::new` leaves `group`
  at `None`, and the player plays a sound at `percent(cue_volume)`
  (the volume 2345's `Cues` picked, carried by `SoundRequest::Sound`)
  times the request's volume times `gains.sound(cue)`, in place of the
  cue's volume. If not, nothing to do (2345 does it, its clause says
  so). The second to land rebases and bumps `pack::VERSION` again.
- **From 2501 (done):** the grid stage (`stage-top-down-grid` in
  `templates/shuyi-default`) saves the engine's `()` settings and has
  no saved form of its own, so there are three saved forms here, and
  2110's clause above versions only the `()` form. The change to
  `templates/grid/crates/game/src/settings.rs` above has no file left
  to change.

**For the game:** the API breaks. The game's PR that moves its pin
follows this one. It:

- replaces `Settings.music_volume` and `sound_volume` with
  `Settings.volumes`, and its saved form reads its old fields with
  `Volumes::set_from_linear`, tested on its old-save fixtures;
- moves `Ctx::voice_volume` to the `voice` group (game#0826 then saves
  it with the others);
- reads `FrameOutput::gains` where its Playdate front end reads
  `music_volume` and `sound_volume` today;
- keeps its `options.row.music_volume` and `options.row.sound_volume`
  text, whose keys don't change;
- shows 97 where an unchanged volume showed 80: the same loudness on
  the new curve.

The PR description lists these.

**Size.** If the PR passes about 600 changed lines (fixtures and
snapshots not counted), the manifest's own `groups` list, its checks
and the cue's `group` field go to a follow-up ticket (`write-ticket`),
After MVP, and this PR keeps only the five built-in groups.

**Out (do not do):**

- ducking (2343);
- effects on a group (filters, reverb), snapshots, sends and a sidechain
  compressor: they need a mixer of our own (3030, parked) and the sound
  research (2902, parked); a ticket when a game asks;
- rows made at run time from the manifest's groups: `OptionRow`'s key
  is a `&'static str` (`crates/ui/src/screens/options.rs`, line 81)
  and `RowValue::Volume` holds `fn` pointers, and changing either is a
  bigger change. A game's own group gets its row in its rows function,
  as the built-in rows are written;
- a group per voice clip (every clip plays in `voice`; 2725's volume
  per character will need one);
- positional sound (2363, parked);
- integers for the request's volume, the cue's volume and the fade gain
  (they stay `f32` and `u8`, as today);
- the Playdate and GBA backends themselves (3029; 3027, parked): one
  written after this applies the gains, and one written before is
  covered under *Either order* (3029's shared player, 3027).

## Implementation steps

1. Read ADR-0004 §6, `crates/assets/src/audio.rs`,
   `crates/ui/src/{audio,settings,ctx,game}.rs`,
   `crates/ui/src/screens/options.rs`,
   `crates/platform/src/{audio,app,headless,harness}.rs` and
   `crates/app/src/{audio,backend}.rs`. Check which of 3029, 3042,
   2110, 2351, 2218, 2344, 2345, 2408 and 2501 are done. List every
   use of the old names outside `tickets/`:
   `grep -rn "music_volume\|sound_volume\|voice_volume\|voice_gain\|music_factor\|sound_factor\|DEFAULT_VOLUME\|MAX_VOICE_VOLUME" crates examples templates docs`.
2. Write the ADR.
3. Groups in the manifest, their checks, and the pack version, with
   tests.
4. `loudness`, `SoundGroups`, `GroupGains` and `Gains`, with tests.
5. `Settings.volumes`, the `Ctx` changes and `FrameOutput`.
6. `AudioFrame`, `App`, the headless speaker, the Harness and
   `shuyi-app`'s player.
7. The option rows, and the existing options tests that expect 80.
8. The saved forms and their old-save tests. The templates are
   outside the workspace's `cargo fmt --all`: format and test them as
   CI's template job does.
9. The docs; run the gates.

## Acceptance criteria

- [ ] A manifest with no `groups` and no cue `group` loads as before,
      and the existing tests of `crates/assets/src/audio.rs` pass
      unchanged.
- [ ] Each manifest error above is reported at the line that names it
      (a test each).
- [ ] `loudness(0)` is 0 and `loudness(100)` is 4096, the table never
      goes down, and a test works out every entry again from the
      formula (with `f64`, in the test only).
- [ ] `position_of_linear(80)` is 97, whose gain (3329) is the nearest
      to today's 0.8 (3277); `position_of_linear(35)` is 85.
- [ ] A group's gain is its `loudness` times its parent's gain, rounded,
      down from `master` (a unit test over three levels with known
      numbers); with `master` at 0 every gain is 0; `Gains::muted`
      answers 0 for every group and cue.
- [ ] The menu sounds and the cursor's tick play in `ui` unless their
      cue names a group; another sound in `sfx`; music in `music`;
      voices in `voice` (a unit test of `SoundGroups`).
- [ ] An old save of each saved form (`music_volume: 35,
      sound_volume: 80`) reads as `music` at 85 and `sfx` at 97, and a
      new save writes `volumes` and no old field. Loaded through `Ctx`,
      that save has no `sfx` entry, since 97 is the default.
- [ ] A change of the music volume reaches the track already playing in
      `shuyi-app`'s player (a test with its fake backend).
- [ ] A Harness test: setting `master` to 0 makes every gain 0 on the
      next frame, and Restore defaults brings back the defaults' gains.
- [ ] `OptionRow::volumes()` gives the rows `options.row.master_volume`,
      `options.row.music_volume`, `options.row.sound_volume` and
      `options.row.voice_volume` in that order, and each moves its own
      group (a unit test).
- [ ] `shuyi-assets`, `shuyi-ui` and `shuyi-platform` still build for
      `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: manifest groups and their errors; `loudness` and
  `position_of_linear`; `SoundGroups` (defaults, a cue's group, a
  built-in's default set by the manifest); `GroupGains` (the chain,
  master at 0) and `Gains::muted`; `Volumes` (set, back to the default,
  restored, `set_from_linear`); `Ctx::load_settings` dropping a volume
  at its default; the volume rows (each moves its group, step, type a
  number, put back on Discard, `volumes()` in order).
- Integration: the Harness tests above (they replace
  `the_volumes_and_fullscreen_are_what_the_last_frame_said` in
  `crates/ui/tests/it/audio.rs` and
  `the_harness_says_what_music_sounds_and_how_loud` in
  `crates/platform/tests/it/main.rs`); `shuyi-app`'s player with its
  fake backend (a sound, a track and a voice at their gains, and a live
  change; they replace `the_players_volumes_scale_sounds_and_music` in
  `crates/app/src/audio.rs` and `the_volume_follows_the_voice_volume`
  in `crates/app/src/audio/voice.rs`); each saved form's old-save test.
- Snapshot: the pack's binary forms
  (`the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
