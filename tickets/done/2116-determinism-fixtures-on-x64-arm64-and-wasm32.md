---
id: "2116"
title: "Determinism fixtures: golden tables of a record's hashes and of decoded assets, the same on x64, ARM64 and wasm32 under wasmtime in CI, and a moved value declared as a break"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-29
---

# 2116 — Determinism fixtures on x64, ARM64 and wasm32

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. None of the four promises that a game plays the same on
every platform, so none checks it: their simulations use floats. The
engines that do promise it pin values and check them in CI. Box2D v3
pins the hash of one scene ("falling hinges") and checks it on several
compilers and CPUs on every PR
(<https://box2d.org/posts/2024/08/determinism/>). Rapier has an
`enhanced-determinism` feature for the same bits on every platform, the
web included (1128 uses it).

Shuyi promises it. ADR-0002 §2: "Same seed and same inputs give the
same state, on every platform." ADR-0007 §5: the same bytes give the
same asset on every platform. What checks it today (read on
2026-10-20):

- **Pinned values, on two CPUs.** The `test` job in
  `.github/workflows/ci.yml` runs on `windows-latest` and
  `ubuntu-latest` (x64) and `macos-latest` (Apple silicon, ARM64). So
  these already cross x64 and ARM64:
  - the bytes `state_hash` feeds, byte for byte
    (`crates/core/src/hash/tests.rs`);
  - PCG32's output (`matches_pcg32_reference_output` in
    `crates/core/src/rng.rs`);
  - the pack's binary forms (the insta snapshot in
    `crates/assets/src/pack/snapshots/`);
  - a seeded play of Hello, Grid to its end screen
    (`examples/hello-grid/tests/it/snapshots/it__snapshot__end.snap`).
