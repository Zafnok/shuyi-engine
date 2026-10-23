---
id: "2520"
title: "shuyi crash and replay --crash: a crash report read, its frames named from the debug file kept for each release, and its record replayed to the panic"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3046"]
nick_input: none
completed:
---

# 2520 — `shuyi crash` and `replay --crash`

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Unreal and Unity keep each build's debug symbols apart
from what ships, so a crash from a player's stripped build can be read
as functions and lines later
(<https://dev.epicgames.com/documentation/en-us/unreal-engine/crash-reporting-in-unreal-engine>).
Services such as Sentry and BugSplat do the same for any game. None of
the benchmarks replays the play that led to a crash.

What Shuyi has and plans (read 2026-10-21):

- **3046** writes a `CrashReport` (`shuyi_platform::crash`): the
  message and place (`file:line:column`), std's backtrace text, the
  versions, the frame, the top screen, the recent log, and a turn
  game's record of its play in the game's own record format. Every
  field but `version` and `message` is `#[serde(default)]`, so fields
  can be added. In a release build (`strip = true`, root `Cargo.toml`,
  line 47) std can't name the frames: they print as `<unknown>`.
- **`shuyi replay`** (`crates/cli/src/contract/replay.rs`): `turns`
  re-runs a turn game's commands with a state hash per step. The game's
  tool program calls it from `GameTools::replay`
  (`crates/cli/src/game.rs`), and `shuyi` runs that program with
  `cargo run`. A panic inside `TurnRules::apply` ends the tool program
  with no answer. `replay` runs one record twice, or two side by side.
- **The contract** is at `VERSION` 4 (`crates/cli/src/contract.rs`,
  line 52), its schemas in `crates/cli/schemas/`, and `docs/contract.md`
  (*The schemas*) says how to change them in four steps. `doctor` and
  `schema` run in `shuyi` itself, without a game (`without_project` in
  `crates/cli/src/contract/cli.rs`).
- **Test builds carry line tables:** `[profile.dev] debug =
  "line-tables-only"` in the root `Cargo.toml`, inside each `.exe` on
  Windows GNU. CI's `test` job runs on `windows-latest` (MSVC, so PDB
  files), `ubuntu-latest` and `macos-latest`.
- **Nothing keeps a build's debug info.** 3004 packages builds
  stripped.
