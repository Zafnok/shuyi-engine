# ADR-0143: Sound effects from a few numbers: `.sfx.ron`, rendered once in integers through the sound hook

- **Status:** Accepted
- **Date:** 2026-11-23
- **Related tickets:** 2225

## Context

- A game made with `shuyi new` was silent, yet the engine's menus and
  cursors play four cues (`ui_move`, `ui_select`, `ui_cancel`,
  `ui_cursor`: `AudioConfig::DEFAULT`). The one game with sounds,
  Hello, Grid, wrote a WAV header and float sine blips by hand.
- An agent asked for a small game can't make a good WAV, but it can
  write a dozen numbers (Nick, 2026-10-19, `docs/decisions.md`: a small
  game made quickly and in few lines). PICO-8 and TIC-80 make a sound
  from a few numbers; the sfxr family from presets. None of Bevy, Godot,
  Unity or Unreal has a text file of a sound an agent can write.
- Sound reaches the speaker as bytes a game's `Files::sound` hook gives;
  the player reads them as a WAV or an OGG, and a FLAC or MP3 one as the
  WAV `audio::decode_to_wav` makes of it (ticket 2234). The manifest's
  check (`audio::format_problem`) picks its check by the file's name.
- Formats are plugins and cargo features of `shuyi-assets` (ADR-0007,
  ADR-0009 §4); a file may be two kinds, as an image is (ADR-0019 §6).
- The MIDI synth planned in ADR-0014 plays live and uses floats. No
  ticket made sound effects from data.

## Decision

### 1. A format, behind the `sfx` feature

- `shuyi_assets::sfx`, behind the feature `sfx = []` of `shuyi-assets`,
  off by default. It adds no dependency (`ron` and `shuyi_core::SimRng`
  are there already) and builds without `std` (CI's `no-std` job builds
  `--no-default-features --features sfx`).
- A **game turns `sfx` on in its own `Cargo.toml`**:
  `shuyi-assets = { workspace = true, features = ["sfx"] }`, as the
  templates do. Not through a feature of `shuyi` (`cargo xtask
  check-modules` reads each of its features as a module). `shuyi-cli`
  turns it on for itself, so a game's tools (`cargo shuyi check`,
  `import`, the Playdate bake) read sound effects whether or not the
  game did; only the game's own load then fails, with the message that
  names the feature: `a .sfx.ron sound needs the sfx feature of
  shuyi-assets`.

### 2. A file is two kinds

- `SfxDef`, the kind `"sound effect"`, is the file's numbers: a `wave`
  (`Square(duty)`, `Triangle`, `Saw`, `Sine`, `Noise`), `hz` sliding to
  `to_hz`, `attack_ms`, `hold_ms` and `fade_ms` (1 to 5 000 ms in all),
  a `volume`, an optional `vibrato` and `step`, and a `seed`. `SfxDefRon`
  (id `sfx`, `.ron`) reads it, checked with every problem on its field's
  line (`sfx::from_source`), and writes it back (`Encode`,
  `sfx::to_text`).
- The `Sound` kind, a file's header, is also read from a `.sfx.ron`:
  `SfxRon` (id `sfx`, `.ron`) gives `Sound::mono(SAMPLE_RATE)` once the
  numbers read. A name that doesn't end in `.sfx.ron` is an error, so a
  stray RON file among the sounds is reported, not read as one.
  Conflicts are per kind (ADR-0019 §2): no other `Sound` format claims
  `ron`.
- The parameters and their names are ours, written from first principles
  (Nick, 2026-10-19: no code copied line by line). No sfxr name or
  algorithm.

### 3. Rendered once, at load, through the sound hook

- `audio::playable(name, bytes)`, always compiled, is what a sound hook
  hands the platform: with `sfx` on, a `.sfx.ron` whose numbers read
  becomes its WAV (`sfx::to_wav(&sfx::render(..))`, 16-bit mono at
  `SAMPLE_RATE`, through `audio::pcm16_wav`, the crate's one WAV
  writer); every other name, a broken file and every file without the
  feature pass unchanged, and the manifest's check says why.
  `run_game`'s sound files go through it, so a game on the short path
  needs no code of its own.
