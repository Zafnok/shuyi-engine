---
id: "2397"
title: "Music intro-then-loop regions with checked metadata and backend mapping"
type: feature
milestone: "Engine: audio"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2902", "2573", "3031", "2013"]
nick_input: none
completed: 2026-11-22
---

# 2397 — Music intro-then-loop regions

## Context

Whole-file looping cannot play an intro once and then repeat a selected
region. 2902 assigns this contract to the chosen hybrid sound path;
[ADR-0066](../../docs/adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
requires automated parity before migration. 3030 supplies sample-level
mixing, 3031 workers/output; 3079 alone removes the old backend.

## Nick input

None.

## Scope

**In:**
- Optional `MusicCue.loop_start_ms`/`loop_end_ms`; both present or neither,
  checked `0 <= start < end <= length_ms`. Update RON/serde/validation,
  constructor/defaults, pack version/pinned binary fixtures and public API
  migration notes as needed. New growable public types follow ADR-0039.
- **From 2210 (done):** `shuyi_sound::vorbis::VorbisSource` has `comment(name)`,
  `comments()`, `position()`, `end_granule()` and `seek_page(frame)`. `seek_page`
  lands on the first page ending at or past the frame, and the packet after it
  gives nothing, so `position()` is `None` until a page ends. An exact boundary
  seeks some way before it, reads until `position()` is known and still before
  the boundary, and discards up to it: a test proves the samples from there equal
  a whole decode's. `lewton`'s own running count between pages isn't exact, so
  don't read it directly.
- Read Ogg `LOOPSTART`/`LOOPLENGTH` sample comments with `VorbisSource::comment`,
  checked addition/unit conversion and known sample rate. Manifest values
  override file defaults; errors name file/field. Do not silently clamp
  invalid/overflowing metadata or make a codec-specific manifest.
- `MusicClock` plays the intro once then wraps end to start, including
  pause/crossfade and elapsed spans across several loops. Preserve length
  meaning and document the half-open region/sample conversion policy.
- A provided shared-backend loop-region method, `Unsupported` by default;
  whole-file-only backends warn once and use their truthful fallback.
  Wire exact worker prefetch/pre-roll/trim and mixer boundaries, not a
  main-frame seek at the audible endpoint. Both crossfade streams can loop.
- ~~Playdate SDK file-player loop ranges map with tested units. Baked PCM
  metadata retains the range; no desktop decoder on handheld is implied.~~
  Split off to 3325 (see the completion notes): the Playdate keeps the
  provided default, and plays a region whole, truthfully.
- **Either order with 3027:** if 3027 is done, map a GBA tracker restart
  point/order-row to this region with checked conversion; reject/report
  unsupported ranges and warn once rather than rounding arbitrary ms.
  If not, nothing to do (3027 implements its mapping).
- **Either order with 2365:** if 2365 is done, wrap `beats_since` and
  `bars_since` through the region and count the loop point once; if not,
  nothing to do (2365 adds that wrapping). Keep the reciprocal clause.
- **From 2573 (done):** `future_region` in `tools/sound-parity/scenarios.ron`
  and `corpus/intro_loop.ogg` (`LOOPSTART` and `LOOPLENGTH` set) wait for this
  ticket: give the scenario steps and name the backend that passes it.
- **From 3030 (done):** `shuyi_sound::mix::stream::LoopedRegion` is the worker's
  way round a region: exact trim, a restart closure and pre-roll, sample-exact in
  `mixer:region-from-a-worker`. It pre-rolls by reading, so a late start in a long
  file is slow: `VorbisSource::seek_page`, above, is this ticket's to wire in through
  the closure if that shows. `Region` is half-open, in frames.
- Extend 2573's actual output and independent oracle tests: expected
  `intro + region + region`, exact sample indices, decoder-page boundaries,
  no added silence/duplicates, queue wrap and producer stalls. Correctness,
  CPU/speed and bounded-memory gates pass before selecting this behavior.
  3017 is still open and reads implemented audio features when it lands.
- Update platform contract, format docs and book; test asset registry/pack
  round trips. **From 2084 (done):** `MusicCue` and `MusicClock` are
  `#[non_exhaustive]` with `MusicCue::new` and `MusicClock::new`, and
  `MusicCommand` is `#[non_exhaustive]`, so the loop fields and a new
  command are additive: no `Breaking:` line is planned for them. Any
  other API break carries Breaking/migration notes and a game pin handoff.
  The new fields stay optional, so a game's files load and play on quad-snd too,
  whole-file with the warning above: a game must never come to need the new
  backend ([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).

**Estimated size:** about 1,600 lines (code 1,100, tests 350, docs 150),
written as the work was done (it had none, and was listed in
`tools/unestimated-tickets.txt`).

**Split:** the Playdate's mapping went to 3325 at about 1,400 lines.

**Out (do not do):** arbitrary seeking UI, stems/sample scheduling/latency
calibration, tempo reading, beat-clock implementation (2365), new GBA
backend (3027), deleting quad-snd (3079).

## Implementation steps

1. Extend cue/metadata validation and serialized forms, then backend method.
2. Implement clock wrap and stream/mixer exact boundaries; map SDK ranges.
3. Settle either-order mappings, extend automated gates and document migrations.

## Acceptance criteria

- [x] Metadata/defaults/manifest precedence and invalid/overflow/range cases are checked.
- [x] Intro plays once; at least two streamed loop boundaries have exact expected samples and bounded PCM storage.
- [x] Clock and, when present, beat/bar queries wrap correctly through pause/crossfade (2365 isn't done: no beat/bar queries yet).
- [x] Unsupported fallback warns once; Playdate/GBA mapping is checked and truthful (the Playdate's is 3325's, and plays whole until then; 3027 isn't done).
- [ ] 2573's correctness/speed gates pass without removing the reference backend (CI's `sound-parity` jobs: the local `mixer` and `decoders` cells pass; the captures are CI's).
- [x] A manifest with a region loads and plays whole-file on quad-snd, with one warning; a test says so.
- [x] All this PR adds is in `tools/sound-parity/footprint.ron`, and CI's `sound-give-up` job passes: giving up stays one command ([ADR-0068](../../docs/adr/0068-the-sound-replacement-stays-reversible-at-every-stage.md)).
- [x] All gates in the `run-gates` skill pass.

## Tests required

Unit/property: ranges, metadata units/overflow, multi-wrap clocks, independent
sample oracle and malformed comments. Integration: real native/browser
emitted loops, two streams/fades, pause/rate, ring/page boundaries,
underruns, SDK/tracker mapping, provided Unsupported warning, pack round trip.

## Completion notes

**What was done.**

- `shuyi-assets`: `MusicCue::loop_start_ms` and `loop_end_ms` (serde
  default `None`), `MusicCue::with_loop` and `loop_region`, and `LoopRegion`
  (`#[non_exhaustive]`, `LoopRegion::new`, `frames`: a millisecond is frame
  `ms × rate / 1000` rounded down). The manifest check refuses one end alone,
  an end not after its start, an end past `length_ms`, and a region on a
  track played once, each as written. Pack version 20.
- `shuyi-sound`: `tags::loop_region`, the checked reading of `LOOPSTART` and
  `LOOPLENGTH` (both or neither, whole numbers, a length of a frame at least,
  a sum that fits, an end within the sound), with `Error::LoopTag` naming the
  tag; `VorbisSource::loop_region`, `seek_exact`, `skip` and `new_at`;
  `LoopedRegion::landing`, a way round whose restart lands on the region's
  start.
- `shuyi-platform`: the provided `Backend::set_loop_region(sound, region) ->
  Result<Looping, Unsupported>` and `Looping`. The player calls it once a
  looped track loads, warns once of a backend that can't (only for a cue that
  names a region), warns of a file's tags that don't read naming the file,
  and reports `music_playing` round the region.
- `shuyi-app`'s `in-house-sound` backend reads a track's tags as it loads,
  plays the cue's region (in the file's frames) or the file's, and goes back
  by an exact seek round a region that starts past `SEEK_LEAD`.
- Parity: `future_region` has its steps and holds on `in-house` and
  `in-house-web`, from the file's own tags (its cue names no region); a test
  holds its checks to the right sound and to quad-snd's whole-file loop.
  `mixer:region-from-the-file` holds the Ogg file's region through a worker,
  a ring that wraps over and over and a producer that stalls every other
  block: the intro and the region twice, every frame the whole decode's.
- [ADR-0136](../../docs/adr/0136-a-tracks-loop-region-is-the-cues-or-its-files-played-by-the-backend-and-the-clock-told-where-it-is.md)
  records the contract, extending ADR-0063 and ADR-0054.

**Deviations.**

- *The clock* (ADR-0136 §3): the ticket put the wrap in `MusicClock`. It is in
  the player instead, which reports a region's track by where it is, and
  `MusicClock::from_elapsed` wraps at the length as before. A wrap in `ui`
  from the cue's region would be wrong where the backend plays the whole
  track (quad-snd), and blind to a region only the file names, which the
  manifest at run time can't see. The headless `Speaker` reports whole
  tracks; a test of a region's clock goes through the player.
- *The file's tags at check time*: they are read where the file is opened,
  the backend, and their problems are warnings as the track loads, naming
  the file and the tag. The manifest's check can't read them: `shuyi-assets`
  doesn't depend on `shuyi-sound`, and the music folder can't be seen where
  the manifest is read at run time.
- *The exact seek*: the ticket left `seek_page` "if that shows". It shows by
  arithmetic: `LoopedRegion::back` read the whole intro again inside one
  `read`, on the worker (and on the web inside the page's audio fill), while
  the ring holds 4,096 frames, under a tenth of a second. An intro whose
  decode takes longer runs the ring dry at every way round.
  `VorbisSource::new_at` seeks where it can, and in a file whose pages are
  longer than the way in (the corpus's own, at its low bitrate) reads on from
  the start.
- *The Playdate* is split to 3325 (the size). The GBA's mapping is 3027's
  (not done), and the beat clock's wrap is 2365's (not done); their clauses
  now say what 2397 left them.
- *No speed cell of its own*: the way round is the worker's, timed by the
  captured runs' `perf` and `old-new` cells as before; `mixer:memory` still
  holds the ring to `max_held_frames`.
- *The ticket was 22 days old*: it was checked against the code. Every type it
  names exists as it says (`VorbisSource::comment`, `seek_page`,
  `LoopedRegion`, `Region`, `MusicCue::new`, `MusicClock::new`); it had no
  size estimate, written above.

**The footprint.** Every file this adds or changes that names the
replacement is one `footprint.ron` owns whole (`crates/sound`,
`crates/app/src/audio/in_house*`, `crates/xtask/src/sound_parity*`,
`tools/sound-parity`), and `giving_up_covers_all_the_chain_added` passes. The
cue's fields, the check, the provided method and the clock are the feature,
which a give-up keeps: quad-snd plays a region whole with its one warning.

Size: 1,595 counted lines, the exact seek can't land after the region: without it the in-house backend runs its ring dry at each way round of any intro longer than a fraction of a second's decode, so the region would ship broken on ordinary music; the Playdate went to 3325.

Breaking: shuyi-assets: `pack::VERSION` and `formats.ron` "audio manifest: audio-manifest": a music cue's binary form has `loop_start_ms` and `loop_end_ms`, and a baked pack is of version 20; a pack baked before is refused, so a game bakes again with `shuyi bake`.

```migration
since: 0.1.0
what: The pack format is version 20, its music cues with a loop region; a pack of version 19 is refused.
old: "pack::VERSION == 19"
new: "pack::VERSION == 20"
before: |
  cargo shuyi bake   # with the engine before 2397
after: |
  cargo shuyi bake   # again, with this engine
```

Behaviour: shuyi-platform: `Audio::music_playing` (and `Player::music_playing`) reports a track that goes round a loop region by where it is in the track, back by the region's length each time round, no longer the seconds since it started; a track played whole is as before.

Behaviour: shuyi-app: on the `in-house-sound` backend, a looped track whose Ogg file has `LOOPSTART` and `LOOPLENGTH` comments goes round that region, where it went round the whole file; quad-snd, the default, is unchanged.

Behaviour: shuyi-assets: the audio manifest's check refuses a music cue with one of `loop_start_ms` and `loop_end_ms`, an end not after its start, an end past `length_ms`, or a region on a track with `looped: false`.

Book: the chapter on games and screens says what a loop region is and where it comes from (`site/content/learn/book/games-and-screens.md`).

Game: nothing to adopt, the game builds its cues with `MusicCue::new` (the new fields are additive), its manifest names no region, and it plays on quad-snd, which loops whole as before; it bakes its pack in a test, which reads version 20 as it bakes it. `gh search code --repo Zafnok/visions-of-shuyi` for `MusicCue`, `music_playing`, `MusicClock`, `pack::VERSION`, `loop_start_ms` and `impl Backend for` found the pins log, the music clock's ADR (game ADR-0037, which this keeps) and its uses of the clock, nothing that names a region or implements the backend trait.

Follow-up: 3325 (the Playdate's file-player loop range and baked tags).

**Nick:** nothing changes in the game. A game can now give a track an intro
that plays once before the loop; the new sound backend plays it, and the
current one plays the whole track as before.
