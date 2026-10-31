---
id: "3027"
title: "Parked: GBA audio: samples and tracker music baked for the GBA, and an integer software mixer in shuyi-gba"
type: feature
milestone: "Parked: export targets"
model: reasoning
effort: high
status: blocked
blocked_by: ["3021"]
nick_input: setup
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

The in-house mixer (3030, parked) is an integer mixer in `shuyi-sound`,
which builds without `std`. Either order works: if 3030 is done first,
use its mixer here and keep only the GBA's timing and 8-bit output; if
this is done first, 3030 moves this mixer into `shuyi-sound`.

## Nick input

**Setup, only if 3022 is done first** (otherwise none), for the
music scenario's re-run on the device (*Scope*). With the GBA and the
flash cart from 3022, copy the bench ROM the session gives you to the
card, run the music scenario from its menu, and give the session the
save file the cart wrote, as in 3022's steps 4 and 5.

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
  **Either order with 3061:** if 3061 is done, its ADR already chose
  the source format, its reader and the effects the DS keeps, and
  `hello-grid` has a song: bake `GbaSong` from that format with that
  reader, extend that ADR with what the GBA's mixer drops rather than
  writing a new one, export that song for `gba` rather than adding a
  second, and run 3061's music scenario on `gba` (*Budgets*); if not,
  nothing to do (3061 reuses this ticket's choice, song and scenario).
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
  - which sound loses when the voices are all busy is 2345's rule, which
    2378 applies (the cue rules in `App`, with the manifest's
    `max_voices`, which 2378 adds, at most this number), so the mixer
    never picks a loser on its own: a play with every voice busy is
    refused, never mixed over another voice and never stealing one;
  - the tracker player steps rows and effects once per frame;
  - it fills one frame's buffer for each of the two sample channels,
    which a timer and DMA play from a double buffer;
  - it is a pure function of its state, so its output on the host equals
    its output on the device, and a host test checks it bit for bit.
- **`Audio` for `shuyi-gba`** (3003's trait): play and stop a sound
  effect, and play, stop and fade music, each at the frame's gain for
  its cue (`AudioFrame::gains`, 2342), which already holds the player's
  master volume and any ducking (2343). If 2342 isn't done, at the
  frame's `sound_volume` and `music_volume`, and 2342 moves it onto the
  gains (its clause says so).
  **Either order with 3029.** If 3029 is done, implement its shared
  backend trait under its player, so that cue volumes, fades and the
  music clock are not written again. Fill in the provided methods this
  mixer can play (pause and resume, and a rate as the voice's step),
  and leave the rest at their defaults. If 2363 is done, the trait has
  a pan: fill it in too, each voice's pan from 2363's `pan_gains`; if
  not, 2363 gives the GBA its pan when it lands (its own clause says
  so). If 3017 is done, list them in `targets/gba.ron`'s audio
  `features`. If 3029 is not done, write the 3003 trait as described,
  and 3029 moves the GBA onto its shared trait.
- **Budgets.** The bench scenarios (3008) get one with music and four
  sound effects at once, run by the emulator runner (3020) against the
  `gba` `frame_cycles` budget.
- **Either order with 3022:** if 3022 is done, its report says audio
  isn't measured: re-run the music scenario on the device with 3022's
  bench build (`shuyi export gba --bench`, then `shuyi bench
  --read-save`), add the numbers to `docs/targets/gba.md`, and re-check
  the `gba` `frame_cycles` calibration against them; if not, nothing to
  do (3022 measures with music on).
- **`hello-grid`** gets one song and two sound effects, exported for
  `gba`.

**Out (do not do):**
- the four tone channels (write a ticket when a game wants them);
- the DS's audio: sound effects are 3024's, and music is 3061's (it may
  reuse this ticket's format and mixer; if 3061 is done first, the
  clause in *The music format* says what is left here);
- streaming long recorded music from ROM (write a ticket if a game
  needs it);
- measuring on the device (3022), apart from the music re-run above.

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
  clipping; a play with every voice busy is refused, never mixed over
  another voice.
- Property: random songs and samples never panic, and a frame always
  gives exactly one frame of samples.
- Integration: the example's ROM plays its song in the emulator, with
  the hash checked in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
