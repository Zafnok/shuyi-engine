---
id: "2909"
title: "Parked: Research: video playback with permissive decoders, the browser's own player on the web, and sync with the music clock"
type: research
milestone: "Engine: visual novels"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: []
nick_input: none
completed:
---

# 2909 — Research: video playback

> **Parked** (Nick, 2026-10-18: "make tickets parked for after visions of shuyi launches"): video comes with the visual novels (2721–2726), so this waits until after Visions of Shuyi launches. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison (`docs/decisions.md`, "a second
comparison"). Of the engines it read, Godot, Unity, Unreal, GameMaker
and Construct 3 play a video file, and Bevy doesn't:

- Godot's feature list has "Video playback with built-in support for
  Ogg Theora"
  (https://docs.godotengine.org/en/stable/about/list_of_features.html);
- Unity has its `VideoPlayer` component, and Unreal its Media
  Framework;
- GameMaker has video functions, and Construct 3 a Video object that
  plays through the browser's own player;
- Bevy has no video crate among its first-party crates.

What Shuyi has today:

- **Cut scenes made of pictures.** `crates/assets/src/cinematic.rs`
  reads a cinematic file: timed shots of the game's own type, over a
  length that is a music cue's `length_ms` (`Length::Music`) or seconds.
  `crates/ui/src/cinematic.rs` draws every frame from the time alone
  (`Timeline`, `CinematicPlayer`, `ShotPainter`), with `Pan` over a
  backdrop picture. A logo card or a cut scene drawn from pictures needs
  no video.
- **A music clock.** The backend reports what really sounds
  (`Audio::music_playing` in `crates/platform/src/audio.rs`),
  `Game::set_music_playing` (`crates/ui/src/game.rs`) turns it into a
  `MusicClock` (`crates/ui/src/audio.rs`), and a screen reads it from
  `Ctx::music_clock`. A track's length is data, `MusicCue::length_ms`
  (`crates/assets/src/audio.rs`), checked against the file within
  `LENGTH_TOLERANCE_MS` through `ogg_length_ms`.
- **A web plugin of our own.** `crates/cli/web/gamepad.js` is a miniquad
  JS plugin written for the engine. Its Rust side is
  `crates/app/src/pads/web.rs`, with no `wasm-bindgen` (`AGENTS.md`,
  *Environment*). ADR-0021 §1 has already chosen wgpu and winit, and
  the web build moves to `wasm-bindgen` when macroquad leaves (3034
  plans how).
- **No streaming from the pack.** ADR-0023: `Pack::open` unpacks every
  compressed file at once, and streaming is out of its scope.
- **Music is read beside the game.** `crates/app/src/audio.rs` loads a
  track from the game's music folder when it is about to play
  (`folder_beside`, `platform_folder`) and frees it when it stops. A
  track only sounds once its file has loaded, and `music_playing`
  counts from that moment. This is the one precedent for a large file
  read at play time.
- **Size caps on formats.** ADR-0019 §4: 16 MiB by default, 64 MiB for
  PNG and sound files.
- **No ticket plays a video file.** 2203 adds still images and audio
  formats only. ADR-0016 and 2721–2726 give visual novels backgrounds,
  event pictures, transitions and voices, but no movie. 2514 writes a
  game's frames as an APNG or GIF, which is the other direction.

Planned work this touches: 2364 (tracks and cues in the cinematic file,
parked), 2365 (a beat clock, parked), 2366 (captions for sounds, not
speech, parked), 2906 (asset memory research), 3017 (target
capabilities, parked), 3010 (the Playdate backend), 3029 (audio on the
Playdate, music streamed through the SDK's file players), 3034
(research on replacing macroquad, parked), 2902 (sound after quad-snd,
parked), 2020 (fuzzing every registered format) and 3039 (the notices
a web build carries).

Why it matters:

- **The game first.** Visions of Shuyi has no video and doesn't ask for
  it. Nothing here goes ahead of its work (`docs/decisions.md`,
  2026-10-20, "Built for one game first").
- **Other 2D games.** Opening movies and animated event scenes are a
  staple of visual novels, the first kind of game here that needs
  video. Studio logos and cut scenes rendered in another tool are
  common in 2D games of every kind.
- The end goal of 2026-10-19 (an agent makes snake or a platformer in
  an hour) doesn't need it.
- **The common paths are closed.** FFmpeg is LGPL or GPL. H.264 and
  HEVC carry patent pools that charge fees. ADR-0002 §8 denies both
  copyleft and anything that needs payment. The permissive options need
  a written comparison before anyone adds a dependency.

This ticket is research: a report, an ADR, and parked tickets for the
build. It promises no crate.

## Nick input

None. Choosing a decoder is technical, and ADR-0002 §8 already rules
out copyleft and anything that needs payment. If the report finds a
case for a codec under a patent licence that costs money or brings
legal duties (H.264, HEVC), that is a question for Nick (the `ask-nick`
skill), not a choice made here.

## Scope

**In:**
- **The needs**, written first: what an opening movie, a studio logo
  and a movie event picture in a visual novel need (size, frame rate,
  length, sound, looping, skipping, and what happens when the game is
  paused or goes to the background, 3042 and 3043). And when a game
  should use a cinematic of pictures instead: it is small, its text
  translates, it draws on a 1-bit screen, and a test reads it from the
  scene.
- **Decoders and containers**, each with the facts of its day: the
  license from its `Cargo.toml` and repository, with
  `default-features = false`; codec patents (cargo-deny doesn't see
  them); pure Rust or C; whether it builds for `wasm32-unknown-unknown`
  without `wasm-bindgen` and on Windows with the GNU toolchain
  (ADR-0007 §5); last releases and maintainers; what it pulls in. At
  least:
  - AV1: `rav1d` (Rust) and `dav1d` (C), both said to be BSD-2-Clause;
    whether either builds without its assembly;
  - Theora: Xiph's `libtheora` (said to be BSD-3-Clause) and any Rust
    decoder, in Ogg (the `ogg` crate is already in `Cargo.lock`);
  - VP8 and VP9: `libvpx` (said to be BSD-3-Clause, with a patent
    grant);
  - AV1 and VP9 are royalty-free from their makers, but Sisvel offers a
    patent pool for both: say what, if anything, it asks of a game that
    ships a software decoder;
  - H.264 and HEVC, through a library or through the system's own
    decoder (Media Foundation on Windows, AVFoundation on macOS, VA-API
    or GStreamer on Linux and the Steam Deck; GStreamer is LGPL, so say
    whether calling the system's copy is allowed): listed to say why
    not, or what it would take;
  - containers: WebM (Matroska), IVF and Ogg, with a demuxer for each
    codec on the shortlist;
  - **a flipbook**: frames packed as atlas pages (2222) and played as a
    clip (1123, 2231), or APNG (`png` 0.18 is in `Cargo.lock`; check
    that it reads APNG frames), stored in the pack with its per-file LZ4
    (ADR-0023). Measured against the best codec, not a new
    frame-sequence format.
- **The license check.** BSD-2-Clause and BSD-3-Clause are on
  `deny.toml`'s allow list. Check each candidate with `cargo deny check
  licenses` in a scratch crate, `default-features = false`. A C library
  behind a `-sys` crate is checked by its own license file too, since
  cargo-deny reads the crate's declared license, not the bundled C
  code's. Say what notice each one adds to 3039's `NOTICES.txt` for web
  builds.
- **Measure the shortlist** in a scratch folder, never committed:
  decode speed at 640×360 and 1280×720, 30 frames a second, on a
  desktop (and the Steam Deck if one is at hand; say so if not); what
  each adds to a native binary and to the web build's `.wasm`; peak
  memory, with the frames in flight. **The test clip** is ours or CC0:
  for example frames of `examples/hello-grid` taken with `frame-png`
  (or 2514's `--every`, if it is done), encoded by a tool run on the
  side and never shipped. It is credited in the report, and is never
  anything of Visions of Shuyi.
- **The web build**: the browser's own `<video>` element through a
  miniquad JS plugin of our own, like `gamepad.js`, never through
  `wasm-bindgen` while macroquad's loader runs the web build:
  - which codecs browsers play on the day;
  - how a frame reaches the canvas (a WebGL texture made from the
    video element, or the element over or under the canvas);
  - autoplay rules: the page's audio already waits for the player's
    gesture (`crates/cli/web/README.md`);
  - how the plugin reports the video's current time;
  - a throwaway prototype in the scratch folder, or a note saying why
    none was tried;
  - **Either order with 3034:** if 3034 is done, the web build uses
    `wasm-bindgen` (ADR-0021 §1): cover the video through `web-sys` and
    wgpu's textures too. If not, cover the macroquad path, and name
    what changes under 3034.
- **The Playdate**: its SDK's own video player and `.pdv` format
  (`playdate->graphics->video` in the C API; read the SDK's docs on the
  day). How a `.pdv` is made, whether it carries sound (if not, whether
  the sound plays through 3029's file player), its frame rate and
  memory, and whether `shuyi bake` could make one from a source video
  in a per-target asset layer (3007). **The GBA and the DS**: one line
  each; expected "not supported".
- **Targets that can't play video** never drop it silently. Such a
  target refuses the game (`shuyi check --target`, through 3017's
  `capabilities` and a module's `needs`) or plays a flipbook instead.
  **Either order with 3017:** if it is done, name the capability; if
  not, write the capability as an edit to 3017's ticket.
- **Sync and determinism**:
  - a video's length is data, like `MusicCue::length_ms`: a manifest
    entry checked against the file within a tolerance, never a clock
    the simulation reads;
  - where the video is: either the backend reports it, as
    `Audio::music_playing` does for music, and a screen reads it as it
    reads `Ctx::music_clock`; or the video follows a music cue, as a
    cinematic's `Length::Music` does. The report picks one, and says
    what happens when decoding falls behind (frames dropped, the sound
    never slowed);
  - the video's sound: decoded with it and handed to the backend's
    audio, or a separate music cue. **Either order with 2902:** if it is
    done, use the sound library it chose; if not, quad-snd as today, and
    each build ticket that plays the video's sound names 2902, whose
    needs list has a line for it and whose PR updates those tickets;
  - ending or skipping a video reaches the rules as a command (turn
    games) or an input (tick games). Skipping is an action the screen
    declares (`Screen::actions` in `crates/ui/src/screen.rs`, 3002),
    never a key (`AGENTS.md` rule 8);
  - decoding lives only in platform backends (`shuyi-app`, its web
    plugin, `shuyi-playdate`), never in `shuyi-core` or `shuyi-ui`
    (`AGENTS.md` rule 5); `shuyi-assets` reads only the header;
  - tests: a test of what happened reads the video item in the scene
    (which video, which millisecond), never its pixels (`AGENTS.md`
    rule 9). Say whether the headless backend and `frame-png` draw a
    real frame for a look test, or a placeholder.
- **Formats as plugins** (ADR-0007, ADR-0019; `AGENTS.md` rule 10):
  - a `Video` asset kind whose format reads only the header at load
    (size, frame rate, length), as `ImageInfo` and `PngInfo` do for
    images (`crates/assets/src/image.rs`, ADR-0019 §6);
  - the frames streamed at play time, past the size caps of ADR-0019
    §4;
  - how a video is stored in the pack and streamed from it (ADR-0023
    leaves streaming out today), and what that needs from 2906's
    lifetimes (either order: if 2906 is done, use its terms; if not,
    leave a note in 2906's ticket);
  - whether a baked form (a flipbook, a `.pdv`) is "the same asset"
    under ADR-0007 §4's rule for lossy formats;
  - ADR-0007 §5 for a decoder: its size limits, its fuzz target
    (2020 fuzzes every registered format; either order: if 2020 is
    done, say how a video format joins it; if not, say how it would
    join the harness 2020's ticket plans), and determinism. A video's
    pixels reach only the screen, never the simulation, and the
    browser's own player can't promise the same pixels on every
    browser: say what that means for content hashes and look tests.
- **Subtitles**: timed text keys the engine draws, so they translate and
  follow the player's text settings, never burned into the picture. A
  movie's subtitles are speech, which 2366 leaves to dialogue: say
  whether they are dialogue lines by id (as a voice clip is) or a timed
  list of text keys, and whether 2366's strip or the text box draws
  them.
- **The module shape**, recommended, not built:
  - a `shuyi-video` module (ADR-0009), or a part of a crate that exists;
  - a picture source a skin draws: a frame item of its own, or a
    `Sprite` (`crates/ui/src/frame/glyph_buffer.rs`) whose image the
    backend refreshes each frame;
  - a request kind beside `AudioRequest` (play and stop, by id);
  - where a game writes one: a ready-made shot for the cinematic file
    (beside 2364's parked work), a stage direction in 2721's table (for
    example `movie <name>`), and the gallery entry 2724 would add.
- **Outputs**:
  - the report, `docs/research/2909-video-playback.md` (create
    `docs/research/` if no earlier research has), with the facts dated;
  - an ADR (the `write-adr` skill) with the decision: the decoder per
    target, the sync rule, the module shape, and the targets that
    refuse. If the answer is "no codec, flipbooks only", the ADR says
    so and why;
  - the build tickets (the `write-ticket` skill), each one PR, each
    `Parked:` with the visual-novel note and milestone "Engine: visual
    novels";
  - the edits the decision needs in 2721's, 2722's, 2724's, 2364's and
    3017's tickets, and the visual-novel line of `docs/ROADMAP.md`'s
    *Parked* section. **Either order:** a ticket already in
    `tickets/done/` is never edited; the change becomes a new parked
    build ticket instead.
- **Size:** if the PR passes about 600 changed lines with the report and
  the ADR written, the build tickets and the edits to the five tickets
  and `docs/ROADMAP.md` become a follow-up ticket with this one's note
  and milestone, blocked by this one; the PR says so.

**Out (do not do):**
- building any of it: the tickets it writes do;
- encoding, editing, and streaming from the network (HTTP requests are
  2601);
- recording movies of a game: 2514 writes a game's frames as an APNG or
  GIF, and a video encoder is a separate gap;
- cut scenes made of pictures (the cinematic file plays them; keyframed
  tracks are 2364), screen transitions (2339) and the beat clock (2365);
- still and animated images used as sprites (2203, 1123);
- captions for sound effects and music (2366);
- reading files on demand and asset lifetimes for every kind (2906):
  this ticket says only what video needs from them;
- reading a decoder's source to copy it (ADR-0021 §2). Reading its docs
  and examples to judge it is fine.

## Implementation steps

1. Read the code and ADRs named in *Context*, and 2721, 2722, 2724,
   2364, 3017, 3010 and 3034 as they stand. Note which of 2020, 2514,
   2902, 2906, 3017 and 3034 are in `tickets/done/`, for the either-order
   rules above.
2. Write the needs into the report.
3. Survey the candidates and fill the table. Check the licenses with
   `cargo deny` in a scratch crate.
4. Make or pick the test clip. Measure the shortlist and the flipbook.
5. Try the web plugin prototype, and read the Playdate SDK's video docs.
6. Write the sections on targets, sync and determinism, formats and the
   module shape, then the recommendation.
7. Write the ADR, the parked build tickets, and the edits to the
   tickets and to `docs/ROADMAP.md`.
8. Run the gates.

## Acceptance criteria

- [ ] `docs/research/2909-video-playback.md` has the needs, a dated
      table of every candidate (license, patents, pure Rust or not,
      `wasm32-unknown-unknown` without `wasm-bindgen`, Windows GNU, size
      added, decode speed, memory), and the flipbook measured against
      the best codec.
- [ ] The report says, for each of Windows, Linux, macOS, the web, the
      Steam Deck, the Playdate, the GBA and the DS, whether it plays
      video, plays a flipbook, or refuses the game through
      `shuyi check --target`.
- [ ] The report names the sync rule, how a skip reaches the rules, and
      how a test reads a playing video from the scene.
- [ ] The report says how a video format meets ADR-0007 §5 (size
      limits, a fuzz target, determinism) and §4 (a baked form equal to
      its source within a stated tolerance).
- [ ] No recommended crate is outside ADR-0002 §8 or needs a patent
      fee, and each was checked with `default-features = false`.
- [ ] The ADR is in `docs/adr/`, linked from the report.
- [ ] The build tickets are in `tickets/open/`, each parked with
      milestone "Engine: visual novels", and `docs/ROADMAP.md`'s
      visual-novel line lists them in order.
- [ ] 2721, 2722, 2724, 2364 and 3017 each carry the edit the ADR
      calls for, or, if done, a new parked ticket does.
- [ ] `cargo xtask ticket-lint` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it writes say their tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
