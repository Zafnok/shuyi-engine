# Replace quad-snd without owning every layer

Research for ticket 2902, checked **2026-10-31**. **Nick chose A conditional
on automated correctness/speed parity, a working first gate (2573), staged
migration and a separate final removal (3079).** His exact answer lives in
[decisions](../decisions.md); [ADR-0066](../adr/0066-sound-uses-library-codecs-and-output-with-an-owned-mixer-and-parity-gates.md)
records the plan. This research adds no runtime dependency or replacement.

**Use permissive decoder libraries, write the small PCM mixer, and use CPAL
for desktop output.** Keep an original Web Audio bridge for the current
macroquad loader, and keep the Playdate SDK and GBA adapters separate. CPAL
adoption is conditional on 3032 verifying and preserving supported desktop
targets; keep the existing backend until that check passes. This
removes the need to implement Vorbis and three desktop device APIs while
preserving Shuyi's control over voices, streaming and constrained targets.
Rodio is a credible alternative if less native mixer code matters more than
sharing the mixer with small targets. Kira offers more built-in music and DSP
facilities, but its built-in streaming excludes wasm, so it does not remove
the hardest browser work. All ownership options still need integration and
tests; none is an all-target drop-in replacement.

## The replacement must exceed today's playback

The shared player already owns cue selection, variants, fades, the music
clock, groups and ducking. Its backend is the seam to replace, rather than
moving those rules into a library. The current desktop music worker decodes
a whole track to PCM before playing; the browser preloads too. **Bounded
streaming is a replacement requirement, not a property of today's code.**
Desktop pause currently mutes rather than holding the cursor. Playdate
already implements pause, resume and rate, correcting the ticket's older
statement that no backend implements rate. Its present round-robin slot
reuse also needs to yield to 2378's single admission policy in `App`.
([Shared backend](../../crates/platform/src/audio/player.rs),
[desktop music](../../crates/app/src/audio/native_music.rs),
[Playdate audio](../../crates/playdate/src/audio.rs),
[ADR-0063](../adr/0063-the-audio-player-shared-by-every-backend-a-small-required-trait-provided-methods-for-what-some-lack-and-audio-baked-for-the-playdate.md))

The tables use **P** for a documented facility, **G** for Shuyi code needed
above it, and **N** for a facility absent from that path. P is not an engine
integration test. Every own-code cell is G because that code is unbuilt.
**R** means required by this research; **O** means noted for later. The
conditional tickets 2363, 2365, 2909, 2910, 2911 and 3017 remain open and
parked on this date. Consequently pan is noted, video/capture/traced sound
introduce no new required integration here, and 3017 will read implemented
audio features when it runs.

### Decoders supply samples and metadata

| Need | hound / lewton / claxon / nanomp3 | Rodio alternate decoder features | Kira, defaults off | Original 2210–2214 plan |
| --- | --- | --- | --- | --- |
| R: WAV / Ogg Vorbis | P / P | P with `hound`, `lewton` | G external decoders | G both |
| R: FLAC / MP3 | P / G MP3 adapter | P `claxon` / G external nanomp3 | G both external | N in those five tickets; G 2203/2234 adapters |
| R: decode music in bounded chunks | P demand-driven reads; G bounded worker buffers | P iterator; G worker/buffer | G external decoder; native streaming only | G source implementation |
| R: exact intro/loop boundary | G prefetch + exact sample trim | G custom looping source | G custom decoder/seek plus native loop facility | G saved state or restart/discard + prefetch |
| R: `LOOPSTART` / `LOOPLENGTH` | P lewton comments; G validation/override | G expose lewton metadata separately | G external decoder metadata | G comments planned in 2212 |
| O: arbitrary seek / start offset | WAV P; Vorbis P page seek + G trim; FLAC G restart/discard; MP3 P | G depends on underlying source, not every codec | G external decoder | N original general-seek scope; G restart/discard |
| R: current browser loader, decoding only | P pure Rust reads of supplied bytes; G adapter | G call decoders independently of stock output | G external decoder/backend | G original decoder |
| R: handheld target support | G bake/adapt to SDK/tracker/PCM; no uniform `no_std` decoder set | N full Rodio on handheld | N full Kira on handheld | G planned `no_std`; runtime budget unproven |

