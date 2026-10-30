---
id: "2364"
title: "Parked: Keyframed tracks, sound cues, markers and shot transitions in the cinematic file"
type: feature
milestone: "Engine: animation"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2109", "2339"]
nick_input: none
completed:
---

# 2364 — Keyframed tracks, sound cues, markers and shot transitions in the cinematic file

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker and Construct 3. All of
them but Bevy let a cut scene be written as data on a timeline:

- Godot's `AnimationPlayer` has property, method and audio tracks
  (<https://docs.godotengine.org/en/stable/tutorials/animation/animation_track_types.html>);
- Unity's Timeline has animation and audio tracks, markers and signals;
- Unreal's Sequencer has property and event tracks;
- GameMaker's Sequences have Moments (events at a frame), and Construct
  3 has Timelines;
- Bevy has no sequencer, but `bevy_animation` has curves on any
  property and animation events.

What Shuyi has today:

- `crates/assets/src/cinematic.rs`: a cinematic file is an `id`, a
  `length` (a music cue's, or seconds), optional shared `actions` and
  timed `shots`. Its docs say "Shots cut: there is no fade." Times are
  whole milliseconds once checked, so `CinematicDef` is `Eq`.
- `crates/ui/src/cinematic.rs`: `Timeline` says which shot a time falls
  in (`Moment`). `ease_in_out` and `Pan` are the only motion helpers.
  `CinematicPlayer::draw` takes one `brightness` for the whole frame
  (`GlyphBuffer::fade`). Every frame is drawn from the time alone.
- Everything else that moves in a shot is the game's Rust, in its
  `ShotPainter`. The toy test (`crates/ui/tests/it/cinematic.rs`) shows
  it: its `Sweep` shot works out its bar's column with `ease_in_out` in
  code.
- `crates/ui/src/audio.rs`: `AudioQueue::play_sound`, `play_music` and
  `stop_music`. Nothing calls them from a cinematic's time. Whoever owns
  the clock does it by hand.

What is planned:

- 2109: integer ease curves (`Ease`) and `Lerp` in `shuyi_core::ease`,
  and tweens. Its `Tween::finished_between` fires when `from < t <= to`.
- 2339: screen transition looks in `shuyi_ui::frame::transition`
  (`TransitionLook`: cut, fade through a colour, wipe, and dissolve in a
  Bayer order so the 1-bit frame gets it), with `progress` and
  `compose`. It leaves "shot enters in cinematics" to this ticket.
- 2340: frame shake and flash.
- 2365 (parked): beats and bars, with a `Tempo::beats_crossed` that
  counts beats in `from_ms < t <= to_ms`.
- 2115: timers in `shuyi_core::timer`.
- 2220: hot reload of any asset kind, cinematics included.
- 2104 (frame events on clips) and 2105 (`events_between` on bone
  clips), both parked: two more rules for "what happened between two
  times".
- 2721 and 2722 (visual novels, parked): stage directions paced by the
  reader's presses, and the VN's own transitions.

Nothing plans keyed values, timed sounds or markers in a cinematic.

Why it matters:

- **The game first.** game#1153's friction sweep measured the game's
  cinematic code at 1614 lines against an estimate of 1050 (1.5×). Its
  follow-ups (a folder loader, 2208; word wrap, 2209) took out loading
  and wrapping, not the shot code. The game doesn't ask for tracks, so
  this is parked. It has an open decision to rework its intro
  cinematic (game#0063). **Unparked early if** the game asks, through
  its `engine-change` skill, for a cinematic that shots alone can't
  make: a fade between shots, a sound at a time, or a value keyed over
  time.
- **Other 2D games.** A title sequence, an intro, a boss's entrance, a
  credits roll. With tracks, an agent writes these in RON,
  `cinematic::check_with` reports mistakes with the file, 2220 reloads
  them, and the values are integers, the same on every port.
- **The end goal** (2026-10-19, the most AI-friendly 2D engine): a data
  file an agent writes, and a check that answers it, cost less than
  painter code. Snake and a platformer need no cinematics, so this is
  not on the end goal's path.

The design, which an ADR records:

- **Tracks are named values the game's painter reads.** The engine
  moves nothing by itself. The game declares the tracks its shots read
  and their kinds, the way `Screen::actions` declares actions, so the
  check refuses a name nothing reads.
- **The checked cinematic stays free of floats and `Eq`.** Key values
  are integers (thousandths) or names.
- **One rule for marks between two times**, written once as
  `shuyi_core::timer::crossed`: a mark at `t` is crossed when
  `prev < t <= now`, the rule of 2109's `finished_between` and 2365's
  `beats_crossed`. Cues and markers use it here, and 2104's frame
  events and 2105's `events_between` when they are unparked.
- **One vocabulary of transitions.** A shot's `enter` is one of 2339's
  looks.

## Nick input

None.

## Scope

The work is in two parts. They don't use each other's code, and each
is about 400 changed lines with its tests, so together they are more
than one PR. The last paragraph of **In** says how to split.

**In:**

**Part 1: tracks.**

- **The file** (`shuyi_assets::cinematic`). New fields, all optional,
  so every file that reads today reads to an equal cinematic:

  ```ron
  (
      id: "intro",
      length: Seconds(12.0),
      tracks: {
          "title_y": [
              (at: 0.0, value: Number(-4.0)),
              (at: 2.0, value: Number(6.0), ease: Out(Back)), // an Ease
          ],
          "caption": [
              (at: 0.0, value: Name("intro.one")),
              (at: 8.0, value: Name("intro.two")),
          ],
      },
      cues: [
          (at: 0.0, play: Music("theme")),
          (at: 2.0, play: Sound("thud")),
      ],
      markers: [(at: 8.0, name: "scroll")],
      shots: [
          (at: 0.0, shot: Title),
          (at: 8.0, shot: Scroll, enter: Fade(0.5)),
      ],
  )
  ```

  (`cues`, `markers` and `enter` are part 2.)
  - `CinematicFile` gains `tracks: BTreeMap<String, Vec<Key>>`
    (`#[serde(default)]`): a map from a track's name to its keys, with
    times from the cinematic's start.
  - `Key { at: f32, value: Value, ease: Ease }`, `deny_unknown_fields`.
    `ease` is 2109's `shuyi_core::ease::Ease` (`#[serde(default)]`, so
    `Linear` when left out); it shapes the way into the key from the
    one before. shuyi-assets already depends on shuyi-core.
  - `Value`: `Number(f32)` and `Point(f32, f32)`, which ease, and
    `Name(String)`, which steps: a name holds until the next key. A
    name is what the painter makes of it: a text key, an atlas region,
    a palette colour (`Palette::lookup`).
  - `CinematicDef` gains `tracks: BTreeMap<String, Vec<KeyDef>>`, with
    `KeyDef { at_ms: u32, value: KeyValue, ease: Ease }` and
    `KeyValue { Number(i32), Point(i32, i32), Name(String) }`, numbers
    in thousandths. `CinematicDef` stays `Eq`.
- **The checks:** a new `cinematic::check_with(file, stem, raw, audio,
  tracks: &[TrackDecl], shots)`, with `TrackDecl { name: &'static str,
  kind: TrackKind }` and `TrackKind { Number, Point, Name }`. `check`
  keeps its signature and calls `check_with` with no tracks declared,
  so a file with tracks needs `check_with`. Each problem is one message
  attributed to the file, as `check`'s are, naming the track and the
  key (`track "title_y", key 2: ...`):
  - a track the game didn't declare, or a key of another kind than the
    declared one;
  - a track with no keys; an `at` that isn't a time (`cinematic::millis`);
    keys not in increasing time; a key not before the end;
  - an `ease` other than `Linear` on a `Name` key;
  - a `Number` or `Point` outside what thousandths in an `i32` hold.

  **Either order with 2237:** if 2237 is done, each problem above carries
  a code from 2237's table (`SH2008` for a track the game didn't
  declare, `SH2010` for the rest), and `check_in` puts it on its track's
  line. If not, nothing to do (2237 gives these problems their codes, by
  its own clause).
- **Sampling** (`shuyi_ui::cinematic`), pure in time:
  - `CinematicPlayer::tracked(&self, t_ms: u64) -> Tracked<'_>`: the
    tracks' values at `t_ms`, wrapped at the length as `Timeline` wraps.
  - `Tracked::number(name) -> Option<i32>` (thousandths),
    `point(name) -> Option<(i32, i32)>` and `name(name) -> Option<&str>`.
    A track that isn't there, or is of another kind, gives `None`.
  - Between two keys at `a` and `b`, the progress `(t - a) × ONE / (b -
    a)` (in `i64`) goes through the later key's `ease` (2109's
    `Easing::ease`), then `Lerp::mix` from the earlier value to the
    later. A `Name` holds the earlier key's name. Before its first key
    a track has the first key's value; at or after its last, the last
    key's.
  - `ShotPainter::frame_tracked(&self, ctx, shared, moment, tracks:
    &Tracked<'_>, screen) -> Self::Frame`: a new method whose default
    calls `frame`, so every painter today compiles unchanged.
    `CinematicPlayer::frame` and `CinematicPlayer::draw` call it.
  - `CinematicPlayer::of` keeps the def's tracks. `CinematicPlayer::new`
    makes a player with none.
