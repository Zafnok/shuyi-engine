---
id: "3441"
title: "shuyi crash --symbols: a crash report's frames named from the debug file kept for its release"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2520"]
nick_input: none
completed: 2026-11-27
---

# 3441 — `shuyi crash --symbols`

## Context

Split from 2520 by its **Size** rule: `crash`, the panic caught in
`turns` and `replay --crash` came to about 1,200 counted lines without
`--symbols`, past the 350 at which 2520 sends the lookup here. This
ticket holds 2520's bullets on `--symbols`, with their acceptance
criteria and tests.

Unreal and Unity keep each build's debug symbols apart from what ships,
so a crash from a player's stripped build reads as functions and lines
later
(<https://dev.epicgames.com/documentation/en-us/unreal-engine/crash-reporting-in-unreal-engine>).
Sentry and BugSplat do the same for any game.

What is there (read 2026-11-26):

- **From 2520 (done):** `shuyi crash <report>` (`Op::Crash`,
  `crates/cli/src/contract/crash.rs`) reads a report in `shuyi` itself,
  an arm of `without_project` in `crates/cli/src/contract/cli.rs`.
  `CrashInput { report }`; `CrashOutput` holds the report's message,
  place, thread, time, versions, frame, top screen, log, std's
  backtrace and `has_record`, and no frames yet. `read_report` gives
  `SH3004` for a file that can't be read and `SH3018` for one that isn't
  a report. `crash::text` writes it for people. Both types are
  `#[non_exhaustive]`.
- **From 2598 (done):** `CrashReport::frames: Vec<CodeFrame>`
  (`shuyi_platform::crash`), `CodeFrame { offset: Option<u64> }`: each
  return address minus the executable's load address, or `None` in
  another module, on Windows and Linux (empty on macOS and the web).
  2598 takes them with the system's unwinder
  (`crates/app/src/crash/frames.rs`: `RtlCaptureStackBackTrace` and
  glibc's `backtrace()`, the executable's range from
  `/proc/self/maps`). `docs/crash-reports.md` (*Reading the frames*)
  reads them by hand with `addr2line`; `crash --symbols` does the same
  lookup (Windows: the image base plus the offset, minus 1).
- **The debug file:** the executable as built before it was stripped,
  with line tables, kept beside the package as `<package>.debug` and
  never shipped. **From 3004 (done):** `shuyi export --release` keeps
  it (`crates/cli/src/export/desktop.rs`), and remaps the build's paths
  (`crates/cli/src/export/paths.rs`: the workspace root to the package's
  name, `CARGO_HOME` to `/cargo`, `RUSTUP_HOME` to `/rustup`, the target
  folder to `/target`). **From 3406 (done):** a game's release from a
  tag keeps each CI-built debug file in the draft release `<tag>-debug`;
  `gh release download <tag>-debug --repo <owner>/<game>` fetches them
  (`docs/crash-reports.md`, ADR-0133). `crash --symbols <file>` reads
  one fetched so; fetching it for the developer is this ticket's to
  decide.
- **Test builds carry line tables only in shuyi-cli:** `[profile.dev]
  debug = "line-tables-only"` with `strip = "debuginfo"`, but the root
  `Cargo.toml`'s `[profile.dev.package.shuyi-cli]` override (`strip =
  "none"`, from 2077) keeps shuyi-cli's: keep it, and put the tests in
  `shuyi-cli`. CI's `test` job runs on `windows-latest` (MSVC, so PDB
  files), `ubuntu-latest` and `macos-latest`.
- **The crates** (read in the local registry on 2026-10-21):
  - `addr2line` 0.25.1 (Apache-2.0 OR MIT) reads DWARF from PE and ELF
    files: `addr2line::Loader::new(path)`, `relative_address_base()`
    (the image base for PE, 0 for ELF), `find_frames(probe)` and
    `find_symbol(probe)`, behind its `loader` feature. It brings `gimli`
    0.32 (MIT OR Apache-2.0); `loader` adds `object` with its
    `compression` feature (its deflate crate, already in `Cargo.lock`,
    and `ruzstd`, MIT), `memmap2` (MIT OR Apache-2.0) and `typed-arena`
    (MIT). It does not re-export `object`;
  - `object` 0.37.3 (Apache-2.0 OR MIT): a PE symbol's `address()` is
    the image base plus its section's address plus its value, and
    `relative_address_base()` is the image base, so the difference is
    the same offset the hook writes;
  - `backtrace` 0.3.76 (MIT OR Apache-2.0) gives each frame's address
    (`BacktraceFrame::ip`) and, on Windows x64, its module's base
    (`module_base_address`); on Linux the base is `None`. Its own lookup
    subtracts 1 from every address (`adjust_ip`). A dev-dependency at
    most: `shuyi-app` keeps its own unwinder (2598).

  Every license is on `deny.toml`'s allow list.

## Nick input

None.

## Scope

**In:**
- **`crash <report> [--symbols <file>]`:**
  - `CrashInput` gains `symbols: Option<String>` (`#[serde(default)]`),
    the debug file from the same folder as the report; the command line
    takes `--symbols <file>` (`crash::parse_args`, `crash::usage`, the
    help line in `crates/cli/src/lib.rs`);
  - `CrashOutput` gains `frames: Vec<NamedFrame>`, the report's frames
    innermost first, with `NamedFrame { offset: Option<String>,
    function: Option<String>, file: Option<String>, line: Option<u32> }`
    (the offset as sixteen hex digits), `#[non_exhaustive]` with a
    constructor (ADR-0039; ADR-0151: no two neighbouring numbers);
  - with `--symbols`, `Loader::new(file)` looks each offset up at
    `relative_address_base() + offset - 1`: a frame's address is where
    its call returns to, and `backtrace` subtracts 1 the same way. An
    inlined function gives a `NamedFrame` of its own. The function's
    name comes from the debug info, or from the symbol table
    (`find_symbol`) when the debug info gives none. A debug file that
    can't be read fails with `SH3004`, one `Loader` can't read with the
    reason (a code of its class that fits, or a new row in
    `crates/cli/src/contract/codes.rs`'s `TABLE` with its page in
    `crates/cli/errors/`);
  - `addr2line` is a dependency of `shuyi-cli` with `default-features
    = false` and the features `loader` and `rustc-demangle`. It adds
    nothing to a shipped game: `shuyi-cli` is the developer's tools;
  - `limits` says: no `--symbols` given (the frames are offsets); how
    many frames the file names no function for ("4 of 19 frames fall
    outside every function: the debug file may be from another build"),
    counted, not an error; a report with no frames (macOS, the web, or
    written before 2598);
  - `crash::text` writes the frames, named or as offsets.
- **Either order with 3071:** if 3071 is done, use the `object` version
  its library audit brought (`crates/cli/Cargo.toml`), so `Cargo.lock`
  holds one; if not, nothing to do (3071 uses this one).
- **The contract's version:** the `crash` schemas regenerated and the
  line `crates/cli/schemas/versions.json` asks for (the steps in
  `docs/contract.md`, *The schemas*). Both new fields are additive, so
  `VERSION` rises only if `cargo xtask contract` finds a break
  (ADR-0150).
- **The MCP server:** `crash`'s tool takes `symbols` with its input;
  its description (`mcp::description`) says so.
- **Docs:** `docs/contract.md`'s `crash` row gains `--symbols` and
  `frames`. `docs/crash-reports.md` (*Reading the frames*) gains
  `shuyi crash --symbols` beside the by-hand recipe, how to keep and
  pass the debug file, and that `shuyi crash` and `NamedFrame.file`
  name files by the remapped names (`/cargo/registry/...`, the package's
  name). Its recipe for making a debug file by hand (build with
  `CARGO_PROFILE_RELEASE_STRIP=false` and
  `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, keep that file, ship
  a copy made with `objcopy --strip-debug`) remaps the paths as 3004
  does, or points at `shuyi export --release`. Stripping after the link
  moves no code, so the offsets agree.
- **The skill pack:** the `shuyi-debug-replay` skill's crash path
  (`skills/shuyi-debug-replay/SKILL.md`, written by 2520) gains
  `--symbols <file>` on its `shuyi crash` line; `check-skills` passes.
- **From 2520 (done):** `CrashOutput` is `#[non_exhaustive]`, so the
  new fields are no break, and no `Breaking:` line is planned.

**Out (do not do):**
- Keeping the debug file when a game is exported or released: 3004
  (done), 3012 (the Steam Deck) and 3050 (a release workflow).
- Windows PDB files: the GNU toolchain writes DWARF. A game built with
  MSVC is out until one asks.
- Frames on macOS and on the web.
- A symbol server, or uploading debug files anywhere. 3047 (parked)
  sends reports, never debug files.
- `replay --crash` and the panic caught in `turns`: 2520 (done).

**Estimated size:** about 700 lines (code 250, tests 300, docs 150),
schemas aside.

## Implementation steps

1. Read `crates/cli/src/contract/crash.rs` (2520), 2598's
   `crates/app/src/crash/frames.rs`, and `docs/crash-reports.md`.
2. Add `addr2line` to `crates/cli/Cargo.toml` (the features above), and
   `object` 0.37 as a dev-dependency with `default-features = false`
   and the feature `read`; `cargo deny check` with no change to
   `deny.toml`.
3. Write the lookup in a module of its own (`crash/symbols.rs`), then
   `NamedFrame`, `CrashOutput::frames`, `CrashInput::symbols`, the
   limits and `text`.
4. Regenerate the schemas and add their line to `versions.json`; raise
   `VERSION` only if `cargo xtask contract` finds a break.
5. On the machine at hand (Windows GNU): build hello-grid for release
   with its debug file kept as the docs say (or `shuyi export windows
   --release`), run the stripped copy with `SHUYI_PANIC_AT_FRAME=3`
   (3046), and run `shuyi crash` with `--symbols`. Put the answer in the
   PR description: its frames named in `shuyi_app` and hello-grid.
6. Write the docs and the skill's line, and run the gates.

## Acceptance criteria

- [x] A `shuyi-cli` test, compiled only on Linux and Windows GNU
      (`cfg(any(target_os = "linux", all(windows, target_env = "gnu")))`,
      where test builds carry DWARF), finds an `#[inline(never)]`
      function of its own test binary by a name unique to the test in
      the symbol table (`object` 0.37). It writes a report whose offset
      is a few bytes into that function (its symbol's `address()` minus
      `relative_address_base()`, plus 4), and `crash --symbols <the test
      binary>` names that offset with the function, its file and a line
      inside it.
- [x] An offset outside every function is counted in `limits`, not an
      error.
- [x] A report with no `frames` reads, and `limits` says why there are
      none.
- [x] On Linux and Windows GNU, a `shuyi-cli` test names a frame of its
      own stack, taken as 2598's hook takes it (the return address
      minus the executable's load address: `backtrace`'s
      `module_base_address` on Windows, the lowest start of the
      executable's lines in `/proc/self/maps` on Linux), with
      `addr2line`'s `Loader` over that binary.
- [x] The PR description has step 5's answer, its frames named in
      `shuyi_app` and hello-grid.
- [x] The schemas have their line in `versions.json`, and
      `contract::VERSION` rose only for a break (`cargo xtask contract`
      passes).
- [x] `cargo deny check` passes with no change to `deny.toml`.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the lookup at `offset - 1` and an inlined frame; the symbol
  table as the fallback name; an offset outside every function; a
  report with no frames.
- Integration: `crash` with and without `--symbols` through the
  contract's command line, with the exit codes
  (`crates/cli/src/contract/cli.rs`'s tests); the `crash` answer as
  JSON against its schema (2520's `conforms` check in
  `crates/cli/src/contract/crash/tests.rs`).

## Completion notes

- `shuyi crash <report> [--symbols <file>]`: `CrashInput::symbols`
  (`#[serde(default)]`), `CrashOutput::frames: Vec<NamedFrame>`, and
  `NamedFrame { offset, function, file, line }` (`#[non_exhaustive]`,
  `NamedFrame::new(offset)`, the offset as sixteen hex digits). Without
  `--symbols` the frames are offsets; with it, each is named from the
  debug file.
- The lookup is `crates/cli/src/contract/crash/symbols.rs`:
  `addr2line::Loader` at `relative_address_base() + offset - 1` (an
  offset of 0 names nothing); each frame `find_frames` gives, an inlined
  function first, is a `NamedFrame` of its own at the same offset; where
  the debug information names no function, the symbol table's name
  (`find_symbol`, demangled), only for an address in the file's `.text`:
  `object`'s symbol map gives the nearest symbol before any address,
  however far, so without that check an offset past every function would
  be named by the last symbol.
- A debug file that can't be opened is `SH3004`; one `Loader` can't read
  is a new code, `SH3020` (`debug-file`, Operation, exit 1), with its row
  in `codes.rs`'s `TABLE` and its page `crates/cli/errors/SH3020.md`.
- `limits`: no `--symbols` (when the report has frames), "N of M frames
  fall outside every function: the debug file may be from another build"
  (M counts the frames with an offset), and a report with no frames.
- `crash::text` writes a `frames:` section before the backtrace: the
  offset, the function, ` at file:line`, or `in another module`.
- The help line in `lib.rs`, `crash::usage`, the MCP tool's description
  (`mcp::CRASH`) and the contract module's table say `--symbols`. The
  `crash` schemas are regenerated; `cargo xtask contract` finds no break
  (both fields additive), so `contract::VERSION` stays 62, and
  `versions.json` gains its line at 62.
- Tests: `crash/symbols/tests.rs`, compiled on Linux and Windows GNU only,
  over the test program's own DWARF: a function found by its symbol
  (`object`), 4 bytes in, named with its file and a line inside it
  through `crash --symbols`; the lookup 1 byte before the return address
  (a function's first byte names what lies before it); an
  `#[inline(always)]` function a frame of its own before its caller; the
  symbol table's name for a dependency's function, which has no DWARF
  (`[profile.dev.package."*"] debug = false`); offsets outside every
  function (far past the code, and 0) counted in `limits`; and a frame
  of the test's own stack taken as 2598's hook takes it (`backtrace`'s
  `ip`, minus `module_base_address` on Windows or the lowest start of the
  program's lines in `/proc/self/maps` on Linux), named. In
  `crash/tests.rs`, on every OS: the offsets without `--symbols` and
  their limit, the text of a named frame, `SH3004`/`SH3020`/`SH3018` in
  that order, the arguments, and the answer with named frames against
  its schema. `cli.rs`'s test runs `--symbols` through the command line
  with its exit codes (0, 1, 2).
- Step 5, on this machine (Windows GNU), at `f4c7be94` (`main`'s tip):
  hello-grid built for release with
  `CARGO_PROFILE_RELEASE_STRIP=false CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`,
  kept as `hello-grid.debug`, its copy stripped with `objcopy
  --strip-all` as `shuyi export --release` strips it, run with
  `SHUYI_PANIC_AT_FRAME=3`. `crash --symbols hello-grid.debug` named all
  17 of the report's frames in the program, as 46 frames with the
  inlined ones: the hook in
  `shuyi_app::crash` (`frames.rs:97`, `crash.rs:349`), the panic in
  `frame_or_crash<hello_grid::Hello>` (`crates/app/src/lib.rs:705-706`),
  `run_app<hello_grid::Hello>` (`lib.rs:570`), hello-grid's `main`
  (`examples/hello-grid/src/main.rs:57`), and the C runtime's `main` and
  `__tmainCRTStartup` from the symbol table; two frames in other
  modules. The stripped program given as `--symbols` counts 17 of 17
  outside every function; a text file is `SH3020`. The output is in the
  PR description.
- Docs: `docs/contract.md`'s `crash` row; `docs/crash-reports.md`,
  *Reading a report* and *Reading the frames*: `shuyi crash --symbols`,
  keeping and passing the debug file, the remapped file names, the
  by-hand recipe (which points at `shuyi export --release` for remapped
  paths) beside the `addr2line` one. The book's *When a player's game
  crashes* and the `shuyi-debug-replay` skill (step 1, and `SH3020`)
  say `--symbols`.
- Deviations:
  - `addr2line` 0.27 (gimli 0.34, `object` 0.40), not 0.25 (gimli 0.32,
    `object` 0.37), with `object` 0.40 as the dev-dependency: the
    `backtrace` the stack test needs resolves to 0.3.77, which brings
    `addr2line` 0.27, so 0.25 would put two of each in `Cargo.lock`. The
    API the ticket names is the same in 0.27. 3071, still open, now says
    to use `object` 0.40.
  - The two dev-dependencies are target-specific (Linux and Windows
    GNU), where their tests compile.
  - Step 5's build turns on `debug_assertions`
    (`CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true`): `SHUYI_PANIC_AT_FRAME`
    is read only with debug tools (`shuyi_ui::DEBUG_TOOLS`) or a game's
    `with_record_in_release`, and hello-grid asks for neither. No code
    moves for it, so the lookup is the same as a shipped build's.
- `tools/compile-budget.ron`'s `dependencies` rises from 150 to 159: a
  new game's tools (`shuyi-cli`) gain `addr2line`, `gimli`, `object`,
  `memmap2`, `typed-arena`, `stable_deref_trait`, `rustc-demangle`,
  `ruzstd` and `twox-hash` (the last two for the zstd-compressed debug
  sections `addr2line`'s loader reads). The ten tutorial steps' locks gain
  the same crates, nothing else moved. Nothing reaches a shipped game.
- 3071's clause on the `object` version, 3337's "3441 (open)" and the
  roadmap's row now say 3441 is done.
- No ticket: the game reads its frames by hand (its `docs/releasing.md`,
  from game ticket 0918, with `addr2line`), which still works, and
  pointing it at `cargo shuyi crash --symbols` is the game's own choice
  at a pin move. Searched the game for
  `CrashOutput` and `shuyi crash` (`gh search code --repo
  Zafnok/visions-of-shuyi`): only that ticket's text, no code.

No break: `CrashInput` and `CrashOutput` are `#[non_exhaustive]`, both
fields are additive, and `NamedFrame` is new.

Behaviour: shuyi-cli: `crash`'s answer has `frames`, and its `limits` say when a report has frames and no `--symbols` was given, and when it has none.

Behaviour: shuyi-cli: `crash`'s text writes a `frames:` section, and its usage line is `crash <report> [--symbols <file>] [--json]`.

Behaviour: shuyi-cli: a new problem code, `SH3020` (`debug-file`).
