# ADR-0072: Session records: App notes what the platform gives, a hash per frame of what happened, and RON that is never migrated

- **Status:** Accepted
- **Date:** 2026-11-01
- **Related tickets:** 2518 (this), 2575 (the Harness's part), 2519 (records
  from real builds), 2357 (rewind), 2029 (a capture plan that plays one),
  2111 (the stamp), 2220 (hot reload), 2321 (the frame cap), 2356 (debug
  commands), 3053 (platform facts), 2521 (the live link)
- **Extends:** ADR-0024 (the platform boundary), ADR-0022 §4 (the state hash)

## Context

Everything that reaches a game crosses the platform boundary (ADR-0024):
`App::frame(platform, now)` is the only way time gets in, and `Platform` gives
the input, the system's events, the store (handed over once, at the load), the
music's position, the system keyboard's answers and the facts of where the game
runs. The load's seed comes from the clock too. The core is deterministic
(ADR-0002). So a run's whole story fits in what crosses that line, and no game
code has to take part.

Nothing kept it. `shuyi replay` re-runs a turn game's rules record, a seed and
commands with a hash a step. It holds no menu, screen, clock, store, music or
keyboard answer. The Harness and `shuyi run` play scripts a person wrote. Tickets
(2905's first option, 2220's flag, `shuyi-debug-replay`) already assumed a record
that nobody built.

The others (read for 2518 on 2026-10-20): Unreal's Replay System records a
session to a stream; Unity's `InputEventTrace` records input events; Factorio's
integration tests compare saved games with checksums; Bevy's `ci_testing` plays
a hand-written list, and Godot has nothing built in.

## Decision

### 1. What a record holds

`shuyi_platform::session::SessionRecord`, `no_std` + `alloc`:

