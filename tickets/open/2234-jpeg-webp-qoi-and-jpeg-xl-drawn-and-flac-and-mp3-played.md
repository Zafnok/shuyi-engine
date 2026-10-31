---
id: "2234"
title: "JPEG, WebP, QOI and JPEG XL pictures drawn and FLAC and MP3 sounds played: shuyi-app and frame-png decode them through the registry, and the audio manifest takes them"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2203"]
nick_input: none
completed:
---

# 2234 — JPEG, WebP, QOI and JPEG XL drawn, FLAC and MP3 played

## Context

2203 (needed first) adds JPEG, WebP, QOI, JPEG XL, FLAC and MP3 to
`shuyi-assets` as formats, each behind its own cargo feature
([ADR-0007](../../docs/adr/0007-formats-are-plugins-standard-files-always-optimized-ones-optional.md)
§3). An image codec is a `Format` of `image::Image` (its RGBA pixels)
with a size-only twin of `image::ImageInfo`. An audio codec is a
`Format` of `audio::Sound` (the header) and of `audio::MusicTrack`
(the header and the length). Its Out leaves drawing and playing them
to this ticket.

Nothing reads them at run time today:

- **Pictures in `shuyi-app`.** `Renderer::new`
  (`crates/app/src/render.rs`) uploads each picture with `upload`, and
  `recoloured` makes the recoloured copies. Both decode with
  macroquad's `Image::from_file_with_format`, as
  `ImageFormat::Png`. A JPEG fails, is logged, and every sprite of it
  is drawn as a magenta rectangle (`MISSING_COLOR`).
- **Pictures in `frame-png`.** `Painter::new`
  (`crates/cli/src/frame_png.rs`) decodes the atlas and each picture
  with `frame_png::decode_image`, which calls `codec::decode_png`, into
  a `shuyi_ui::frame::raster::Image` (2332 moved the painter there). A
  JPEG doesn't decode, and its sprites are drawn in
  `raster::MISSING_COLOR`: the same magenta.
- **Sounds.** `Audio::load` (`crates/app/src/audio.rs`) hands each
  sound file's bytes to `Backend::load_sound`, macroquad's
  `load_sound_from_bytes`. On native, quad-snd 0.2.8 decodes with
  `audrey`, built for WAV and Ogg Vorbis only, and `unwrap`s, so a FLAC
  sound would panic the game.
- **Music.** On native, `Music::load`
  (`crates/app/src/audio/native_music.rs`) decodes on a worker thread
  inside `catch_unwind`, and takes the length from `ogg_length_ms`. On
  the web, `load_music` hands the file's bytes to the browser:
  quad-snd's `audio.js` calls Web Audio's `decodeAudioData`.
- **The manifest.** `Check::sound` and `Check::music`
  (`crates/assets/src/audio.rs`) refuse every name but `.ogg` and
  `.wav` for a sound and `.ogg` for a track, and `format_problem` picks
  its check by the extension. So a game can't even name a FLAC file.

ADR-0007 §3 says a game never has to convert a common file before
using it. A format the game can load but not see or hear breaks that.

**The shape: one switch per format.** `docs/modules.md` (*Format
plugins*) says a game turns a format on as a feature of `shuyi-assets`,
in its own `Cargo.toml`. Cargo turns a feature on for every crate of
the build that uses `shuyi-assets`, so code in `shuyi-assets` that is
`#[cfg]`'d on it follows the game's choice. `shuyi-app` and
`shuyi-cli` then need no feature of their own:

- pictures go through the registry: `Formats::standard()` holds the
  `Image` formats whose features are on;
- sounds become a WAV in memory, which every backend reads, with a
  function in `shuyi-assets` that is always compiled;
- on the web a music track goes to the browser as it is:
  `decodeAudioData` reads MP3, and FLAC in current browsers, off the
  main thread. Our decoder would hold up a frame for a whole track.

## Nick input

None.

## Scope

**In:**

- **Before you start: what 2203 shipped.** Read its completion notes:
  which formats it shipped, their type and feature names (`flac` and
  `mp3` below stand for its names), where its fixtures are, and which
  decoder crates it added. A format 2203 left out (MP3, say) is left
  out here too. Each of its `Image`, `Sound` and `MusicTrack` formats
  must be registered by `Formats::register_standard` under its feature,
  as `image::Png` is under `png` (`crates/assets/src/format.rs`). If
  2203 left one out of it, register it there and update the registry's
  test.

