# ADR-0136: A track's loop region is the cue's or its file's, played by the backend, and the clock told where it is

- **Status:** Accepted
- **Date:** 2026-11-22
- **Related tickets:** 2397, 2365, 3027, 2902
- **Extends:** ADR-0063 (a provided `Backend` method), ADR-0054 (the clock on
  the newest track that sounds)

## Context

A looped track used to go round the whole file. Music is often written as an
intro that plays once and a part that repeats, and music tools mark that part
in the file itself: the `LOOPSTART` and `LOOPLENGTH` comments of an Ogg Vorbis
file, in samples. Ticket 2397 adds it.

Three things have to agree: what is heard, where the music clock says it is
(`MusicClock`, which screens keep time with), and what a backend can do. The
game's manifest is read where the music folder can't be seen (`Files::music`
is `None` at run time), so the file's comments are known only to whatever
opens the file: the backend. Backends differ: `shuyi-app`'s `in-house-sound`
streams through a worker and can go round any region exactly (`LoopedRegion`,
ticket 3030); quad-snd loops whole files only; a Playdate file player has a
loop range of its own. ADR-0068 says a game must never come to need the new
backend.

## Decision

1. **The cue says, or the file does.** `MusicCue` has `loop_start_ms` and
   `loop_end_ms` (`shuyi-assets`), both or neither, checked `0 <= start < end
   <= length_ms` on a looped track, refused as written, never clamped. Left
   out, the region is the one the file's tags name, and the cue's wins where
   both say. A region is half-open; a millisecond is the frame `ms × rate /
   1000` rounded down (`LoopRegion::frames`), and a file's tags are frames
   already, played exactly.
2. **The backend resolves and plays it.** `Backend::set_loop_region(sound,
   region)` (`shuyi-platform`'s player) is provided and `Err(Unsupported)` by
   default. The player calls it once a looped track has loaded, with the cue's
   region or `None`; the backend answers the `Looping` it will play (`Whole`,
   `Region { start, end }` in seconds, or `Unread(why)` when the file's tags
   don't read). The way round is the backend's to make exact ahead of time,
   never a seek at the moment it is heard. A backend without it plays the
   whole track; the player warns once, and only for a cue that names a region.
   A file's unreadable tags are warned of naming the file and the tag.
3. **The clock is told where the track is.** `Player::music_playing` (and so
   `Audio::music_playing`) reports a track that goes round a region by where
   it is in the track: past the region's end, back by its length each time
   round. `MusicClock::from_elapsed` keeps wrapping at `length_ms`, which
   leaves such a position as it is. So the clock follows what is heard on
   every backend: round the region where it is played, round the whole track
   where it isn't, round the file's own region though the manifest never
   names it.
4. **In the in-house backend**, the track reads its tags as it loads
   (`VorbisSource::loop_region`, `shuyi_sound::tags`), and a looped play is a
   `LoopedRegion`. Round a region that starts past `SEEK_LEAD` frames, the
   worker's decoder lands on its first frame by an exact seek
   (`VorbisSource::new_at`, `LoopedRegion::landing`), so a long intro isn't
   decoded again each time round, which would hold the ring up.

## Consequences

- A game's files load and play everywhere: a region is a cue's data, which a
  whole-file backend plays whole with one line in the log.
- A screen's clock is right on every backend without knowing which it runs on.
- What `music_playing` reports for a region's track is a position, no longer
  the seconds since it started. A beat clock that counts across the wrap
  (2365) reads the region from the cue and counts the loop point once; the
  headless `Speaker` reports whole tracks, so a test of a region's clock uses
  the player.
- The Playdate's file-player loop range and baked metadata, and a tracker's
  restart point (3027), each implement `set_loop_region` in their own ticket.

## Alternatives considered

- **The clock wraps through the cue's region in `ui`.** Simple, but wrong
  where the backend plays the whole track (it would show the region while the
  tail plays), and blind to a region only the file names.
- **The platform reports the region beside the elapsed time.** Truthful, but
  it changes `Audio`, `Game::set_music_playing` and the session record for
  one number the player can fold into the position it reports.
- **The file's tags read at check time into the manifest.** The manifest is
  read at run time where the music can't be seen, so a file's region would be
  missing exactly where it plays.
- **A seek on the main thread when the end is heard.** Late by a frame or
  more, and audible; the worker's way round is decoded ahead into the ring.