For **every decoder candidate**, per-copy handles, pause/rate/pan, fades,
crossfades, voice admission, variants, clocks, groups/ducking, effects,
stems/sample scheduling, output delay, video sync, capture and room effects
are outside this layer: the mixer/output/policy rows below apply. Decoding
a format does not itself implement those needs.

Hound exposes sample reads and sample-index seeking, but the inspected API
does not establish IMA ADPCM support; retain the existing Playdate baking
path rather than silently promising every WAV variant. Lewton exposes
Vorbis comments and page-level seeking; exact loop points need decode and
trim. Claxon reads incrementally and exposes comments but has no documented
high-level seek. Nanomp3 supplies sample-accurate reading and gapless
delay/padding handling; its newness is a maintenance consideration, not an
independent audit. ([Hound](https://docs.rs/hound/3.5.1/hound/struct.WavReader.html),
[Lewton](https://docs.rs/lewton/0.10.2/lewton/inside_ogg/struct.OggStreamReader.html),
[Claxon](https://docs.rs/claxon/0.4.3/claxon/struct.FlacReader.html),
[Nanomp3](https://docs.rs/nanomp3/0.2.0/nanomp3/))

### Mixing mechanics stay below the shared player's rules

This table compares the supported PCM-capable paths. Kira's streaming
facilities below are native-only; its wasm alternative requires custom
streaming code. The archived miniaudio Rust wrapper exposes low-level
decoding/output rather than the C library's high-level engine, so its column
requires an additional Shuyi mixer.

| Need | Own small mixer | Rodio 0.23 | Kira 0.12 | miniaudio Rust 0.10 wrapper |
| --- | --- | --- | --- | --- |
| R: WAV/OGG effects and FLAC/MP3 | G decoder adapters above | G permitted features/adapters | G external decoders | G decoder setup, optional Vorbis |
| R: bounded streamed music | G bounded source queues | P source iterator, G avoid decode/I/O in callback | P native; N built-in on wasm | G decoder + queues |
| R: whole loops and fades | G cursor/gain ramps | P loop/volume; G connect fade rules | P loop/tween handles | G mixer |
| R: two music streams with opposite fades | G two voices + gain ramps | P simultaneous players; G shared fade rules | P multiple sounds/tweens | G mixer |
| R: true pause/resume | G hold cursor | P per Player | P per handle | G mixer |
| R: playback rate/pitch | G resampling step | P `set_speed` | P `set_playback_rate` | G resampler |
| R: independent volume/stop for each copy | G generation-safe handles | P one Player per copy | P per-playback handle | G handles |
| R: App alone decides voice loss | G reject full, never steal | P no built-in voice cap; G App admission | G size every capacity for admitted voices; propagate rejection | G mixer admission |
| R: gapless streamed region loop | G exact boundary + prefetch | G custom Source, not frame-polled seek | P native loop region, G decoder/boundary verification | G mixer |
| R: variants and MusicClock | G retain shared player | G retain shared player | G retain shared player | G retain shared player |
| R: groups/ducking | G existing policy over per-voice volume | G same | G same; no second policy tree | G same |
| O: stereo gains/pan per voice | G two gains | G custom channel-gain Source | P panning | G mixer |
| O: group reverb/filter/limiter | N initial scope; G later DSP | G source DSP + bus wiring | P tracks/effects | G low-level DSP wiring |
| O: seek / initial offset | G decoder position | G fallible seek/skip, codec-dependent | P native seek/start position; G decoder | G decoder/mixer |
| O: synchronized stems | G common sample timeline | G combined Source/common cursor | G clocks plus streaming readiness | G mixer timeline |
| O: sample scheduling | G common sample counter | G source delay/common cursor | G clocks/start time; no blanket sample guarantee | G mixer timeline |
| O: total output delay | G backend timestamps + queue depth | G CPAL timing and source queues | G backend timing and queues | G timing adapter |
| O: video PCM kept with picture (2909) | G future stream/timing policy | G custom Source/timing | G custom Sound/timing | G PCM/timing |
| O: capture beside output (2910) | G output-backend input path | G optional recording/routing | N manager capture; G CPAL beside it | P capture device; G routing |
| O: traced occlusion/room reverb (2911) | N initial DSP; G later | G DSP + geometry policy | G effects + geometry policy | G DSP + geometry policy |
| R: Playdate/GBA integration | G optional `no_std` mixer or native adapter | N desktop mixer; G separate adapter | N desktop mixer; G separate adapter | N supported `no_std` wrapper |

Rodio documents independent player controls and an uncapped simultaneous
mix. Its custom Source and mixer interfaces make the G cells possible,
but those cells remain real integration work. Kira offers native region
loops, per-sound rate/pan and track effects. Its documented capacity errors
must never become a second stealing rule, and its streaming module warns
that starts/seeks can be delayed. Neither library's defaults satisfy the
license policy. ([Rodio Player](https://docs.rs/rodio/0.23.0/rodio/struct.Player.html),
[Rodio Source](https://docs.rs/rodio/0.23.0/rodio/source/trait.Source.html),
[Rodio mixer](https://docs.rs/rodio/0.23.0/rodio/mixer/index.html),
[Kira streaming](https://docs.rs/kira/0.12.6/kira/sound/streaming/index.html),
[Kira settings](https://docs.rs/kira/0.12.6/kira/sound/streaming/struct.StreamingSoundSettings.html),
[Kira capacities](https://docs.rs/kira/0.12.6/kira/track/struct.TrackBuilder.html))

### Output accepts PCM; platform support stays explicit

| Need | CPAL native | CPAL browser | Own desktop APIs | Own current-loader Web Audio |
| --- | --- | --- | --- | --- |
| R: mixed PCM playback | P | P with wasm-bindgen; N current loader | G WASAPI/ALSA/CoreAudio | G JS imports/PCM bridge |
| R: Windows GNU | P local build/link measured below; device lifecycle untested | — | G GNU API integration | — |
| R: Linux/macOS | P documented; target floors/libraries need audit | — | G both | — |
| R: current web build | N native path | N macroquad loader without adaptation | N native path | G current-loader plugin |
| R: Playdate/GBA | N use their adapters | N | N desktop APIs; G SDK/GBA | N |
| R: formats and streaming | G decode/mixer layer | G decode/mixer layer | G decode/mixer layer | G bounded PCM, not whole-track AudioBuffer |
| R: pause/rate/handles/loops/crossfades/caps | G mixer layer | G mixer layer | G mixer layer | G mixer layer |
| R: variants/clock/groups/ducking | G shared player | G shared player | G shared player | G shared player |
| O: pan/DSP/seek/offset/stems/sample scheduling | G mixer timeline | G mixer timeline | G mixer timeline | G mixer timeline |
| O: latency | P callback/presentation timestamp; G queue total | G timing adapter | G OS timestamps | P `outputLatency` when available; G queues |
| O: capture | P input stream; G routing | P browser path; G permissions/routing | G native input implementations | G getUserMedia/lifecycle |
| O: video sync/traced sound | G future policy/DSP above output | G same | G same | G same |

The stock CPAL browser path uses wasm-bindgen. Its optional AudioWorklet
path additionally needs atomics, isolation headers and nightly `build-std`;
that is a requirement of that backend, not every possible Web Audio bridge.
An original bridge must specify bounded queues, autoplay unlock, suspend,
underruns and hosting requirements. A copied shared-memory/worklet sketch
is not enough. CPAL timestamps and Web Audio `outputLatency` describe device
presentation estimates, not measured acoustic delay or all Shuyi buffers.
([CPAL platform guide](https://docs.rs/crate/cpal/0.18.2),
[CPAL timestamp](https://docs.rs/cpal/0.18.2/cpal/struct.OutputStreamTimestamp.html),
[AudioWorklet](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorklet),
[outputLatency](https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/outputLatency))

Playdate retains its SDK sample/file players and baked audio. GBA retains
tracker/PCM output and a budgeted integer mixer; `no_std` compilation does
not establish that live Vorbis/MP3 decoding fits its CPU or memory. Music
clock policy stays shared even when sample generation differs by target.

## Current libraries reduce work, with specific limits

Versions and release timestamps below came from direct crates.io API
metadata, checked against manifests and repository license files. Owners
are observed publishers, not promises of support. Mutable repository
activity is dated as observed on 2026-10-31.

| Layer / crate | Stable release date | License, manifest and repository checked | Owners/activity and feature implications |
| --- | --- | --- | --- |
| WAV: hound 3.5.1 | 2023-09-25 | Apache-2.0; [manifest](https://github.com/ruuda/hound/blob/release/Cargo.toml), [license](https://github.com/ruuda/hound/blob/release/license) | ruuda; latest commit 2026-08-09; no runtime dependencies or features; `std::io`, not `no_std` |
| Ogg: ogg 0.9.2 | 2025-01-12 | BSD-3-Clause; [manifest](https://github.com/RustAudio/ogg/blob/master/Cargo.toml), [license](https://github.com/RustAudio/ogg/blob/master/LICENSE) | est31/RustAudio; byteorder; async dependencies optional; lewton actually uses 0.8 |
| Vorbis: lewton 0.10.2 | 2021-01-20 | MIT OR Apache-2.0; [manifest](https://github.com/RustAudio/lewton/blob/master/Cargo.toml), [license](https://github.com/RustAudio/lewton/blob/master/LICENSE) | est31/RustAudio; latest commit 2023-03-02; default ogg, byteorder/tinyvec; no supported `no_std` reader |
| FLAC: claxon 0.4.3 | 2020-08-09 | Apache-2.0; [manifest](https://github.com/ruuda/claxon/blob/master/Cargo.toml), [license](https://github.com/ruuda/claxon/blob/master/license) | ruuda; latest commit 2025-12-03; no runtime dependencies/features; `std::io` |
| MP3: nanomp3/core 0.2.0 | 2026-09-30 | MIT OR Apache-2.0; [manifests](https://github.com/robbie01/nanomp3/blob/master/Cargo.toml), [license](https://github.com/robbie01/nanomp3/blob/master/LICENSE-MIT) | robbie01; latest commit 2026-10-07; defaults std/wide/layer12; defaults off permits allocation-free core or alloc slice reader |
| Output: CPAL 0.18.2 | 2026-08-16 | Apache-2.0; [tagged manifest](https://raw.githubusercontent.com/RustAudio/cpal/v0.18.2/Cargo.toml), [license](https://raw.githubusercontent.com/RustAudio/cpal/v0.18.2/LICENSE) | tomaka/RustAudio CPAL team; empty defaults; dasp_sample + target bindings; optional ASIO/JACK/PipeWire/realtime are separate choices; `std` |
| Mixer: rodio 0.23.0 | 2026-10-11 | MIT OR Apache-2.0; [tagged manifest](https://raw.githubusercontent.com/RustAudio/rodio/v0.23.0/Cargo.toml), [license](https://raw.githubusercontent.com/RustAudio/rodio/v0.23.0/LICENSE-MIT) | tomaka/RustAudio rodio team; CPAL, dasp_sample, thiserror, num-rational; defaults include recording, dither and denied Symphonia; `std` |
| Mixer: kira 0.12.6 | 2026-10-28 | MIT OR Apache-2.0; [tagged manifest](https://raw.githubusercontent.com/tesselode/kira/v0.12.6/crates/kira/Cargo.toml), [license](https://raw.githubusercontent.com/tesselode/kira/v0.12.6/LICENSE-MIT) | tesselode/Andrew Minnich; glam/mint/rtrb/atomic-arena/triple_buffer/pastey; defaults CPAL/realtime/D-Bus and denied Symphonia codecs; `std` |

Direct release metadata: [hound](https://crates.io/api/v1/crates/hound),
[ogg](https://crates.io/api/v1/crates/ogg),
[lewton](https://crates.io/api/v1/crates/lewton),
[claxon](https://crates.io/api/v1/crates/claxon),
[nanomp3](https://crates.io/api/v1/crates/nanomp3),
[CPAL](https://crates.io/api/v1/crates/cpal),
[rodio](https://crates.io/api/v1/crates/rodio),
[Kira](https://crates.io/api/v1/crates/kira).

**Retaining lewton is an explicit maintenance compromise.** Removing
quad-snd removes its old output/mixing stack, not automatically the Vorbis
decoder family. Hound has a 2023 release and 2026 repository activity;
Claxon has a 2020 release and 2025 activity. Lewton's 2021 release and 2023
activity make it the quietest dependency in this set. Nanomp3 is recent
(September 2026) but has one observed publisher and little release history.
This is a usable, mixed-maintenance set rather than a uniformly active
long-term foundation. A newer release date alone would not justify writing
a compressed codec. Use defaults off: Rodio's alternative features are
`hound`, `lewton`, `claxon`; ordinary `wav`/`vorbis`/`flac`/`mp3` features
select Symphonia instead. MP3 should use an external nanomp3 adapter.
Kira requires external decoders for this policy, since its codec features
select Symphonia. Recheck the full graph with `cargo deny` when adopting.

**A native C FFI decoder is an escape path, not a blocked option.** Xiph's
Vorbis repository is not archived and had activity in August 2026, although
its latest release remains 1.3.7 from July 2020. Permissive `vorbis_rs`
bindings exist, so Shuyi can replace a codec behind its sample/source
adapter without replacing the mixer. That wrapper currently documents no
seeking and no `wasm32-unknown-unknown` support; it cannot replace lewton on
every target unchanged. A native fallback brings C build/ABI, packaging,
notice and fixture work, and still needs a portable browser decoder or
separate browser integration. Its maintenance story is an alternative
to evaluate at integration time, not evidence of a newer released codec.
([Xiph activity](https://api.github.com/repos/xiph/vorbis/commits?per_page=1),
[Vorbis 1.3.7 release](https://github.com/xiph/vorbis/releases/tag/v1.3.7),
[vorbis_rs limits](https://docs.rs/vorbis_rs/0.5.6/vorbis_rs/))

The surveyed **miniaudio Rust wrapper 0.10.0 is from 2020-07-23 and archived**,
not a maintained 2026 replacement. Its MIT manifest and repository agree,
but its packaged notices were not fully audited. C upstream maintenance
does not maintain this wrapper. `vorbis_rs` 0.5.6 (2026-07-30, BSD-3-Clause)
is maintained but documents neither seeking nor `wasm32-unknown-unknown`
support. Minimp3's current docs advise against new projects because of
memory unsoundness; Rodio's distinct old `minimp3_fixed` option is not the
chosen escape path. Symphonia remains denied under ADR-0002 §8.
([miniaudio metadata](https://crates.io/api/v1/crates/miniaudio),
[archived wrapper](https://github.com/ExPixel/miniaudio-rs),
[vorbis_rs](https://docs.rs/vorbis_rs/0.5.6/vorbis_rs/),
[minimp3 warning](https://docs.rs/minimp3/0.6.1/minimp3/),
[license policy](../adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md))

Miniaudio was rejected before the measured shortlist. For completeness,
its decoder API supplies incremental reads and seeks (P); Shuyi would still
need format/metadata selection, loop-comment interpretation and bounded
queues (G), with exact format fixtures unverified here. Its low-level device
API supplies desktop PCM output (P), while lifecycle handling is G. No
current macroquad-loader or supported `no_std` handheld path was established
(N for this decision). Every remaining playback, policy, synchronization,
DSP and capture requirement maps to its mixer column above or needs the
same output adapters; the archived wrapper is not counted as a qualified
solution to any required all-target path.
([Decoder API](https://docs.rs/miniaudio/0.10.0/miniaudio/struct.Decoder.html),
[device API](https://docs.rs/miniaudio/0.10.0/miniaudio/struct.Device.html))

CPAL's documented macOS floor is 14.2 and its upstream Windows CI uses
MSVC. The local GNU build below addresses the latter build question,
not device recovery or other operating systems. Shuyi's current package
documentation names both Mac architectures without an explicit OS version
floor, and Linux already requires ALSA. Therefore 14.2 is an adoption audit,
not a demonstrated regression. Verify both Mac architectures and the actual
deployment floor, then resolve any incompatibility through a compatible
library version or retained backend before switching. A library choice does
not authorize a higher OS floor or abandoning planned silent-launch behavior.
([Platform guide](https://docs.rs/crate/cpal/0.18.2),
[upstream CI](https://raw.githubusercontent.com/RustAudio/cpal/v0.18.2/.github/workflows/platforms.yml),
[Shuyi desktop packages](../targets/desktop.md))

## Measurements distinguish local evidence from integration promises

The scratch probe used **Windows 11, Intel Core Ultra 9 275HX, 24 logical
processors, Rust 1.99.1 `x86_64-pc-windows-gnu` and MSYS2 GCC**. Each release
build had a fresh target directory, downloaded dependencies already cached,
`--offline --locked`, default optimization 3/LTO off/16 codegen units and
`strip = true`. Runs were serial with no workspace build competing. These
are single clean-build observations, not median CI budgets or an integrated
engine size forecast. The same argument/timing wrapper was in the empty
baseline; candidate APIs were reachable to retain decoder/output code.

| Scratch configuration | Build seconds | Added seconds vs empty | Stripped EXE bytes | Added KiB vs empty | Windows normal/build crates, excluding probe |
| --- | ---: | ---: | ---: | ---: | ---: |
| Empty wrapper | 0.70 | — | 873,472 | — | 0 |
| lewton 0.10.2, `ogg` | 1.95 | 1.25 | 1,034,752 | 157.5 | 4 |
| CPAL 0.18.2, defaults off | 27.41 | 26.71 | 1,112,064 | 233.0 | 17 |
| Rodio 0.23.0, playback/hound/lewton/claxon | 30.45 | 29.75 | 1,726,464 | 833.0 | 32 |
| Kira 0.12.6, CPAL + external lewton | 27.83 | 27.13 | 1,619,968 | 729.0 | 29 |

All optional dependency defaults were disabled. Rodio includes WAV/FLAC
decoder features that the Kira/CPAL probes do not, so subtracting these
rows does **not** isolate mixer cost. The future nanomp3 adapter is excluded
from these figures. CPAL retained device stream creation, Rodio retained
its device sink/Player and decoding iterator, and Kira retained manager
creation/main-track volume plus an independent lewton decoder. The Kira
probe does not retain a custom streaming adapter or StreamingSoundData,
so its size is a conservative construction probe, not a completed backend.
A muted device smoke probe succeeded for all three on this GNU host; it did not
measure subjective quality, hotplug recovery, latency or microphone input.

The longest of 45 top-level Ogg music files in the local game checkout was
read **in place**, without copying any game asset into the engine. It was
5,140,796 bytes, stereo 44,100 Hz, 11,521,152 frames and 261.250612 seconds.
Its SHA-256 is
`eb0926a643b4e96bccd97994a6145995698c3b794eb606c7970c4c4aa5cd29a7`.
The benchmark consumed all 23,042,304 interleaved samples through
`black_box`. Input reading preceded the timer; trial 0 warmed the code,
and the table reports the median of trials 1–5.

| Decode path | Full-track median milliseconds | What the result establishes |
| --- | ---: | --- |
| lewton directly, i16 packets | 341.135 | Permitted Vorbis decoding is comfortably faster than real time on this desktop. |
| Rodio alternate lewton decoder | 331.297 | This includes its f32 iterator adapter and an input-byte copy. |
| Kira probe's external lewton | 329.015 | This is external decoder throughput, not Kira's built-in streaming pipeline. |
| CPAL | — | Output only; no decoder to measure. |
| Own proposed code | — | No implementation exists; no measured build/size/decode claim. |

All three decode paths use **the same lewton 0.10.2**; their small differences
and different API work do not establish a faster mixer or decoder. An
earlier lewton run had a 326.295 ms median, illustrating run-to-run variation.
This also is not an integrated quad-snd 0.2.8 benchmark. The decoded sample
payload alone is about 44 MiB as i16 or 88 MiB as f32, calculated from the
sample count rather than measured peak allocations. That makes bounded
streaming worth implementing even though desktop decode CPU is small.
No Linux, macOS, browser or handheld throughput was measured, and no
gapless-loop/concurrent-stream regression was run in this research.

Local scratch evidence is under `target/2902-bench/`: `Cargo.toml`,
`Cargo.lock`, `src/main.rs`, `build.ps1`, `build-results.json`,
`transitive-counts.json`, and six-trial `decode-*.txt` logs. To repeat,
build each feature separately with a fresh target directory and the
profile above, then pass the read-only input path as the probe's sole
argument. Do not use its `device` branch for decode timing. These scratch
files are ignored research outputs; no game path or asset is shipped.

## Ownership determines the follow-up plan

**A. Own the small sound layer — recommended.** Use the permitted decoder
stack, an original integer PCM mixer, CPAL native output and an original
current-loader Web Audio bridge. A limited analogy is Godot's own
AudioServer plus third-party codecs; this does not claim Godot uses CPAL
or Shuyi's `no_std` design. It keeps game sound rules and small-target
adapters under Shuyi's control.
([Godot AudioServer](https://docs.godotengine.org/en/stable/classes/class_audioserver.html),
[Godot third-party inventory](https://raw.githubusercontent.com/godotengine/godot/master/thirdparty/README.md))

**B. Adapt a mixer library — like Bevy.** Use Rodio with permitted codecs
for native playback, and preserve a separate current-loader web streaming
adapter and handheld backends. This minimizes new native mixer code but
adds library adaptation and dependencies; it is not a shortcut past
browser streaming. Kira is an alternative if its effects and native music
facilities become worth the additional adaptation. Bevy's checked audio
manifest uses Rodio with defaults off and a wasm-bindgen browser feature.
([Bevy audio manifest](https://raw.githubusercontent.com/bevyengine/bevy/v0.20.0/crates/bevy_audio/Cargo.toml))

Both A and B **fully remove quad-snd and macroquad's audio feature** after
all consumers migrate. B can feed Rodio's mixer PCM to an original web
output with stock playback disabled, or keep an original web mixer; that
adapter remains unproved. Neither choice requires keeping quad-snd as a
permanent browser backend.

**C. Own decoding, mixing and output — the original Shuyi plan.** Keep
the eight original build tickets and write independently from specifications.
This maximizes ownership and takes the most implementation, hardware
testing and maintenance work. Existing codec tickets cover WAV/Ogg/Vorbis,
not FLAC/MP3, so those still need 2203/2234 adapters or additional work.
([ADR-0021](../adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md))

| Choice | Estimated implementation and testing effort | Estimated new authored lines, including tests | Main ownership risk |
| --- | --- | --- | --- |
| A: codecs + own small mixer + CPAL | 10–20 engineer-equivalent working days | 3,000–7,000 total; about 1,000–3,000 production | Shuyi must get streaming, resampling and mixer real-time behavior right. |
| B: codecs + library mixer/output | 7–15 days | 2,000–5,000 total; about 700–2,000 production | Library semantics/capacities must match Shuyi policy; the browser adapter still needs work. |
| C: original owned WAV/Ogg/Vorbis, mixer and native/web output | 40–90 days | 12,000–30,000 total; about 7,000–18,000 production | Codec conformance/security plus device and browser lifecycle become Shuyi's maintenance. |

These are **engineering judgments for the original replacement scope**,
not measured delivery times, agent calendar estimates or commitments. The
new mandatory automated parity infrastructure is additional, unmeasured
work: 2573 must establish feasibility and refine the budget before progress.
Unresolved capture can block a port; these estimates promise no deadline.
A/B ranges overlap substantially because
browser streaming and cross-platform tests dominate both. C excludes an
original FLAC/MP3 implementation, estimated at another 20–60 working days,
and every range excludes the future GBA backend. All include new adapter
and playback tests but not new optional DSP/video/voice-chat features.
No surveyed proposed configuration requires a license purchase.

The measured standalone library probes add hundreds of KiB, with CPAL
accounting for most cold build time; they do not show an insurmountable
bloat problem. A/B's smaller **authored** line counts also do not measure
their full transitive code. The shared technical risks are unproved
browser streaming/region-loop integration and the desktop adoption audit.
No insurmountable blocker was demonstrated, and no parity claim is justified
until actual browser loop, device and lifecycle tests pass. **Whichever
choice was made, the build chain remains parked until Nick starts it.**

### The chosen path advances only through measured gates

2573 must make real old native Linux and original-loader Chromium output
capture, correctness fault detection and repeated equal-work performance
checks work in CI **before any replacement implementation**. Compare
supported behavior as a black box; true pause/rate/region loops use
independent expected signals instead of preserving old limitations.
Calibrate alignment/tolerances and performance noise from repeated baselines,
not arbitrary percentages from this report. Silent/missing/stale captures,
mandatory unsupported cells and inconclusive noise fail progress.

| Stage | Gate before progress | Old path |
| --- | --- | --- |
| 2573 first | Actual native/browser baseline, independent oracles, fault/slowdown self-tests and versioned budgets run in CI | Unchanged/default |
| 2210/3030 | Decoder PCM/error comparisons, exact independent mixer samples, equal-work CPU/speed and bounded decoded memory | Retained |
| 3031 Windows/web | Actual port reference capture/performance works before that port's new output code; then old/new emitted PCM, browser loader, queues and lifecycle qualify each consumer | Retained/selectable |
| 3032 Linux/macOS | Same port prerequisite and comparative suite, both Mac architectures, deployment/package/device audit; unavailable Mac capture blocks that port | Retained/selectable |
| 2397 regions | Independent intro-plus-region sequence, exact repeated boundaries/clock mapping, two streams and bounded queues | Retained |
| 3079 final | Fresh complete required matrix, same-config historical-reference comparison and existing stage listening/device evidence with recorded availability limits; only then dependency/source/plugin/test-consumer removal | Removed from current tree |

The first ticket defines every platform's mandatory evidence contract;
Windows/macOS capture adapters can be established first in 3031/3032.
This lets portable work progress after common baselines without treating
unproved hosted Mac capture as a pass. Unchanged SDK regressions remain;
future GBA implementation is not a prerequisite to removing a dependency
it never used. No parity harness or new emitted-output evidence is claimed
to exist from this research alone.

Under the chosen A, the concrete disposition is:

| Ticket | Proposed disposition and required evidence |
| --- | --- |
| 2210 | Rewrite as engine-independent PCM/source core with optional std hound/lewton adapters; keep core `no_std`; format fixtures, limits/errors and wasm/GNU builds remain required. Preserve existing Playdate IMA ADPCM baking rather than promising hound decodes it. |
| 2211 | Close original clean-room Ogg implementation; lewton's Ogg reader supplies the container. |
| 2212 | Close original Vorbis header/codebook implementation; lewton exposes comments to the loop importer. |
| 2213 | Close original Vorbis floors/residues implementation. |
| 2214 | Close original inverse-MDCT/Vorbis decoder; source adapter and bounded decoding move into rewritten 2210/3030/3031. |
| 3030 | Keep integer `no_std` mixer; remove 2214 prerequisite; add bounded stream contract, exact region-boundary tests, pause/rate/independent handles and refusal without stealing. |
| 3031 | Rewrite Windows output around CPAL and an original macroquad-loader Web Audio bridge; null output, queue/underrun, gesture/suspend and device-change tests plus listening sign-off. |
| 3032 | Rewrite Linux/macOS integration around CPAL; establish port capture first, verify both Mac architectures, deployment floor and ALSA packaging, then qualify consumers while retaining the reference. Resolve incompatibility before migration. Hound/lewton remain intentional dependencies. |
| 2203 / 2234 | Keep optional FLAC/MP3 format/playback work; point at claxon/nanomp3, preserve browser/no-std fences and ensure replacement music decodes in bounded chunks. |
| 2397 | Parked intro-then-loop region ticket on the chosen path, described below. |
| 2573 / 3079 | New first working parity gate / separate final removal, bracketing every replacement stage. |

There is **no complete stock candidate for exact, bounded, gapless region
loops across all required targets**. Ticket 2397 supplies the cue/backend
contract; 3030/3031 supply mixer and stream-boundary behavior. It adds
`MusicCue.loop_start_ms` and `loop_end_ms`, validates them against
`length_ms`, and makes `MusicClock` wrap to the region start after its intro.
Lewton's `LOOPSTART`/`LOOPLENGTH` sample comments supply defaults, with
manifest overrides and checked conversion; page seeks need exact trimming
and prefetch before the boundary, not a frame-polled jump at EOF.

The backend method defaults to `Unsupported`; a whole-file-only backend
warns once. Playdate maps the region to SDK file-player loop points, with
unit conversion tested. GBA maps supported loop/restart points to tracker
positions; unrepresentable regions warn and fall back explicitly rather
than pretending arbitrary millisecond loops exist. Since 2365 is not done,
its beat/bar clock will handle region wrapping when it lands, with the loop
point counted once. The new ticket and 2365 must name each other.

Future 2909, 2910 and 2911 follow the chosen stream/mixer/output seams.
They remain responsible for video synchronization, capture/voice policy
and room DSP respectively. Sample starts, stems and latency calibration
remain future work when a game asks; selecting a library does not promise
those features in the first replacement. This research writes no library
implementation and reads no candidate implementation source.

## What 2573 measured afterwards

Added 2026-11-01. The parity infrastructure called unmeasured above came to about
**4,300 authored lines**, tests, scenarios and capture among them, against 1,000 to
1,400 planned. Read the estimates for A with that on top, and more for each port's
own capture on Windows and macOS. It is engine tooling, and none of it ships.

The first real captures changed one claim here. Today's native loops aren't merely
fully decoded: **they aren't gapless**. In the 4,096-frame block holding a pass's end,
the next pass lands on top of it at the wrong offset or the block falls silent, while
web output is gapless. So a gapless native loop is a behaviour the replacement adds,
as bounded streaming is, not a parity it keeps. Measured on test signals through
the gates (`tools/sound-parity/README.md`), which also hold the figures for
latency, decode and CPU that this report had no way to measure.
