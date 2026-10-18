---
id: "3027"
title: "Parked: GBA audio: samples and tracker music baked for the GBA, and an integer software mixer in shuyi-gba"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3021"]
nick_input: none
completed:
---

# 3027 — GBA audio

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Split out of 3021, the GBA backend, to keep that ticket one PR.
[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§2 and §4:

- the GBA has two 8-bit sample channels and four simple tone channels,
  and music is mixed in software (ADR-0012, *The hardware*; 3018
  re-checks it in `docs/targets/gba.md`);
- audio is baked like images: samples to 8-bit at the target's rate, and
  music to a tracker-style format the backend's mixer plays;
- the format is picked here, under a permissive license (ADR-0002 §8);
- the `gba` budget (§7) leaves about 80 k of a frame's 280 k cycles to
  the mixer and the interrupts, so the mixer is counted in the emulator
  runner (3020).

Until this lands, `shuyi-gba`'s `audio()` is silent (3021).

## Nick input

None.

## Scope

**In:**
- **The music format.** The source is a standard tracker module, so it
  always loads (ADR-0007): pick `.mod`, `.xm` or `.it` (or more than one)
  from what composers use and what a permissive reader exists for. The
  bake turns it into our own compact form, `GbaSong`: patterns, the
  instruments' samples at 8 bits, and the effects the mixer supports.
  Record the choice, the reader crate and its license, and the effects
  dropped, in an ADR (the `write-adr` skill). An effect the mixer
  doesn't play is a bake error naming the song, the pattern and the
  effect, never a silent drop.
- **Samples.** `shuyi bake --target gba` turns sound effects, in any
  audio format the engine loads (ADR-0007), into signed 8-bit PCM at the
  mixer's rate, in
  integers. Pick the rate so a frame holds a whole number of samples
  (18 157 Hz gives 304 a frame at about 59.73 Hz; re-check).
- **Equality** (ADR-0007 §4): decoding a baked sample or `GbaSong`
  equals converting the decoded source with the same settings, on every
  fixture.
- **The mixer**, `shuyi_gba::mix`, `no_std`, integers only:
  - a fixed number of voices (set in the profile's audio settings),
    each with a sample, a position in 16.16, a step, a volume and a pan;
  - the tracker player steps rows and effects once per frame;
  - it fills one frame's buffer for each of the two sample channels,
    which a timer and DMA play from a double buffer;
  - it is a pure function of its state, so its output on the host equals
    its output on the device, and a host test checks it bit for bit.
- **`Audio` for `shuyi-gba`** (3003's trait): play and stop a sound
  effect, play, stop and fade music, and a master volume from the
  game's settings.
- **Budgets.** The bench scenarios (3008) get one with music and four
  sound effects at once, run by the emulator runner (3020) against the
  `gba` `frame_cycles` budget.
- **`hello-grid`** gets one song and two sound effects, exported for
  `gba`.

**Out (do not do):**
- the four tone channels (write a ticket when a game wants them);
- the DS's audio (3024);
- streaming long recorded music from ROM (write a ticket if a game
  needs it);
- measuring on the device (3022).

## Implementation steps

1. Survey the tracker formats and Rust readers with permissive licenses.
   Pick, and write the ADR.
2. Write `GbaSong`, the sample conversion, their `Format`s and the
   equality tests.
3. Write the mixer and the tracker player on the host, with fixture
   songs and checked-in output buffers.
4. Wire the double buffer, the timer and the DMA in `shuyi-gba`, through
   3021's fake hardware for the host tests.
5. Add the bench scenario and the example's audio.

## Acceptance criteria

- [ ] A fixture song's mixed output matches its checked-in buffer, on
      the host and in the emulator (the ROM prints a hash of it).
- [ ] An effect the mixer doesn't play fails the bake, naming the song,
      the pattern and the effect.
- [ ] The equality test passes on every fixture.
- [ ] The music scenario stays under the `gba` `frame_cycles` budget in
      the emulator runner.
- [ ] The ADR exists and names the format, the reader and its license.
- [ ] `cargo deny check` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: sample conversion, each supported effect, voice mixing and
  clipping.
- Property: random songs and samples never panic, and a frame always
  gives exactly one frame of samples.
- Integration: the example's ROM plays its song in the emulator, with
  the hash checked in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