- **Pictures in `shuyi-app`** (`crates/app/src/render.rs`):
  - `Renderer` keeps a `Formats::standard()`, made in `Renderer::new`,
    whose signature stays.
  - A private `pixels(formats, path, bytes) -> Result<(u16, u16,
    Vec<u8>), String>`. Bytes that start with `image::PNG_SIGNATURE`
    are decoded by macroquad as today, so a PNG is drawn exactly as
    before. Any other bytes go through
    `formats.decode::<image::Image>(&AssetPath::new(path), bytes,
    &DecodeCx::new())`. Its error is the registry's message, which
    names the formats it has. The sides fit in `u16`
    (`image::MAX_SIDE` is 4096).
  - `Renderer::new` uploads each picture from `pixels` with the
    nearest filter, and `recoloured` recolours the pixels `pixels`
    gives. A file no format reads is logged and drawn in
    `MISSING_COLOR`, as today.
  - **Either order with 2222:** if 2222 is done, `render.rs` already
    uploads atlas pages from their pixels: upload a picture with that
    function. If not, write `upload_rgba(width, height, rgba) ->
    Texture2D` there (`Texture2D::from_rgba8`, nearest filter) and use
    it, and 2222 uploads its pages with it.
- **Pictures in `frame-png`,** in `frame_png::decode_image`
  (`crates/cli/src/frame_png.rs`), which `Painter::new` calls for each
  picture: bytes that start with `image::PNG_SIGNATURE` are decoded as
  a PNG, and any other file through the same registry call, into
  `raster::Image { width, height, rgba }`. `Painter::new` and
  `decode_image` keep their signatures. So `frame-png` keeps showing
  what players see.
  - **Either order with 2565:** if 2565 is done, `codec::decode_png` is
    gone and `decode_image` reads a PNG with
    `shuyi_assets::bitmap::convert::png_pixels`, the decoder
    `image::Png` calls: keep that for a PNG and send other bytes to the
    registry, so the new decoders sit beside `png_pixels` in
    `shuyi-assets` and `frame-png` reaches them through `decode_image`;
    if not, a PNG goes to `codec::decode_png` as today, and 2565 moves
    that branch to `png_pixels`.
  - **Either order with 2391:** if 2391 is done, the raster's `Image` is
    `shuyi_assets::image::Image`, the type the registry decodes: hand
    the decoded picture to the painter as it is, without copying its
    fields; if not, copy its fields into `raster::Image { width, height,
    rgba }` as above, and 2391 drops the copy.

- **Sounds in `shuyi_assets::audio`** (`crates/assets/src/audio.rs`).
  The functions are always compiled, also without `std`; only their
  FLAC and MP3 arms are under the features:
  - `decode_to_wav(name: &str, bytes: &[u8]) -> Option<Result<Vec<u8>,
    String>>`:
    - `None`, nothing to decode: a name that doesn't end in `.flac` or
      `.mp3`, or bytes that already are a WAV (`RIFF`, then `WAVE`: a
      game's hook decoded them);
    - `Some(Ok(wav))`: a 16-bit PCM WAV with the file's channels and
      rate. FLAC samples are shifted to 16 bits; an MP3 decoder's
      floats are rounded and clamped;
    - `Some(Err(why))`: bytes that don't decode, a WAV that would pass
      `MAX_DECODED_BYTES` (decoding stops as soon as it would), or a
      format whose feature is off: "a .flac file needs the flac
      feature of shuyi-assets".
  - `MAX_DECODED_BYTES`: 256 MiB, about 25 minutes of stereo at
    44.1 kHz. The bytes may come from mods (ADR-0007 §5).
  - `pub(crate) fn pcm16_wav(channels, sample_rate, samples: &[i16])
    -> Vec<u8>`: the 44-byte header, then the samples.
  - `track_length_ms(name, bytes) -> Option<u32>`: `ogg_length_ms`
    for `.ogg`; for `.flac` and `.mp3`, the `length_ms` of 2203's
    `MusicTrack` format of it (`None` with its feature off).
  - **Either order with 2378** (cue rules, part two): if 2378 is done,
    `sound_length_ms` gives a `.flac` or `.mp3` file the length of
    `track_length_ms`, so a FLAC sound of a cue with `max_copies` has
    a length. If not, nothing to do (2378 does it).
  - **The decoders** are the crates 2203 added, under the same
    features. If 2203 read a header by hand and added no decoder, add
    one under that feature: check its license from its `Cargo.toml`
    and repository on the day (ADR-0002 §8; `symphonia` is MPL-2.0
    and denied, ADR-0004 §7), use `default-features = false`, check
    that it builds for `wasm32-unknown-unknown` without
    `wasm-bindgen`, add its line to `shuyi-assets`' `outside` list in
    `tools/layers.ron`, and run `cargo deny check`.
  - **The WAV is no asset.** Nothing hashes it, and 2203's Out pins no
    decoded samples. So an MP3 decoder's floats are fine here
    (`shuyi-assets` has `floats: true` in `tools/layers.ron`).
- **The manifest check** (same file):
  - `Check::sound` takes `.flac` and `.mp3` beside `.ogg` and `.wav`
    ("must be .ogg, .wav, .flac or .mp3"). `Check::music` takes them
    beside `.ogg` ("must be .ogg, .flac or .mp3") and checks
    `length_ms` against `track_length_ms`. A FLAC or MP3 track whose
    length can't be read gets a message of its own, not "(no last Ogg
    page)".
  - `format_problem` on a `.flac` or `.mp3` name: bytes that are a WAV
    are checked as a WAV. Other bytes are read by 2203's `Sound`
    format of it, whose message is the problem (one or two channels at
    44.1 kHz, as for OGG). With the feature off, the problem names the
    feature. That branch is a private function that takes the format
    as an `Option` (`None` with the feature off), so a unit test
    covers the message with the feature on.
  - **Either order with 2225:** if 2225 is done, `Check::sound`
    already takes `.sfx.ron`: add `.flac` and `.mp3` beside it, with
    one message that lists all five names, put the new branches of
    `format_problem` beside its `.sfx.ron` one, and make `sfx::to_wav`
    call `pcm16_wav` with one channel, so the crate has one WAV
    writer. If not, nothing to do (2225 adds `.sfx.ron` beside these).
