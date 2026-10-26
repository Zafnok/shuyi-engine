---
id: "2518"
title: "Session records: every input, clock reading, music position, keyboard answer and stored file that reaches a game, recorded at the platform boundary and played back exactly in the Harness, with a hash per frame"
type: feature
milestone: After MVP
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["2111"]
nick_input: none
completed:
---

# 2518 — Session records and their playback in the Harness

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. What the others have:

- Unreal's Replay System records a session to a stream (in memory or a
  file) and plays it back, with scrubbing.
- Unity's Input System records input events (`InputEventTrace`) and
  replays them.
- Factorio runs integration tests that load saved games and compare
  them with checksums saved beforehand
  (https://factorio.com/blog/post/fff-60). Its per-tick checksums are
  its multiplayer desync check, a different thing.
- Bevy's `ci_testing` plays a hand-written RON list of events by frame,
  and its remote protocol (BRP) gives agents a running game, but neither
  records a session. Godot has nothing built in.

What Shuyi has (read on 2026-10-20, checked again on 2026-10-21):

- Everything that reaches a game crosses the platform boundary
  (ADR-0024): `App::frame(&mut self, platform, now: Millis)`
  (`crates/platform/src/app.rs:372`) is the only way time gets in, and
  the `Platform` trait (`crates/platform/src/platform.rs:189-219`) gives
  the input (`poll_input`), the system's events (`system_events`), the
  storage (handed over once, in `App::load`, `app.rs:475-499`), the
  sound device (`Audio::music_playing` returns the music's position as
  an `f64` from the device, read in `step`, `app.rs:600`) and the system
  keyboard (`text_input`, answered at `app.rs:595-599`, opened at
  `app.rs:618-624`). The load's seed is `mix(now.0)` (`app.rs:479`), so
  it comes from the clock too.
- No record of a whole session exists:
  - `shuyi replay` (`crates/cli/src/contract/replay.rs`) re-runs a turn
    game's rules record. Hello, Grid's `StageRecord` (a seed and
    commands, `examples/hello-grid/src/tools.rs:222-230`) is written
    only by `bots`. 1119's `TickRecord` and 2706's `CommandRecord` are
    parked. None of them holds menus, screens, the clock, the storage,
    the music or the keyboard's answers.
  - The Harness (`crates/platform/src/harness.rs`) and `shuyi run` play
    scripts a person wrote (keys, pad buttons, waits), never input
    captured from a run.
  - The types that cross the boundary can't be stored: `RawInputEvent`
    (`crates/ui/src/game.rs:27`), `PadKind`
    (`crates/ui/src/input/pad.rs:37`), `SystemEvent` (`platform.rs:149`)
    and `TextAnswer` (`crates/ui/src/system_keyboard.rs:36`) derive no
    serde. Nor do the frame's audio requests the hash below reads:
    `SoundRequest` (`crates/platform/src/audio.rs:22`) and
    `MusicCommand` (`crates/ui/src/audio.rs:320`).
- Tickets already assume a record nobody builds: 2905's first option
  "replays the record so far"; 2220 planned a `Record::reloaded` flag on
  a record no ticket made; ADR-0005 §3's `shuyi-debug-replay` skill says
  "Reproduce a bug from a record".
- Later tickets build on this one: 2519 (records from real builds),
  2357 (rewind) and 2905 (the code reload research). 2220, 2321, 2356,
  2405 and 3053 add their values to the record in either order. 2029
  plays a record in 2018's capture mode. 2514 and 3046 leave work with
  records to follow-ups.

Why it matters:

- **The game first.** A softlock in a menu or a skipped line in a
  playtest of Visions of Shuyi's debug build becomes a file. An agent
  plays it headless to the frame where it went wrong, fixes the bug and
  keeps the file as a regression test. This ticket builds the record and
  its playback; 2519 makes the real builds write one.
- **Every game on the engine** gets it for nothing: no game code records
  anything, because the record is taken where the platform hands things
  to `App`. Bevy and Godot have nothing built in for this; Shuyi can do
  it because its core is deterministic (ADR-0002) and its boundary is
  narrow.
- **The end goal** (2026-10-19, `docs/decisions.md`): games made by
  agents. An agent that can replay a player's session reads what
  happened instead of guessing.

**How it relates to the rules' records.** `StageRecord`, `TickRecord`
(1119) and `CommandRecord` (2706) replay a game's rules: a seed and
commands, a hash per step. They stay as they are. A session record is
one level down: what the platform gave the whole game, menus and
screens included, with a hash per frame. Playing a session runs the
same rules, so a tick game's steps or a turn game's commands come out
of it; they are never stored in it.

**Why `App` records, not a wrapper around the `Platform`.** A wrapper
(`Recording<P>`) was considered. `Platform::audio` lends a
`&mut dyn Audio`, so a wrapper can't hand out a device of its own that
notes what `music_playing` answered. And only `App` sees the order of
calls in a frame, the keyboard's handshake and the load. So `App`
records, and the backend only switches it on.

**Why playback doesn't go through `Harness::frame(events, dt)`.** The
Harness drives `App::frame_after` with `f32` seconds
(`harness.rs:307-309`). `Clock::after` (`app.rs:269-278`) turns them
into a `Duration` through `f32`, which rounds differently from the
whole milliseconds `Clock::at` (`app.rs:258-266`) gets, so the fixed
steps would differ. And `Harness::new` starts from a running game
(`App::running`, `harness.rs:88`), skipping the load whose seed comes
from the clock. So playback reuses the Harness and the headless backend,
but drives `App::frame` with the recorded milliseconds, from the game's
real `Start`.

## Nick input

None. Recording in shipped builds, and sending a record as a bug
report, are out of scope (they hold a player's input and saves: a
privacy question for Nick, which parked 3047 asks for crash reports).

## Scope

**In:**
- **Serde on the boundary's types:** `RawInputEvent`, `PadKind` and
  `TextAnswer` in `shuyi-ui`; `SystemEvent` and `DisplayInfo` in
  `shuyi-platform` (`Chord`, `Key`, `Button` and `Color` have it
  already). For the frame's hash, `MusicCommand` (`shuyi-ui`) and
  `SoundRequest` (`shuyi-platform`) gain `Serialize` only
  (`state_hash` takes floats by their bits). `shuyi-platform` gains
  `serde` and `ron` (both already workspace dependencies, MIT OR
  Apache-2.0, allowed by `deny.toml`), and both join
  `shuyi-platform`'s `outside` list in `tools/layers.ron`, with a
  comment (session records), in the same PR. It also gains `shuyi-core`
  (for `state_hash` and 2111's `Made`), an edge `tools/layers.ron`
  already allows (ADR-0032). All three come without default
  features; its `std` feature turns on theirs. No new crate enters the
  tree.
- **`shuyi_platform::session`** (new, `no_std` + `alloc`):
  - `SessionRecord`:
    - `format_version: u32` (`FORMAT_VERSION`, 1);
    - `made: Option<Made>` (2111): the build that made it;
    - `display: DisplayInfo`: what `App::load` was given;
    - `platform: PlatformFacts` (either order with 3053): what
      `App::load` got from `Platform::facts`, with serde derives on
      `PlatformFacts`. Playback hands it to
      `HeadlessPlatform::with_facts`, so a session plays back as the
      target it was recorded on. If 3053 isn't done, leave it out, and
      3053 adds it;
    - `storage: BTreeMap<String, String>`: every key and value in the
      store when it was handed over (`list`, then `read` of each), and
      `storage_complete: bool`, false if the keys or a value couldn't be
      read;
    - `voice_manifest: Option<String>`: the manifest's text a backend
      put into `Setup` through `Start::before_load`, which `App` can't
      see; the backend gives it with `App::session_mut` (2519 does for
      `shuyi-app`);
    - `start: u64`: the first frame's `Millis`;
    - `frames: Vec<FrameRecord>`;
    - `truncated: bool`: recording stopped early (below).
  - `FrameRecord`, one per `App::frame`, in the order `App` reads the
    platform (`docs/platform-backends.md`, *What one frame does*):
    - `dt: i64`: this frame's `now` minus the last one's, in ms
      (signed: a clock may go back, and `App` must see that again);
    - `system: Vec<SystemEvent>`;
    - `input: Vec<RawInputEvent>`: the frame's one poll, kept or
      dropped as `App` did;
    - `keyboard: Option<KeyboardFrame>`, where `KeyboardFrame { present:
      Option<bool>, answer: Option<TextAnswer>, opened: Option<bool> }`
      holds whether the system keyboard is there (written when it
      changes), the answer `poll` gave, and what `open` answered after
      an ask;
    - `music: Option<f64>` and `track: Option<String>`: what
      `music_playing` answered, the track's cue written only when the
      sounding track changes (an empty cue for silence);
    - `hash: Option<u64>`: the frame's hash (below), written only when it
      differs from the last frame's.

    Empty lists and unchanged values are skipped (`#[serde(default,
    skip_serializing_if = ...)]`), so an idle frame is written as its
    `dt` alone, or its `dt` and `music` while music sounds.
  - Its text form: `SessionRecord::to_ron(&self) -> String` writes one
    frame per line (`ron`'s `PrettyConfig` with a depth limit), so two
    records diff frame by frame; `from_ron(&str) -> Result<Self,
    SessionError>` reads it with `deny_unknown_fields` and refuses
    another `format_version`. A record is never migrated.
  - `SessionError`, with `Display`: `Text(String)` (ron's error, with
    its line and column); `Version(u32)` ("a session record of version
    2; this engine reads version 1"); `Made(Mismatch)` (2111's text).
  - `Stamps`: `Check` (the default) or `Ignore`, for playback.
  - **The version rule:** a field added later is optional
    (`#[serde(default)]`, skipped when empty), so older records still
    read and `FORMAT_VERSION` stays. A change to what a field means
    raises it. The either-order fields below, 2220's `reloaded_at` and
    2357's `branch` follow it.
  - A record is a run's log, not an asset kind, so it gets no `Format`
    (as 2706 says of `CommandRecord`; ADR-0007 is about assets). The
    suggested file name is `*.session.ron`.
- **The frame's hash: what happened, not the picture.** After each
  frame `App` hashes, with `shuyi_core::state_hash`, a value of its
  own: the phase (loading, failed with its errors' text, or running),
  the open screens' names bottom first (`Game::screens`), whether the
  frame quit, the frame's `SoundRequest`s and `MusicCommand`s, and the
  game's own state hash if its `Start` gives one:
  `Start::state_hash(self, hash: fn(&Game<G>) -> Option<StateHash>)
  -> Self`. The frame's buffer is never hashed: a skin or palette change
  must not break every recorded session (`AGENTS.md` rule 9; looks are
  tested by `frame-png` snapshots).
- **`App` records:**
  - `Start::made(self, made: Made) -> Self` (a new field of `Start`,
    `None` by default): the build's stamp, copied into the record;
  - `App::record(&mut self) -> bool` turns recording on; `false` once a
    frame has run, since a record must start at the first frame to play
    back;
  - `App::session(&self) -> Option<&SessionRecord>`,
    `App::session_mut(&mut self) -> Option<&mut SessionRecord>` and
    `App::take_session(&mut self) -> Option<SessionRecord>`;
  - `App::frame_hash(&self) -> Option<StateHash>`: the last frame's
    hash, computed while recording or when `App::hash_frames(&mut self,
    on: bool)` is on (playback);
  - it notes each value at the point where `app.rs` reads it today
    (`frame`, `system_events`, `drain`, `step`, `load`), with no change
    to what the game sees;
  - recording stops after `MAX_SESSION_FRAMES` (216 000, an hour at 60
    frames a second) and sets `truncated`;
  - `frame_after` while recording also stops it and sets `truncated`
    (records are for drivers of `App::frame`, which every backend is).
- **The headless backend plays a record:**
  `HeadlessPlatform::from_session(&SessionRecord)` gives the recorded
  display and a `MemoryStorage` holding the recorded store (writes stay
  in memory), and `push_frame(&FrameRecord)` queues one frame's system
  events, input, keyboard and music. `Speaker` answers `music_playing`
  with the recorded position while one is queued. A recorded keyboard
  of its own (a private `TextInput`) is there exactly on the frames the
  record says, gives its answer on its frame and answers `open` as
  recorded. `ScriptedKeyboard` and the headless backend's other uses
  don't change.
- **The playback loop, outside the Harness:** `session::Playback`, built
  from a record. `Playback::next(&mut self, app: &mut App<G>, platform:
  &mut HeadlessPlatform) -> Option<Result<u32, FrameDifference>>` queues
  the next frame, runs `App::frame` at its recorded time, and compares
  `App::frame_hash` with the record's (a frame without a hash has the
  last one written); `None` once the record is played. It needs no
  `harness` feature, so a running build can replay a record too
  (rewind, 2357, uses it). `App::hash_frames(true)` is set by it.
- **The Harness:**
  - `Harness::start(start: Start<G>) -> Result<Self, String>`: the game
    from its real `Start`, as a device runs it: the card frame (at 0 ms)
    and the load frame (one `FRAME_DT` later) run at once, and the
    load's errors come back as the `Err`. Its frames go through
    `App::frame` on a clock of whole milliseconds the Harness keeps in
    microseconds, so `FRAME_DT` frames come out as 16, 17 and 17 ms. It
    doesn't hold back undeclared actions, as a device doesn't
    (`Harness::new` and `on` still do). The scripts (`keys`, `pad`,
    `wait`, `system`, …) work as before.
  - `Harness::start_recording(start)`: the same, recording from the
    first frame; `Harness::session()` gives the record so far. Its docs
    say what isn't recorded: changes made around the trait (`act`,
    `ctx_mut`, `screen_mut`).
  - `Harness::from_session(start: Start<G>, record: &SessionRecord,
    stamps: Stamps) -> Result<Self, SessionError>`: refuses a record
    whose `made` differs from the start's (2111's `Mismatch` text)
    unless `Stamps::Ignore`; applies the record's voice manifest with
    `Start::before_load`; plays nothing yet.
  - `Harness::play_to(&mut self, frame: u32) -> &Played` plays the
    record's frames up to `frame` (or its end) through `Playback`, with
    the Harness's per-frame bookkeeping, and stops at the first frame
    whose hash differs, so the game is left on that frame.
    `Harness::play_all()` plays to the end. It answers
    `Played { frames: u32, of: u32, first_difference:
    Option<FrameDifference { frame, recorded: StateHash, played:
    StateHash }>, limits: Vec<String> }`; `limits` says when the record
    or the start has no stamp, `Stamps::Ignore` was used, the record was
    truncated, or its store was incomplete.
  - After the record's last frame, the Harness goes on with its own
    scripts and clock from the record's last time; the music then comes
    from the headless speaker, and `limits` says so.
- **Either order:**
  - with 2220 (hot reload): if it is done, a frame whose
    `Platform::changed_files` named files is recorded as
    `reloaded_at: Option<u32>` (the first such frame) on
    `SessionRecord`, and `Played::limits` warns that frames after it may
    differ (the files' new bytes aren't in the record). If not, 2220
    does it (its own clause says so);
  - with 2321 (the frame cap): if it is done, the `Pacing` `App` hands
    the game each frame is kept on `FrameRecord` (`pacing:
    Option<Pacing>`, written when it changes, with serde derives on
    `Pacing`) and `Playback` hands it back. If not, 2321 adds it (its
    own clause says so);
  - with 2356 (debug commands): if it is done, a command run between
    frames goes through `App::command(&mut self, line: &str) ->
    Result<String, String>` (new), which runs `Game::command` and notes
    the line on the next `FrameRecord` (`commands: Vec<String>`);
    playback runs it before that frame. If not, 2356 adds it (its own
    clause says so);
  - **from 2521 (done)** (the live link, ADR-0047): a test sends a key
    over its link (`shuyi_platform::link::Server`, its input added after
    the devices' in `poll_input`, as `crates/platform/src/link/tests.rs`
    drives it) while recording, and finds it in the record's input. If
    2356 is done too, the link's `Request::Command`, which ran through
    `App::game_mut`, goes through `App::command` instead, so a command
    sent over the link is on the record (a test);
  - the new public types follow ADR-0039, and `Start`, which gains two
    fields here, moves under it in this PR (its rule for an existing
    type).
- **An ADR** (the `write-adr` skill, the next free number): what a
  session record holds and why `App` records it; the frame's hash and
  why it leaves the picture out; the text form and the version rule;
  how it relates to the rules' records; that a Harness started from a
  `Start` doesn't hold back undeclared actions (ADR-0025 stays true for
  `Harness::new` and `on`); and the rule for later work: **a new value
  that reaches the game from the platform gets a field in the record
  and its playback in the same PR**. It extends ADR-0024
  (`shuyi-platform` now depends on `shuyi-core`, the edge ADR-0032
  allows) and builds on ADR-0022 §4's state hash.
- **Docs:** `docs/platform-backends.md` gets a section *Session
  records*: the rule above, and that a backend hands the game nothing
  around the trait (`App::game_mut`) while recording, except through
  `App::session_mut` for what it puts into `Setup`.
- **For the game:** `Start` gains two fields set by builders (`made`,
  `state_hash`); a game that builds its `Start` with `Start::new`
  doesn't notice, but a struct literal of it no longer builds. The
  boundary's types gain serde derives, and `SoundRequest` and
  `MusicCommand` gain `Serialize`. Say so in the PR, with 2010's
  `Breaking:` line; the game's PR that moves the pin
  follows.
- **Size:** if the PR passes about 600 changed lines (the ADR and
  docs aside), the Harness part (`Harness::start`, `start_recording`,
  `from_session`, `play_to`, `play_all` and their tests) goes to a
  follow-up ticket (`write-ticket`), After MVP, blocked by this one,
  which 2519 and 2357 then also wait for. This PR keeps the record,
  `App`'s capture, the headless backend's playback and `Playback`,
  tested on `App` directly, and says so.

**Out (do not do):**
- `shuyi-app` and the web build writing records, a debug menu entry to
  save one, `shuyi run --session`, MCP, and golden session files: 2519.
- Recording in shipped builds, or by players; sending a record anywhere
  (privacy: Nick's decision, as parked 3047 asks for crash reports).
- A crash report that carries or names a session record: 3060.
  Replaying a crash's rules record (2520), rewind (2357), the live link
  (2521).
- A hash of the presented picture as a look check: rule 9; `frame-png`
  snapshots test looks. A follow-up if a game wants one.
- A compact binary form of the record: RON only. Write a ticket if real
  records grow too big to keep (measure first).
- Turning a record into a Harness script of keys and waits: lossy (frame
  times, music and the keyboard don't fit a script).
- Playing a record from another engine version or other content:
  refused (2111's stamp); reading older record versions: never.
- Changing the rules' records (`StageRecord`, 1119, 2706).
- Recording on the Playdate or GBA backends: `App` records on any
  backend, but writing a record is each backend's, and 2519 does only
  `shuyi-app`.

## Implementation steps

1. Read ADR-0024, ADR-0022 §4, ADR-0025, `crates/core/src/hash.rs`,
   `app.rs` (`frame`, `system_events`, `run`, `load`, `step`, `drain`),
   `headless.rs`, `harness.rs`, `docs/platform-backends.md`, and 2111's
   `shuyi_core::made`.
2. Add the serde derives and the dependencies, and the
   `tools/layers.ron` lines; check the `no-std` job and
   `cargo xtask check-layers` locally (`run-gates`).
3. Write `shuyi_platform::session`: `SessionRecord`, `FrameRecord`,
   `KeyboardFrame`, `SessionError`, `Stamps`, `FrameDifference` and
   `Played`, the text form and the version check, with round-trip
   tests.
4. Make `App` record and hash: `Start::made`, `Start::state_hash`,
   `App::record`, `session`, `session_mut`, `take_session`,
   `frame_hash` and `hash_frames`, the capture points, the cap and the
   `frame_after` rule.
5. Teach the headless backend to play a frame record (`from_session`,
   `push_frame`, the recorded music and keyboard), and write
   `Playback`.
6. Add `Harness::start`, `start_recording`, `session`, `from_session`,
   `play_to` and `play_all`, sharing the per-frame bookkeeping of
   `Harness::frame` (the audio log, the screen log, the quit frame).
7. Add the either-order parts whose partner is done (2220, 2321, 2356,
   3053), and the link's test (2521, done).
8. Write the property, divergence and boundary tests on the toy game
   (`shuyi_ui::toy`). Tests that need a store, a system keyboard or a
   clock that goes back record on `App` and a `HeadlessPlatform` built
   with `with_storage` or `with_keyboard`, calling `App::frame` with
   the times they choose.
9. Write the ADR and the docs section, and run the gates.

## Acceptance criteria

- [ ] For random scripts on the toy game (keys, pad buttons, waits of 0
      to 100 ms frames, a suspend and a resume), a session recorded with
      `Harness::start_recording` and played with `Harness::from_session`
      has no differing frame, ends on the same screens, and leaves the
      same storage (property test).
- [ ] A session recorded on `App` directly, with a clock that goes back
      twice, plays back through `Playback` with no differing frame.
- [ ] `SessionRecord::from_ron(&r.to_ron())` equals `r` for random
      records, music positions bit for bit (property test).
- [ ] `SessionRecord::from_ron` on any text gives a record or an error,
      never a panic (property test).
- [ ] Removing one key press from a recorded toy session at frame N
      makes `play_all` report its first difference at frame N.
- [ ] A session recorded over a store with saved settings
      (`HeadlessPlatform::with_storage`) plays back with them from the
      record alone, and the playback's writes stay in its own
      `MemoryStorage`.
- [ ] A session recorded with a system keyboard answer
      (`HeadlessPlatform::with_keyboard`) plays back with the same text
      in the game, on a playback platform given no `ScriptedKeyboard`.
- [ ] A screen that reads `Ctx::music_clock` plays back the same with a
      different `Speaker` load delay, since the recorded positions are
      used.
- [ ] Another `format_version`, and another `Made`, are refused with
      messages naming both; `Stamps::Ignore` plays and says so in
      `limits`; a start without a stamp plays and says so in `limits`.
- [ ] An hour of idle frames with music sounding (216 000 synthetic
      frames) writes under 16 MB of text.
- [ ] `App::record` after the first frame returns `false`; recording
      stops at `MAX_SESSION_FRAMES`, and on a `frame_after`, with
      `truncated` set.
- [ ] Each either-order partner that is done has its value recorded and
      played back, with a test.
- [ ] `shuyi-platform` and `shuyi-ui` still build for
      `thumbv7em-none-eabihf`, and `cargo xtask check-layers` passes.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the frame hash (phase, screens, quit, audio, the game's hash);
  idle frames written as `dt` alone; a frame without a hash compared
  with the last one written; the version refusal; the stamp check; the
  cap and the `frame_after` stop; the keyboard frame on its own
  (present, answer, open refused).
- Property: record then play gives no difference (above); the text
  form round-trips; `from_ron` never panics.
- Integration: the divergence frame; a clock that goes back; storage;
  the keyboard; the music clock; the size of an hour of idle frames.

## Completion notes

*(Filled in by the session that completes the ticket.)*
