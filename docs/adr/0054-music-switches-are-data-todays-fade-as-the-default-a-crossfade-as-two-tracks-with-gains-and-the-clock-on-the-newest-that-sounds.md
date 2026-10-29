# ADR-0054: Music switches are data: today's fade as the default, a crossfade as two tracks with gains, and the clock on the newest that sounds

- **Status:** Accepted; extends ADR-0004 §6
- **Date:** 2026-10-29
- **Related tickets:** 2341 (this), 2365 (switches on a beat or a bar,
  parked), 2136 (fades on `shuyi_core::ease`), 3029 (the player's rules
  for every backend), 2902 (sound after quad-snd, parked), 2393 (the
  `Speaker` and a Harness test with a load delay, the follow-up)

## Context

- **Before this ADR.** ADR-0004 §6 kept audio as the game had it. The
  audio manifest has one fade for the whole game (`music_fade_ms`), and
  `shuyi_ui::audio::MusicState` switches one way only: the old track
  fades out, then the new one starts at full gain. Nothing fades in and
  two tracks never sound together.
- **The other engines** (compared on 2026-10-20): Godot's
  `AudioStreamPlaylist` has a `fade_time` between tracks and its
  `AudioStreamInteractive` fades one clip out while the next fades in;
  Unity games use two `AudioSource`s or a mixer snapshot's
  `TransitionTo`; Unreal uses `FadeIn` and `FadeOut` on two audio
  components; Bevy leaves it to the game.
- **The backends can already do it.** `shuyi-app`'s player holds a track
  per cue, at most two of them (the one fading out and the next, game
  ADR-0026 §6). On native each track has a quad-snd context of its own
  (game ADR-0028); on the web every sound shares macroquad's context,
  each with its own volume. Two tracks with gains moving opposite ways
  is a crossfade, with no new backend code. quad-snd 0.2.8's
  `PlaySoundParams` has only `looped` and `volume`: no loop regions and
  no start part-way in.
- **A track sounds late.** It plays only once its file has loaded (80 to
  580 ms on native, says `native_music.rs`; on the web the download
  comes first). The platform reports the track really sounding as the
  music clock (game ADR-0037), which `Game::set_music_playing` sets on
  `Ctx` every frame.

## Decision

1. **A switch is data.** `shuyi_assets::audio::MusicSwitch { out_ms,
   in_ms, overlap }`, re-exported from `shuyi_ui::audio`: how long the
   old track fades out, how long the new one fades in (0: it starts at
   full gain), and whether the new one starts as the old one begins to
   fade (a crossfade) or once it has stopped. It is
   `#[non_exhaustive]` with `const fn` constructors `new`,
   `after_fade(out_ms)` and `crossfade(ms)` (ADR-0039 kind 2), so 2365
   can add when a switch happens.
2. **Today's fade is the default.** The manifest gains
   `music_fade_in_ms` and `music_overlap`, both optional in the file
   (`#[serde(default)]`: 0 and `false`), so an existing `audio.ron`
   reads the same and switches as before.
   `AudioManifest::music_switch()` is the default switch; `Game` builds
   its `MusicState` with it (`MusicState::with_switch`).
   `AudioQueue::play_music(cue)` uses it; `play_music_with(cue, switch)`
   names one, carried as `AudioRequest::PlayMusic`'s `switch`. A stop
   fades over the default's `out_ms`.
3. **A crossfade is two tracks with gains.** `MusicCommand::Start`
   carries the gain a track starts at (1 at full gain, 0 to fade in),
   and the platforms play every track started and not stopped, each at
   its gain times its volume times the music volume. The engine has no
   mixer: the fades are `Gain` commands each frame, as before.
4. **At most two tracks are loaded**, as game ADR-0026 §6 says. A
   switch while two sound stops the older at once; the newer fades out
   from the gain it had reached. Asking for a track that is fading out
   brings it back from its gain. A gain moves in a straight line at one
   whole fade per `in_ms` or `out_ms`, so a fade from part-way takes its
   share of the time, and a fade already going the same way at the same
   pace keeps its progress (today's rule for a switch during a fade).
5. **Fades wait for the new track to sound.** `MusicState::heard(clock)`
   is told each frame, from `Game::set_music_playing`, what the clock
   names; a track sounds once the clock has named it since its `Start`.
   A fade-in starts then, and in a crossfade the old track holds its
   gain until then. After `MusicState::LOAD_WAIT_SECS` (3 s) the fades
   run anyway, so a missing file never holds the old track. `heard`
   takes the whole clock, so 2365 can read beats from it.
6. **The clock is the newest track that sounds**: in a crossfade the old
   one until the new one has loaded, then the new one. `shuyi-app`'s
   player already reports the latest started; the docs of
   `Ctx::music_clock` and `Audio::music_playing` say so.
7. **Straight lines only.** Other curves wait for 2136, which moves the
   fades onto `shuyi_core::ease`'s `Tween`.

## Consequences

- A game gets a title screen that fades in, a town theme that blends
  into a battle theme, or a cut followed by a fade-in, by data or one
  call, with no mixer of its own. A game that changes nothing hears
  nothing new.
- `AudioRequest::PlayMusic` and `MusicCommand::Start` gain a field and
  become `#[non_exhaustive]` variants (built with `play_music` and
  `MusicCommand::start`), and `AudioManifest` becomes
  `#[non_exhaustive]` (built with `default()` and assignments): a
  break, with a `Breaking:` line in 2341. The pack's binary form of
  the manifest changes, so `pack::VERSION` is 4.
- Every backend must play two tracks at once and start one at a gain.
  `shuyi-app` does both today; 3029's shared player keeps them.
- In a game whose platform never reports the clock, fade-ins and a
  crossfade's old track wait 3 s. The default switch never waits.
- The headless `Speaker` still remembers only the track started last,
  so a Harness test sees a crossfade only in the commands and the
  clock; 2393 makes it keep both tracks and their gains.

## Alternatives considered

- **A fade length per track in the manifest** — a switch involves two
  tracks, so the length belongs to the switch, not to either track; a
  per-track value can't say whether to overlap.
- **Mixing in the engine** (one stream, our own sums) — needs a mixer
  quad-snd doesn't have; 2902 weighs one. Two tracks with gains work on
  every backend today.
- **Fades counted from the request** — the new track sounds up to half
  a second later on native and more on the web, so a crossfade would
  dip into silence or cut the old track before the new one is heard.
- **A fixed duration for every fade, from any gain** — a fade from 0.25
  would take as long as one from 1, and a switch during a fade would
  restart it, unlike today's rule that a fade keeps its progress.