- `audio::decode_to_wav` renders a `.sfx.ron` too, beside FLAC and MP3,
  so the player's default `load_sound_file` plays one even from a hook
  that didn't render it, and the Playdate's bake reads it like any WAV
  (then ADPCM, ticket 3029).
- `audio::format_problem` on a `.sfx.ron`: bytes that are a WAV (a hook
  rendered them) are checked as a WAV; other bytes are read as numbers,
  and the first problem is given with its line (`line 3: hz 30000 is
  over 12000`). The manifest accepts a `.sfx.ron` file name beside
  `.ogg`, `.wav`, `.flac` and `.mp3`.
- There is no live synth: the mixer and the backends don't change.

### 4. Integers, for byte equality and `no_std`

- The render is integers only, with checked ranges and no table: a pitch
  in 1/256 Hz in `i64` (the slide, then the step, then a triangle
  vibrato), a `u32` phase that wraps once a cycle, the waves from the
  phase (the sine a parabola with one refining step, within 66 of
  `32 767 × sin`), noise a new `SimRng` level every half cycle, and an
  envelope gain of 0 to 65 536. A number out of range is clamped into it
  before rendering, so no definition overflows or renders more than
  5 seconds.
- So the same file gives the same bytes on every platform: the ten
  presets' WAV hashes are rows of `crates/assets/tests/golden/sfx.ron`,
  checked on x64, ARM64 and wasm32 (ticket 2116).
- This is not the simulation's float rule: audio is not the simulation.
  It is for equal bytes and the Playdate. A live synth (ADR-0014) may use
  floats.

### 5. The pack keeps the file's own bytes

A pack holds a `.sfx.ron` as it holds any sound, as its own bytes
(`docs/formats.md`, *The pack*): the hook renders it after reading.
Nothing is baked to a WAV in the pack, and `pack::VERSION` doesn't move.

### 6. Presets as data

Ten `.sfx.ron` files in `crates/assets/src/sfx/presets/`, embedded with
`include_str!` (`sfx::PRESETS`, `sfx::preset(name, seed)`): `blip`,
`tick`, `select`, `cancel`, `coin`, `jump`, `hit`, `explosion`,
`power_up` and `laser`. Seed 0 is the file; another seed moves the
pitches by up to 10 % and `hold_ms` by up to 20 %, in range, so a cue's
variants can be seeds of one preset. The templates' four menu sounds are
copies of `blip`, `select`, `cancel` and `tick`.

## Consequences

- A new game sounds in its menus from the start, with no code and no
  audio file: four `.sfx.ron` files and an `audio.ron` in its `files/`.
- A sound is a text file an agent can write and `cargo shuyi check`
  checks like any asset, with the line of each problem.
- Rendering at load costs a few milliseconds per sound once, and memory
  of the WAV (at most 441 KB, 5 s of mono).
- Two formats claim `.ron` for two kinds; a RON file named otherwise is
  not a sound. `shuyi import` without `--kind` reads a `.sfx.ron` as
  both kinds and says so.
- The waves are few and fixed (no filter, no duty sweep). A richer sound
  is a recorded file or, later, the synth.
- A game that names a `.sfx.ron` without turning on `sfx` fails its own
  load with a message that says what to do; its tools pass, as they turn
  the feature on themselves.

## Alternatives considered

- **A live synth on the audio thread** — needs the in-house mixer
  (3030) in every backend and a sound changed while it plays; out of
  scope, and the bytes would differ by backend.
- **Floats in the render** — simpler, but not the same bytes on every
  platform, so no golden hash, and slow on the Playdate.
- **A sine table** — a second sine table beside the core's `Angle`
  (2335); the parabola is within 0.2 % with none.
- **Baking a `.sfx.ron` into a WAV in the pack** — a second form of the
  same file to keep equal, and `pack::VERSION` moves; rendering at load
  costs little.
- **An `sfx` feature of `shuyi`** — `check-modules` reads every feature
  of `shuyi` as a module, which this is not: a format is a feature of
  `shuyi-assets` (ADR-0009 §4).
- **Only `SfxDef`, with each hook calling `render`** — then the manifest's
  `Sound` check and a game's own format couldn't read a `.sfx.ron` as
  the sound it is.
