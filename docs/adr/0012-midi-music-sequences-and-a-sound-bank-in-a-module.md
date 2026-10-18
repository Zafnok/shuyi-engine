# ADR-0012: MIDI music: sequences and a sound bank, in a module

- **Status:** Accepted; its tickets are parked
- **Date:** 2026-10-17
- **Related tickets:** 2205 (the formats and the music concern), 2206
  (the synth and playback), 2502 (the wizard asks about music), 3010
  (the Playdate backend)
- **Extends:** ADR-0004 §6 (audio stays as it is: cues by id), ADR-0007
  (formats are plugins), ADR-0009 (modules), ADR-0002 §2 and §8 (layers
  and licenses)

## Context

Nick, 2026-10-17 (`docs/decisions.md`): the new-game wizard asks "if they
want MIDI sound". He chose to park MIDI, because Visions of Shuyi
doesn't need it.

- **Today's audio** (ADR-0004 §6) plays recorded files (WAV, OGG, and
  more in 2203) through cues by id. `shuyi-app` plays them.
- **A MIDI file** (`.mid`, Standard MIDI File) holds notes and their
  timing, not sound. Something must turn notes into samples: a synth,
  usually with a **SoundFont** (`.sf2`), a bank of recorded instruments.
- **Why a game wants it:** music that is a few kilobytes, tempo and
  instruments that change while it plays, and a retro sound. The
  Playdate's SDK has its own synth and MIDI player, which suits a 16 MB
  device.
- **Candidate crates, to check on the day** (licenses from their
  `Cargo.toml` and repositories, not from memory):
  - `midly` reads `.mid` files, `no_std`, and is believed to be under
    the Unlicense;
  - `rustysynth` is a SoundFont synth with a MIDI sequencer, pure Rust,
    and is believed to be MIT.
- **Sound banks are art,** and ADR-0002 §8 allows art and audio only
  under CC0-1.0, CC-BY-3.0 or CC-BY-4.0. Many General MIDI banks have
  their own licenses. One that passes must be found, or the engine ships
  none and each game brings its own.

## Decision

### 1. Music is a concern, and MIDI is one of its answers

The catalog (`docs/modules.ron`, ADR-0009 §2) gains a concern:

| Concern | Answers | Where |
| ------- | ------- | ----- |
| Music | recorded files; MIDI with a sound bank | default (ADR-0004 §6); `shuyi-midi` (2205, 2206) |

`pick` is `Many` (ADR-0011 §1): a game may use both, for example
recorded music in the menus and MIDI in the levels.

### 2. A module, `shuyi-midi`

- The crate `crates/midi` (`shuyi-midi`), with the module duties of
  ADR-0009 §4, behind the `midi` feature of the `shuyi` crate.
- **Two formats**, registered by the module at start-up (ADR-0007 §2):
  - `mid` reads a `.mid` file into a `Sequence`: tracks of events at
    integer ticks, the tempo map, and the length. It never uses floats;
  - `sf2` reads a `.sf2` file into a `SoundBank` and checks it.
- **A `Sequence` is plain data.** It can be built in code, so a game can
  make music by rules, and its text form, `*.seq.ron`, can be read and
  diffed. The `.mid` file stays the source a composer edits.

### 3. Playing is the backend's job

- **Cues** (ADR-0004 §6) gain a kind: a cue plays a recorded file, or a
  sequence with a bank. Logic starts and stops cues by id as today, and
  the Harness reports "sounds started" the same way for both.
- **The synth** turns a sequence and a bank into samples. It is a pure
  function in `shuyi-midi`: `Synth::render(&mut self, left: &mut [f32],
  right: &mut [f32])`, with no I/O and no clock. It uses floats; it is sound, not
  simulation (ADR-0002 §2).
- **`shuyi-app`** feeds the synth's samples to the audio device, behind
  its own `midi` feature.
- **Live control** (tempo, muting a track, changing an instrument) goes
  through cue commands, so logic stays deterministic and the sound
  follows.
- **The Playdate backend** (3010) plays a sequence with the SDK's own
  synth instead, when its ticket adds it.

### 4. Tests

- The `mid` format decodes fixtures to checked-in `Sequence` hashes,
  with a fuzz-style test (ADR-0007 §5).
- The synth renders a fixture sequence offline. The test compares the
  samples to a checked-in hash on one platform, and within a stated
  tolerance on the others, because float sums may differ across
  platforms.
- The engine's own fixture sequences and banks are made by us under
  CC0-1.0.

## Consequences

- A game can have music in kilobytes, and music it changes while it
  plays.
- The wizard can ask about music like any other concern, and shows MIDI
  as parked until Nick unparks 2205.
- A sound bank is a large file and may carry its own license. Until a
  bank under an allowed license is found, games bring their own, and
  the module's README says so.
- The synth runs on the audio thread of the desktop and web builds and
  costs CPU there. 2206 measures it against the budgets (3008).

## Alternatives considered

- **MIDI out to the player's own synth or device** — sounds different on
  every machine, and the web build and the Playdate have no MIDI port.
- **Render MIDI to OGG at build time** — simple, but it loses what MIDI
  is for: the size, and changing the music while it plays.
- **MIDI in the default engine** — every game would pay for the synth
  and its dependency, which ADR-0009 §1 rules out.
- **A tracker format (`.mod`, `.xm`) instead** — a fine module of its
  own later, but Nick asked for MIDI, and `.mid` is what most composing
  tools export.
