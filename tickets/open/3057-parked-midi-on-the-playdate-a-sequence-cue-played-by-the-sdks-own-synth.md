---
id: "3057"
title: "Parked: MIDI on the Playdate: a sequence cue played by the SDK's own synth"
type: feature
milestone: "Engine: MIDI music"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2206", "3029"]
nick_input: none
completed:
---

# 3057 — MIDI on the Playdate

> **Parked** (Nick, 2026-10-17, asked whether MIDI should be parked because the game doesn't need it, chose "Parked"): a follow-up of the parked MIDI tickets (2205, 2206), and a follow-up of parked work is parked, so it starts after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`), after 2206 is unparked and done, and after the Playdate's audio (3029). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0014](../../docs/adr/0014-midi-music-sequences-and-a-sound-bank-in-a-module.md)
§3: the desktop and the web play a sequence with `shuyi-midi`'s own
synth, and the Playdate plays it with the SDK's own synth instead.

- 2205 reads `.mid` files into a `Sequence` and `.sf2` files into a
  `SoundBank`.
- 2206 adds the `Synth` (over `rustysynth`), the cue kind `Sequence {
  sequence, bank }` with its tempo and mute commands, and playback in
  `shuyi-app`.
- 3010 is the Playdate backend, `shuyi-playdate`, and `shuyi export
  playdate`.
- 3029 moves the audio player's rules to a place every backend shares,
  and gives the Playdate its sounds and music.

3010 and 3029 leave MIDI on the Playdate out, because it waits for the
parked 2205 and 2206. 3010 promised a follow-up ticket for it. This is
that ticket, written now, so the work is not lost when 3010 lands
first. 2206, 3010 and 3029 name it.

Why the SDK's synth and not 2206's: `rustysynth` renders floats on the
CPU and reads the whole `.sf2` bank into memory. The Playdate has 16 MB
of RAM, a Cortex-M7 at about 168 MHz and a mono speaker, and its SDK
already has synths, instruments and a sequencer (ADR-0014, *Context*).
The SDK can't read a `.sf2` bank, though, so the music's instruments
come from somewhere else (*Instruments*, below).

## Nick input

None.

## Scope

**In:**
- **The `Sequence` on the device.** The pack (2204) carries assets
  already decoded, so a cue's `Sequence` reaches the device as a blob
  of the pack, whether it came from a `.mid`, a `.seq.ron` or code.
  That needs `shuyi-midi`'s `Sequence` and its serde without `std`. If
  2205 and 2206 left `shuyi-midi` needing `std`, split it here: with
  `default-features = false`, `Sequence` builds as `no_std` with
  `alloc`, and the default `std` feature brings the formats,
  `SoundBank` and `Synth`. Add `shuyi-midi` without its default
  features to 3005's `thumbv7em-none-eabihf` build in CI.
- **The bank stays home.** `shuyi bake --target playdate` leaves a
  sequence cue's `.sf2` out of the pack (2204's `--target` hook), so a
  `.pdx` never carries a bank.
- **The SDK's sequence, built from a `Sequence`.** When a cue loads,
  `shuyi-playdate` turns its `Sequence` into an SDK sequence with
  tracks of note events (the SDK's `sequence`, `track`, `instrument`,
  `synth` and `channel` calls; check their names in the SDK's C API
  docs on the day):
  - Each event's tick becomes a step through the `Sequence`'s tempo
    map, at a fixed number of steps per second (for example a
    thousand; check the SDK's limits), in integers. One SDK tempo then
    plays every tempo change in the map.
  - A note on and its note off become one `addNoteEvent(step, length,
    key, velocity)`, paired by channel and key. A note on with velocity
    0 is a note off. A note with no off lasts to the sequence's end.
  - There is one SDK track per `Sequence` track, channel and program.
    A program change starts a new SDK track for the notes after it,
    because an SDK track has one instrument. The backend keeps which
    SDK tracks came from which `Sequence` track, so muting a track
    mutes all of them.
  - A volume (7) or pan (10) control change before a track's first
    note sets its instrument's volume and pan. Other control changes
    and pitch bend are not played. The first one in a sequence gives
    one warning through the player's warnings (3029's
    `take_warnings`).
  - The floats the SDK takes (tempo, velocity, volume) are made at
    this boundary and go no further.