- **The toy:** a second toy cinematic in
  `crates/ui/tests/it/cinematic.rs` whose painter reads a bar's column
  from a `Number` track instead of working it out in code, and a
  `Name` track. The existing toy file stays as it is.
- **The pack:** `CinematicFile`'s binary form changes, so `pack::VERSION`
  (`crates/assets/src/pack.rs`, 2 today) becomes the next number and
  the snapshot of
  `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`
  (`crates/assets/src/pack/tests.rs`) is accepted again (ADR-0023 §5).
- **Docs:** the module docs of both `cinematic` modules (the example
  gains a track), and in `docs/formats.md` the cinematic row of the
  kinds table and the cinematic line of "Checks after loading"
  (`check_with`).

**Part 2: cues, markers and shot enters.**

- **The file:**
  - `CinematicFile` gains `cues: Vec<Cue>` and `markers: Vec<Marker>`
    (both `#[serde(default)]`). `Cue { at: f32, play: Play }` with
    `Play { Sound(String), Music(String), StopMusic }`;
    `Marker { at: f32, name: String }`.
  - `TimedShot` gains `enter: Enter` (`#[serde(default)]`):
    `Cut` (the default), `Fade(f32)` or `Dissolve(f32)`, the length in
    seconds. shuyi-assets can't depend on shuyi-ui, so this is plain
    data whose variants have the names of 2339's looks.
  - `CinematicDef` gains `cues: Vec<CueDef>` and `markers:
    Vec<MarkerDef>`, and `ShotDef` gains `enter: EnterDef` (`Cut`,
    `Fade { ms: u16 }`, `Dissolve { ms: u16 }`), all in whole ms and
    still `Eq`.
