---
id: "2341"
title: "Music crossfades and fade-ins: how one track gives way to the next, per request, with today's fade-out-then-start as the default"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2341 — Music crossfades and fade-ins

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Godot's `AudioStreamPlaylist` has a `fade_time` between
tracks, and its `AudioStreamInteractive` fades one clip out while the
next fades in
(https://docs.godotengine.org/en/stable/classes/class_audiostreamplaylist.html).
Unity games do it with two `AudioSource`s or a mixer snapshot's
`TransitionTo`, and Unreal with `FadeIn` and `FadeOut` on two audio
components. Bevy leaves it to the game, which changes two sinks'
volumes itself.

What Shuyi has today:

- `crates/assets/src/audio.rs`: one fade for the whole game,
  `AudioManifest::music_fade_ms` (read from `RawFile`, whose fields are
  `deny_unknown_fields`), and nothing per track or per switch.
- `crates/ui/src/audio.rs`: `MusicState` (pure) switches tracks one way
  only: "the old one fades out, then the new one starts". Its
  `MusicCommand::Start` always plays "from the start at full gain", so
  nothing fades in and two tracks never sound together.
  `AudioQueue::play_music(cue)` has no way to say how to switch.
  `Game::with_stack` (`crates/ui/src/game.rs`) builds the state with
  `MusicState::new(fade_secs)` from `music_fade_ms`.
- `crates/app/src/audio.rs` and `crates/app/src/audio/native_music.rs`:
  the player already holds two tracks (the one fading out and the next).
  Each native track has a quad-snd `AudioContext` of its own (game
  ADR-0028). Its test `of_two_tracks_the_one_started_last_is_reported`
  already plays two started tracks and reports the newer. On the web,
  every sound and track shares macroquad's context, each with its own
  volume. So a crossfade needs no new backend code: it is two tracks
  with gains moving opposite ways.
- `crates/platform/src/headless.rs`: the `Speaker` remembers only the
  track started last, so it can't model two tracks sounding.

Why it matters:

- **The game** switches with a 0.5 s fade-out and cuts its tracks to
  their loop (game ADR-0026 §1 and §6). It doesn't need this, and with
  the default below nothing it hears changes. Game ADR-0026's
  consequences rule out crossfades while the game stays on quad-snd.
  That was written before each native track had a context of its own,
  and it stays the game's choice.
- **Other 2D games:** a town theme that blends into a battle theme, a
  title screen whose music fades in, a menu that fades its music in
  after a cut. Each engine above does this without the game writing a
  mixer.

The gap also asked for intro-then-loop regions (a track's intro plays
once, then a part of it loops). They are not here: quad-snd 0.2.8's
`PlaySoundParams` has only `looped` and `volume`, so neither desktop nor
the web can loop part of a file, and a region would always fall back to
looping the whole file. They wait for the sound research (2902, parked),
which writes that ticket with the backend that can play it. Beat and bar
switches are 2365 (parked, and blocked by this ticket).

## Nick input

None.

## Scope

**In:**
- **`MusicSwitch`** in `shuyi_assets::audio`, re-exported from
  `shuyi_ui::audio`: `{ out_ms: u32, in_ms: u32, overlap: bool }`,
  deriving `Debug, Clone, Copy, PartialEq, Eq`:
  - `out_ms`: how long the old track fades out;
  - `in_ms`: how long the new track fades in (0: it starts at full gain,
    as today);
  - `overlap`: the new track starts when the old one begins to fade (a
    crossfade), or after it has stopped (today's).
  - `const fn` constructors: `MusicSwitch::new(out_ms, in_ms, overlap)`,
    `MusicSwitch::after_fade(out_ms)` (today's rule: `in_ms` 0, no
    overlap) and `MusicSwitch::crossfade(ms)` (`out_ms` and `in_ms` both
    `ms`, overlapping).
- **The manifest's default switch:** `AudioManifest` gains
  `music_fade_in_ms: u32` and `music_overlap: bool`, beside the required
  `music_fade_ms`. In the file they are optional: `RawFile` gets both
  with `#[serde(default)]` (0 and `false`), and `from_source` copies
  them. `AudioManifest::music_switch()` returns
  `MusicSwitch::new(music_fade_ms, music_fade_in_ms, music_overlap)`. An
  existing `audio.ron` reads the same.
- **A switch per request:** `AudioRequest::PlayMusic` gains
  `switch: Option<MusicSwitch>` (`None`: the manifest's default).
  `AudioQueue::play_music(cue)` stays and sends `None`;
  `AudioQueue::play_music_with(cue, MusicSwitch)` is new.
  `AudioRequest::cue` and `is_known` (`crates/ui/src/game.rs`) match it
  with `..`.
- **`MusicCommand::Start` gains `gain: f32`:** the gain the track starts
  at (1.0 for today's start, 0.0 for one that fades in). In `shuyi-app`'s
  player (`Audio::music`, the `MusicCommand::Start` arm in
  `crates/app/src/audio.rs`), `track.gain` takes it instead of 1.0, and
  a ready track plays at `track.volume * track.gain * music_volume`
  (today it plays at `track.volume * music_volume`). `poll` already uses
  `track.gain` for a track that loads later. The contract's music log
  (`fn music` in `crates/cli/src/contract/session.rs`) matches
  `Start { cue, .. }`: its answer doesn't change, so `contract::VERSION`
  stays.
- **`MusicState`,** in `crates/ui/src/audio.rs`:
  - `MusicState::new(fade_secs)` keeps its meaning
    (`MusicSwitch::after_fade`); a new `MusicState::with_switch(default:
    MusicSwitch)` takes the default switch, and `Game::with_stack` builds
    it from `audio_manifest().music_switch()`;
  - `current()` is the outgoing track during a switch and `target()` the
    incoming one, as today.
- **`MusicState` rules:**
  1. The default switch gives exactly today's commands: every existing
     `MusicState` test passes with only its `play` helper changed to
     carry `switch: None` and its `start` helper to carry `gain: 1.0`.
  2. **Fade-in:** a track whose switch has `in_ms > 0` starts at gain 0
     and rises to 1 over `in_ms`, in `Gain` commands, as fades do
     today. The first track after silence fades in too.
  3. **Overlap:** on a switch, the new track gets `Load` and
     `Start { gain: 0.0 }` at once (or 1.0 when `in_ms` is 0). The old
     track's gain falls to 0 over `out_ms`, then it gets `Stop`.
  4. **Waiting for the new track to sound:** a track only sounds once
     its file has loaded (80 to 580 ms on native, says
     `native_music.rs`; on the web the download comes first). A fade-in
     counts from when the track sounds, and in an overlap the old track
     holds its gain until then. `MusicState` learns it from the music
     clock: `Game::set_music_playing` calls a new
     `MusicState::heard(clock: Option<&MusicClock>)` every frame, with
     the clock it has just set on `Ctx`, and a track sounds once the
     clock names it (and stays sounding after). After
     `MusicState::LOAD_WAIT_SECS` (3.0) without that, the fades run
     anyway, so a missing file never holds the old track forever. The
     method takes the whole clock, not only its cue, so 2365 can read
     beats from it without changing the signature.
  5. **At most two tracks,** as today's memory budget says (game
     ADR-0026 §6): a switch during an overlap stops the older outgoing
     track at once; the track that was fading in becomes the outgoing
     one and fades out from the gain it had reached; the new one loads
     and starts.
  6. **Asking again** for the track that is fading out brings it back:
     it fades in from its current gain over the switch's `in_ms` (at
     once when 0, as today), and the incoming track fades out (overlap)
     or is stopped (no overlap, as today).
  7. `StopMusic` fades to silence over the default switch's `out_ms`,
     as today. During an overlap both tracks fall to 0 from the gains
     they have, then both get `Stop`.
- **The clock during an overlap:** the newest track that sounds. That is
  what `shuyi-app`'s `Audio::music_playing` already reports (the latest
  `started_at`). Update the docs that say "during a fade, the track
  fading out": `Ctx::music_clock` (`crates/ui/src/ctx.rs`), the
  `Audio::music_playing` trait method (`crates/platform/src/audio.rs`)
  and `shuyi-app`'s `Audio::music_playing` (`crates/app/src/audio.rs`).
- **The `Speaker`** (`crates/platform/src/headless.rs`) keeps every
  track started and not stopped, with its start time and its last gain
  (from `Start` and `Gain`):
  - `music_playing` reports the newest one that sounds (past its load
    delay), else the one before it, so a slow load in a crossfade keeps
    the old track's clock;
  - a new `Speaker::playing() -> Vec<(&str, f32)>` lists the tracks
    sounding and their gains, for tests of a crossfade.
- **The pack:** `AudioManifest`'s binary form changes, so
  `shuyi_assets::pack::VERSION` (`crates/assets/src/pack.rs`) goes to the
  next number and the snapshot of
  `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
  (`crates/assets/src/pack/tests.rs`) is accepted again.
- **An ADR** (the `write-adr` skill) that extends ADR-0004 §6 ("Audio
  stays as it is"): music switches are data, today's fade is the
  default, a crossfade on quad-snd is two tracks with gains, at most two
  tracks are in memory, and the clock follows the newest track that
  sounds.
- **Either order with 3029** (the player's rules shared by every
  backend):
  - if 3029 is done, the player's rules are in their shared place:
    change `Start`'s gain there, and add a crossfade case to its
    Playdate tests with the fake SDK (two file players sound, their
    volumes move opposite ways);
  - if not, change `shuyi-app`'s player; 3029 moves it with the rest
    (3029 gets an edit for this).
- **Public types (ADR-0039):** the types this PR adds or adds to
  follow it, as its *When* says: `MusicSwitch` is `#[non_exhaustive]`
  with the three constructors (so 2365 can add a field without a
  break); `AudioManifest`, and the `PlayMusic` and `Start` variants,
  move under the ADR's kinds if they aren't already, with a constructor
  (such as `MusicCommand::start(cue, gain)`) for any variant made
  non-exhaustive that another crate builds.
- **For the game:** the API breaks. `AudioRequest::PlayMusic` and
  `MusicCommand::Start` gain a field, and `AudioManifest` two public
  fields, so code that builds or matches them without `..` changes. The
  completion notes carry a `Breaking:` line in 2010's form (`Breaking:
  shuyi-assets, shuyi-ui: ...`), the PR says so, and the game's PR that
  moves its pin follows it. Its `audio.ron` needs no change.

**Size.** If the PR passes about 600 changed lines (the ADR and the
pack snapshot not counted), the `Speaker`'s changes and the Harness
test with a load delay go to a follow-up ticket (`write-ticket`),
After MVP, with their acceptance criteria, and the PR says so.

**Out (do not do):**
- intro-then-loop regions: they need a backend that loops part of a
  stream, which quad-snd can't. 2902 writes that ticket;
- beat and bar switches (2365, parked). Fade lengths in beats stay a
  game's: it writes `tempo.beat_ms() * n` as the milliseconds (2365);
- fade curves other than a straight line: 2109's `shuyi_core::ease` can
  shape them once both are done (a follow-up, if a game hears the
  difference);
- a fade length per `stop_music` (it uses the default's `out_ms`);
- sound groups and volumes (2342), ducking music under voices (2343),
  pausing audio on suspend or under a pause menu (2344);
- starting a track part-way in, and stems that play in sync (they need
  a mixer: after 2902);
- any game's music list.

## Implementation steps

1. Read `crates/ui/src/audio.rs` (`MusicState`, `MusicCommand`,
   `MusicClock`, and the tests' `play`, `start` and `frame` helpers),
   `crates/ui/src/game.rs` (`with_stack`, `collect_audio`,
   `set_music_playing`, `is_known`), `crates/app/src/audio.rs`
   (`Audio::music`, `poll`, `music_playing` and their tests) and
   `crates/platform/src/headless.rs` (`Speaker`). Check whether 3029
   is done.
2. `MusicSwitch`, the two manifest fields (`RawFile` and
   `AudioManifest`), `music_switch()`, their tests, and the pack
   version.
3. `AudioRequest::PlayMusic`'s switch, `play_music_with`, and
   `MusicCommand::Start`'s gain, with every literal and match updated
   (`grep -rn "PlayMusic\|MusicCommand::Start" crates templates
   examples`), the doc test at the top of `crates/ui/src/audio.rs`
   included.
4. `MusicState::with_switch` and `Game::with_stack`. Then rules 1 to 7,
   then `heard`, called from `Game::set_music_playing`. Write rule 1's
   check first: run the existing tests after each step.
5. `shuyi-app`'s `Start` gain (or 3029's shared rules), and the
   `Speaker`.
6. A Harness test of a crossfade with a load delay, from a screen in
   `crates/ui/tests/it/audio.rs` that calls `play_music_with(cue,
   MusicSwitch::crossfade(1000))`. Then the docs, the ADR, and the
   gates.

## Acceptance criteria

- [ ] With the default switch, every existing `MusicState`, `Game`,
      `shuyi-app` audio and Harness audio test passes, changed only
      where `PlayMusic` now carries `switch: None` and `Start` carries
      `gain: 1.0`.
- [ ] A crossfade of 1 s: in the frame of the switch the new track gets
      `Load` and `Start { gain: 0.0 }`; once it sounds, half a second
      later the old track's gain is 0.5 and the new one's 0.5; at 1 s
      the old one gets `Stop` and the new one is at 1.0.
- [ ] With a 0.3 s load delay on the Harness's `Speaker`
      (`Harness::music_load_delay`), the old track keeps its gain until
      the new one sounds, and `Ctx::music_clock` names the old track
      until then and the new one after.
- [ ] A track whose file never loads: the fades run after
      `LOAD_WAIT_SECS`, and the old track is stopped.
- [ ] A third request during an overlap: never more than two tracks
      between `Load` and `Stop`, checked over random request sequences.
- [ ] The first track after silence fades in when `music_fade_in_ms`
      is set.
- [ ] An `audio.ron` without the new fields reads as before; one with
      them gives the switch it says.
- [ ] `shuyi-app`: a `Start` with gain 0.0 plays at volume 0, and two
      tracks play at once during an overlap (the fake backend's calls).
- [ ] `pack::VERSION` is one higher and the pack snapshot is accepted.
- [ ] The ADR is written, with its row in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `MusicState` rules 2 to 7, `with_switch`, `heard` and the load
  wait; `music_switch()` and the three constructors; the manifest's new
  fields read and left out; the `Speaker`'s `playing()` and its clock
  during an overlap; `shuyi-app`'s `Start` gain.
- Property: over random sequences of switches, stops, `heard` calls and
  frame times, at most two tracks are between `Load` and `Stop`, and
  every gain stays in 0 to 1.
- Integration: a crossfade through the Harness
  (`crates/ui/tests/it/audio.rs`), with and without a load delay.
- Snapshot: the pack's binary forms, accepted with the new version.

## Completion notes

*(Filled in by the session that completes the ticket.)*
