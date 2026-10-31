---
id: "2902"
title: "Research: sound after quad-snd: our own decoders, mixer and output, or a library, and the tickets that follow"
type: research
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3003", "3029"]
nick_input: decision
completed: 2026-10-31
---

# 2902 — Research: sound after quad-snd

> Nick unparked this research on 2026-10-31 by asking to do ticket 2902. The replacement build tickets remain parked.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): quad-snd goes first, and "for
audio, we can look at our options as well when we get there". quad-snd
0.2.8's last release was in May 2024, and the decoders it brings
(`audrey`, `lewton`) were last released in January 2021. It plays the
engine's sound on desktop and the web (`crates/app/src/audio.rs`,
`crates/app/src/audio/native_music.rs`), and macroquad's `audio` feature is
built on it.

A plan for our own code is already written, parked behind this ticket:
WAV (2210), Ogg (2211), Vorbis (2212–2214), a mixer (3030), and output on
Windows and the web (3031) and on Linux and macOS (3032).
[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§1 leaves the choice to this ticket. Bevy, the engine Nick names as the
one to rival someday, uses rodio (over cpal) for sound.

The 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6 and Unreal
5.8 found that a needs list copied from today's code is too short.
Bevy's sink pauses, changes speed, seeks and mutes. Godot's web export,
in its default Sample mode, drops bus effects, reverb and Doppler.
quad-snd 0.2.8's `PlaySoundParams` is only `{ looped, volume }`. Judged
against today's features alone, the research could pick a stack no
bigger than quad-snd.

This ticket decides with the facts of its day, then keeps, changes or
drops those tickets.

## Nick input

**Decision:** once the options are weighed, ask Nick (the `ask-nick`
skill) which way to go, with a recommendation. The options are product
calls: how much is ours, how long it takes, and what it costs the game.

## Scope

**In:**
- **What the engine needs from sound**, from the code, from 3029's shared
  backend trait and from the after-MVP sound tickets. Mark each need
  required or noted:
  - required, as today: sound effects (WAV, OGG), streamed music with
    fades and loops, variants, the music clock, the web, the Playdate
    and the GBA (3027, 3029), and the formats of 2203 (FLAC, MP3);
  - required, from 3029's provided methods and the tickets that call
    them: pausing and resuming a playing sound (3029's `pause` and
    `resume`, for 2394; quad-snd can only mute); a playback rate for
    pitch (a provided `set_rate` on 3029's trait, by its rule for a
    feature some backends lack; no backend plays one today, and 3030's
    resampler plans one); handles that set the volume of one playing
    copy of a sound or stop it alone, while other copies play on (2344's
    fades, 2378's copy caps, 2725's voices); and a voice limit (2378)
    whose one rule for which sound loses is 2345's, applied in `App`, so
    a stack that drops or steals voices on its own must let that be
    turned off (2345, 2378);
  - required for music: looping a region of a streamed track (an intro
    played once, then a region looped with no gap: the ticket under
    **Outputs**), and two streams sounding at once with opposite fades
    (2341's crossfade);
  - nothing more for group volumes and ducking (2342, 2343): they are
    the player's rules over `set_volume`;
  - noted, and required if 2363 is unparked first: a stereo pan per
    voice (a left and a right volume for each sound, for positioned
    sound, 2363);
  - noted, not required: effects on groups (reverb, filters, a
    limiter), and seeking to any time in a streamed track. 2211 and 2214
    leave seeking out. The loop region above jumps back to the region's
    start, by seeking or by decoding from the top again, so say whether
    each decoder can seek.
  - noted, not required, for music in time (2341 and 2365 leave them
    to a mixer, after this ticket): starting a streamed track part-way
    in, stems (several streams started together that stay in sync),
    scheduling a sound to the sample, and the output's delay, for
    calibrating it. Say which candidate plays each; each is a ticket
    when a game asks.
  - noted, and required if 2909 is done and its ADR hands a video's
    sound to the backend's audio: a sound stream decoded with a video,
    kept in time with its picture. **Either order with 2909:** if 2909
    is done, judge each candidate on it, and its parked build tickets
    that play that sound over quad-snd are among the tickets updated
    (**Outputs**). If not, nothing to do: 2909 uses the sound library
    chosen here.
  - noted, and required if 2910 is done and Nick chose voice chat:
    microphone capture beside the output. **Either order with 2910:**
    if 2910 is done, it wrote here what capture needs: judge each
    candidate on it, and its parked build tickets are among the tickets
    updated (**Outputs**). If not, nothing to do: 2910 follows the
    sound path chosen here.
  - noted, for traced sound (2911, parked: occlusion through walls and
    reverb shaped by the room). **Either order with 2911:** if 2911 is
    done, judge each candidate on the needs its report gives traced
    sound (the line it adds to this list); if not, nothing to do (2911
    fits the mixer chosen here).

  Judge every candidate, in each layer, on every need, so that the stack
  chosen plays at least what 3029's trait carries.
- **The options**, each for decoding, mixing and output separately, since
  the answer may differ per layer (for example, our own decoders and
  mixer over a library's output):
  - our own code, as 2210–2214 and 3030–3032 plan;
  - libraries maintained on the day, for example cpal for output, rodio
    or kira for mixing, and decoders other than symphonia (MPL-2.0, denied
    by ADR-0002 §8). For each: license (from its `Cargo.toml` and
    repository, on the day), last releases, maintainers, what it pulls in,
    its default features, `no_std` support, and whether it works in the web
    build at that time (`wasm-bindgen` or not, 3034).
- **Measure** what each candidate adds to build time and binary size, and
  how fast it decodes the game's longest track, in a scratch folder.
- **Outputs:**
  - a report, `docs/research/2902-sound-after-quad-snd.md`;
  - Nick's answer in `docs/decisions.md`;
  - an ADR if the choice adds a dependency or changes ADR-0021;
  - a ticket for intro-then-loop regions on the chosen sound path (the
    `write-ticket` skill; 2341 and 2365 leave it here): `loop_start_ms`
    and `loop_end_ms` on `MusicCue`, checked against `length_ms`;
    `MusicClock` wrapping into the region, with 2365's beat clock as
    the sub-bullet below says; the loop points read from the Ogg file's
    `LOOPSTART` and `LOOPLENGTH` comments (2212 keeps the comments; with
    a library decoder, its comment reader), with the manifest able to
    override them (ADR-0007); a provided method on 3029's shared backend
    trait, with its `Unsupported` default, and one warning on a backend
    that can only loop the whole file. It says how the Playdate file
    player's loop range (3029) and a GBA tracker song's restart point
    (3027) map onto the same fields;
    - **Either order with 2365:** if 2365 is done, the ticket also
      wraps 2365's `beats_since` and `bars_since` into the region. If
      not, it leaves them to 2365 ("nothing to do (2365 does it)") and
      puts its own id in 2365's clause on loop regions;
  - the tickets updated: kept as they are, rewritten for the chosen
    library, or closed (moved to `tickets/done/` with a completion note
    saying why they were dropped).
  - **Either order with 3017:** if 3017 is done, each ticket written or
    rewritten here that gives a backend an audio feature (`Pause`,
    `Rate`, `Pan`, `LoopRegion`) says it adds the feature to the
    profiles of that backend (`capabilities.audio.features`); if not,
    nothing to do (3017 reads each backend's features on the day).

**Out (do not do):**
- Building the replacement.
- Reading the source of any library to copy it (ADR-0021 §2). Reading a
  candidate's documentation and examples to judge it is fine.

## Implementation steps

1. List the needs (scope, first bullet) from the code, 3029's trait,
   2341 to 2345, and 2378.
2. Survey the candidates and fill a table in the report.
3. Measure the shortlist.
4. Ask Nick, with a recommendation, and record his answer.
5. Update or close the tickets, write the intro-then-loop ticket
   (Outputs), and update `docs/ROADMAP.md`'s order.

## Acceptance criteria

- [x] The report covers decoding, mixing and output, with the facts dated.
- [x] The report's table marks each need of the first bullet for each
      candidate and each layer: played, played with our code on top, or
      not. A required need that no candidate plays is named, together
      with the ticket that would write it.
- [x] Nick's answer is in `docs/decisions.md`, verbatim.
- [x] Every ticket in 2210–2214 and 3030–3032 is kept, rewritten or closed
      to match, the intro-then-loop ticket is in `tickets/open/`, and the
      ticket lint passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it keeps or writes say their tests.

## Completion notes

Completed the dated [report](../../docs/research/2902-sound-after-quad-snd.md)
with per-layer requirement matrices, primary license/maintenance sources,
GNU build/size and read-only longest-track decode measurements. No game
asset or library implementation source was copied or inspected.

Nick selected A conditional on automated correctness/speed parity and
staged migration; his exact words are in `docs/decisions.md`. ADR-0066
records library codecs/CPAL with an owned no_std PCM/mixer core and
current-loader Web Audio bridge. New parked 2573 must establish a working
actual-output/performance baseline before replacement implementation;
new parked 3079 alone removes quad-snd after the complete matrix qualifies.
Port-specific capture must work before that port's new output code;
unavailable/inconclusive evidence blocks that port, never passes silently.

2210 is rewritten around library adapters; 2211–2214 are closed as dropped,
with struck original criteria rather than fictitious implementation.
3030–3032 retain/extend the mixer and staged platform work, keeping the old
path. New parked 2397 owns loop metadata/cue/clock/backend mapping and
reciprocal 2365/3027 handoffs. Existing related tickets and ROADMAP reflect
the chosen path. All replacement build tickets remain parked.

The research adds no runtime dependency, implementation or public API
change, and requires no game pin change. Codec C FFI remains an adapter
escape path subject to target/license checks. Effort estimates describe
the original replacement scope; mandatory parity infrastructure is extra,
unmeasured work whose feasibility/budget 2573 must establish.

Validation: all applicable run-gates passed: full workspace code/doc/license
checks, native/web and target checks, layers/modules, no_std host/target
builds, generated grid/blank template gates, and repository ticket, docs,
links, changelog, pins/toolchain, typos and whitespace checks.