- **Instruments.** The SDK can't read the bank, so each General MIDI
  program the sequences use needs an SDK instrument. There are two
  ways. Pick one on the day, after step 1, and record the pick and why
  in the ADR below.
  - **Sampled, from the cue's bank.** `shuyi export playdate` reads the
    bank with `rustysynth` on the host and keeps only the presets and
    drum keys the game's sequences use. Each zone becomes a plain
    record: its key range, root key, sample, loop points and volume
    envelope (whole milliseconds, and a sustain level in 1/256). The
    samples go to the Playdate's ADPCM through 3029's conversion, and
    into the `.pdx` the way 3029 puts sounds there. On the device each
    zone is an SDK synth playing its sample (`setSample`, with the
    loop as its sustain part), added to its program's instrument over
    its key range (`addVoice`, with a transpose from the root key).
    The records are a new type, `shuyi_midi::sampled::SampledBank`,
    `no_std`, `#[non_exhaustive]` with a constructor (ADR-0039), read
    by a `Format` (ADR-0007) with an equality test: decoding the baked
    records equals reducing the decoded bank with the same settings.
  - **A waveform table.** `shuyi-playdate` keeps a table from each of
    the 16 General MIDI families (pianos, organs, guitars, basses,
    strings, brass, leads, pads and the rest) to an SDK waveform
    (square, triangle, sine, sawtooth, noise or the SDK's others) and
    an envelope. Channel 10 plays drums: noise with a short decay for
    most keys, and a short low sine for the bass drums (keys 35 and
    36). Nothing of the bank ships.
  - **The pick.** Prefer the sampled way, because the music then
    sounds like its bank. Take the table if the SDK's sampled synth
    can't loop a part of its sample, if an instrument can't map a key
    range with a transpose, or if the example's sampled instruments
    come to more than a tenth of the `playdate` profile's heap budget.
    If the sampled way makes this PR much more than one PR's worth
    (about 600 changed lines), ship the table here, and write the
    sampled way as a ticket of its own after this one, parked like
    this one (the `write-ticket` skill).
- **Playing a cue.** 2206 and 3029 are both done when this starts, so
  a sequence cue already reaches a backend through the shared player.
  Read how. If the shared backend trait carries sequences (by 3029's
  ADR: provided methods whose default returns `Unsupported`),
  implement them in `shuyi-playdate`. If 2206 kept sequences in
  `shuyi-app` alone, first move that part to the shared trait as
  provided methods by the same rule, with `shuyi-app`'s tests
  unchanged. If such a method takes the `SoundBank`, which the
  Playdate never loads, make it take the cue's bank by id. A changed
  public API gets a `Breaking:` line (ADR-0038). Then:
  - the cue starts and stops by its `SoundId` (2344, done by then,
    since 2206 waits for it): the SDK sequence plays, and stops with
    all its notes off;
  - it loops as 2206's cue says, with the SDK's `setLoops`;
  - the tempo command sets the SDK's tempo to the base steps per
    second times the percent, over 100. The mute command mutes that
    track's SDK tracks;
  - each playing cue has an SDK sound channel holding its
    instruments, at the cue's group gain from the shared player (2342,
    done by then too), set again when the gain changes. The gains
    already hold the master volume and any ducking;
  - a pause or a `Suspend` (3029's pause, by 2344's rule) stops the
    SDK sequence and keeps its step. A resume sets that step and plays
    on;
  - stopping or unloading the cue frees its SDK objects.
- **A `midi` feature** on `shuyi-playdate`, like `shuyi-app`'s (2206),
  so a game without MIDI builds none of this. The edge from
  `shuyi-playdate` to `shuyi-midi` gets its line in `tools/layers.ron`
  (ADR-0032).
