---
id: "2280"
title: "Sound file lengths, measured from their headers: sound_length_ms, a cue's lengths_ms and the Sound kind's length_ms"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2345"]
nick_input: none
completed: 2026-11-26
---

# 2280 — Sound file lengths, measured from their headers

## Context

2378 (cue rules, part two: copy caps, priority and a voice limit) passed
its own size valve of about 600 changed lines, so its lengths go first,
as its **Size** says. ADR-0139 §4 decides that a copy of a sound ends at
its start plus its file's length, read from the file's header when the
manifest is checked. 2378's `Cues` counts the copies sounding from these
lengths; this ticket only measures and keeps them.

What the code has (2026-11-26):

- The `Sound` kind (`crates/assets/src/audio/formats.rs`) is `channels`
  and `sample_rate`, `#[non_exhaustive]` (2095), built with
  `Sound::mono` and `Sound::stereo`.
- `ogg_length_ms` (`crates/assets/src/audio.rs`) measures a whole OGG
  file; `track_length_ms` (`audio/decode.rs`) a FLAC or MP3 file
  (2234); `wav_format` reads a WAV's `fmt ` chunk, and nothing reads its
  `data` chunk. The tests' `wav` helper writes no `data` chunk.
- A `.sfx.ron` sound (2225) renders to `n` samples at `SAMPLE_RATE`, `n`
  from its `SfxDef` (`sfx::render`).
- `SoundCue` (`#[non_exhaustive]`, built with the `const`
  `SoundCue::new`) has no length; `pack::VERSION` is 22.

## Nick input

None.

## Scope

**In** (`crates/assets`):
- `pub fn sound_length_ms(name: &str, bytes: &[u8]) -> Option<u32>`, to
  the nearest millisecond, in checked `u64` integers:
  - a WAV's (by its bytes, whatever its name) is its `data` chunk's size
    over the block align of its `fmt ` chunk, times 1 000, over its
    sample rate; `None` with no `data` chunk, or a block align or a rate
    of 0;
  - an OGG's is `ogg_length_ms`, a `.flac` or `.mp3` file's
    `track_length_ms`;
  - a `.sfx.ron`'s, with the `sfx` feature, the length it renders to
    (`n × 1000 / SAMPLE_RATE`, to the nearest), whether the bytes are its
    RON text or the WAV a game's hook rendered (`audio::playable`).
- `SoundCue::lengths_ms: Vec<Option<u32>>`, one per file, filled by
  `Check::sound` (`None` for a file `sound_left_out` leaves out, or one
  not found), `#[serde(default)]` and never written in the RON.
  `SoundCue::new` leaves it empty and stays `const`. A list shorter than
  `files` means the rest are unknown: `SoundCue::length_ms(file)` reads
  it so.
- `AudioManifest::measure_sounds(&mut self, read: &dyn Fn(&str) ->
  Bytes)` fills the lengths of a manifest built in code.
- The `Sound` kind gains `length_ms: Option<u32>` (`#[serde(default)]`,
  additive: `#[non_exhaustive]`), from `sound_length_ms`; `Sound::mono`
  and `Sound::stereo` start it at `None`, and `Sound::with_length_ms`
  sets it. `sfx::SfxRon` fills it.
- The manifest's binary form changes: `pack::VERSION` becomes 23, and the
  pinned snapshot in `crates/assets/src/pack/tests.rs` is accepted.
- Docs: `docs/formats.md` (lengths are measured, never written).

**Out (do not do):**
- everything else of 2378: the copy cap, priority, the voice limit,
  `CopyId`, `StopCopy`, and the check that a capped cue's file has a
  length (2378);
- a length for a sound a game adds at run time (`SoundRequest::Add`): its
  samples say it.

**Estimated size:** about 600 lines (code 220, tests 320, docs 60).

## Implementation steps

1. `wav_length_ms` beside `wav_format`, and `sound_length_ms` over it,
   `ogg_length_ms`, `track_length_ms` and `sfx`.
2. `SoundCue::lengths_ms` and `SoundCue::length_ms`, filled in
   `Check::sound`; `AudioManifest::measure_sounds`.
3. The `Sound` kind's `length_ms` in `Wav`, `OggVorbis`, `Flac`, `Mp3`
   and `SfxRon`; fix its literal in `format/builtin_tests.rs`.
4. `pack::VERSION` and its snapshot; `docs/formats.md`.

## Acceptance criteria

- [x] `sound_length_ms` gives the length to the millisecond of mono and
      stereo WAV bytes with a `data` chunk and of OGG bytes, built in
      the tests; it gives `None` for a WAV with no `data` chunk and for
      a block align or a rate of 0, and the length of a `.sfx.ron`
      (text or rendered) and of a FLAC file.
- [x] `Check::sound` fills `lengths_ms` (with `None` for a file left
      out), the `Sound` kind's `length_ms` agrees, and every manifest in
      the repository's tests loads unchanged.
