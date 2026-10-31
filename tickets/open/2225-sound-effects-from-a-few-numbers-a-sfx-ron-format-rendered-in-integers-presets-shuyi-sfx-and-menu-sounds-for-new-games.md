---
id: "2225"
title: "Sound effects from a few numbers: a .sfx.ron format of the Sound kind rendered in integers at load, presets, shuyi sfx, and menu sounds for new games"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: todo
blocked_by: ["2511"]
nick_input: none
completed:
---

# 2225 — Sound effects from a few numbers

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the small-game tools beside them.

**What the benchmarks have.**

- PICO-8 and TIC-80 ship a sound editor where a sound is a few numbers
  (a wave, a pitch, a speed, an effect), so a small game has sound with
  no audio file
  (https://www.lexaloffle.com/dl/docs/pico-8_manual.html).
- Godot makes sound in code with `AudioStreamGenerator`
  (https://docs.godotengine.org/en/stable/classes/class_audiostreamgenerator.html).
  Unreal has MetaSounds graphs, and Unity fills buffers in
  `OnAudioFilterRead`. Bevy has nothing built in. None of the four has a
  text file of a sound that an agent can write.
- The sfxr family (sfxr, jsfxr) makes game sounds from presets ("coin",
  "jump", "hit"), and jsfxr keeps a sound's parameters as JSON.

**What Shuyi has today.**

- **New games are silent.** The blank template's `main.rs` calls
  `shuyi::prelude::run_game`, whose `basic_config` hands the platform
  a `no_sound` reader (`crates/app/src/lib.rs` line 326), and
  `BasicData` loads `AudioManifest::default()`
  (`crates/ui/src/basic.rs` line 118). The grid template keeps its own
  `no_sound` in `main.rs` (line 26) and `AudioManifest::default()` in
  `lib.rs` (line 126). Yet the engine's menus and cursors play four
  cues: `ui_move`, `ui_select`, `ui_cancel` and `ui_cursor`
  (`AudioConfig::DEFAULT`, `crates/ui/src/audio.rs` line 69, the
  default of `GameData::audio_config` in `crates/ui/src/ctx.rs` line
  98).
- **The one game that has them writes them by hand.**
  `examples/hello-grid/src/sound.rs` builds a WAV header and sine blips
  in `f32`, with four `#[allow]` casts (lines 74 to 82). An agent making
  a game would write the same hundred lines.
- **How sound reaches the speaker.** The `Sound` kind
  (`crates/assets/src/audio/formats.rs` line 24) is a file's header,
  read by `Wav` and `OggVorbis`. The platform plays the bytes a game's
  `Files::sound` hook gives (`crates/assets/src/audio.rs` line 194;
  `Audio::load` in `crates/app/src/audio.rs` line 101 hands them to the
  backend). The manifest check accepts only `.ogg` and `.wav` names
  (`Check::sound`, `crates/assets/src/audio.rs` line 438), and
  `format_problem` (line 218) picks the check by the extension.
  `AudioManifestRon` reads a manifest's sound files raw from the bundle
  beside it and checks them with `format_problem`.
- **From 2511 (done):** the blank template is on the short path:
  `BasicData` loads its files and `run_game` runs it, with an empty
  manifest and silent sound functions. It left the sounds to this
  ticket.
- **Planned:**
  - MIDI and a synth (2205, 2206, parked: music, floats, a sound bank
    the game brings); more decoders (2203, no encoding); decoded
    `Pcm` samples, and maybe a kind of them, in the parked in-house
    sound (2210). Nothing makes sound effects from data.

**Why it matters.**

- Visions of Shuyi ships its own recorded sounds and doesn't need this.
  It is for the other games: a game made with `shuyi new` should sound
  in its menus from the start, and a push, a coin or a hit should be a
  few numbers in a text file, checked like any asset.
- The end goal (Nick, 2026-10-19, `docs/decisions.md`): an agent asked
  for a small game makes it quickly and in few lines. Agents can't make
  good WAV files; they can write a dozen numbers. Until real time is
  unparked, that means turn-based games such as the tutorial's box
  pushing (2803).
- Rendering in integers gives the same bytes on every platform and
  without `std`. So a sound's hash can be a fixture, and the Playdate's
  build turns it into ADPCM like any WAV (3029).

**The shape.**

- It is a format, so it is a cargo feature of `shuyi-assets`, off by
  default, as ADR-0009 §4 says of formats. The templates turn it on.
- A sound is rendered once, when the game loads it, through the game's
  `Files::sound` hook, as Hello, Grid's blips are today. There is no
  live synth, so the mixer and the backends don't change. In
  `shuyi-app`, only `run_game`'s sound reader (2511) does.
- Integers are chosen for byte equality and `no_std`, not because of
  the simulation's float rule: audio is not the simulation, and
  ADR-0014's MIDI synth uses floats because it plays live.
- The parameters and their names are our own, written from first
  principles (Nick, 2026-10-19: no code copied line by line).

## Nick input

None. Nick's "Split by group" (2026-10-20) put this after the MVP, not
parked.

## Scope

**In:**

- **An ADR** (the `write-adr` skill, the next free number): `.sfx.ron`
  as the `sfx` feature of `shuyi-assets`; two kinds, as an image is two
  (ADR-0019 §6); rendered at load through the sound hook
  (`audio::playable`); integers for byte equality and `no_std`; the
  pack keeps the file's own bytes; the names are ours. It also says
  that a game turns `sfx` on in its own `Cargo.toml`: its tools turn it
  on for themselves through `shuyi-cli`, so `cargo shuyi check` passes
  without it, and only the game's own load then fails, with the message
  that names the feature.
- **`shuyi_assets::sfx`** (`crates/assets/src/sfx.rs`), behind a new
  feature `sfx = []` in `crates/assets/Cargo.toml`. `sfx` joins `png` in
  the features of the crate's dev-dependency on itself, so `cargo test
  -p shuyi-assets` always tests it. It is `no_std` with `alloc`, uses no
  float, and adds no dependency (`shuyi_core::SimRng` and `ron` are
  already there).
  - `SfxDef`, the kind `"sound effect"` (`AssetKind::NAME`), with
    `Serialize`, `Deserialize`, `Clone`, `PartialEq` and `Debug`, read
    with `deny_unknown_fields`:
    - `wave: Wave`: `Square(duty: u8)` (percent, 1 to 99), `Triangle`,
      `Saw`, `Sine` or `Noise`;
    - `hz: u32` (20 to 12 000), and `to_hz: u32` (same range, default
      `hz`): the pitch slides in a straight line from one to the other;
    - `attack_ms`, `hold_ms`, `fade_ms: u32` (`attack_ms` defaults to
      0): the loudness rises, holds, then falls to nothing. Their sum,
      the length, is 1 to 5 000 ms (`MAX_MS`);
    - `volume: u8` (0 to 100, as a cue's volume, `audio::MAX_VOLUME`);
    - `vibrato: Option<Vibrato>`, `Vibrato { hz: u8, percent: u8 }`
      (1 to 50 each): the pitch wobbles by `percent` of itself;
    - `step: Option<Step>`, `Step { at_ms: u32, percent: u16 }`
      (`at_ms` inside the length, `percent` 10 to 400, and
      `max(hz, to_hz) × percent / 100` at most 12 000): from `at_ms`
      the pitch is multiplied by `percent / 100` (a coin's jump up);
    - `seed: u64` (default 0): the noise's luck.
  - `sfx::from_source(file, source) -> Result<SfxDef, Vec<ContentError>>`,
    every problem with its line (`error::line_of`, as
    `audio::from_source` finds a cue's line).
  - `sfx::render(&SfxDef) -> Vec<i16>`, mono at `audio::SAMPLE_RATE`:
    - `n = SAMPLE_RATE × length_ms / 1000` samples;
    - the pitch of sample `i`, in 1/256 Hz, in `i64`: the slide, then
      the step, then the vibrato (a triangle at `vibrato.hz`);
    - a `u32` phase that wraps once a cycle, moved by
      `pitch × 2^32 / (SAMPLE_RATE × 256)`, computed in `u64`;
    - the waves, from the phase, in −32 767 to 32 767: the square by
      its duty, the saw, the triangle, the sine as an integer parabola
      with one refining step (no table, so no second sine table beside
      2335's `Angle`), and the noise as a new level from
      `SimRng::new(seed)` every half cycle;
    - the envelope as a gain of 0 to 65 536, then the volume;
    - checked or saturating arithmetic, so no in-range `SfxDef`
      overflows, and no `#[allow]` (`AGENTS.md` rule 6).
  - `sfx::to_wav(&[i16]) -> Vec<u8>`: a 44-byte header, 16-bit mono
    PCM at `SAMPLE_RATE`, the form `audio::format_problem` accepts.
  - **Presets**, as data: ten `.sfx.ron` files in
    `crates/assets/src/sfx/presets/`, embedded with `include_str!`:
    `blip`, `tick`, `select`, `cancel`, `coin`, `jump`, `hit`,
    `explosion`, `power_up` and `laser`. `sfx::PRESETS` lists their
    names. `sfx::preset(name, seed) -> Option<SfxDef>`: seed 0 is the
    file as written; another seed moves `hz` and `to_hz` by up to 10 %
    and `hold_ms` by up to 20 % with `SimRng::new(seed)`, and sets
    `seed`, so a cue's variants can be several seeds of one preset.
- **`audio::playable(name: &str, bytes: Bytes) -> Bytes`**
  (`crates/assets/src/audio.rs`), always compiled. With the `sfx`
  feature, a name that ends in `.sfx.ron` whose bytes read as an
  `SfxDef` gives the WAV of `sfx::to_wav(&sfx::render(..))`. Any other
  name, a file that doesn't read, and every file without the feature
  give `bytes` unchanged (the manifest check reports why). Every sound
  hook goes through it. So `run_game` needs no feature of its own: a
  game that turns `sfx` on gets its files rendered, as
  `register_standard` registers `Png` when `png` is on.
- **Two formats**, registered by `Formats::register_standard` under
  `#[cfg(feature = "sfx")]`, as `image::Png` is under `png`
  (`crates/assets/src/format.rs` line 486):
  - `SfxDefRon` (id `sfx`, extension `ron`) reads an `SfxDef`, and
    implements `Encode` with RON's pretty writer (as
    `crates/ui/src/input/bindings.rs` line 498 uses it);
  - `SfxRon` (id `sfx`, extension `ron`) reads the `Sound` kind:
    `Sound { channels: 1, sample_rate: SAMPLE_RATE }` once the file
    reads as an `SfxDef`. A name that doesn't end in `.sfx.ron` is an
    error that says so, so a stray RON file in a folder of sounds is
    reported, not read as a sound. Conflicts are per kind (ADR-0019 §2),
    and no other `Sound` format claims `ron`.
  - The registry's test (`crates/assets/src/format/tests.rs` line 644)
    expects `["wav", "ogg-vorbis", "sfx"]` for `Sound`, `["sfx"]` for
    `SfxDef`, and 13 kinds (12 today).
- **The manifest** (`crates/assets/src/audio.rs`):
  - `Check::sound` accepts a `.sfx.ron` name; its message becomes "must
    be .ogg, .wav or .sfx.ron";
  - `format_problem` on a `.sfx.ron` name: bytes that are a WAV (a hook
    that rendered them) are checked as a WAV; other bytes are read with
    `sfx::from_source`, and its first problem is given with its line
    (`line 3: hz 30000 is over 12000`). Without the `sfx` feature, the
    problem is "a .sfx.ron sound needs the sfx feature of
    shuyi-assets". That branch is a private function that takes the
    `.sfx.ron` reader as an `Option` (`None` without the feature), so a
    unit test covers the message with the feature on. So
    `AudioManifestRon`, which reads the file's own text from the bundle,
    and a game's loader, which reads through its hook, both check it.
- **Menu sounds in every new game.** In each template's
  `crates/game/files/`:
  - `sfx/move.sfx.ron`, `sfx/select.sfx.ron`, `sfx/cancel.sfx.ron` and
    `sfx/cursor.sfx.ron`, from the presets `blip`, `select`, `cancel`
    and `tick`;
  - `audio.ron`, a manifest with the four cues of
    `AudioConfig::DEFAULT`, each one file, `credit: Own`.

  How each template plays them:
  - **The `base` part (`templates/shuyi-default/base`, 2501), on
    2511's short path.** `BasicData::load`
    (`crates/ui/src/basic.rs`) reads `audio.ron` through the registry
    when the bundle has it, and the empty manifest otherwise.
    `run_game`'s sound `Files` (`crates/app/src/lib.rs`) read the
    installed bundle through `audio::playable`, in place of 2511's
    silent ones. So every game on the short path gets its sounds from
    its files, with no code of its own: 2512's cap allows no new code
    lines in the minimal template. **From 2003 (done):** the blank
    template's root `[workspace.dependencies]` has only `shuyi`,
    `shuyi-cli` and `include_dir`, and its game crate depends on
    `shuyi` alone. So put `shuyi-assets = "{{engine.assets}}"` back
    in the base part's root `Cargo.toml`, as the grid part's piece has
    it (`shuyi new` merges the two, and the same value twice is
    fine), and turn the feature on in
    `crates/game/Cargo.toml`: `shuyi-assets = { workspace = true,
    features = ["sfx"] }`. If nothing in the crate names
    `shuyi_assets`, list it under `[package.metadata.cargo-machete]
    ignored` with a comment that it is there for the feature. Don't
    give `shuyi` an `sfx` feature instead: `cargo xtask check-modules`
    reads each of its features as a module and fails without a
    `shuyi-sfx` row in `docs/modules.md`.
  - **The grid stage** (`templates/shuyi-default/stages/top-down-grid`,
    **from 2501, done**): it is on the short path, so `BasicData::load`
    and `run_game` play its sounds as the base's, and it needs no code
    of its own for them.
  - A test in the base part's `crates/game/tests/it/main.rs`, which
    every composed game runs: every cue of
    `AudioConfig::DEFAULT` is in the game's manifest, and
    `format_problem` accepts what `audio::playable` gives for each of
    its files.
- **Hello, Grid** (`examples/hello-grid`): its four blips become
  `files/sfx/*.sfx.ron` files, and `src/sound.rs` keeps `manifest()`
  and `files()` over them through `audio::playable`. The float `wav`
  function, its `#[allow]`s and its test go. Its `Cargo.toml` turns on
  `sfx`. `check --baked` still holds: the pack keeps a `.sfx.ron` as it
  keeps any sound, as its own bytes (`docs/formats.md`, *The pack*).
- **`shuyi sfx`**, a contract op that needs no game (like `doctor`), in
  `crates/cli/src/contract/sfx.rs`:
  - `shuyi sfx <preset | file.sfx.ron> [--seed N] [--out <file.sfx.ron>]
    [--wav <file.wav>] [--json]`;
  - `SfxInput { source, seed, out, wav }` and `SfxOutput { presets,
    text, length_ms, samples, peak, problems, written }`: the preset's
    or the file's text (from `SfxDefRon`'s `Encode`), its length, its
    loudest sample, and the files written. `--wav` writes the render so
    a person can listen;
  - a file with problems answers `ok: false` with them (file, line,
    message); an unknown name answers an error that lists the presets;
  - `Op::Sfx` in `crates/cli/src/contract.rs` (`Op::ALL`, `name`, the
    schema pair in `schemas()`); `without_project` in
    `crates/cli/src/contract/cli.rs` runs it as it runs `doctor`, and
    its tests name it among the ops without a project;
    `sfx.input.json` and `sfx.output.json` under `crates/cli/schemas/`,
    `contract::VERSION` raised by one and its line in `versions.json`
    (`docs/contract.md`, *The schemas*);
  - one row in each template's `AGENTS.md` command table: `cargo shuyi
    sfx coin --out crates/game/files/sfx/coin.sfx.ron`.
- **`shuyi import`:** `shuyi-cli` turns on `shuyi-assets`' `sfx`
  feature. `Importers::standard()`
  (`crates/cli/src/contract/import.rs` line 170) adds the `SfxDef` kind
  with `kind_with_text`, so `shuyi import x.sfx.ron --kind "sound
  effect" --print` prints the file's text. Without `--kind`, the file
  reads as both `sound` and `sound effect`, and the answer's limits say
  so, as for any file of two kinds.