- **Sounds in `shuyi-app`** (`crates/app/src/audio.rs` and
  `audio/native_music.rs`):
  - `Audio::load` runs each sound file's bytes through
    `audio::decode_to_wav` before `Backend::load_sound`, on every
    target (sounds are short). `None` hands the bytes over as today,
    `Some(Ok(wav))` hands the WAV, and `Some(Err(why))` is a warning,
    `sound "<cue>": <file>: <why>`. Those bytes never reach the
    backend, since quad-snd panics on a file it can't read.
  - Native music: `Music::load` gets its bytes and length from a
    helper, `track_bytes(path, bytes) -> Result<(Cow<[u8]>,
    Option<u32>), String>`: the length from `track_length_ms`, and the
    bytes through `decode_to_wav`, still on the worker thread.
  - Web music: `load_music` (wasm32) takes the length from
    `track_length_ms` and hands the file to the browser as it is.
  - The docs of `Backend::length` and `Clip` say OGG, FLAC or MP3.
  - **Either order with 3031** (parked, the in-house backend): if 3031
    is done, its `load_music` also runs a FLAC or MP3 track through
    `decode_to_wav` and plays the WAV whole, on the web too (its web
    path doesn't use the browser's decoder). If not, nothing to do
    (3031 does it).

- **No feature of their own.** `shuyi-app` and `shuyi-cli` add none.
  With every format feature off nothing changes: no new dependency in
  their normal builds, and a PNG is drawn and a WAV or OGG played as
  today.
- **The tests see the formats.** `shuyi-app`'s and `shuyi-cli`'s
  `[dev-dependencies]` get `shuyi-assets` with 2203's features, as
  `shuyi-assets`' own self dev-dependency has them.
