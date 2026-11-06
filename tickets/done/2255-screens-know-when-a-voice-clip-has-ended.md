---
id: "2255"
title: Screens know when a voice clip has ended
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
---

# 2255 — Screens know when a voice clip has ended

## Context

1. **The game ticket:** `game#0859`, auto-advance when the voice ends
   (github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/0859-auto-advance-when-the-voice-ends.md).
   It shipped with a marked workaround (`// engine-friction: engine#2255`):
   the dialogue screen takes a clip to last 0.08 s a character, because
   it can't know when the clip really ends. `game#0863` removes it once
   this ticket is done.
2. **The job:** a screen that asked for a voice clip learns, a frame or
   so later, that the clip is over, so it can move on by itself.
3. **No friction review:** a missing feature.
4. **The generic shape:** the platform reports each frame which voice
   clip is sounding (or loading to sound), the way it reports the music
   with `Game::set_music_playing` → `Ctx::music_clock`. Today a screen
   only asks (`Ctx::play_voice`, `AudioRequest::PlayVoice`); the voice
   player in `crates/platform/src/audio/player/voice.rs` knows when the
   clip ends (`Speaking::ends`, from `Backend::length`, held while the
   backend pauses it) and when an asked-for clip is skipped (`LATE_SECS`),
   but nothing goes back to the screens.
5. **Another game:** a visual novel's auto mode, or a farming game whose
   shopkeeper's voiced greeting must finish before the shop menu opens.

## Nick input

None.

## Scope

**In:**
- What the platform reports each frame: the line (and variant) of the
  voice clip that is sounding or still being asked for, or none. A clip
  asked for and skipped as late, one that failed to load, one stopped
  and one that has ended all report none. A clip paused with the
  backend (the window hidden) still reports as sounding.
- `Game::set_voice_playing` (beside `set_music_playing`), called by the
  platform's frame loop before `Game::frame`, and `Ctx::voice_playing()
  -> Option<&LineId>` (or a small `#[non_exhaustive]` struct if the
  variant is needed) for screens.
- The request made this frame isn't seen until the platform has read it:
  document that a screen sees its own `play_voice` from the next frame,
  and that a screen should wait for the line to report sounding (or for
  a frame past the late limit) before taking "none" as "ended".
- The headless backend and the Harness: a test clip has a length, so a
  Harness test can step time past a clip's end (a fixed length a test
  can set, or the length the headless speaker already knows).
- Records (`Recorder`) carry it as they carry the music clock, so a
  replay sees the same frames.
- Additive API only (new method, new field on a non-exhaustive type).

**Out (do not do):**
- Auto-advance itself, or anything about dialogue: the game's
  `game#0863`.
- The Playdate front end's voices (the game's 1048).
- The clip's length in the voice manifest.

## Implementation steps

1. `Voices::playing_line(&self) -> Option<(&LineId, &V)>` in the voice
   player: the `asked` or `playing` clip (map file back to line).
2. `Player`/`platform.audio()` exposes it; `app.rs`'s frame loop passes
   it to `Game::set_voice_playing` next to `set_music_playing`, and the
   recorder writes it.
3. `Ctx` keeps it; `Ctx::voice_playing()` reads it.
4. Headless speaker: test clips get a length; Harness tests step time.
5. The book's audio chapter: a paragraph on waiting for a voice.

## Acceptance criteria

- [x] A Harness test: after `play_voice`, `voice_playing()` names the line from the next frame until its length has passed, then is `None`.
- [x] Stopped, replaced, skipped as late and failed clips report `None` (unit tests on the voice player).
- [x] A clip paused with the backend still reports as sounding.
- [x] A record replays with the same reports.
- [x] The API change is additive (CI's `api` job).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the voice player's report through ask, start, end, stop, late
  skip, failure and pause.
- Snapshot / integration: the Harness test above; a record round trip.

## Completion notes

- The sound device reports the voice clip sounding by its file:
  `Audio::voice_playing(now) -> Option<&str>`, a provided method that
  answers `None` (so a backend with no voices needs nothing).
  `PlayerDevice` answers from the voice player (`Player::voice_playing`,
  `Voices::sounding`): the clip asked for and not yet late, or playing
  and not past its end; a clip held by a backend's pause still sounds.
- `App`'s frame loop reads it after `music_playing`, maps the file back
  to the line it last asked for (`Lists::voice`, filled from the frame's
  `PlayVoice` requests), and calls `Game::set_voice_playing(Option<&LineId>)`.
  Screens read `Ctx::voice_playing() -> Option<&LineId>`; its rustdoc
  says a screen sees its own `play_voice` from the next frame and when to
  take `None` as the end.
- The headless `Speaker` keeps the voice asked for and gives clips a
  fixed length a test sets (`Speaker::set_voice_length`,
  `Harness::voice_length`); with none set a clip sounds until stopped or
  replaced. It mutes rather than pauses, as for the music.
- Session records write `voice` (the clip's file, when it changes; empty
  for none) like `track`; playback answers from it. An optional field,
  so the format's version stays 1.
- Deviation: step 1 named `Voices::playing_line` returning the line and
  variant; the voice player knows only files, so it reports the file and
  `App` maps it back to the line. `Ctx` keeps the line alone: no screen
  needs the variant yet.
- Book: `games-and-screens.md` has a paragraph and a snippet
  (`VoiceWait`) on waiting for a voice; `docs/platform-backends.md` lists
  `voice_playing` and its place in the frame.
- No break: every change is an addition (a provided trait method, new
  methods, an optional record field, a field on the non-exhaustive
  `FrameRecord`).
- The game: `game#0863` removes the 0.08 s-a-character workaround and
  moves the pin to this change.
