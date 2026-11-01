---
id: "2020"
title: "Fuzzing in CI: cargo-fuzz over every registered format and the pack, crash inputs kept as regression tests, and a threat model for untrusted files"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2020 — Fuzzing every registered format and the pack in CI

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6 and
Unreal 5.8.

- **Bevy** fuzzes in CI: a job runs `cargo fuzz run` on every target of
  `bevy_ecs`, 100 000 runs each
  (<https://github.com/bevyengine/bevy/blob/main/.github/workflows/ci.yml>).
- **Godot:** in 2026 a heap out-of-bounds write was found in its binary
  resource loader, reached from an untrusted `.res` file, and Godot
  classed it as not a security issue
  (<https://seclists.org/oss-sec/2026/q4/159>). Which files an engine
  treats as hostile is better written down before a report arrives.

**What Shuyi has today:**

- ADR-0007 §5: bytes may come from mods or downloads, so a decoder never
  panics, has size limits, and has "a fuzz test (`cargo fuzz` target or
  a property test over random bytes)". Every format took the property
  test.
- `crates/assets/src/format/builtin_tests.rs`: a property test per
  built-in format (proptest's default 256 cases): random bytes, bytes
  after a format's magic, shaped PNG, one-bit, WAV and Ogg headers, and
  the engine's RON fixtures with up to three characters changed.
- `crates/assets/src/pack/tests.rs`: 64 cases each, among them
  `a_pack_changed_under_a_right_checksum_never_panics`, which makes the
  checksum right again (`resummed`) so a changed index and changed files
  reach `postcard` and `lz4_flex`.
- `format::decode_with` refuses a file over its format's `max_bytes` or
  the registry's `Limits` before decoding (ADR-0019 §4). The pack caps
  its files, their total and the LZ4 ratio (`MAX_FILES`,
  `MAX_UNPACKED_BYTES`, `MAX_RATIO` in `crates/assets/src/pack.rs`).
- The root `Cargo.toml` forbids `unsafe` (`unsafe_code = "forbid"`;
  `shuyi-app` has `deny`, for its web pads). Godot's kind of bug, memory
  corruption, can't come from the engine's own decoders. What is left: a
  panic, memory or time without bound, two results from the same bytes,
  and `unsafe` inside a dependency.

**What it doesn't have:** coverage-guided fuzzing, a `fuzz/` folder or a
CI job (the only mention of `cargo fuzz` is ADR-0007 §5), a folder of
inputs that once crashed, and a threat model. "Never a panic" in
`docs/formats.md` is one rule, and `SECURITY.md` (2005) will be a
disclosure policy.

**What brings more untrusted bytes:** 2207 (layers of a player's files,
for game#1012 and game#1014, the game's mods), 2203 (more decoders
behind features), the parked sound-source and library decoder adapters
(2210, selected by 2902), the parked network messages (1140), and scripts (2101).

**Why it matters:**

- The property tests explore shapes written by hand. libFuzzer follows
  the branches the code takes, keeps the inputs that reach new ones, and
  limits memory and time per input. It finds what the shapes miss: a
  header that asks for a huge allocation, nesting deep enough to
  overflow the stack, a slow path.
- The game first: once mods ship through 2207, a mod's file that crashes
  a decoder crashes the game.
- Other 2D games: any game with mods, shared levels or downloaded
  content reads files it didn't ship.
- The end goal (2026-10-19): an agent that adds a format registers it,
  and the harness covers it with no fuzz code of the agent's own.

## Nick input

None. The license exception below is for a crate that never ships, so
it is technical.

## Scope

**In:**

- **The drivers, in `shuyi-assets` behind a new `fuzz` feature**
  (`crates/assets/src/fuzz.rs`; `#[cfg(feature = "fuzz")] pub mod fuzz;`
  in `crates/assets/src/lib.rs`; `fuzz = ["std", "png",
  "sample-actions"]` in `crates/assets/Cargo.toml`, with a comment
  saying what it is for). They are ordinary Rust on stable, so
  the gates lint and test them (the crate's dev-dependency on itself
  turns the feature on, below), and they sit inside the crate, so they
  reach its private parts without making those parts public. The
  drivers panic on purpose when a check fails, so each public one has a
  `# Panics` section in its doc comment (clippy's pedantic
  `missing_panics_doc`).
  - **Sample types of its own**, private to `fuzz.rs`, for the formats
    of a game's types: a two-variant `Shot` enum (`Serialize`,
    `Deserialize`, `PartialEq`), a `Table { rows: Vec<u8> }` kind, a
    `terrain` function for two terrains (`plain`, `water`), a
    `MapFormat` with two looks, and a `DialogueLimits` with the numbers
    of `dialogue::tests::LIMITS` (that module is test-only, so the
    numbers are written again). `builtin_tests.rs` and `pack/tests.rs`
    keep theirs: their assertions and the pack's pinned snapshot depend
    on them.
  - **`fuzz::fixtures() -> &'static Bundle`**, the files the formats
    name, built once in a `OnceLock` from a table of static bytes with
    `bundle::Layer::table` (paths sorted): a PNG header of 128 × 16 at
    `fonts/atlas.png`, a WAV header at `audio/sfx/tick.wav`, and the
    screen text at `lang/en/ui.ron` and `lang/xx/ui.ron` (the values
    `builtin_tests.rs`'s `fixtures()` holds today). `builtin_tests.rs`'s
    `fixtures()` is removed, and its `decodes` uses this one, so there
    is one copy. Its helpers (`bundle`, `wav`, `png_header`) stay where
    they are: the static table needs none of them.
  - **`fuzz::registry() -> &'static Formats`**, built once in a
    `OnceLock`:
    - `Formats::standard()`, with `png` on, so `image::Png` decodes
      pixels;
    - the formats a game registers, with the sample types:
      `KeymapRon<keymap::sample::Action>`, `MapText::new(<the MapFormat>,
      terrain)`, `CinematicRon<Shot>`, `LangPackRon::new(<the limits>,
      &NoTokens)` and `ron::Ron<Table>`. `Dlg<NoDirective>` is standard
      already; a game's directive type stays with the property test;
    - `pack::Packed<K>` for every engine kind the pack bakes (each kind
      of `every_kind()` in `crates/assets/src/pack/tests.rs`, with this
      module's sample types): today the kinds ADR-0023 §6 lists
      (`ImageInfo`, `Image`, `Sound`, `MusicTrack`, `AudioManifest`,
      `CreditsFile`, `FontAtlasDef`, `PaletteDef`, `Lighting`, `Names`,
      `DialogueFile<NoDirective>`, `GridMap<u8, NoFeature>`,
      `KeymapDef<sample::Action>`, `CinematicFile<Shot>`, `LangPack`,
      `TargetProfile`), the one-bit `PatternMapDef`, which that list
      lacks, and `Table`.
  - **The walk over the registry:** the private `Kind` trait in
    `crates/assets/src/format.rs` gets a method, under
    `cfg(feature = "fuzz")`, that runs each of its kind's formats on the
    bytes through `format::decode_with` (so the limits hold). It takes
    the bytes, a `DecodeCx`, and a function from a format's id and
    extensions to the path to decode it at, or `None` to skip that
    format. It returns, per format run, a crate-private
    `Outcome { kind: &'static str, format: &'static str, sniffed: bool,
    result: Result<(), Vec<ContentError>> }` (`Debug`, `PartialEq`). The
    asset is dropped, since `AssetKind` asks for no `PartialEq`;
    `ContentError` is `PartialEq`. `Formats` gets a crate-private method
    that collects them over every kind, in registration order.
  - **`fuzz::formats(input: &[u8])`**: makes the same pass over
    `input` twice and asserts the two lists of outcomes are equal: the
    same sniff answer, and both decoded or the same errors. A pass is
    two walks:
    - every format alone (`DecodeCx::new()`, at
      `a.<its first extension>`, or `a` when it claims none);
    - then, inside `fixtures()` (`DecodeCx::in_bundle`), the formats
      that read other files a file names, each at the path where it
      finds them. Today there are three: `font-atlas` at
      `fonts/atlas.ron`, `audio-manifest` at `audio/audio.ron`,
      `lang-pack` at `lang/xx/lang.ron`. The path function skips every
      other format in this walk.
    - Every input goes through every format; there is no selector byte.
      That is slower per input, but a seed is a plain file, and a kept
      input still reaches the same format after another is registered
      (an index into the registry would move).
  - The drivers take the registry as an argument inside the crate
    (`fuzz::formats` passes `registry()` to a crate-private function),
    so unit tests can hand them a registry of toy formats.
  - **`fuzz::pack(input: &[u8])`**, with the stamp `"fuzz"`:
    1. `pack::info` and `Pack::open` on `input` as it is (the header,
       version and checksum paths);
    2. `input` read as a plan for a pack, written with `pack::write`
       (stamp `"fuzz"`, no target), changed, its checksum made right
       again with `pack::checksum` (made `pub(crate)`), and opened with
       `Pack::open`. The plan, written in the function's doc comment:
       - the first byte modulo 4 is the number of files, at the paths
         `f0`, `f1`, `f2`;
       - each file is one byte that picks its content (0 is
         `Content::Raw`; any other value `b` is `Content::Baked` under
         the `AssetKind::NAME` of entry `b % n` of the `Packed` list
         above, `n` its length), one byte of length, then that many
         bytes (fewer when `input` ends first);
       - what is left comes in groups of three bytes, an offset
         (`u16`, little-endian) and a mask: each XORs the body's byte
         at `offset % (body length)` (the body is what follows the
         24-byte header), when there is a body.
       Every input gives a pack, the same pack each time, so no seed
       pack is needed;
    3. when it opens, each file's bytes go through the walk above, alone
       at the file's own path.
    Never call `Pack::into_layer`, which leaks on purpose: memory would
    grow per input, and LeakSanitizer reports it.
- **The fuzz crate, `fuzz/`**, outside the workspace: its own
  `[workspace]` table, `publish = false`, `license = "MIT OR
  Apache-2.0"`, `[package.metadata] cargo-fuzz = true`, and its
  `Cargo.lock` committed.
  - `fuzz_targets/formats.rs` and `fuzz_targets/pack.rs`, each one
    `fuzz_target!` calling its driver.
  - Dependencies: `libfuzzer-sys` 0.4 and `shuyi-assets` (by path, with
    the `fuzz` feature).
  - A `.gitignore` for `target/`, `corpus/`, `artifacts/` and
    `coverage/`.
  - It builds on Linux, on a dated nightly, in CI only. Never on
    Windows, never on Nick's machine, never in the gates.
- **Kept inputs:** `crates/assets/tests/fuzz-regressions/formats/` and
  `crates/assets/tests/fuzz-regressions/pack/`, with a `README.md`
  saying what goes there and how files are named.
  - `crates/assets/tests/it/fuzz.rs` (a module of the `it` test
    program, `mod fuzz;` in `tests/it/main.rs`) runs every file of each
    folder through its driver, skipping the README.
  - The crate's dev-dependency on itself (`crates/assets/Cargo.toml`)
    gains the `fuzz` feature, so this runs on stable in `cargo test
    --workspace` on Windows, macOS and Linux, and on Nick's machine.
  - Two inputs to start, so the test isn't empty: an empty file in
    `formats/`, and a pack header with nothing after it in `pack/`.
- **CI: `.github/workflows/fuzz.yml`**, on the pattern of `mutants.yml`.
  - Triggers: `pull_request`, a weekly `schedule` (`0 6 * * 0`, Sunday,
    away from Monday's mutants), and `workflow_dispatch`.
  - `.github/workflows/changes.yml` gets a second output, `assets`: true
    on any other event, and on a pull request that touches
    `crates/assets/`, `crates/core/`, `fuzz/`, `Cargo.lock` or
    `.github/workflows/fuzz.yml`. The fuzz jobs run when it is true and
    are skipped otherwise, which still reports a status.
  - One job per target, `fuzz (formats)` and `fuzz (pack)`, on
    `ubuntu-latest`, with `permissions: contents: read` and every action
    pinned by SHA as the other workflows pin them:
    - a dated nightly set once in the workflow's `env`
      (`FUZZ_TOOLCHAIN: nightly-YYYY-MM-DD`, the newest that builds on
      the day), installed with `rustup toolchain install --profile
      minimal`. A comment beside it says when it moves: when it no
      longer builds the fuzz crate, or cargo-fuzz needs a newer one, to
      the newest nightly that builds, in a PR of its own. It never moves
      with the stable pin in `rust-toolchain.toml`;
    - `cargo-fuzz` 0.13.2 (MIT OR Apache-2.0) installed as a tool, not a
      dependency (`cargo install --locked cargo-fuzz --version 0.13.2`);
    - `cargo fmt --manifest-path fuzz/Cargo.toml --check`: `cargo fmt
      --all` doesn't reach a crate outside the workspace, as with the
      templates;
    - cargo-deny over `fuzz/Cargo.toml` against the root `deny.toml`
      (the `cargo-deny-action` step `ci.yml` pins, with
      `manifest-path: fuzz/Cargo.toml`);
    - seeds, copied flat into `fuzz/corpus/<target>/`, each named by
      its `sha1sum` (as libFuzzer names its own) so two files of one
      name don't collide: for both targets, the kept inputs of
      `crates/assets/tests/fuzz-regressions/<target>/` (not the
      README); for `formats`, also the repository's own asset files:
      `examples/*/files/**`, `templates/*/crates/game/files/**`,
      `targets/*.ron` and `crates/assets/src/keymap/sample.ron`. `pack`
      needs no more;
    - the corpus restored from the newest weekly cache
      (`actions/cache/restore`, a key per target with the run id,
      restored by its prefix);
    - `cargo +$FUZZ_TOOLCHAIN fuzz run <target> fuzz/corpus/<target> --
      -max_total_time=<seconds> -timeout=10 -rss_limit_mb=2048
      -malloc_limit_mb=512`: 60 seconds on a pull request, 900 per
      target on the weekly run (30 minutes in all).
      - 512 MiB is eight times the biggest allocation a small file may
        cause today: a PNG's pixels at `image::MAX_SIDE`, 4096 × 4096 ×
        4 bytes. An input that needs more is a finding.
      - cargo-fuzz's default build keeps debug assertions and overflow
        checks, so an arithmetic overflow is a crash. The job runs on
        64-bit Linux: an overflow that only a 32-bit `usize` reaches
        (the web build's) is not found here.
    - weekly only: `cargo fuzz cmin` on the corpus, then the corpus is
      saved to the cache (`actions/cache/save`; a pull request's run
      never saves);
    - on failure: `fuzz/artifacts/<target>/` is uploaded
      (`actions/upload-artifact`, pinned as in `mutants.yml`).
  - A crash fails the job. It becomes a bug ticket (the `write-ticket`
    skill), and its fix's PR puts the input, minimized with `cargo fuzz
    tmin` when that works, in `fuzz-regressions/`.
- **`deny.toml`:** `libfuzzer-sys` is `(MIT OR Apache-2.0) AND NCSA`
  (crates.io, 0.4.13). NCSA, the University of Illinois/NCSA license, is
  permissive and OSI-approved but isn't on ADR-0002 §8's list, which is
  the list for what ships. Add `{ crate = "libfuzzer-sys", allow =
  ["NCSA"] }` to `[licenses] exceptions`, with a comment: the fuzz crate
  never ships, and NCSA permits commercial use. Check the fuzz crate's
  other dependencies (`arbitrary`, `cc` and theirs) on the day; the
  job's cargo-deny step does it on every run. The workspace's own
  `cargo deny` never sees the fuzz crate.
- **The threat model:** a new section `## Untrusted files` in
  `docs/formats.md`, after *The rules*. Four short lists:
  - **untrusted:** every file a run-time layer brings (2207: mods, a
    player's levels), downloaded content, packs, saves and records a
    player shares, and later network messages (1140);
  - **promised** for registered formats and the pack: no undefined
    behaviour in the engine's decoders (`unsafe` is forbidden in
    `shuyi-assets`); no panic; memory within the documented limits
    (ADR-0019 §4, `MAX_SIDE`, the pack's limits); a decode that ends
    (the fuzz job's 10 seconds per input); the same result from the
    same bytes within a run (across machines: 2116);
  - **not promised:** a pack's authenticity (its checksum catches
    damage, not attackers: ADR-0023); which files a mod may replace (the
    game's mod rules); running untrusted code (a format is a crate
    compiled in, ADR-0007 §2; scripts are 2101's sandbox);
  - **read but not fuzzed here**, each with what tests it today:
    - the backend's decoders: `shuyi-app` hands PNG bytes to macroquad's
      `Image::from_file_with_format` (`crates/app/src/render.rs`,
      `crates/app/src/lib.rs`; `image` 0.24 with `png` 0.17), and sound
      bytes to quad-snd (`crates/app/src/audio.rs`; `audrey` with
      `lewton` and `hound`). For music,
      `crates/app/src/audio/native_music.rs` catches a panic in
      quad-snd's `Sound::load`. 2210 introduces the shared source and
      permitted decoder adapters, with 2573's malformed-input and
      correctness checks; 3079 alone removes the reference backend;
    - files `shuyi-ui` and `shuyi-core` read from storage:
      `Profiles::from_ron` (`crates/ui/src/profiles.rs`), the key
      bindings (`crates/ui/src/input/bindings.rs`) and `SaveHeader`
      (`crates/core/src/save.rs`);
    - session records and replays (`crates/cli/src/contract/replay.rs`).
  - *Never a panic* in *The rules*, and *5. Testing it* under *Writing a
    format plugin*, say a registered format is also fuzzed in CI.
  - A short *Fuzzing* part: what runs where, and how to replay a crash
    on stable: put the input in its folder and run `cargo test -p
    shuyi-assets --test it fuzz`.
- **How a format gets into the fuzz registry**, said in
  `docs/formats.md` and in one line each of
  `.claude/skills/work-ticket/SKILL.md` and
  `.claude/skills/write-ticket/SKILL.md`, beside their "fuzz-style
  test" lines, which stay:
  - a format in `Formats::standard()` is there already;
  - a format of a game's types gets a line in `fuzz::registry()` with a
    sample type;
  - an engine format behind a cargo feature adds that feature to
    `fuzz`'s list in `crates/assets/Cargo.toml`;
  - a format that reads other files a file names joins the walk inside
    `fixtures()`, and those files join `fixtures()`;
  - an engine kind the pack bakes (a line of `every_kind()` in
    `pack/tests.rs`) adds its `Packed<K>` to `fuzz::registry()`.
- **An ADR** (the `write-adr` skill, the next free number) that extends
  ADR-0007 §5:
  - registered formats and the pack are fuzzed in CI, and the property
    tests stay required (they run on stable, everywhere);
  - the fuzz crate lives outside the workspace, on a nightly, in CI
    only;
  - every input goes through every format, and why;
  - kept inputs are replayed on stable;
  - the NCSA exception, and why.
  ADR-0007's row in `docs/adr/README.md` says it is extended by the new
  ADR.
- **Either order:**
  - with 2005, and 1132 (parked) if it lands first: if `SECURITY.md`
    exists, link *Untrusted files* from it; if not, the ticket that
    writes it adds the link;
  - with 2011 (done): add `/fuzz` to `.github/dependabot.yml`'s cargo
    update (as `directories: ["/", "/fuzz"]`);
  - with 2012 (done): `check-toolchain` checks only the stable pins,
    so `FUZZ_TOOLCHAIN` stays outside it. Add one line to its
    `bump-rust` skill's *Don'ts*: the fuzz nightly moves on its own, by
    the comment in `fuzz.yml`;
  - with 2203: formats it has already added behind features join
    `fuzz`'s list; formats it adds later follow the skills' line;
  - from 2116 (done): its `crates/assets/src/format/golden_tests.rs`
    takes its fixtures and inputs from `builtin_tests.rs`; point it at
    `crate::fuzz::fixtures()` when the one in `builtin_tests.rs` is
    removed, and its rows in `tests/golden/formats.ron` stay the same;
  - with 2224: if it is done, `Packed<ProportionalFont>` is in the
    registry (it is in `every_kind()`), the fixture font's page headers
    are in `fixtures()` (they move with the rest of `builtin_tests.rs`'s
    `fixtures()`), and `proportional-font` and `bmfont` join the walk
    inside `fixtures()`, each at its fixture's path, so that walk and
    its unit test have five formats; if not, 2224 adds them;
  - with 2225: if it is done, add `sfx` to `fuzz`'s list, and have the
    path function give the two `sfx` formats `a.sfx.ron` in the walk of
    every format alone, since `SfxRon` refuses any other name; if not,
    2225 adds both;
  - with 2230: if it is done, register `Json<Table>` and a `Csv<T, R>`
    of a small kind and row type of `fuzz.rs`'s own in
    `fuzz::registry()`, and add the `json` and `csv` features to
    `fuzz`'s list; if not, 2230 adds them by the skills' line;
  - with 2515: both add a method to the private `Kind` trait. If 2515
    is done, add this ticket's method beside its `Formats::kinds()`,
    and the registry's coverage test may read the kind names and
    format ids from it; if not, 2515 adds its method beside this one;
  - with 2909 (parked research): if it is done, nothing to do: its
    report says how a video format joins this harness, and the ticket
    that builds one follows it; if not, nothing to do (2909 says how);
  - with 3054 (parked): if it is done, its `LeaderboardsRon` joins the
    registry as the skills' line says (a format in `Formats::standard()`
    is there already); if not, nothing to do (3054 adds it that way).

**Out (do not do):**

- OSS-Fuzz and ClusterFuzzLite: plain cargo-fuzz in Actions, as Bevy
  does.
- Fuzzing on Windows or on Nick's machine. The replay test is what runs
  there.
- Changing or removing the per-format property tests: they stay the
  fast gate on stable. Editing the open tickets that promise a
  "fuzz-style test": their property test still holds.
- A target for the backend's decoders in `shuyi-app`. The threat model
  names them. 2210's decoder adapters retain registry fuzz-style tests
  and input limits; third-party implementation fuzzing and 2573's
  comparative decoder probes have distinct coverage, stated explicitly.
- Network messages: 1140 adds its own target when messages exist.
- Files `shuyi-ui` and `shuyi-core` read from storage, and records and
  replays: if the threat model finds one that no random-bytes test
  covers, write a ticket for it.
- Signing packs (ADR-0023), and decoders giving the same result on
  every machine (2116).
- Scripts (2101's sandbox).
- The run-time layers that bring a player's files (2207). It should
  land before game#1014 ships mods to players, but neither ticket waits
  for the other.

## Implementation steps

1. Read ADR-0007 §5, ADR-0019 §4, ADR-0023,
   `crates/assets/src/format.rs`, `format/builtin_tests.rs`, `pack.rs`,
   `pack/tests.rs`, and `.github/workflows/mutants.yml` and
   `changes.yml`.
2. Add the `fuzz` feature (turned on in the crate's dev-dependency on
   itself), the walk on `Kind` and `Formats`, `pack::checksum` as
   `pub(crate)`, and `crates/assets/src/fuzz.rs` with its sample types,
   `fixtures()`, `registry()`, the two drivers and its unit tests.
   Remove `builtin_tests.rs`'s `fixtures()` and point its `decodes` at
   `crate::fuzz::fixtures()`.
3. Add the replay test and the two starting inputs.
4. Add `fuzz/` in the layout `cargo fuzz init` makes, with the two
   targets.
5. Add the `assets` output to `changes.yml`, write `fuzz.yml`, and add
   the `deny.toml` exception. Let the PR's jobs run.
6. Fix what they find in this PR, each fix with its input kept in
   `fuzz-regressions/`. A fix of more than about 30 lines goes in a bug
   ticket of its own (the `write-ticket` skill) and its own PR first,
   with the input in that PR's test; this PR waits for it, then keeps
   the input in `fuzz-regressions/` too. Say so in the PR.
7. Show the harness catches a crash: a throwaway commit makes one
   format panic on input that starts with `FUZZME` and adds a file
   `crates/assets/tests/fuzz-regressions/formats/planted` holding
   `FUZZME`, which the job takes as a seed. The PR's fuzz job fails and
   uploads the input (the stable replay test fails too). Revert the
   commit, and link the failed run in the PR description.
8. Write the docs, the ADR, its README row and the skills' lines. Run
   the gates.

Size: if the diff runs well past 600 changed lines, move the pack
target, the weekly run and the threat model's docs to a follow-up
ticket, and keep the harness, the formats target, the PR job and the
regression folder here.

## Acceptance criteria

- [ ] `fuzz (formats)` and `fuzz (pack)` ran on the PR for 60 seconds
      each and passed (links in the PR description).
- [ ] The planted panic failed the PR's job and uploaded its input
      (link in the PR description), and the commit that planted it is
      reverted.
- [ ] The fuzz registry holds every format of `Formats::standard()`
      (with `png`), the game-registered formats listed above, and
      `Packed<K>` for each kind `pack/tests.rs`'s `every_kind()` bakes
      (unit test comparing kind names and format ids).
- [ ] A toy format that panics on some bytes, and a toy whose second
      decode differs from its first, each make the formats driver panic
      (unit tests with `catch_unwind`, on a registry they build).
- [ ] `builtin_tests.rs` has no `fixtures()` of its own; its property
      tests use `fuzz::fixtures()`.
- [ ] Every file in `crates/assets/tests/fuzz-regressions/` runs
      through its driver in `cargo test --workspace`, on stable.
- [ ] `fuzz/` is outside the workspace (its own `[workspace]` table),
      with its `Cargo.lock` committed, and nothing in the gates builds
      it.
- [ ] `fuzz.yml` has the weekly `schedule` and `workflow_dispatch`; the
      weekly run differs from the PR run only in its time, `cmin` and
      the saved cache.
- [ ] cargo-deny over `fuzz/Cargo.toml` passes in the job, with the
      NCSA exception commented in `deny.toml`.
- [ ] `docs/formats.md` has *Untrusted files* with the four lists, and
      the ADR is in `docs/adr/` with its row.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/assets/src/fuzz.rs`): the registry's coverage; the
  panicking and the nondeterministic toy formats; the formats driver
  runs the three bundle formats a second time inside `fixtures()` and
  no other (toy formats with the ids `font-atlas`, `audio-manifest`,
  `lang-pack` and one more, which record the paths they decode at);
  `fuzz::pack` turns a few hand-written plans into packs that open
  before any change, and hands each opened file to the walk; every
  fixture constant of `builtin_tests.rs` (`PALETTE`, `ATLAS`,
  `MANIFEST`, `SCRIPT`, `MAP`, `FILM`, `PACK_INFO` and the rest) goes
  through the formats driver without a panic.
- Integration: the replay of `fuzz-regressions/`
  (`crates/assets/tests/it/fuzz.rs`).
- CI: the two fuzz jobs on the PR, and the planted panic's failed run.

## Completion notes

*(Filled in by the session that completes the ticket.)*