- **Docs:**
  - `docs/formats.md`: replace 2203's note that `shuyi-app` and
    `frame-png` decode only PNG pixels and macroquad plays only WAV
    and OGG with what draws and plays these formats now: the registry
    in `shuyi-app` and `frame-png`; `decode_to_wav` for sounds on
    every target and for tracks on native; the browser for tracks on
    the web. The WAV and OGG Vorbis entry (that `format_problem`
    requires a sound's content to match its extension) gains FLAC and
    MP3. Add that an MP3 has silence at its ends from the encoder, so
    a looped MP3 track has a gap: loop an OGG or a FLAC.
  - The module docs of `crates/assets/src/image.rs` ("A game that
    draws through `shuyi-app` needs none"),
    `crates/assets/src/audio/formats.rs` ("Only headers are read; the
    platform decodes the samples") and `crates/app/src/render.rs`.
- **Public types (ADR-0039).** No new type: `decode_to_wav`,
  `track_length_ms` and `MAX_DECODED_BYTES` are a function, a function
  and a constant.
- **For the game.** No API break: the new items are additions, and
  `Renderer::new`, `Painter::new` and `frame_png::decode_image` keep
  their signatures. The game's next pin bump needs nothing.

**Out (do not do):**

- encoding, and converting pictures or sounds at bake time (the pack
  keeps a file's own bytes);
- a kind of decoded samples: the WAV goes to the backend and is no
  asset;
- streaming a FLAC or MP3 track: it is decoded whole, as quad-snd
  decodes an OGG today;
- resampling: the manifest check refuses a file at another rate, as it
  does an OGG;
- voice clips in FLAC or MP3: a clip's file ends in `.ogg`
  (`voice::clip_file`);
- the font atlas, which stays a PNG;
- a picture format the game registers itself, outside
  `Formats::standard()`: `Look` carries no registry;
- the rest of the in-house sound (2210, 3030 and 3032, parked): sounds
  reach any backend as a WAV through `Audio::load`.

## Implementation steps

1. Read 2203's completion notes and code: its formats, features,
   fixtures and decoder crates. Then read `crates/app/src/render.rs`,
   `crates/app/src/audio.rs`, `crates/app/src/audio/native_music.rs`,
   `crates/cli/src/frame_png.rs`, `crates/assets/src/audio.rs`,
   `crates/assets/src/audio/formats.rs` and
   `crates/assets/src/format.rs` (`register_standard`, `decode`).
2. Make sure `register_standard` has 2203's formats under their
   features (Scope, *Before you start*).
3. Write `pcm16_wav`, `decode_to_wav`, `MAX_DECODED_BYTES` and
   `track_length_ms`, with their tests.
4. Change the manifest check: `Check::sound`, `Check::music` and
   `format_problem`.
5. Run sounds through `decode_to_wav` in `Audio::load`, write
   `track_bytes` for native music, and change the web's `load_music`.
6. Decode pictures through the registry in `render.rs` and
   `frame_png::decode_image`.
7. Add the dev-dependencies, write the docs, and settle the *Either
   order* items.
8. Check by hand, on native and with `cargo shuyi web`: in a game made
   with `shuyi new`, turn on 2203's features in its `Cargo.toml`, draw
   a sprite of each image fixture, and name a FLAC and an MP3 sound
   and track in its manifest. Each is seen or heard, and the web
   build's console has no error. Don't commit that game. Say in the PR
   what you checked.
9. Run the gates.

## Acceptance criteria

- [ ] For each image fixture of 2203 (JPEG, WebP, QOI, JPEG XL, those
      it shipped), a sprite of it drawn by `frame-png`'s `Painter`
      equals the same sprite drawn from a PNG of the pixels 2203's
      format decodes.
- [ ] `shuyi-app`'s `pixels` gives the registry's pixels and size for
      those fixtures, and macroquad's for a PNG. Bytes no format reads
      give the registry's message.
- [ ] A PNG is drawn as before: every existing `frame_png` test and
      hello-grid's PNG snapshots pass unchanged.
- [ ] `decode_to_wav` on 2203's FLAC fixture gives a WAV that
      `audio::Wav` reads with the header 2203's FLAC `Sound` format
      reads, and with the samples the decoder crate gives, shifted to
      16 bits (FLAC is lossless). On the MP3 fixture, the header
      matches too.
- [ ] `decode_to_wav` gives `None` for a `.wav`, an `.ogg`, and a
      `.flac` whose bytes are already a WAV, and an error for broken
      bytes.
- [ ] `Audio::load` hands a `.flac` sound to the backend as a WAV (the
      `Fake` backend's test), and a broken one is a warning that never
      reaches the backend.
- [ ] `track_bytes` gives a FLAC track's WAV and its length, and an
      OGG track's own bytes and length.
- [ ] The manifest check takes a `.flac` and an `.mp3` sound, and a
      `.flac` and an `.mp3` track with the right `length_ms`. A wrong
      `length_ms` gets the value to write. With the feature off, the
      message names the feature (unit test through the private
      function).
- [ ] `cargo tree -p shuyi-app -e normal` and `cargo tree -p shuyi-cli
      -e normal` with no format feature on list no crate they didn't
      list before.
- [ ] `cargo deny check` passes with every feature on, and `cargo
      xtask check-layers` passes.
- [ ] The PR says what was checked by hand (step 8), on native and in
      the web build.
- [ ] `docs/formats.md` says what draws and plays each format, and no
      longer says that they aren't drawn or played.
- [ ] Each *Either order* item is settled: done here, or added to the
      open ticket's file.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `shuyi-assets`:
  - `pcm16_wav`'s header bytes, mono and stereo;
  - `decode_to_wav` on 2203's FLAC and MP3 fixtures, on a `.wav`, an
    `.ogg`, a `.flac` holding a WAV, and broken bytes;
  - the size limit, with a small limit passed to a private function;
  - `track_length_ms` on OGG, FLAC and MP3 fixtures;
  - the manifest check: the new names, a wrong `length_ms`, broken
    bytes, and the message with the feature off.
- Unit, `shuyi-app`: `pixels` on each image fixture and a PNG;
  `Audio::load` with a `.flac` and a broken `.flac` (the `Fake`
  backend); `track_bytes` on a FLAC and an OGG.
- Unit, `shuyi-cli`: the `Painter` with each image fixture against its
  PNG twin; a file no format reads draws in `MISSING_COLOR`.
- Property: random bytes named `.flac` or `.mp3` never make
  `decode_to_wav` panic, and its WAV is never over the limit
  (ADR-0007 §5).
- By hand: step 8, on native and the web.

## Completion notes

*(Filled in by the session that completes the ticket.)*
