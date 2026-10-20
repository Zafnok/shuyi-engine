---
id: "2902"
title: "Parked: Research: sound after quad-snd: our own decoders, mixer and output, or a library, and the tickets that follow"
type: research
milestone: "Engine: in-house platform"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3003", "3029"]
nick_input: decision
completed:
---

# 2902 — Research: sound after quad-snd

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

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

This ticket decides with the facts of its day, then keeps, changes or
drops those tickets.

## Nick input

**Decision:** once the options are weighed, ask Nick (the `ask-nick`
skill) which way to go, with a recommendation. The options are product
calls: how much is ours, how long it takes, and what it costs the game.

## Scope

**In:**
- **What the engine needs from sound**, from the code and 3029's backend
  trait: sound effects (WAV, OGG), streamed music with fades and loops,
  variants, the music clock, the web, the Playdate and the GBA (3027,
  3029), and the formats of 2203 (FLAC, MP3).
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
  - the tickets updated: kept as they are, rewritten for the chosen
    library, or closed (moved to `tickets/done/` with a completion note
    saying why they were dropped).

**Out (do not do):**
- Building the replacement.
- Reading the source of any library to copy it (ADR-0021 §2). Reading a
  candidate's documentation and examples to judge it is fine.

## Implementation steps

1. List the needs (scope, first bullet) from the code and 3029.
2. Survey the candidates and fill a table in the report.
3. Measure the shortlist.
4. Ask Nick, with a recommendation, and record his answer.
5. Update or close the tickets, and `docs/ROADMAP.md`'s order.

## Acceptance criteria

- [ ] The report covers decoding, mixing and output, with the facts dated.
- [ ] Nick's answer is in `docs/decisions.md`, verbatim.
- [ ] Every ticket in 2210–2214 and 3030–3032 is kept, rewritten or closed
      to match, and the ticket lint passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None shipped (research). The tickets it keeps or writes say their tests.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