- **1134** plans a `shuyi-debug-replay` skill (ADR-0005 §3: "Reproduce
  a bug from a record"), but nothing makes a record from a crash.
- **The crates for it** (read in the local registry on 2026-10-21):
  - `backtrace` 0.3.76 (MIT OR Apache-2.0) gives each frame's address
    (`BacktraceFrame::ip`) and, on Windows x64, its module's base
    (`BacktraceFrame::module_base_address`, `src/backtrace/win64.rs`).
    On Linux and macOS (`src/backtrace/libunwind.rs`) the base is
    `None`. Its own lookup subtracts 1 from every address
    (`adjust_ip`, `src/symbolize/mod.rs`). It brings `addr2line` 0.25,
    `object` 0.37 and `rustc-demangle` (MIT/Apache-2.0); `cfg-if`,
    `libc`, `miniz_oxide` 0.8 and `windows-link` are in `Cargo.lock`
    already;
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
    the same offset the hook writes.

  Every license is on `deny.toml`'s allow list.

Why it matters:

- **The game first.** A report from a Visions of Shuyi player, made by
  the build as shipped on Steam, names the functions and lines in the
  game's code, and its record replays to the panic.
- **Other games** get the same two commands.
- **The end goal** (2026-10-19, `docs/decisions.md`: the most
  AI-friendly engine). An agent goes from a player's report to a
  reproduced panic, a fix, and proof that the record no longer panics,
  with contract operations alone.

## Nick input

None.

## Scope

**In:**
- **Frames a debug file can name,** written by 3046's hook
  (`crates/app/src/crash.rs`):
  - `CrashReport` gains `frames: Vec<CodeFrame>` (`#[serde(default)]`),
    `CodeFrame { offset: Option<u64> }`: the frame's address minus the
    executable's load address, or `None` for a frame in another module
    (a system library). The report's `version` stays 1;
  - taken with `backtrace::Backtrace::new_unresolved()`, never resolved
    in the hook. `backtrace` joins `shuyi-app`'s native-only
    dependencies (`[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`),
    in the versions of `addr2line` and `object` this ticket uses, so
    `Cargo.lock` holds one of each;
  - the executable's place, found once by `crash::install`:
    - Windows: the module base of a frame of `install` itself (from
      `backtrace::trace`); a frame is in the executable when its
      `module_base_address` is that base;
    - Linux (the Steam Deck too): the lines of `/proc/self/maps` whose
      path is `std::env::current_exe()` give its mappings. A frame is
      in the executable when its address is inside one of them, and
      the lowest start is the load address (Rust links PIE
      executables, whose first segment's address is 0). The parsing is
      a pure function of the file's text;
    - macOS and the web: `frames` stays empty, and the docs say so;
  - the PR gives hello-grid's stripped release size before and after.
- **The debug file:** the executable as built before it was stripped,
  with line tables, kept beside the package as `<package>.debug` and
  never shipped. 3004's `shuyi export --release` keeps it.
  `docs/crash-reports.md` says how to make one by hand: build with
  `CARGO_PROFILE_RELEASE_STRIP=false` and
  `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, keep that file, and
  ship a copy made with `objcopy --strip-debug` (MSYS2's mingw-w64
  binutils on Windows GNU). Stripping after the link moves no code, so
  the offsets agree.
  - **Either order with 3004:** if 3004 is done, the by-hand recipe
    also remaps the build's paths as 3004's exporters do, or points at
    `shuyi export --release`. Then the copy shipped doesn't carry the
    builder's home folder, and the docs say that `NamedFrame.file`
    shows the remapped names (`/cargo/registry/...`, the package's
    name). If 3004 isn't done, 3004 adds this when it lands (its scope
    says so).
- **`crash <report> [--symbols <file>]`**, a new contract operation
  (`crates/cli/src/contract/crash.rs`, `Op::Crash`, after `Op::Replay`
  in `Op::ALL`). Like `doctor`, it runs in `shuyi` itself, without the
  game's program: an arm in `without_project`, with paths from the
  folder it is given (the project's root inside a project), and a line in the help of `crates/cli/src/lib.rs`,
  which also says `crash` needs no project:
  - `CrashInput { report: String, symbols: Option<String> }`;
  - `CrashOutput`: the report's fields (message, location, thread,
    when, engine, game, target, frame, top screen, log, std's backtrace
    text), `has_record: bool`, and `frames: Vec<NamedFrame>`, where
    `NamedFrame { offset: Option<String>, function: Option<String>,
    file: Option<String>, line: Option<u32> }` (the offset as sixteen
    hex digits);
  - the report is read with `CrashReport::parse` (3046). A file that
    can't be read or parsed fails with the reason;
  - with `--symbols`, `Loader::new(file)` looks each offset up at
    `relative_address_base() + offset - 1`: a frame's address is where
    its call returns to, and `backtrace` subtracts 1 the same way. An
    inlined function gives a `NamedFrame` of its own. The function's
    name comes from the debug info, or from the symbol table
    (`find_symbol`) when the debug info gives none. `addr2line` is a
    dependency of `shuyi-cli` with `default-features = false` and the
    features `loader` and `rustc-demangle`;
  - `ok` when the report reads. `limits` says: no `--symbols` given (the
    frames are offsets); how many frames the file names no function
    for ("4 of 19 frames fall outside every function: the debug file
    may be from another build"); a report with no frames (macOS, the
    web, or written before this ticket);
  - `crash::text` writes the report and its frames for people.
- **`turns` catches a panic:**
  - `ReplayStep` gains `panic: Option<PanicAt>` (`#[serde(default)]`),
    with `PanicAt { message: String, location: Option<String> }`;
  - `turns` runs each `apply` inside `catch_unwind(AssertUnwindSafe(..))`.
    On a panic, that step gets `panic` (the payload's text when it is a
    `&str` or a `String`), its `hash` is the state as the panic left
    it, the run stops there, and `outcome` is `None`;
  - the place comes from a panic hook `turns` installs once
    (`std::sync::Once`), after taking the hook that was there. It keeps
    the last panic's `file:line:column` in a thread-local and prints
    nothing while a step of `turns` runs on this thread (a thread-local
    flag). Any other panic goes to the hook it took, so tests that
    panic on other threads print as before;
  - `replay` without `--crash` on a record that panics: `ok` is false,
    and `limits` says "the rules panicked at step N of <record>:
    <message>". `replay::text` shows the step's panic.
- **`replay --crash`:**
  - `ReplayInput` gains `crash: bool` (`#[serde(default)]`), and the
    command line takes `--crash` (the `replay` arm of
    `crates/cli/src/contract/cli.rs`, its usage line, and the help line
    in `crates/cli/src/lib.rs`). With it, `record` names a crash report,
    read with `CrashReport::parse`: its `record` text goes to
    `GameTools::replay` as a `Record` whose `file` is the report's path.
    It runs once: one run, no `first_difference`, and not the "ran
    twice" line in `limits`. `other` with `--crash` is a wrong argument
    (exit code 2);
  - `ReplayOutput` gains `crash: Option<CrashReplay>`, with
    `CrashReplay { reported: PanicAt, replayed: Option<PanicAt>, step:
    Option<u32>, same_place: bool }`. `same_place` is true when the two
    places have the same line and column, and their files agree in the
    last three parts of the path (all of them when the shorter path has
    fewer), with `\` read as `/`. So a report
    from a Windows build, or from a build whose paths 3004 remapped,
    matches a replay on Linux;
  - `ok` with `--crash` is "the record plays to its end without a
    panic". An agent sees `ok: false` while the panic reproduces, fixes
    the code, and sees `ok: true`;
  - `limits`: when the record plays through, "only the rules were
    replayed: a panic in a screen, a skin or the platform does not run
    here". A report with no record fails with the reason: a web report,
    a panic before a stage was open, or a game without `crash_record`;
  - the stamp: `replay --crash` goes through the same path as `replay`
    (`GameTools::replay`, then the stamp check), so 2111's refusal of a
    record made with other content covers it whichever lands first. If
    2111 is done, a test shows it.
- **The contract's version:** the `crash` schemas
  (`crash.input.json`, `crash.output.json`) and the `replay` ones
  regenerated, `VERSION` raised by one, and the line
  `crates/cli/schemas/versions.json` asks for (the four steps in
  `docs/contract.md`).
- **Either order:**
  - with 2509 (problem codes): if it is done, each refusal here gets a
    problem code and its help. If not, 2509 gives them codes (its
    either-order list names 2520);
  - the MCP server (1133, done) makes a tool of each operation in
    `Op::ALL`: give `crash` its arms in `mcp::host::one_off` and
    `mcp::description`, and `replay`'s `crash` flag comes with its
    input;
  - with 2010 (public API diffs): if it is done, the completion notes
    carry a `Breaking:` line for each public struct that gained a field
    (`ReplayInput`, `ReplayStep`, `ReplayOutput`, and `CrashReport`
    unless it is `#[non_exhaustive]`). If not, the PR body says the same;
  - the new public types (`CodeFrame`, `CrashInput`, `CrashOutput`,
    `NamedFrame`, `PanicAt`, `CrashReplay`) follow ADR-0039
    (`#[non_exhaustive]` and constructors).
- **Docs:** `docs/contract.md`'s table gains `crash` and `replay
  --crash`. `docs/crash-reports.md` (3046) gains: reading a report with
  `shuyi crash`, keeping and passing the debug file, and reproducing
  with `replay --crash`.
- **For the game:** its tools call `turns`, whose signature doesn't
  change. Code that builds `ReplayStep` or `ReplayInput` by hand gains
  a field. Say so in the PR; the game's PR that moves the pin follows.
- **Size, decided after step 4.** Count the changed lines of `crash`
  without `--symbols`, the panic in `turns` and `replay --crash`, with
  their tests (schemas aside). If they pass about 350, the frames
  (`CodeFrame`, the hook's part, the executable's place) and `--symbols`
  (the lookup, `NamedFrame`'s names, the debug file's docs) go to a
  follow-up ticket (the `write-ticket` skill, After MVP, `blocked_by:
  ["2520"]`), written in this PR from this ticket's bullets, with their
  acceptance criteria and tests. `CrashOutput.frames` and its schema
  then come with the follow-up too. The lines
  of 3004, 3012, 3047 and 3050 that name 2520's debug file then name
  the follow-up.

**Out (do not do):**
- Keeping the debug file when a game is exported or released: 3004
  (`shuyi export --release`), 3012 (the Steam Deck) and 3050 (a release
  workflow).
- Windows PDB files: the GNU toolchain writes DWARF. A game built with
  MSVC is out until one asks.
- Frames on macOS and on the web.
- A session record (2518, 2519) in a report, and its replay: the
  follow-up ticket 3046's Out names.
- The `shuyi-debug-replay` skill: 1134.
- A symbol server, or uploading debug files anywhere. 3047 (parked)
  sends reports, never debug files.
- Panics outside the rules (screens, skins, the platform): a rules
  replay never runs them.

## Implementation steps

1. Read `crates/cli/src/contract/replay.rs`, `crates/cli/src/game.rs`,
   `crates/cli/src/contract.rs`, `contract/cli.rs` and `lib.rs`,
   `docs/contract.md`, and 3046's `crates/app/src/crash.rs` and
   `shuyi_platform::crash`.
2. Write the `crash` operation without `--symbols`: `CrashInput`,
   `CrashOutput`, `Op::Crash`, its arm in `without_project`, `text`.
3. Make `turns` catch a panic, with its hook, then add `--crash` to
   `replay`.
4. Count the changed lines and apply **Size** above.
5. Add the frames to the report and to the hook, with the executable's
   place on Windows and Linux. Add `--symbols` and the lookup with
   `addr2line`.
6. Regenerate the schemas, raise `VERSION`, add the line to
   `versions.json`.
7. On the machine at hand (Windows GNU): build hello-grid for release
   with its debug file kept as the docs say, run the stripped copy with
   `SHUYI_PANIC_AT_FRAME=3` (3046), and run `shuyi crash` with
   `--symbols`. Put the answer in the PR description.
8. Write the docs, and run the gates.

## Acceptance criteria

- [ ] A `shuyi-cli` test, compiled only on Linux and Windows GNU
      (`cfg(any(target_os = "linux", all(windows, target_env = "gnu")))`,
      where test builds carry DWARF), finds an `#[inline(never)]`
      function of its own test binary by a name unique to the test in
      the symbol table (`object`
      0.37, a dev-dependency with `default-features = false` and the
      feature `read`). It writes a report whose offset is a few bytes
      into that function (its symbol's `address()` minus
      `relative_address_base()`, plus 4), and `crash --symbols <the test
      binary>` names that offset with the function, its file and a line
      inside it.
- [ ] An offset outside every function is counted in `limits`, not an
      error.
- [ ] A report with no `frames` reads, and `limits` says why there are
      none.
- [ ] 3046's `crates/app/tests/crash_hook.rs`: on Windows and Linux
      the stored report has frames, at least one of them in the
      executable. On Linux and Windows GNU, `addr2line`'s `Loader` over
      the test binary (a dev-dependency of `shuyi-app`) names one of
      them as the test's function that panicked. On macOS `frames` is
      empty.
- [ ] The `/proc/self/maps` parser finds the executable's mappings and
      load address in a sample text, and skips other files' lines.
- [ ] `replay --crash` through a test game whose `GameTools::replay`
      runs `turns` over rules that panic on one command: a report whose
      record panics at step 3, with the place the replay gives, answers
      `ok: false`, `step: 3` and `same_place: true`; with another line
      in the report, `same_place: false`; with the rules fixed, `ok:
      true` and the rules-only line in `limits`.
- [ ] `same_place` matches `src\rules.rs:9:5` with
      `/cargo/git/checkouts/x/1/src/rules.rs:9:5`, and not with
      `src/rules.rs:10:5`.
- [ ] `replay` on a record that panics answers `ok: false` with the
      panicking step instead of ending the program.
- [ ] A report with no record fails with the reason, and `--crash` with
      a second record exits with 2.
- [ ] The PR description has step 7's answer, its frames named in
      `shuyi_app` and hello-grid, and the size difference.
- [ ] The schemas, `VERSION` and `versions.json` agree (the schema test
      passes).
- [ ] `cargo deny check` passes with no change to `deny.toml`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the lookup at `offset - 1` and an inlined frame; the symbol
  table as the fallback name; `same_place`; the `/proc/self/maps`
  parser; `turns` stopping at a panic with its message and place; the
  quiet hook passing other threads' panics on.
- Integration: `crash` (with and without `--symbols`) and `replay
  --crash` through the contract's command line, with the exit codes
  (`crates/cli/src/contract/cli.rs`'s tests and the fake game in
  `crates/cli/src/contract/fake.rs`); the `crash` answer as JSON against
  its schema; the hook's frames in `crash_hook.rs`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