- [x] `measure_sounds` fills a manifest built in code.
- [x] `pack::VERSION` is 23, with the pack snapshot updated.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `sound_length_ms` on built WAV (mono, stereo, no `data`, block
  align 0, rate 0), OGG, FLAC and `.sfx.ron` bytes; `lengths_ms` from
  `from_source`, a file left out; `measure_sounds`; the `Sound` kind's
  `length_ms` from each format.
- Snapshot: the pack's pinned binary forms.

## Completion notes

Written and done in one PR: 2378 (cue rules, part two) passed its own size
valve of about 600 lines (its estimate is now about 1,400 without the
lengths), so, as its **Size** said, the lengths went first as this ticket.
2378 is now blocked by it, carries an `**Estimated size:**` and a
`**Split:**` line, and is off `tools/unestimated-tickets.txt`; its scope
says what this ticket built (*From 2280 (done)*).

What was done (`crates/assets`):

- `audio::sound_length_ms(name, bytes)`: a WAV's (by its bytes, whatever
  its name) from its `data` chunk over the block align and rate of its
  `fmt ` chunk, counting only what the file holds when the chunk says
  more; an OGG's by `ogg_length_ms`; a FLAC's or MP3's by
  `track_length_ms`; a `.sfx.ron`'s, with the `sfx` feature, the samples
  `sfx::render` makes over `SAMPLE_RATE`. All in checked integers, to the
  nearest millisecond (`samples_ms`). `wav_format` now reads its chunk
  through the same `wav_chunk` walk.
- `SoundCue::lengths_ms` (one per file, `#[serde(default)]`, never read
  from the RON) and `SoundCue::length_ms(file)`; `Check::sound` fills it,
  `None` for a file left out, not found or of a bad extension.
  `AudioManifest::measure_sounds(read)` fills a manifest built in code.
- The `Sound` kind's `length_ms` (`#[serde(default)]`) and
  `Sound::with_length_ms`; `Wav`, `OggVorbis`, `Flac`, `Mp3` and
  `sfx::SfxRon` fill it.
- `pack::VERSION` is 23; the pack's snapshot and the golden table
  `crates/assets/tests/golden/formats.ron` are blessed.
- Docs: `docs/formats.md`, the audio manifest's **A sound's length**.

Deviations:

- The ticket's "a block align of 0" case also covers a rate of 0 and a
  WAV with no `fmt ` chunk, all `None`.
- `Mp3`, read for a sound, now walks every frame (as `Mp3Track` does) to
  give its length; it read the first frame only.

Follow-up tickets: none; the rest is 2378.

Searched the game for what this touches (`gh search code --repo
Zafnok/visions-of-shuyi` for `SoundCue {`, `Sound {`, `sample_rate`,
`lengths_ms` and `pack::VERSION`): it builds no `SoundCue` or `audio::Sound`
by hand (its `Sound {` hits are `SoundRequest`s and its own types), and
names `pack::VERSION` only in `docs/engine/pins.md`.

Breaking: shuyi-assets: `pack::VERSION` and `formats.ron` "audio manifest: audio-manifest": a sound cue's binary form holds its files' lengths, and a baked pack is of version 23; a pack baked before is refused, so a game bakes again with `shuyi bake`.

```migration
since: 0.1.0
what: The pack format is version 23, its sound cues with their files' lengths; a pack of version 22 is refused.
old: "pack::VERSION == 22"
new: "pack::VERSION == 23"
before: |
  cargo shuyi bake   # with the engine before 2280
after: |
  cargo shuyi bake   # again, with this engine
```

Breaking: shuyi-assets: `formats.ron` "sound: wav", "sound: ogg-vorbis", "sound: flac", "sound: mp3" and "sound: sfx": a decoded `Sound` holds its `length_ms`, so its state hash moves; a game's own golden row of a sound is blessed again.

```migration
since: 0.1.0
what: A decoded Sound has its length_ms, so a golden row of one moves.
old: "Sound { channels, sample_rate }"
new: "Sound { channels, sample_rate, length_ms }"
before: |
  cargo test   # a golden row of a decoded sound fails
after: |
  cargo xtask golden --bless   # or the game's own way of blessing its table
```

Behaviour: shuyi-assets: a decoded `Sound` says how long it plays (`length_ms`), so a test that compares one to `Sound::mono(rate)` or `Sound::stereo(rate)` adds `.with_length_ms(...)`; `SoundCue`s from `from_source` hold `lengths_ms`, so one compared to `SoundCue::new(...)` differs by it.

Behaviour: shuyi-assets: the `Mp3` format, reading a sound, walks every frame for its length, as `Mp3Track` does: slower on a long file, the same answer otherwise.

Book: none, the lengths are measured by the engine and nothing a game writes changes; `docs/formats.md` says what they are.

Skills: none, no field a game writes in the manifest changed: `lengths_ms` is measured, never written, and the skill pack's audio pages name the fields a game writes.