- once: `format_version`, `made` (2111's stamp, from `Start::made`), `display`,
  `platform` (`PlatformFacts`), `storage` and `storage_complete` (every key and
  value when the store was handed over, and whether all could be read),
  `voice_manifest`, `start` (the first frame's time), `reloaded_at` and
  `truncated`;
- per frame, a `FrameRecord`, in the order `App` reads the platform: `dt` (signed
  milliseconds: a clock may go back, and `App` must see that again), `system`,
  `input` (the frame's one poll, kept or dropped as `App` did), `keyboard`
  (`present` when it changes, the `answer` its poll gave, what `opened` answered
  after an ask), `music` and `track` (the position `music_playing` answered, and
  the cue when the sounding track changes, an empty cue for silence), `pacing`
  when it changes, and `hash` when it differs from the last frame's.

Empty lists and unchanged values are left out, so an idle frame is its `dt`
alone. What `Platform::clock`, `memory` and `heap_snapshot` give is left out on
purpose: they measure a run, and a game's rules and screens don't read them.

**The rule for later work: a new value that reaches the game from the platform
gets a field in the record and its playback in the same PR.** 2356's commands
and 2405's pointer events follow it.

### 2. `App` records, not a wrapper around the `Platform`

`App::record()` turns it on before the first frame (a record must start there to
play back), and `App` notes each value where it reads it. `App::session`,
`session_mut` and `take_session` give the record. A backend only switches it on,
and adds through `session_mut` what it puts into `Setup` around the trait (the
voice manifest). While recording it hands the game nothing else around the trait
(`App::game_mut`), since that isn't in the record.

Recording stops, and says so in `truncated`, at `MAX_SESSION_FRAMES` (216 000, an
hour at 60 frames a second) and on `App::frame_after`, whose time isn't whole
milliseconds and couldn't be replayed on `App::frame`.

### 3. The frame's hash: what happened, not the picture

After each frame `App` hashes (`shuyi_core::state_hash`, ADR-0022 §4) a value of
its own: the phase (loading, failed with its errors' text, or running), the open
screens' names bottom first, whether the frame quit, the frame's `SoundRequest`s
and `MusicCommand`s, and the game's own state hash if its `Start` gives one
(`Start::state_hash`). The frame's buffer is never hashed: a skin or palette
change must not break every recorded session (`AGENTS.md` rule 9). Looks are
tested by `frame-png` snapshots.

`App::frame_hash` gives the last frame's hash, while recording or with
`App::hash_frames(true)`; otherwise nothing is hashed, so a build that isn't
recording pays nothing.

### 4. The text form and the version rule

RON, one frame per line, so two records diff frame by frame
(`SessionRecord::to_ron`, `from_ron`; `deny_unknown_fields`; the suggested name
is `*.session.ron`). An hour of idle frames with music is under 16 MB. A record
is a run's log, not an asset kind, so it has no `Format` (ADR-0007 is about
assets), as 2706 says of `CommandRecord`.

The text is the same on every OS: `to_ron` pins the line ending to `\n`, which
RON would otherwise take from the OS. `RawInputEvent::Text` is written as a string
of one character, since RON escapes a string and writes a `char` as it is, so a
typed line break would break the line it is on.

A field added later is optional (`#[serde(default)]`, left out when empty), so
older records still read and `FORMAT_VERSION` stays. A change to what a field
means raises it. **A record is never migrated:** another version is refused
(`SessionError::Version`), whatever else its text holds.

### 5. Playback

`HeadlessPlatform::from_session` gives the recorded display and facts and a
`MemoryStorage` holding the recorded store, whose writes stay in memory.
`push_frame` queues one frame: while a record plays, `Speaker` answers with the
recorded position whatever its own load delay says, and a private keyboard is
there on the frames the record says, gives its answer and answers `open` as
recorded.

`session::prepare(start, &record, stamps)` checks the record's stamp against the
start's (`Stamps::Check` refuses with 2111's `Mismatch`; `Stamps::Ignore` plays),
puts the voice manifest in the setup, and gives the lines of what the playback
can't promise: no stamp, stamps ignored, a truncated record, an incomplete store,
files reloaded.

`session::Playback` drives `App::frame` at the recorded milliseconds, from the
game's real `Start`, and compares each frame's hash with the record's. A frame
with no hash has the last one written. It needs no `harness` feature, so a
running build can replay a record too. Frames count `App::frame` calls from 1, as
`App::frame_number` does.

It doesn't go through `Harness::frame(events, dt)`: the Harness drives
`App::frame_after` with `f32` seconds, which round differently from whole
milliseconds, so the fixed steps would differ, and `Harness::new` starts from a
running game, skipping the load whose seed comes from the clock.

### 6. A Harness from a `Start` doesn't hold back undeclared actions

2575 adds `Harness::start`, which runs the game from its real `Start` as a device
does. Since a device doesn't hold back the actions a screen didn't declare, that
Harness doesn't either. ADR-0025 stays true for `Harness::new` and `Harness::on`.

### 7. How it relates to the rules' records

`StageRecord`, `TickRecord` (2137) and `CommandRecord` (2706) replay a game's
rules: a seed and commands, a hash a step. They stay as they are. A session
record is one level down, what the platform gave the whole game. Playing it runs
the same rules, so a tick game's steps or a turn game's commands come out of it;
they are never stored in it.

### 8. What changes at the boundary

`shuyi-platform` gains `serde` and `ron` as plain dependencies, both already its
optional ones, and uses `shuyi-core` (ADR-0032 allows that edge) for `state_hash`
and `Made`. `RawInputEvent`, `PadKind`, `Device`, `Keyboard`, `TextAnswer`,
`PlatformFacts`, `Pacing`, `SystemEvent` and `DisplayInfo` gain `Serialize` and
`Deserialize`. `MusicCommand`, `SoundId` and `SoundRequest` gain `Serialize` only,
for the hash. `Start` moves under ADR-0039: `#[non_exhaustive]`, with `made` and
`state_hash` set by builders.

## Consequences

- Any game on the engine gets session records for nothing, on any backend. A
  softlock in a menu becomes a file an agent plays to the frame where it went
  wrong and keeps as a regression test.
- Each value from the platform costs one more field and its playback, for good
  (the rule in §1). A value left out shows up as a frame that differs.
- `App` carries the capture's branches on its frame path. Off, each is a test of
  an `Option`; the hash isn't computed.
- A record holds a player's input and saves. Recording in shipped builds and
  sending a record anywhere stay out, Nick's privacy decision (parked 3047).
- A record played by another build is refused by default: the game's content
  moves, and an old record then says nothing. It's a run's log, not a save.
- `f64` music positions read back bit for bit, except NaN, which a sound device
  has no reason to answer and RON writes as one value.
- A frame's picture can change without any record noticing: that's what `frame-png`
  snapshots are for.

## Alternatives considered

- **A `Recording<P>` wrapper around the `Platform`** — `Platform::audio` lends a
  `&mut dyn Audio`, so a wrapper can't hand out a device of its own that notes what
  `music_playing` answered. And only `App` sees the order of calls in a frame, the
  keyboard's handshake and the load.
- **Hashing the frame's buffer** — every skin or palette change would break every
  recorded session, against rule 9, and it says nothing of a sound.
- **A compact binary form** — no measurement asks for one: RON diffs, and an hour
  fits in 16 MB. A ticket if real records grow too big to keep.
- **Turning a record into a Harness script of keys and waits** — lossy: frame
  times, the music and the keyboard don't fit a script.
- **Migrating old records** — a record outlives neither its build nor its content,
  and the code to migrate one would outlive both.
- **Absolute times per frame instead of a signed `dt`** — bigger on every line, for
  nothing a difference can't give, and diffs of two runs would differ on every
  line. `wrapping_sub` and `wrapping_add_signed` round-trip any two `u64`s exactly.