- **The fake SDK** (3010's, which 3029 gave sound) gains the sequence,
  track, instrument, synth and channel calls, and records them, so the
  host tests read what the backend asked the SDK for.
- **The example.** 2206's `examples/midi-music/` exports for the
  Playdate: a frame that fits 400 × 240, a `playdate` entry in its
  `shuyi.ron` `targets`, a `targets."playdate"` table in its keymap
  (3002) and a pattern map, `files/palette@onebit.ron` (3006).
- **An ADR** (the `write-adr` skill) that extends ADR-0014 §3: the
  SDK's sequence built from a `Sequence` rather than by the SDK's own
  `.mid` loader, the steps per second, what is not played, and the
  instrument pick.
- **Docs** (the `document` skill). `docs/targets/playdate.md` says
  what plays (sequence cues, with their tempo, mutes, loops and
  gains), what doesn't, and how the instruments sound.
  `crates/midi/README.md` says the Playdate plays sequences with the
  SDK's synth, and links there. If `skills/shuyi-midi/` exists, add
  the same there; if not, nothing to do (the skill pack writes the
  skill from the README).

**Out (do not do):**
- the SDK's own `.mid` loader (`loadMIDIFile`): the pack carries the
  decoded `Sequence`, and a sequence built in code or read from a
  `.seq.ron` has no `.mid` to load;
- 2206's `Synth` on the Playdate;
- MIDI on the GBA and the DS (ADR-0014 §3: they mix tracker music
  instead);
- MIDI devices in or out, and effects (as in 2206);
- pitch bend and control changes other than the volume and pan above:
  write a ticket when a game asks;
- measuring the SDK synth's cost on a device: write a ticket if a
  game's music makes its frames slow there.

## Implementation steps

1. Read the Playdate SDK's sound docs (the C API's synth, instrument,
   track, sequence and channel) for the SDK version 3010 recorded.
   Check the steps per second a sequence takes, whether a sampled
   synth loops a part of its sample, whether `addVoice` maps a key
   range with a transpose, and how a sequence pauses. Read how 2206
   and 3029 carry a sequence cue to a backend.
2. If `Sequence` doesn't build without `std`, split `shuyi-midi` as
   *Scope* says, and add it to the `no_std` build in CI.
3. Write the conversion from a `Sequence` to SDK calls in
   `crates/playdate/src/midi.rs`, against the fake SDK, with its
   tests.
4. Pick the instruments, and write them: the table, or the bake of the
   sampled zones with its `Format` and equality test.
5. Wire the cue: start, stop, loops, tempo, mute, gains and pause.
6. Leave the bank out of the pack, export the example, and run it in
   the Simulator if 3010's Simulator build works.
7. Write the ADR and the docs.

## Acceptance criteria

- [ ] `shuyi export playdate` on `examples/midi-music` makes a `.pdx`
      with no `.sf2` in it, and its size is in the report.
- [ ] The example plays its sequence in the Playdate Simulator, and
      its tempo and mute actions change what plays, or the Completion
      notes say why it can't.
- [ ] Host tests with the fake SDK:
  - [ ] a fixture sequence with a tempo change gives its note events
        at the expected steps;
  - [ ] a program change starts a new SDK track with the new
        program's instrument;
  - [ ] starting and stopping the cue by its `SoundId` plays and stops
        the SDK sequence;
  - [ ] the tempo command at 50 sets half the base steps per second,
        and muting a track mutes only its SDK tracks;
  - [ ] the cue's channel volume follows its group gain;
  - [ ] after a pause and a resume, the sequence goes on from the same
        step.
- [ ] `shuyi-midi` without its default features builds for
      `thumbv7em-none-eabihf` in CI.
- [ ] If the sampled way was picked, its equality test passes on the
      fixture bank.
- [ ] The ADR names the instrument pick and why.
- [ ] `docs/targets/playdate.md` says what plays and what doesn't.
- [ ] `cargo xtask check-layers` and `cargo deny check` pass.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (host, fake SDK): the conversion (ticks to steps, note pairs,
  tracks per program, the control changes kept and the one warning for
  the rest), the cue's commands, its gain, and pause and resume.
- Property: a random `Sequence` never panics in the conversion, and
  every note event ends at or before the sequence's length in steps.
- Format: for the sampled way, the equality test, and arbitrary bytes
  into the format never panic (ADR-0007 §5). For the table, every
  program from 0 to 127, and channel 10, has a voice.
- Integration: the example's export in CI, if the SDK's license lets
  CI download it (3010's rule); otherwise on Nick's machine, and the
  PR says so.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