- **The checks**, where `check` does its own (in `check_with` once part
  1 has made it), one message each with the file:
  - a sound or music cue that `audio` doesn't have;
  - a cue or marker whose `at` isn't a time, out of time order, or not
    before the end; two markers with one name;
  - an `enter` longer than its shot, or than 65.535 seconds (2339's
    looks hold `u16` ms).
- **The rule for marks between two times:**
  `shuyi_core::timer::crossed(marks: &[u32], prev: Option<u64>, now:
  u64, wrap: Option<u32>) -> Vec<usize>`, the indices of the marks
  crossed, in the order crossed. `no_std`, integers only.
  - A mark at `m` (and with a wrap, at every `m + k × wrap`) is crossed
    when `prev < m <= now`. `prev = None` is the first call, from
    before 0, so a mark at 0 is crossed then.
  - `now <= prev` crosses nothing. A backward jump fires nothing. A
    seek is the caller passing the time it sought as `prev`, so nothing
    at or before it fires.
  - With a wrap (a loop), a range longer than one loop crosses each
    mark once, not once per loop. `Some(0)` acts as `None`.
  - With no wrap (a clip played once), nothing past the last mark is
    crossed.
  - `crossed` goes into `crates/core/src/timer.rs`, with its tests in
    `crates/core/src/timer/tests.rs`, and the module docs name it.
  - **Either order with 2104 and 2105:** if either is done, it wrote
    `crossed` in `shuyi_core::timer` by this rule (their scopes write
    it when 2364 isn't done): use it, add the edge tests listed under
    **Tests required** that it lacks, and write neither `crossed` nor
    the module again. If neither is done, write it here, and they use
    it when they are worked.
- **Playing** (`shuyi_ui::cinematic`), pure in time:
  - `CinematicPlayer::of` also keeps the def's cues, markers and
    enters.
  - `CinematicPlayer::play_cues<V>(&self, prev_ms: Option<u64>, now_ms:
    u64, audio: &mut AudioQueue<V>)`: each cue `crossed` (wrapped at the
    cinematic's length) becomes `play_sound`, `play_music` or
    `stop_music`, in order. The screen that plays a cinematic keeps the
    time it drew last and passes it with `&mut ctx.audio`.
  - `CinematicPlayer::markers_crossed(&self, prev_ms: Option<u64>,
    now_ms: u64) -> Vec<&str>`, and `marker_ms(&self, name) ->
    Option<u32>`, for a skip that jumps to a marker.
  - `enter_look(enter: EnterDef, palette: &Palette) -> TransitionLook`:
    `Fade` is 2339's `TransitionLook::fade(ms, palette.engine(
    EngineColor::Background))`, the colour `draw` clears to today;
    `Dissolve` is `TransitionLook::dissolve(ms)`.
  - In `draw`, for the first `ms` of a shot with an `enter`: the shot
    before it is drawn at its last millisecond (1 ms before this shot
    starts) into a second `GlyphBuffer`, and the two are composed with
    2339's `compose` at `progress(into_ms, ms)`. Shot 0 enters from the
    cleared background, also when the cinematic loops. `brightness`
    still applies to the whole frame after.
- **The toy:** the second toy gains a cue, a marker, and a `Fade` and a
  `Dissolve` enter.
- **The pack:** `pack::VERSION` becomes the next number again, and the
  snapshot is accepted again.
- **Docs:** the module docs (the example gains a cue, a marker and an
  enter; "Shots cut: there is no fade" goes), and the cinematic row of
  `docs/formats.md`.

**For both parts:**

- **An ADR** (the `write-adr` skill), written with whichever part comes
  first and covering both: tracks as named values the painter reads,
  declared by the game; integers in the checked cinematic; the
  `crossed` rule, shared with 2109 and 2365; `enter` as 2339's looks.
- **Either order with 2365:** if 2365 is done, the `at` of keys, cues
  and markers is its `cinematic::At`, read and checked as
  `TimedShot::at` is, so they take beats too. If not, 2365 does it, as
  its scope says.
- **From 1134 (done):** its `shuyi-cinematic` skill
  (`skills/shuyi-cinematic/SKILL.md`) gets a recipe for what this PR
  adds: keyed tracks, sound cues, markers and shot enters (`check_with`,
  `Tracked`, `play_cues`, `markers_crossed`), with them in its worked
  example (`skills/shuyi-cinematic/example.ron`, which CI's `skills`
  job runs); `check-skills` passes.
- **What changes for the game:** the API grows, and nothing is renamed.
  Code that builds `CinematicDef`, `ShotDef`, `CinematicFile` or
  `TimedShot` as a struct literal must add the new fields; follow
  ADR-0039's rule for public types. The PR says which call
  sites change, and the game's PR that moves its pin follows it.
- **The split.** Do first the part that holds what the game asked for,
  if this ticket was unparked for a game's request; otherwise part 1.
  When that part is done, count the changed lines against `main`
  (`git diff --stat`, leaving out fixtures and snapshots). Above about
  250, stop there, and write the other part as its own ticket (the
  `write-ticket` skill), blocked by this one, in the same milestone,
  with that part's scope, criteria and tests from here. It is
  `status: todo` if Nick unparked this ticket whole, and parked with
  this ticket's note if he unparked it for one part. In the same PR,
  make the tickets that name 2364 for that part name the new ticket:
  2104, 2105, 2339, 2366 and 2722 for cues, markers, enters and
  `crossed`; 2109, 2122 and 2362 for tracks; 2365 for both (keys are
  part 1, cues and markers part 2).

**Out (do not do):**
- actors the engine moves by itself, and tracks bound to a game's fields
  by name: the painter reads values and moves things itself;
- tracks on a shot, with times from its start (a follow-up if a game
  asks: a file track keyed at the shot's times does the same);
- typed `Color` and `Glyph` kinds checked against the palette or a font
  (a follow-up if a game asks; a `Name` carries either today);
- `Wipe` as a shot's enter (a follow-up if a game asks: the file would
  need an edge), and slides and crossfades of two frames (2339 leaves
  them out);
- camera rigs and camera moves (1120, 2362);
- paths in space (2122): a `Point` track moves in a straight line
  between keys, eased in time;
- a beat clock (2365);
- volumes per sound group (2342), and variants and random volume per
  cue (2345); pitch (2345 leaves it to 3030's resampler); voice lines
  (2721);
- captions for cues (2366);
- a shot that plays a video file (2909 researches video and says
  whether the cinematic file gets one);
- shake and flash as cinematic data: a game calls 2340's from
  `markers_crossed`;
- a timeline editor (no ticket; the editor shell is 2507, parked), and
  a writer of the cinematic format (2218 leaves it out);
- nested timelines, and blending one cinematic into another;
- hot reload (2220 reloads any kind already);
- moving `Pan` onto 2109's ease (2136 does, tested equal; this ticket
  leaves `Pan` as it finds it, whichever lands first), and onto tracks
  (a follow-up if a game asks).

## Implementation steps

1. Read ADR-0004 §3, ADR-0007, ADR-0019, ADR-0023 §5, 2109's ADR and
   `shuyi_core::ease`, 2339's `shuyi_ui::frame::transition`, both
   `cinematic` modules and their tests, and the pack's pinned-form test.
   Check whether 2104, 2105 and 2365 are done. Read
   the game's cinematic shot code (its repository is public; game#1153's
   row names `ui/cinematic*`) and say in the PR, in the engine's words,
   which kinds of shot code the part being done could replace.
2. Write the ADR.
3. Part 1: `Key`, `Value`, `KeyDef`, `KeyValue`, `TrackDecl`,
   `TrackKind` and `check_with` with the track checks, a fixture for
   each problem, and the pack version.
4. Part 1: `Tracked`, `CinematicPlayer::tracked`,
   `ShotPainter::frame_tracked`, and the toy's tracks. Then the docs.
5. Count the lines and split if needed ("The split", above).
6. Part 2: `shuyi_core::timer::crossed`, with unit and property tests
   (if 2104 or 2105 wrote it, only the tests it lacks).
7. Part 2: `cues`, `markers` and `enter` in the file, with their checks,
   fixtures and the pack version.
8. Part 2: `play_cues`, `markers_crossed`, `marker_ms`, `enter_look` and
   the enter in `draw`; finish the toy; the docs.
9. The skill recipe and its worked example.
10. Run the gates.

## Acceptance criteria

Part 1:

- [ ] Every cinematic fixture that reads today reads to an equal
      `CinematicDef`, with no tracks.
- [ ] Each track check has a fixture, and its message names the file,
      the track and the key.
- [ ] A track sampled at a key's time gives that key's value; between
      two `Linear` keys the value stays between them; a `Name` holds
      until the next key; before the first key and after the last the
      value is the first's and the last's.
- [ ] The second toy's painter reads its column from a track, and its
      frames at chosen times equal values worked out by hand in the
      test.
- [ ] Every `ShotPainter` in the engine's tests compiles unchanged.

Part 2:

- [ ] Every cinematic fixture that reads today reads with no cues or
      markers, and every shot's `enter` `Cut`.
- [ ] Each cue, marker and enter check has a fixture, and its message
      names the file and the cue, marker or shot.
- [ ] `crossed(a, b)` then `crossed(b, c)` gives the same marks as
      `crossed(a, c)`, for any `a <= b <= c` with no wrap, and whenever
      `c - a` is shorter than the wrap with one.
- [ ] Playing the second toy at 16 ms steps for three loops asks for
      each cue exactly once per loop, in order. After a backward seek
      (the new time passed as `prev`), nothing is asked again.
- [ ] One look test reads the cells of a `Fade` and a `Dissolve` enter
      at progress 0, 500 and 1000 thousandths (`AGENTS.md` rule 9: only
      this test reads cells).

Both:

- [ ] `CinematicDef` is still `Eq`, and `shuyi-core` and `shuyi-ui`
      (with `shuyi-assets` under it) still build for
      `thumbv7em-none-eabihf`.
- [ ] `pack::VERSION` is the next number for each part that changed
      `CinematicFile`, and the pinned snapshot is accepted.
- [ ] The ADR is written.
- [ ] If the work was split, the follow-up ticket is in `tickets/open/`
      and the tickets that named 2364 for its part name it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each check; sampling each kind; the edges of a track (before
  the first key, at a key, after the last); `crossed` at its edges (a
  mark at `prev`, at `now`, at 0 on the first call and on a wrap,
  `prev == now`, `now < prev`, a range longer than a loop, no wrap);
  `play_cues` giving each `AudioRequest`; `marker_ms`; `enter_look`.
- Property: `crossed` split at any point; sampling is pure and stays
  between `Linear` keys; random bytes and random edits never panic
  `CinematicRon` (the existing test in
  `crates/assets/src/format/builtin_tests.rs`, with a second film
  fixture that has the new fields).
- Snapshot / integration: the second toy's audio requests and markers
  over three loops, each with its time, as a text snapshot; the look
  test of the enters; the pinned pack bytes.

## Completion notes

*(Filled in by the session that completes the ticket.)*