- **No test runs on a 32-bit target**, and wasm32 is what the web build
  ships (the Pages examples, and the game's web build). The `wasm` job
  builds Hello, Grid for the web and runs no test there. The `no-std`
  job builds for `thumbv7em-none-eabihf` and runs the tests on the
  host. `state_hash`
  (`crates/core/src/hash.rs`) hashes lengths as `u64` "so it is the same
  on 32 and 64 bits", but no test has run it on 32 bits.
- **No record is checked in.** `examples/hello-grid/tests/it/determinism.rs`
  and the contract's `replay` (`crates/cli/src/contract/replay.rs`)
  compare two runs in one process. `replay` says so in its `limits`:
  "what differs from one machine or build to another needs a second
  record made there".
- **No decoded asset's hash is checked in.** The built-in formats' tests
  (`crates/assets/src/format/builtin_tests.rs`) compare within one run.
  ADR-0023 says LZ4's output differs between a 64-bit and a 32-bit
  baker while the assets read back don't. 2204 noted it; no test
  checks it.
- **No shared job.** Many tickets plan their own check "on native and
  web (or the CI equivalent)": 1119 (`Fx`, under `wasm32-wasip1`),
  1128, 2701, 2705, 3101, 2203, 2205, 2707, 2709, 2721, 2727,
  3007, 3019 and 3030. The comparison asks each to use this ticket's
  job instead. Others already point here: 2111, 2114, 2117, 2118, 2119,
  2120, 2121, 2122, 2124, 2225, 2362 and 2735 to 2740. No ticket owns
  the job they assume.
- **No rule** says a moved value is a break. 2010 asks for a `Breaking:`
  line when the public API breaks, 2804 lists those lines first in the
  changelog, and 2005 writes the version policy. None names a replay
  that plays differently.

Why it matters:

- **The game first.** Visions of Shuyi's replays, bots and bug records
  rest on the same-state promise. A record from a player of the web
  build must replay the same on a developer's x64 or ARM64 machine. An
  engine change that moves the game's hashes should be seen and written
  down when it is made, not found at the game's next pin bump.
- **Tests are the review** (`AGENTS.md` rule 6). A pinned value that
  moves fails a test, so the change is deliberate and named.
- **Later work brings the risk.** Fixed-point numbers (1119), rapier's
  floats (1128), scripts (2101), noise (2118) and decoders with floats
  (2203) are where platforms part. One job and one way to pin values
  beats a copy in each ticket.
- **The end goal** (2026-10-19, `docs/decisions.md`): the most
  AI-friendly engine. An agent that debugs a game replays its records,
  so a record must replay the same wherever the agent runs it.

## Nick input

None. What players keep across engine updates (old saves, old replays)
is 2110's and 2111's.

## Scope

**In:**

- **`shuyi_assets::golden`**, behind a new feature `golden`. No new
  dependency, `no_std` + `alloc`, and no I/O, so a crate whose tests
  also run without `std` can use it as a dev-dependency with
  `default-features = false`:
  - `Golden::new(path: &'static str, text: &str) -> Golden`: a table
    from its text, a RON map from row name to value, both strings:
    `{"step 000": "1a2b3c4d5e6f7a8b"}`. A text that doesn't parse is
    reported by `finish`, with its line.
  - `golden!("tests/golden/formats.ron")`, a `#[macro_export]` macro:
    `Golden::new` with the path under the calling crate's
    `CARGO_MANIFEST_DIR`, and the text through `include_str!`. A test
    reads no file at run time, so it runs the same under wasmtime, and
    later on a device. `include_str!` needs the file to exist, so a new
    table starts as a file holding `{}`: its test then fails with every
    row added, and the bless writes it.
  - `Golden::check(&mut self, name: impl Into<String>, value: impl
    Display)`. A name checked twice is a problem `finish` reports.
  - `Golden::finish(self) -> Result<(), GoldenReport>`: it passes when
    every row checked equals the table's and every row of the table was
    checked. `GoldenReport` holds the rows changed (old and new values),
    added and removed, and `table`, the text this run gives.
    `Golden::assert(self)` panics with the report; tests call it.
  - The report's `Display`, for a person or an agent reading a failed
    test:
    1. the table's path and the counts, then one line per row changed,
       added or removed;
    2. what to do. Only added rows: "bless the table with `cargo xtask
       golden --bless`; new rows are not a break". A changed or removed
       row: "this build no longer gives what an earlier one gave. If you
       meant it, bless the table and add a `Breaking:` line naming it to
       the ticket's completion notes. If not, find what moved it.";
    3. the new table, between a line `--- new golden table: <path> ---`
       and a line `--- end of golden table ---`, which the bless step
       reads.
  - `table_text(rows: &BTreeMap<String, String>) -> String`, the one
    writer: `{`, one `"name": "value",` line per row in name order, `}`,
    `\n` line ends and a final newline, names and values written as RON
    strings. A table on disk is always this function's output.
  - The module docs say: what goes in a table (values a simulation, a
    seeded generator or a decoder gives; never the pixels of a look,
    rule 9, and never bytes a compressor may write differently); rows
    never depend on the target or the features; row names sort as meant
    (zero-padded numbers); inputs are fixtures frozen for the table,
    never a file edited for other reasons (such as `targets/*.ron`); how
    to bless; how a game or a new example adds a table; and the break
    rule below.
  - The crate has a module named `ron`: reach the crate as `::ron`, as
    `crates/assets/src/ron.rs` does.
  - `shuyi-assets`'s self dev-dependency gets `golden` beside `png`
    (`features = ["png", "golden"]`), so the module's own tests and
    `golden_tests.rs` build in every test run.

- **Tables for what exists today:**
  - **`crates/assets/tests/golden/formats.ron`**, checked by a new unit
    test module `crates/assets/src/format/golden_tests.rs`, beside
    `builtin_tests.rs`. Its inputs are the ones `builtin_tests.rs`
    already builds: `PALETTE`, `LIGHTS`, `NAMES`, `ATLAS`, `CREDITS`,
    `MANIFEST`, `SCRIPT`, `MAP`, `FILM`, `ENGLISH_UI`, `PACK_INFO`,
    `PACK_UI`, `TABLE`, `wav`, `ogg` (made `pub(crate)`), `ogg_file`,
    `png_file`, `map_text()`, `lang_pack()`, `fixtures()` and
    `bundle()`, and `crate::image::png_header`. A format with no input
    there takes one its own module's tests build: `bitmap.rs`'s
    `sample` and `target.rs`'s `profile`, both private today, made
    `pub(crate)` with their `tests` modules (as `dialogue.rs` does).
    **Either order with 2020:** if 2020 is done, `fixtures()` is
    `crate::fuzz::fixtures()`, which replaced `builtin_tests.rs`'s; if
    not, use `builtin_tests.rs`'s, and 2020 points this module at its
    own when it removes that one.
    - One row per format, named `<kind>: <format id>` (`AssetKind::NAME`
      and `Format::id`, as in `"image: png"` and `"image pixels: png"`):
      the `state_hash` of what the format decodes from its input. Every
      kind these formats decode derives `Serialize`; floats (a
      cinematic's seconds) hash by their bits.
    - The formats: `PngInfo`, `Png`, `OneBitInfo`, `Wav`, `OggVorbis`,
      `OggVorbisTrack`, `AudioManifestRon`, `CreditsRon`,
      `FontAtlasRon`, `PaletteRon`, `LightingRon` (it rounds floats
      with `libm`), `NamesRon` and `TargetRon`; and, with the test
      types `builtin_tests.rs` uses, `Dlg`, `MapText`, `KeymapRon`,
      `CinematicRon`, `LangPackRon` and `Ron<T>`.
  - **A guard**, written as a function over a registry so a test can
    hand it one with a toy format: for each kind the table covers,
    every id `Formats::standard().ids::<K>()` lists has a row. So a new
    standard format of a covered kind fails until it has one.
    **Either order with 2515:** if `Formats::kinds()` exists, walk it
    instead and fail on any standard format with no row. If not, 2515
    moves the guard onto `kinds()`.
  - **`examples/hello-grid/tests/golden/hello-grid.ron`**, checked by a
    new test target `examples/hello-grid/tests/golden.rs`:
    - **A record:** `examples/hello-grid/tests/records/seed-11.ron`, a
      `StageRecord` (`hello_grid::tools`), made once from
      `examples/hello-grid` with `cargo shuyi bots 1 --seed 11 --json`
      (the play's `record` field, pretty-printed). Its play must end:
      if seed 11's doesn't, take the next seed whose play does. It
      holds no stamp: take out the `made` field `bots` writes (2111),
      so a content change shows as rows moving, not as a refusal. The test reads it with `include_str!`, like the tables,
      and replays it with `HelloTools.replay` (`GameTools::replay`, the
      path `shuyi replay` takes, given a `Record { file, text }`). Rows:
      `step 000`, `step 001`, and so on (each step's state hash), and
      `outcome`.
    - **The game's assets from its files:** each asset
      `Hello::load_from` reads, through `hello_grid::formats()` over
      `Bundle::new(vec![hello_grid::layer()])`: `en.ron`, `keymap.ron`,
      `palette.ron`, `intro.dlg`, `stage.map`, `font/atlas.ron`, and
      the `ImageInfo` of `tiles.png` and `font/atlas.png`. One row each,
      `asset <path>`.
    - **The same assets from a pack**, baked in the test (the `bytes`
      of `hello_grid::bake().bake(&formats, &bundle, None)`, as
      `examples/hello-grid/tests/it/pack.rs` bakes them), opened with
      `Pack::open(&bytes, PACK_STAMP)`, and read with the formats
      `bake().register` adds, as `Hello::from_pack` does. Each must
      equal its loose row (`assert_eq!`, no second row). The pack's
      bytes are not pinned. On wasm32 the bake runs a 32-bit LZ4, so
      this checks in CI what ADR-0023 states.
  - The pins that exist (`hash/tests.rs`, `rng.rs`, the path tests, the
    pack's snapshot) stay as they are: the job below runs them on
    wasm32. **From 3006 (done):** its one-bit fixture's bits are pinned
    (`the_fixture_converts_to_the_same_bits_everywhere` in
    `crates/ui/src/frame/onebit/roles/tests.rs`), but ran natively only,
    as its notes say. `shuyi-ui` is in the job's wasm32 run, so the pin
    is checked there with no change.
  - Each test that checks a table has `golden` in its name, so `cargo
    test -- golden` runs them all.

- **Hello, Grid builds without its window**, so its test builds for a
  target that has none:
  - `shuyi-app` and `macroquad` become optional, behind a new default
    feature `window`, and the `[[bin]]` gets `required-features =
    ["window"]`. Only `src/main.rs` uses them.
  - The self dev-dependency becomes `hello-grid = { path = ".",
    default-features = false, features = ["tools"] }`. A dev-dependency
    on `shuyi-assets` with `features = ["golden"]` is added.
  - `build.rs` passes `--allow-undefined` only when
    `CARGO_CFG_TARGET_OS` is `unknown` (the web build, whose shell
    provides the JS plugins). On WASI a missing symbol then fails the
    link with its name, instead of failing in wasmtime.
  - `tools` brings `shuyi-cli` into the wasm32 build. It uses only `std`
    and pure-Rust crates, so it should build there. If it doesn't, make
    the smallest `cfg(target_os = "wasi")` change that builds it, and
    name it in the PR.
  - `cargo shuyi web` (`cargo build -p hello-grid --target
    wasm32-unknown-unknown`), `cargo shuyi check --target all` and
    Pages build with the default features, so they don't change.

- **The tests run under WASI preview 1**, which has no threads, no
  unwinding and no processes:
  - **proptest:** the workspace's `proptest` becomes `{ version = "1",
    default-features = false, features = ["std", "bit-set"] }`. Its
    default `fork` and `timeout` features pull `rusty-fork` and
    `wait-timeout`, and `wait-timeout` 0.2.1 has code for unix and
    windows only, so it doesn't build for WASI. No test uses `fork` or
    `timeout`: the only `ProptestConfig` calls are `with_cases`.
  - **Tests that need threads or unwinding** get
    `#[cfg_attr(target_os = "wasi", ignore = "needs threads and
    unwinding, which WASI preview 1 lacks")]`. Three are known today:
    `story_only_hides_the_test_layers_from_this_thread_until_it_returns`
    and `reading_makes_this_thread_read_another_bundle_until_it_returns`
    in `crates/assets/src/bundle.rs` (`catch_unwind` and
    `std::thread::spawn`), and `long_waits_are_refused` in
    `crates/ui/tests/it/scripts.rs` (`catch_unwind`). Another test the
    job finds gets the attribute only for the same reasons, and the PR
    names each one.
  - libtest reports `#[should_panic]` tests as ignored on wasm targets
    by itself. If the pinned toolchain (1.98.1) doesn't, they get the
    same attribute.
  - **A test that fails on wasm32 for any other reason is a determinism
    bug.** Fix it in this PR if the fix is small; if not, stop, write a
    bug ticket (`write-ticket` skill) and say so. Never ignore it.
  - wasm32 has `panic = "abort"`, so a failing test ends its whole test
    binary. The job runs with `--nocapture`, so the message shows, and
    with `--no-fail-fast`, so the other binaries still run.

- **The CI job `determinism`** in `.github/workflows/ci.yml`, on
  `ubuntu-latest`, with `needs: changes` and `if:
  needs.changes.outputs.code == 'true'`, and added to `ci-result`'s
  `needs`:
  - checkout (`persist-credentials: false`, `fetch-depth: 2`), `rustup
    show`, `rustup target add wasm32-wasip1` (as the `no-std` job adds
    its target; `rust-toolchain.toml` keeps its list), and
    `Swatinem/rust-cache`;
  - wasmtime at a pinned version, through `taiki-e/install-action`
    (already pinned by hash in `ci.yml`) if it carries `wasmtime`, else
    the release from `github.com/bytecodealliance/wasmtime/releases`
    with its checksum checked. It is a CI tool, not a dependency, so
    `deny.toml` doesn't apply; its license, Apache-2.0 WITH
    LLVM-exception, is on the allow list anyway;
  - `env:`
    - `CARGO_TARGET_WASM32_WASIP1_RUNNER`: `wasmtime run
      --dir=${{ github.workspace }}::${{ github.workspace }}
      --env=CI=true --env=INSTA_UPDATE=no
      --env=INSTA_WORKSPACE_ROOT=${{ github.workspace }}`. The
      workspace is mapped at its own path, so the paths compiled into
      the tests (insta's snapshots, proptest's regressions) open as
      they do natively;
    - `HEAD_REF: ${{ github.head_ref }}` and `EVENT_NAME: ${{
      github.event_name }}`, never `${{ }}` inside `run:`, as the
      `tickets` job does;
  - `cargo test --target wasm32-wasip1 --locked --no-fail-fast --lib
    --tests -p shuyi-core -p shuyi-assets -p shuyi-ui -p shuyi-platform
    -- --nocapture`: every unit and integration test of the crates that
    build without a device backend. Doctests stay native;
  - `cargo test --target wasm32-wasip1 --locked --no-fail-fast -p
    hello-grid --no-default-features --test golden -- --nocapture`;
  - on a pull request only (`if [ "$EVENT_NAME" = "pull_request" ]`, as
    the `tickets` job does), `cargo xtask golden --breaks --base HEAD^1
    --pr-branch "$HEAD_REF"` (below). As in `changes.yml`, `HEAD` is the
    PR's merge commit and `HEAD^1` the base's tip;
  - if a test overflows the stack under wasmtime (wasm32's default
    stack is 1 MiB), the two wasm32 steps set `RUSTFLAGS: -D warnings
    -C link-arg=-zstack-size=8388608` in their own `env:`, never a
    change to the test. Not the job's `env:`: the native `cargo xtask`
    build would get the wasm linker flag too;
  - a comment above the job, in `ci.yml`'s style: x64 and ARM64 are the
    `test` job's legs, and this job adds the 32-bit target the web
    build ships.

- **`cargo xtask golden`** (`crates/xtask/src/golden.rs`, with its lines
  in `main.rs`'s `USAGE` and `match`):
  - **`--bless`:** runs `cargo test --workspace --locked --no-fail-fast
    -- golden` natively, with `SHUYI_BLESS=1` set (for a test that
    blesses itself, such as 2519's golden session), and reads its
    output. For each `--- new golden table: <path> ---` block (the last
    one per path wins) it writes the text to the path, refusing a path
    outside the repository. It prints the tables written and `git diff
    --stat` over them, then runs the tests again and fails unless they
    pass.
  - **`--breaks --base <rev> [--pr-branch <branch>]`:** reads `git diff
    -U0 --no-renames <base> -- ':(glob)**/tests/golden/*.ron'`. A
    removed row line (a `-` line holding a `"name": ` entry) is a
    changed or removed value; added rows are not. Then:
    1. none: pass;
    2. a table of an example (`examples/<name>/tests/golden/`) whose
       `src/`, `files/`, `tests/records/` or `Cargo.toml` changed in the
       same diff: its rows moved with its own rules, not with the
       engine. Pass, and say so;
    3. otherwise, when `--pr-branch` names a ticket
       (`ticket_id_from_branch`), that ticket's file in `tickets/done/`
       must have, under `## Completion notes`, a `Breaking:` line that
       contains each changed table's file name. If not, it fails,
       printing the rows and an example line: `Breaking: shuyi-assets:
       formats.ron "lights: lights": <what a game sees>`;
    4. with a branch that names no ticket (`sonar/…`, `dependabot/…`):
       fail. A PR without a ticket doesn't move a golden value; the
       message says to write one.
    Without `--pr-branch` (a local run) it prints what it found and
    passes.
  - Every `git` and `cargo` call goes through 2010's `Runner` trait
    (`crates/xtask/src/api.rs`), which the tests fake, as `api.rs`'s
    own tests do.
  - **With 2010** (done): reuse its `breaking_lines` and
    `ticket_id_from_branch`, both `pub(crate)` in
    `crates/xtask/src/tickets.rs`.

- **The rule, written down:**
  - **Either order with 2005:** its version-policy ADR counts a changed
    or removed golden value as a breaking change: in an engine crate's
    table, or in an example's table when the example's own files didn't
    change. If 2005 is not done, 2005 writes it. If 2005 is done and its
    ADR already says so, nothing to do. If it is done and doesn't, don't
    edit its accepted ADR (`write-adr` skill, step 5): write a short ADR
    of this ticket's own with the `write-adr` skill (golden tables, the
    `determinism` job, and a moved value as a break under 2005's
    policy), with its row in `docs/adr/README.md`.
  - The `run-gates` skill gets a section "Golden tables (CI's
    `determinism` job)":
    - what runs where: the tables natively in `test` (x64, ARM64), and
      the four crates' tests and Hello, Grid's `golden` on wasm32 in
      `determinism`;
    - `cargo xtask golden --bless`, and the `Breaking:` line;
    - the wasm32 run is not a local gate (it needs `rustup target add
      wasm32-wasip1` and wasmtime: `cargo install --locked
      wasmtime-cli@<the pinned version>`). Run it with the job's two
      commands and runner before pushing a change to number code,
      hashing, a decoder or the pack. On Windows only `-- golden`
      passes there: the snapshot tests open the Windows paths compiled
      into them, which WASI can't;
    - a new example with a golden table adds its own `cargo test
      --target wasm32-wasip1 -p <example> --test golden` step to the
      job, with its window feature off, as Hello, Grid's is.
  - The `write-ticket` skill's line "A new file format follows ADR-0007
    (a `Format`, fuzz-style test, determinism fixture)" says what the
    fixture is: a row in its crate's golden table, which CI's
    `determinism` job checks on wasm32.

- **Either order with tickets that pin values:**
  - **From 2111 (done):** the fixed bundle's fingerprint, pinned as a
    constant by `a_fixed_bundle_has_a_pinned_fingerprint`
    (`crates/assets/src/bundle/fingerprint.rs`), becomes a row
    `fingerprint` of `formats.ron`, and Hello, Grid's table gets
    `fingerprint files`, `hello_grid::made().content`, with its pack's
    fingerprint `assert_eq!` to it (beside
    `the_pack_has_the_fingerprint_of_the_files_it_was_baked_from` in
    `tests/it/pack.rs`). This ticket adds both rows.
  - **2114:** its golden values stay as asserts in `rng.rs`'s tests,
    which this job runs on wasm32. Nothing to move.
  - **2225:** if it is done, its presets' hashes become rows of
    `crates/assets/tests/golden/sfx.ron`, and `sfx` joins the features
    of `shuyi-assets`'s self dev-dependency, so the job builds them. If
    not, 2225 does it.
  - **2203:** if it is done, its formats get rows in `formats.ron`, each
    the hash of the asset its format promises (ADR-0007 §5: pixels for
    an image, the header for a sound), and their features join
    `shuyi-assets`'s self dev-dependency. If not, 2203 adds them.
  - **2230:** if it is done, `Json<T>` and `Csv<T, R>` get rows in
    `formats.ron` (`<kind>: json` and `<kind>: csv`, the `state_hash`
    of what each decodes from a small fixture). They are not in
    `Formats::standard()`, so the guard doesn't ask for them. If not,
    2230 adds them.
  - **2408:** if it is done, `RumbleRon` is a format of
    `Formats::standard()`, so it gets a row `rumble cues: rumble` in
    `formats.ron`, the `state_hash` of what it decodes from 2408's
    fixture. If not, 2408 adds it.
  - **2101:** if it is done, `-p shuyi-script` joins the job's wasm32
    run, and `crates/script/tests/golden/scripts.ron` holds what a few
    fixture scripts return on fixed views. If not, 2101 does it.
  - **2519:** if it is done, its golden session test moves into Hello,
    Grid's `tests/golden.rs`, with `golden` in its name, so the job
    plays it on wasm32 and `cargo xtask golden --bless` blesses it. If
    not, 2519 puts it there.
  - **1140** (parked: lockstep, the `shuyi-net` crate): if it is done,
    `-p shuyi-net` joins the job's wasm32 run. If not, 1140 adds it
    (its clause with 2116 says so).
  - **2736 and 2737:** if either is done, nothing to do: an example
    that lands first keeps no golden table, and the module docs say how
    one adds it. If not, each gives its example a golden table of one
    record, as Hello, Grid's, with its own step in this job (2736 in
    its part 2 ticket).

**Out (do not do):**

- An `i686-unknown-linux-gnu` leg. wasm32 is the 32-bit target that
  ships, and i686 needs `gcc-multilib` and 32-bit system libraries.
- An `ubuntu-24.04-arm` leg: `macos-latest` already runs every test on
  ARM64 (step 1 checks it).
- Hardware and emulator targets: thumbv7em, the Playdate, the GBA and
  the DS (3020, 3021, 3024).
- `shuyi-app` and `shuyi-cli` tests on wasm32: one is the device
  backend, the other is tools that run natively. The web build in a
  browser is 2019's.
- Saves and records across engine versions (2110, 2111), and keeping old
  executables to replay old records.
- Decoded audio samples. No engine format decodes them (`Sound` and
  `MusicTrack` are headers), and the backend decodes them at play time,
  outside any simulation.
- The pixels of a look (`frame-png` snapshots, rule 9), and a pack's
  bytes.
- New pins for the RNG and `state_hash`'s form: they exist, and the job
  runs them.
- Golden tables in the templates: a new game's author doesn't bless
  files they didn't make. The module docs say how a game adds one.
- Doctests on wasm32.
- Fuzzing the formats and the pack (2020), and random legal play
  checked at every step (2527).
- The `Breaking:` rule for public API changes, and the `api` job
  (2010); this ticket shares its `breaking_lines` (below).

## Implementation steps

1. Check that `macos-latest` is ARM64 (the runner images' README). If
   GitHub moved it to x64, add an `ubuntu-24.04-arm` leg to `test`
   instead of leaving ARM64 out.
2. Change the workspace's `proptest`, split Hello, Grid's `window`
   feature (and its `build.rs` line), and make `cargo test --target
   wasm32-wasip1` of the four crates, and of an empty `golden` target in
   Hello, Grid, build and run under wasmtime on Linux. Mark the tests
   that need threads or unwinding. Look into any other failure (see
   Scope).
3. Write `shuyi_assets::golden`, `golden!` and `table_text`, with their
   unit tests.
4. Write `formats.ron`'s test and its guard. Start the table as `{}`,
   run the test, and copy the report's new table into the file
   (`--bless` comes in step 6).
5. Make the record, write Hello, Grid's `golden.rs`, and fill its table
   the same way.
6. Count the diff (`git diff --stat main`, tables and the record
   aside). Past about 450 lines, `--breaks` won't fit: write it as a
   follow-up ticket (`write-ticket` skill, After MVP) and leave it out
   here (see **Size**). Then write `cargo xtask golden` (`--bless`, and
   `--breaks` if it stays), tested with fake runs.
7. Add the job and its line in `ci-result`. Prove it on a demo PR,
   closed unmerged, from a branch `t2116-golden-demo` cut from this
   ticket's branch once this ticket is in `done/` there (`work-ticket`
   §5), so the ticket lint passes on it. Its base is this ticket's
   branch, not `main`: against `main` every row is new, so `--breaks`
   would pass whatever changed. On it:
   - change the input of one row in `golden_tests.rs` without blessing:
     the table's test fails with its report;
   - bless: `--breaks` fails, asking for a `Breaking:` line;
   - add the line to this ticket's completion notes on that branch:
     `--breaks` passes.
   Link the three runs from this PR's body.
8. Write the skills' lines and the rule (in 2005's ADR, or this
   ticket's own: see Scope), and run the gates.

**Size.** About 600 changed lines, not counting the tables and the
record: the module and its tests about 200, the two table tests about
120, the manifests and the ignore attributes about 40, `--bless` about
80, `--breaks` about 130, the job and the skills about 60. If step 6's
count says `--breaks` won't fit, it goes to its follow-up ticket: the
demo PR then shows the failing test and the bless only, the acceptance
line on `--breaks` moves to that ticket, and the completion notes name
it.

## Acceptance criteria

- [x] CI's `determinism` job runs every unit and integration test of
      `shuyi-core`, `shuyi-assets`, `shuyi-ui` and `shuyi-platform`, and
      Hello, Grid's `golden` test, on `wasm32-wasip1` under wasmtime,
      and passes. The PR lists every test ignored there, and why.
- [x] `formats.ron` has a row for every format of `Formats::standard()`
      (with `png`, so `Dlg` and `TargetRon` too) and for `MapText`,
      `KeymapRon`, `CinematicRon`, `LangPackRon` and `Ron<T>`.
- [x] The guard fails on a registry with a toy format of a covered kind
      that has no row (unit test).
- [x] Hello, Grid's table holds one row per step of the checked-in
      record, its outcome, and one row per asset; the assets read back
      from a pack baked on wasm32 equal them.
- [x] Changing a value fails the table's test with the report described
      above, its new table between the two marker lines.
- [x] `cargo xtask golden --bless` rewrites exactly the tables whose
      tests failed, and the tests pass after it.
- [x] ~~`cargo xtask golden --breaks` fails on a PR that changes or
      removes a golden value without a `Breaking:` line naming its
      table, and passes with one, or when only rows were added (unit
      tests and the demo PR).~~ (not done: **Size** moved it out, so
      this line is 2035's)
- [x] The demo PR's runs (step 7: three, or two without `--breaks`)
      are linked from this PR.
- [x] The `no-std` job's test run still passes with the new `proptest`
      features.
- [x] `cargo shuyi web --release` and `cargo shuyi check --target all`
      from `examples/hello-grid` work as before.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit, `shuyi_assets::golden`: a table that matches; a changed, an
  added and a removed row, each in the report; a name checked twice; a
  text that doesn't parse; `table_text`'s order and escaping (a name
  with a quote), and that its output parses back to the same rows.
- Unit, the guard: a toy format of a covered kind with no row.
- Unit, `crates/xtask/src/golden.rs`: the marker parser (two tables, the
  same table twice, a path outside the repository); the `--breaks`
  rules on fake diffs: only added rows, a changed row with and without
  the line, an example whose own `src/` changed, and a branch with no
  ticket.
- Golden: `formats.ron` and Hello, Grid's table, natively in the `test`
  job (x64, ARM64) and on wasm32 in the `determinism` job.
- CI: the job itself, shown by the demo PR.

## Completion notes

What was done:

- **`shuyi_assets::golden`** behind the new feature `golden`
  (`crates/assets/src/golden.rs`, `no_std` + `alloc`, no new
  dependency): `Golden::new`, `check`, `finish`, `assert`, the report
  (`GoldenReport`, `#[non_exhaustive]`, ADR-0039 kind 3) with its
  `Display` and the two marker lines, `table_text`, and the
  `golden!` macro. The module docs hold what goes in a table, how to
  bless, how a game or an example adds one, and the break rule. Unit
  tests in `golden/tests.rs`. `shuyi-assets`'s self dev-dependency
  turns on `golden`.
- **`crates/assets/tests/golden/formats.ron`**, 21 rows, checked by
  `crates/assets/src/format/golden_tests.rs`: one per format of
  `Formats::standard()` (with `png`), `pattern-map` included, which the
  ticket's list missed; `MapText`, `KeymapRon`, `CinematicRon`,
  `LangPackRon` and `Ron<T>` with `builtin_tests.rs`'s test types; and
  `fingerprint`. The guard is `missing_rows`, over a registry, with a
  toy-format test. 2020 isn't done, so it uses `builtin_tests.rs`'s
  inputs (`ogg` and `pack_files` made `pub(crate)`; `bitmap.rs`'s and
  `target.rs`'s test modules and their `sample` and `profile` too).
  2515 isn't done, so the guard walks a fixed list of kinds.
- **From 2111:** the fixed bundle's fingerprint moved from the constant
  of `a_fixed_bundle_has_a_pinned_fingerprint` to the row
  `fingerprint` (same value, `4a15fc0ed2ac24d7`); the test that stays
  checks the offset basis only. Hello, Grid's table has
  `fingerprint files`.
- **Hello, Grid:** `tests/records/seed-11.ron` (seed 11's play ends,
  caught on turn 43, 86 commands; no `made`), `tests/golden.rs` and
  `tests/golden/hello-grid.ron`: 87 step rows, `outcome`, 8 asset rows
  and `fingerprint files`; the pack baked in the test reads back equal
  (asserted, no second row). A default feature `window` gates macroquad
  and the binary (`required-features`); `link` turns `window` on.
  `build.rs` passes `--allow-undefined` on `wasm32-unknown-unknown`
  only.
- **proptest** without `fork` and `timeout`. The `no-std` job's test
  run passes with it.
- **CI's `determinism` job**, in `ci-result`'s `needs`, with wasmtime
  49.0.3 through the pinned `taiki-e/install-action` (it carries
  wasmtime; 49.0.3 is the newest its manifest knows). No stack-size
  flag was needed. Step 1: `macos-latest` is macOS 26 on arm64 (the
  runner images' README, 2026-10-28), so ARM64 stays the `test` job's.
- **`cargo xtask golden --bless`** (`crates/xtask/src/golden.rs`),
  through `api::Runner`, which gained `run_with_env` (a default that
  refuses, as `script::Runner::run_with_input` does) for
  `SHUYI_BLESS=1`. Tested with fake runs; run for real it restored two
  tables broken by hand and the tests passed after it.
- **Docs:** the `run-gates` skill's section *Golden tables (CI's
  `determinism` job)*, the `write-ticket` skill's determinism fixture
  line, `docs/formats.md`'s *Determinism* bullet, the book's testing
  chapter (*Golden values*), Hello, Grid's README, the assets crate
  doc. `_typos.toml` skips `**/tests/golden/*.ron` (hex hashes).
- **The rule:** 2005 already says a changed or removed golden value is
  a break (its *Golden values* bullet), so nothing to add here.

Deviations:

- **`--breaks` moved to 2035.** Step 6's count: about 930 changed
  lines before `--bless`, tables and the record aside (the module's
  docs and tests alone are over 400), past the 450 the ticket sets. So
  2035 holds `--breaks`, its CI step (`fetch-depth: 2`, `HEAD_REF`,
  `EVENT_NAME`) and its demo, and the demo PR here shows the failing
  test and the bless only.
- **The `shuyi` front door has no `app` on WASI.** Hello, Grid depends
  on `shuyi`, which always pulled `shuyi-app` (the ticket assumed a
  direct dependency). Its test linked under WASI, but wasmtime refused
  it: miniquad's web callbacks import `env::sapp_set_cursor`, which
  nothing provides there. `shuyi-app` is now a dependency of `shuyi` on
  `cfg(not(target_os = "wasi"))` only, and `shuyi::app` and
  `prelude::run_game` are gated the same: a target dependency, not a
  cargo feature, so `check-modules` (a feature names a module) holds.
  Nothing changes on any target that has a window.
- **`Golden::finish` returns `Result<(), Box<GoldenReport>>`**, not a
  bare `GoldenReport`: clippy's `result_large_err`.
- **`shuyi-cli` built for `wasm32-wasip1` with no change.**
- **`long_waits_are_refused`** is ignored with "needs unwinding, which
  WASI preview 1 lacks": it uses no thread.

Ignored on wasm32 (the `determinism` job), by attribute:
`bundle::tests::story_only_hides_the_test_layers_from_this_thread_until_it_returns`
and `bundle::tests::reading_makes_this_thread_read_another_bundle_until_it_returns`
(`shuyi-assets`: `catch_unwind` and `std::thread::spawn`), and
`scripts::long_waits_are_refused` (`shuyi-ui`'s `it`: `catch_unwind`).
libtest ignores every `#[should_panic]` test on wasm by itself (1.99.1
does): 11 in `shuyi-core`, 1 in `shuyi-assets`, 10 in `shuyi-ui`'s lib
and 2 in its `it`, 1 in `shuyi-platform`'s lib and 1 in its `it`. No
other test failed on wasm32: no determinism bug found.

Follow-up tickets: 2035 (`cargo xtask golden --breaks` and its CI step).

No break: no golden value moved (the tables are new; `fingerprint`
keeps the pinned value), and the public API only grows
(`shuyi_assets::golden`, behind a feature). `shuyi::app` is gone on WASI
only, where it never ran.
