---
id: "2269"
title: "FLAC and MP3 tracks streamed by the in-house sound backend: shuyi-sound adapters over claxon and nanomp3"
type: feature
milestone: "Engine: in-house platform"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2234"]
nick_input: none
completed:
---

# 2269 — FLAC and MP3 tracks streamed by the in-house sound backend

## Context

2203 added FLAC and MP3 to `shuyi-assets` as formats (features `flac`
and `mp3`, decoders `claxon` and `nanomp3`), and 2234 plays them on
the quad-snd path: a sound becomes a 16-bit WAV in memory
(`shuyi_assets::audio::decode_to_wav`, called by the default
`Backend::load_sound_file` for every backend), a native track too, and
the browser decodes a web track itself.

The `in-house-sound` backend (`crates/app/src/audio/in_house.rs`,
3031) gets FLAC and MP3 *sounds* that way, as WAV. Its *tracks* and
voice clips stream instead: a `Feeding` decodes ahead into a bounded
ring through `shuyi-sound`'s `Source` (2210). `Kind::of` knows only
`RIFF` and `OggS`, so a FLAC or MP3 track there is refused ("not a
readable OGG or WAV file"). 2234 split this part off to stay under the
PR size (its **Split**); 3031's handoff to 2234 asked that a streamed
track never become one whole WAV or `AudioBuffer`.

## Nick input

None.

## Scope

**In:**

- **Adapters in `shuyi-sound`,** each behind a feature of its own,
  as `wav` and `vorbis` are (`crates/sound/Cargo.toml`): `flac`
  (`claxon`, needs `std`) and `mp3` (`nanomp3`, `alloc`). The same
  versions and `default-features = false` as `shuyi-assets`, so the
  lock file gets no second copy; their lines in `tools/layers.ron`'s
  `outside` list of `shuyi-sound`.
  - `FlacSource<B: AsRef<[u8]>>` and `Mp3Source<B>`, `Source`s that
    decode as they are read, in bounded chunks (one FLAC block or MP3
    frame held at a time), FLAC's samples shifted to 16 bits and MP3's
    floats rounded and clamped, as `decode_to_wav` does. Each opens
    through `guard::guarded`, refuses what `Limits` refuses, and gives
    `total_frames` when the file says it (FLAC's `STREAMINFO`, MP3's
    LAME tag or a scan).
  - No `rewind_to` unless it is cheap: `looping` starts the decoder
    again, as for WAV.
- **In `in_house.rs`:** `Kind::{Flac, Mp3}`, sniffed as 2203's formats
  sniff (`fLaC`; an `ID3` tag or a frame sync), with `decode`,
  `Track::new` (its length from `shuyi_assets::audio::track_length_ms`)
  and `track_source` reading them. The app's `in-house-sound` feature
  turns on the two adapter features.
- **Tests:** each adapter against `decode_to_wav` on 2203's fixtures
  (FLAC equal, MP3 equal), broken and cut bytes, `Limits`; a property
  test that random bytes never panic; the in-house backend loads a
  FLAC and an MP3 track (as its WAV and OGG tests do).

**Out (do not do):**

- seeking inside a FLAC or MP3 file for a far loop region (an Ogg
  file's `new_at`): a region loops by decoding from the start again;
- the quad-snd path, which 2234 did;
- the rest of the replacement (3032, 3079), and the parity gates'
  baselines for these formats (2573's corpus stays WAV and OGG).

**Estimated size:** about 650 lines (code 300, tests 300, docs 50).

## Implementation steps

1. Read `crates/sound/src/{wav,vorbis,source,guard}.rs` and
   `crates/app/src/audio/in_house.rs` (`Kind`, `decode`, `Track`,
   `track_source`).
2. Write `FlacSource` and `Mp3Source` with their features and tests.
3. Join them in `in_house.rs`, and turn the features on from the app's
   `in-house-sound` feature.
4. `tools/layers.ron`, `cargo deny check`, the size table in
   `tools/sound-parity/footprint.ron` if it lists the adapters.
5. Docs: `docs/formats.md` (what plays a FLAC or MP3 track on the
   in-house backend), the crate docs of `shuyi-sound`.

## Acceptance criteria

- [ ] A FLAC and an MP3 track play on the `in-house-sound` backend,
      streamed through the ring (no whole-track buffer).
- [ ] Each adapter's samples equal `decode_to_wav`'s for 2203's
      fixtures.
- [ ] Random bytes never panic an adapter.
- [ ] `cargo deny check` and `cargo xtask check-layers` pass.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each adapter on the fixtures, broken and cut bytes, limits;
  `Kind::of` on each magic; `Track::new` on a FLAC and an MP3.
- Property: random bytes after each magic never panic.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