- **The `no_std` build:** CI's `no-std` job
  (`.github/workflows/ci.yml` line 175) and the `run-gates` skill
  (line 64) also build `shuyi-assets` with `--no-default-features
  --features sfx`, beside today's line without it.
- **Docs:** `docs/formats.md` (both formats in the built-in table, what
  a `.sfx.ron` reads, with an example file, the preset names, the WAV
  and OGG Vorbis entry at line 115 that says a sound's content must
  match its extension, and that a game turns `sfx` on itself);
  `docs/contract.md` (the `sfx` row, and the templates' menu sounds).
- **Either order:** where a ticket below is still open, this PR adds
  its part to that ticket's file.
  - **The templates.** 2511 (MVP, done first) makes the blank template
    the minimal one: its sounds go through `BasicData` and `run_game`,
    as above. **From 2501 (done):** the sound files and `audio.ron` go
    in the built-in template's `base` part, and its grid part, on the
    short path too, needs nothing more.
  - **2512.** If it is done, the minimal template's code count doesn't
    move: its sounds are data, and their code is in the engine. If not,
    nothing to do.
  - **From 1134 (done):** its `shuyi-new-game` skill
    (`skills/shuyi-new-game/SKILL.md`) gets a short section on sound
    effects: a cue names a `.sfx.ron` file, `shuyi sfx <preset> --out
    <file>` writes one, `--wav` renders it to listen to, and which
    fields to change. `check-skills` passes.
  - **2803.** If it is done, its step `08-polish` plays `.sfx.ron`
    sounds made with `shuyi sfx`, and its page says how. If not, 2803
    uses them.
  - **3029.** If it is done, its Playdate export renders a `.sfx.ron`
    with `sfx::render` before the ADPCM step. If not, 3029 does it.
  - **2003.** `docs/modules.md` lists the `sfx` feature with the
    format plugins, as planned: set its row to ready.
  - **2234** (JPEG, WebP, QOI and JPEG XL drawn, FLAC and MP3
    played). If it is done, `Check::sound` already takes `.flac` and
    `.mp3`: add `.sfx.ron` beside them, with one message that lists
    all five names ("must be .ogg, .wav, .flac, .mp3 or .sfx.ron");
    put the `.sfx.ron` branch of `format_problem` beside its `.flac`
    and `.mp3` ones; and make `sfx::to_wav` call its
    `audio::pcm16_wav` with one channel, so the crate has one WAV
    writer. If not, nothing to do (2234 adds `.flac` and `.mp3`
    beside `.sfx.ron`, and makes `sfx::to_wav` call its writer).
  - **MCP.** `shuyi mcp` (1133, done) makes its tools from `Op::ALL`; a new operation needs its arm in `mcp::host::one_off` and in `mcp::description`, which the compiler asks for, and is in `contract::cli::runs_the_game` if it runs the game (so it takes `project` and runs in the game's tool program).
  - **From 2116 (done):** the presets' WAV hashes are rows of
    `crates/assets/tests/golden/sfx.ron` (through
    `shuyi_assets::golden`, `golden!("tests/golden/sfx.ron")` in a test
    with `golden` in its name), not a table of this ticket's own,
    blessed with `cargo xtask golden --bless`, and CI's `determinism`
    job checks them on wasm32, since `sfx` is in the features of
    `shuyi-assets`' self dev-dependency (above). The two formats also
    get rows in `crates/assets/tests/golden/formats.ron` (`sound: sfx`
    and `sound effect: sfx`), a `rows.format(…)` line each in
    `crates/assets/src/format/golden_tests.rs`: its guard
    `missing_rows` covers `Sound` and fails without the first, and
    `SfxDef` joins its list of covered kinds (unless 2515 has moved
    the guard onto `Formats::kinds()`).
  - **2378** (cue rules, part two, which adds the `Sound` kind's
    `length_ms` and `sound_length_ms`). If it is done, `SfxRon` fills
    `length_ms` with the sound's length (`n × 1000 / SAMPLE_RATE`), and
    `sound_length_ms` gives a `.sfx.ron`'s length whether its bytes are
    its text or the rendered WAV. If not, nothing to do (2378 does it).
  - **2020.** If it is done, add `sfx` to the `fuzz` feature's list in
    `crates/assets/Cargo.toml` (its skills' line), and have
    `fuzz::formats`' path function give the two `sfx` formats
    `a.sfx.ron` in its walk of every format alone, since `SfxRon`
    refuses any other name. If not, 2020 does both.
  - **2736, 2737 and 2738** (the parked gallery examples). Each gives
    its example sounds from the presets if this is done; if not, the
    example ships silent, its README says so, and the sounds are left
    to this PR. If none has landed, nothing to do. If one has, this PR
    gives its example the sounds and drops that README line:
    - the match-3 (`examples/match-three`, 2736's part 2 ticket) and the
      card battle (`examples/card-battle`, 2737): the four menu cues as
      the blank template has them, `files/sfx/*.sfx.ron` and
      `files/audio.ron`, with the `sfx` feature on. Both are on the
      short path, so no code changes. If 2736 is done and its part 2
      ticket is still open, that ticket's *Sounds* item becomes these
      files;
    - the brick-breaker (`examples/bricks`, 2738): a bounce, a break and
      a lost-life sound from the presets, played on its `Bounced`,
      `Broke` and `LifeLost` events.
  - **2739** (the snake example). If it is done,
    `examples/snake` gets an eat and a death sound from the presets,
    played on its `Ate` and `Died` events, and its README drops the
    line that says it has none. If not, nothing to do: 2739 uses the
    presets.

**Out (do not do):**

- an editor or a player for sounds;
- music, sequences and tracks (2205, 2206, parked), and a beat clock
  (2365, parked);
- a live synth: a sound changed while it plays, rendering on the audio
  thread, filters and echoes (the in-house mixer is 3030, parked);
- more waves, filters, or a duty that sweeps;
- baking a `.sfx.ron` into the pack's binary form (the hook reads it
  raw, like a WAV), rendering it to a WAV inside the pack, and an
  `SfxDef` fixture in the pack's tests and pinned binary form
  (`crates/assets/src/pack/tests.rs`), so `pack::VERSION` stays;
- a decoded-samples kind (`Pcm`): 2210 (parked) decides whether to add
  one, and if it does, adds a `.sfx.ron` format of it too;
- cue rules: variants without repeats and random volume (2345), copy
  caps, priority and a voice limit (2378);
- a fuzz target of its own for the two formats: 2020's harness walks
  every registered format (the *Either order* item above);
- the GBA's tone channels: 3027 (parked) leaves them out too, until a
  game wants them;
- code or parameter names copied from sfxr or any other synth.

## Implementation steps

1. Read ADR-0007, ADR-0009 §4, ADR-0019, `crates/assets/src/audio.rs`,
   `crates/assets/src/audio/formats.rs`, Hello, Grid's `src/sound.rs`,
   both templates, and 2511's `BasicData` and `run_game`. Write the
   ADR.
2. Write `SfxDef`, its checks, `from_source` and the two formats, with
   the fuzz-style tests. Add the feature, put it in the self
   dev-dependency, and update the registry's test.
3. Write `render` and `to_wav`, with the unit and property tests.
4. Write the presets and `preset(name, seed)`, and pin each preset's
   WAV hash (`shuyi_core::state_hash`) as a row of
   `crates/assets/tests/golden/sfx.ron` (2116). Tune them by ear with
   the WAVs, and say in the PR how they were checked.
5. Change the manifest check, write `audio::playable`, and add the
   `import` kind with `shuyi-cli`'s feature.
6. Give the templates their sounds: the files, `BasicData::load` and
   `run_game` for the blank template, the long form for the grid
   template, and their tests. `cargo fmt --all` doesn't format
   `templates/`; CI's templates job does, so run `cargo fmt` inside a
   game made from each template.
7. Move Hello, Grid onto `.sfx.ron` files.
8. Write `shuyi sfx`, its schemas and the `AGENTS.md` rows.
9. Write the docs and the `no_std` line, settle the *Either order*
   items, and run the gates.

If the diff is well past 600 lines (presets, fixtures and schemas not
counted) after step 6, ship steps 1 to 6 with step 9, and write steps 7
and 8 as one follow-up ticket (the `write-ticket` skill, After MVP),
blocked by this one. It carries their criteria below.
It also takes the `shuyi sfx` parts of the *Either order* items: step
9 then writes the skill's section (from 1134) and settles 2803's
`08-polish` without the command (a game copies a template's `.sfx.ron`
and edits it), and points their `shuyi sfx` lines at the follow-up's
number, which adds them.

## Acceptance criteria

- [ ] The ADR is merged, and `docs/formats.md` lists `SfxDefRon` and
      `SfxRon`.
- [ ] `cargo build -p shuyi-assets --no-default-features --features sfx
      --target thumbv7em-none-eabihf` passes, in CI's `no-std` job and
      in the `run-gates` skill, beside the build without the feature.
- [ ] Each preset renders to the bytes its row in
      `crates/assets/tests/golden/sfx.ron` pins, on x64, ARM64 and
      wasm32, and rendering twice gives the same bytes.
- [ ] Over a cycle, the sine stays within 66 (0.2 % of 32 767) of
      `32 767 × f64::sin`, in a host test.
- [ ] A game made with `shuyi new` from each template has the four menu
      sounds: its test passes, and CI's templates job passes.
- [ ] In such a game, a `.sfx.ron` with an unknown field or a number
      out of range fails `cargo shuyi check` with the sound file, its
      line and the message.
- [ ] Without the `sfx` feature, a manifest that names a `.sfx.ron`
      gets the message that names the feature (a unit test).
- [ ] `shuyi import x.sfx.ron --kind "sound effect" --print` prints the
      file's text.
- [ ] Hello, Grid's `src/sound.rs` has no float and no `#[allow]`; its
      tests, `check --baked` and its web build pass.
- [ ] `shuyi sfx coin --out x.sfx.ron --wav x.wav --json` matches its
      schema and writes both files; `x.sfx.ron` reads back to the same
      `SfxDef`; `shuyi sfx nope` answers an error that lists the
      presets.
- [ ] If steps 7 and 8 were split off, their follow-up ticket is in
      `tickets/open/`, blocked by 2225, with the two criteria above,
      and the *Either order* items' `shuyi sfx` lines name it.
- [ ] Each *Either order* item is settled: done here, or added to the
      open ticket's file.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `shuyi-assets`:
  - each check (an unknown field, each range, a step past the end or
    too high), with its line;
  - each wave's first samples; the envelope (silent at the start with
    an attack, full in the hold, silent at the end); a slide and a step
    change how often the wave crosses zero;
  - `to_wav`'s header bytes (as Hello, Grid's test checks them today);
  - `audio::playable` renders a `.sfx.ron`, and passes other names and
    broken files through unchanged;
  - `preset(name, 0)` equals its file; another seed stays inside its
    ranges and gives the same `SfxDef` twice;
  - `format_problem` on a `.sfx.ron` as text (with the problem's line),
    as a rendered WAV, and with no reader (the feature's message);
  - the registry's lists of formats and kinds.
- Unit, `shuyi-ui`: `BasicData::load` with an `audio.ron` that names a
  `.wav` (so the test holds with or without `sfx`), and without one
  (the empty manifest).
- Unit, `shuyi-app`: `run_game`'s config reads its sounds through
  `audio::playable`: on a bundle installed in that one test, it gives a
  WAV for a `.sfx.ron`. A dev-dependency on `shuyi-assets` with `sfx`
  lets the test see the feature.
- Property:
  - random bytes, and the presets with a few characters changed, never
    make either format panic (ADR-0007 §5);
  - any in-range `SfxDef` renders exactly `n` samples, none louder than
    `32 767 × volume / 100`;
  - `Encode` then decode gives the same `SfxDef`.
- Golden: the ten presets' WAV hashes.
- Integration: the templates' sound test; Hello, Grid's tests; a
  contract test of `shuyi sfx` (a preset, a file, a broken file, an
  unknown name); the schemas test.

## Completion notes

*(Filled in by the session that completes the ticket.)*
