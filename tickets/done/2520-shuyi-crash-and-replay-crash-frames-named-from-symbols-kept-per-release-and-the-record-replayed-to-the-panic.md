---
id: "2520"
title: "shuyi crash and replay --crash: a crash report read, its frames named from the debug file kept for each release, and its record replayed to the panic"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3046"]
nick_input: none
completed: 2026-11-27
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
- **The contract** is at `VERSION` 62 (`crates/cli/src/contract.rs`,
  line 84; re-read 2026-11-26), its schemas in `crates/cli/schemas/`,
  and `docs/contract.md` (*The schemas*) says how to change them in five
  steps. `doctor` and
  `schema` run in `shuyi` itself, without a game (`without_project` in
  `crates/cli/src/contract/cli.rs`).
- **Test builds carry line tables:** `[profile.dev] debug =
  "line-tables-only"` in the root `Cargo.toml`, inside each `.exe` on
  Windows GNU. CI's `test` job runs on `windows-latest` (MSVC, so PDB
  files), `ubuntu-latest` and `macos-latest`.
- **From 3004 (done):** `shuyi export --release` keeps the program
  with its line tables beside the `windows` and `linux` packages as
  `<package>.debug`, and ships a copy stripped with `objcopy
  --strip-debug` (`crates/cli/src/export/desktop.rs`). Nothing else
  keeps a build's debug info.
- **1134** (done) wrote a `shuyi-debug-replay` skill
  (`skills/shuyi-debug-replay/SKILL.md`: reproduce a bug from a
  record), whose crash path says it is still to come: nothing makes a
  record from a crash.
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
- **From 2598 (done):** the report's frames. `CrashReport` has
  `frames: Vec<CodeFrame>` (`shuyi_platform::crash`), `CodeFrame {
  offset: Option<u64> }`: each return address minus the executable's
  load address, or `None` in another module, on Windows and Linux (empty
  on macOS and the web). 2598 took them with the system's unwinder
  (`crates/app/src/crash/frames.rs`: `RtlCaptureStackBackTrace` and
  glibc's `backtrace()`, the executable's range from `/proc/self/maps`),
  not the `backtrace` crate, so `shuyi-app` gained no crate; this ticket
  doesn't add `backtrace` either. `docs/crash-reports.md` (*Reading the
  frames*) reads them by hand with `addr2line`; `crash --symbols` does
  the same lookup (Windows: the image base plus the offset, minus 1).
- **The debug file:** the executable as built before it was stripped,
  with line tables, kept beside the package as `<package>.debug` and
  never shipped. 3004's `shuyi export --release` keeps it.
  **From 3406 (done):** a game's release from a tag keeps each CI-built
  debug file in the draft release `<tag>-debug`, which only people who
  can change the game see and nothing publishes; `gh release download
  <tag>-debug --repo <owner>/<game>` fetches them (`docs/crash-reports.md`,
  ADR-0133). `crash --symbols <file>` reads one fetched so; fetching it
  for the developer is this ticket's to decide.
  `docs/crash-reports.md` says how to make one by hand: build with
  `CARGO_PROFILE_RELEASE_STRIP=false` and
  `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, keep that file, and
  ship a copy made with `objcopy --strip-debug` (MSYS2's mingw-w64
  binutils on Windows GNU). Stripping after the link moves no code, so
  the offsets agree.
  - **From 3004 (done):** `shuyi export --release` remaps the build's
    paths (`export::paths`, `crates/cli/src/export/paths.rs`:
    `--remap-path-prefix` through `CARGO_ENCODED_RUSTFLAGS`, the
    workspace root to the package's name, `CARGO_HOME` to `/cargo`,
    `RUSTUP_HOME` to `/rustup`, the target folder to `/target`). The
    by-hand recipe also remaps them the same way, or points at `shuyi
    export --release`. Then the copy shipped doesn't carry the
    builder's home folder, and the docs say that `shuyi crash` and
    `NamedFrame.file` name files by the remapped names
    (`/cargo/registry/...`, the package's name).
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
    features `loader` and `rustc-demangle`. **Either order with
    3071:** if 3071 is done, use the `object` version its library audit
    brought (`crates/cli/Cargo.toml`), so `Cargo.lock` holds one; if
    not, nothing to do (3071 uses this one);
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
    record made with other content (`SH3007`) covers it. **From 2111
    (done):** a test here shows `replay --crash` refusing a crash
    report whose record is stamped with other content (`made` with
    another `content`), as hello-grid's
    `a_record_made_with_other_content_is_refused_unless_any_stamp`
    does for `replay`.
- **The contract's version:** the `crash` schemas
  (`crash.input.json`, `crash.output.json`) and the `replay` ones
  regenerated, and the line `crates/cli/schemas/versions.json` asks
  for (the steps in `docs/contract.md`). `VERSION` rises only if
  `cargo xtask contract` finds a break (ADR-0150).
- **Either order:**
  - with 2509 (done, problem codes, ADR-0028): each refusal here is
    `Answer::failed(op, code, error)`. `--crash` with a second record
    is `SH0001` (`wrong-arguments`), and a report that can't be read
    `SH3004` (`file`), as `replay`'s records are. A report that doesn't
    parse, or has no record, gets the code of its class that fits, or
    a new row in `crates/cli/src/contract/codes.rs`'s `TABLE` with its
    page in `crates/cli/errors/` (the gate in `codes/tests.rs` checks
    both). A panic the replay finds, with `--crash` or not, is a
    verdict: `ok: false` and no code, as `replay`'s differing runs are;
  - the MCP server (1133, done) makes a tool of each operation in
    `Op::ALL`: give `crash` its arms in `mcp::host::one_off` and
    `mcp::description`, and `replay`'s `crash` flag comes with its
    input;
  - with 2010 (done, public API diffs): the structs that gain a field
    (`ReplayInput`, `ReplayStep`, `ReplayOutput`, `CrashReport`) are
    `#[non_exhaustive]`, and so is `Op`, with `Op::ALL` a slice
    (**from 2086, done**), so neither the new fields nor the new
    operation is a break, and no `Breaking:` line is planned for them;
  - the new public types (`CodeFrame`, `CrashInput`, `CrashOutput`,
    `NamedFrame`, `PanicAt`, `CrashReplay`) follow ADR-0039
    (`#[non_exhaustive]` and constructors).
  - with 3060 (a debug build's crash report names the session it was
    recording): if 3060 is done, `CrashOutput` also carries the
    report's `session` (`Option<String>`, a file name), and
    `crash::text` shows it. When a report names a session, the refusal
    of a report with no record and the rules-only line in `limits` add
    "its session plays the whole game: `shuyi run --session <file>`".
    `docs/crash-reports.md`'s paragraph on `shuyi crash` names the
    field. If not, nothing to do (3060 adds them, its clause says so).
  - **Either order with 3337:** if 3337 is done, `CrashOutput` also
    carries the report's `fault` (`NativeFault`: kind, code, address)
    and `dump` (the minidump's file name), and `crash::text` shows
    them; if not, nothing to do (3337 adds them, its clause says so).
  - **From 2077 (done):** dev and test programs are built without
    debug info (`strip = "debuginfo"`), but shuyi-cli's: the DWARF
    test relies on the root `Cargo.toml`'s
    `[profile.dev.package.shuyi-cli]` override (`strip = "none"`), so
    keep it, and put the test in `shuyi-cli`.
- **Docs:** `docs/contract.md`'s table gains `crash` and `replay
  --crash`. `docs/crash-reports.md` (3046) gains: reading a report with
  `shuyi crash`, keeping and passing the debug file, and reproducing
  with `replay --crash`.
- **The skill pack. From 1134 (done):** write its `shuyi-debug-replay`
  skill's crash path (`skills/shuyi-debug-replay/SKILL.md`), in place
  of the paragraph 3046 left there (the report's `record` copied to a
  file and replayed by hand, the one command still to come): `shuyi crash <report> --symbols
  <file>` reads the report, `shuyi replay --crash <report>` reproduces
  it (`ok: false` while it panics), and the skill loops fix, then
  replay, until `ok: true`; `check-skills` passes. If **Size** moves
  `--symbols` to the follow-up, the skill leaves `--symbols` out and
  the follow-up adds it.
- **For the game:** its tools call `turns`, whose signature doesn't
  change. Code that builds `ReplayStep` or `ReplayInput` by hand gains
  a field. Say so in the PR; the game's PR that moves the pin follows.
- **Size, decided after step 4.** Count the changed lines of `crash`
  without `--symbols`, the panic in `turns` and `replay --crash`, with
  their tests (schemas aside). If they pass about 350, `--symbols` (the
  lookup, `NamedFrame`'s names, the debug file's docs) goes to a
  follow-up ticket (the `write-ticket` skill, After MVP, `blocked_by:
  ["2520"]`), written in this PR from this ticket's bullets, with their
  acceptance criteria and tests. `CrashOutput.frames` and its schema
  then come with the follow-up too. The lines of 3012, 3047 and 3050
  that name 2520's debug file then name the follow-up.

**Out (do not do):**
- Keeping the debug file when a game is exported or released: 3004
  (`shuyi export --release`), 3012 (the Steam Deck) and 3050 (a release
  workflow).
- Windows PDB files: the GNU toolchain writes DWARF. A game built with
  MSVC is out until one asks.
- Frames on macOS and on the web.
- A session record (2518, 2519) in a report, and its replay: 3060.
- The rest of the `shuyi-debug-replay` skill: 1134 wrote it.
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
5. Add `--symbols` and the lookup with `addr2line` over the frames
   2598 writes.
6. Regenerate the schemas and add their line to `versions.json`;
   raise `VERSION` only if `cargo xtask contract` finds a break.
7. On the machine at hand (Windows GNU): build hello-grid for release
   with its debug file kept as the docs say, run the stripped copy with
   `SHUYI_PANIC_AT_FRAME=3` (3046), and run `shuyi crash` with
   `--symbols`. Put the answer in the PR description.
8. Write the docs, and run the gates.

## Acceptance criteria

- [x] ~~A `shuyi-cli` test finds an `#[inline(never)]` function of its
      own test binary in the symbol table, and `crash --symbols <the test
      binary>` names an offset a few bytes into it with the function, its
      file and a line inside it.~~ (not done: `--symbols` moved to 3441 by
      **Size**, with this criterion)
- [x] ~~An offset outside every function is counted in `limits`, not an
      error.~~ (not done: moved to 3441 with `--symbols`)
- [x] ~~A report with no `frames` reads, and `limits` says why there are
      none.~~ (not done: moved to 3441 with `CrashOutput.frames`; a report
      older than its fields reads here)
- [x] ~~On Linux and Windows GNU, a `shuyi-cli` test names a frame of a
      report the hook wrote in a test binary (2598's frames) with
      `addr2line`'s `Loader` over that binary.~~ (not done: moved to 3441
      with `--symbols`)
- [x] `replay --crash` through a test game whose `GameTools::replay`
      runs `turns` over rules that panic on one command: a report whose
      record panics at step 3, with the place the replay gives, answers
      `ok: false`, `step: 3` and `same_place: true`; with another line
      in the report, `same_place: false`; with the rules fixed, `ok:
      true` and the rules-only line in `limits`.
- [x] `same_place` matches `src\rules.rs:9:5` with
      `/cargo/git/checkouts/x/1/src/rules.rs:9:5`, and not with
      `src/rules.rs:10:5`.
- [x] `replay` on a record that panics answers `ok: false` with the
      panicking step instead of ending the program.
- [x] A report with no record fails with the reason, and `--crash` with
      a second record exits with 2.
- [x] The PR description has step 7's answer and the size difference;
      ~~its frames named in `shuyi_app` and hello-grid~~ (not done: the
      named frames are 3441's step 5)
- [x] The schemas have their line in `versions.json`, and
      `contract::VERSION` rose only for a break (`cargo xtask contract`
      passes).
- [x] `cargo deny check` passes with no change to `deny.toml`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the lookup at `offset - 1` and an inlined frame; the symbol
  table as the fallback name; `same_place`; `turns` stopping at a panic with its message and place; the
  quiet hook passing other threads' panics on.
- Integration: `crash` (with and without `--symbols`) and `replay
  --crash` through the contract's command line, with the exit codes
  (`crates/cli/src/contract/cli.rs`'s tests and the fake game in
  `crates/cli/src/contract/fake.rs`); the `crash` answer as JSON against
  its schema.

## Completion notes

**What was done.**

- `shuyi crash <report>` (`Op::Crash`, `crates/cli/src/contract/crash.rs`):
  a report read in `shuyi` itself, an arm of `without_project`, the MCP
  server's `crash` tool, a help line. `CrashInput { report }`;
  `CrashOutput` holds the report's message, place, `fatal`, thread, time,
  versions, frame, top screen, log, std's backtrace and `has_record`.
  `read_report` is shared with `replay --crash`.
- `turns` catches a panic (`crates/cli/src/contract/replay.rs`): each
  `apply` runs in `catch_unwind`, under a panic hook installed once
  (`Once`) over the one that was there. It keeps a quiet panic's
  `file:line:column` in a thread-local and passes every other panic on.
  The step gets `panic: Option<PanicAt>`, its hash is the state the
  panic left, the run stops, and `outcome` is `None`. `replay` without
  `--crash` on such a record is `ok: false` with the limit "the rules
  panicked at step N of <record>: <message>"; `replay::text` shows it.
- `replay --crash <report>`: `ReplayInput::crash`, the record a report
  holds run once through `GameTools::replay` and the stamp check, and
  `ReplayOutput::crash: Option<CrashReplay>` (`reported`, `replayed`,
  `step`, `same_place`, with `replay::same_place` comparing the last
  three parts of the paths). `ok` is "plays to its end without a
  panic"; the rules-only line goes in `limits` when it does.
- Two problem codes with their pages: `SH3018` (`crash-report`, a file
  that isn't a crash report) and `SH3019` (`no-crash-record`).
- `docs/contract.md`'s table (`crash`, `replay --crash`, the panic in
  `replay`), `docs/crash-reports.md` (*Reading a report*, *Replaying
  it*), the `shuyi-debug-replay` skill's crash path (read, reproduce,
  fix until `ok: true`), the README's operations.
- Tests: `crash` (reads, an old report, unreadable and not a report, the
  arguments, the answer against its schema with a small checker of the
  JSON Schema the contract uses); `turns` stopping at a panic with its
  message and place; the quiet hook passing another thread's panic on;
  `same_place`; `replay` on a panicking record; `replay --crash` through
  the fake game's `turns` over rules that panic on command 9 (step 3,
  `same_place` true, false with another line, `ok: true` with the rules
  fixed) and its refusals (`SH3019`, `SH0001` exit 2, `SH3018`, `SH3004`,
  `SH3002`, `SH3007` and `--any-stamp`); the exit codes through the
  command line; hello-grid's stamp test refusing a crash report whose
  record has other content (`SH3007`) and replaying it with
  `--any-stamp`.
- Schemas: `crash.input.json` and `crash.output.json` new,
  `replay.input.json` and `replay.output.json` regenerated, a line in
  `versions.json`. `cargo xtask contract` finds no break, so `VERSION`
  stays 62 (ADR-0150).

**Step 4, the size.** `crash` without `--symbols`, the panic in `turns`
and `replay --crash`, with their tests, came to about 1,150 counted
lines (`cargo xtask pr-size`, schemas aside), against the 350 at which
**Size** sends `--symbols` on. So `--symbols` went to **3441**
(`shuyi crash --symbols`, After MVP, `blocked_by: ["2520"]`), written in
this PR from this ticket's bullets: the lookup with `addr2line`,
`NamedFrame`, `CrashOutput.frames` with its schema, the limits on frames,
the debug file's docs, and step 7's named frames. The lines of 3012 and
3047 that named 2520's debug file now name 3441; 3050 is no longer open.
The PR would split cleanly between `crash` and the panic caught in
`turns` with `replay --crash`; it stayed one, under 1,500 lines.

**Deviations.**

- With `--symbols` the acceptance criteria on frames went to 3441 and
  stay unticked here: the symbol-table test, the offset outside every
  function, a report with no frames (its limit is about the frames
  `CrashOutput` no longer carries), the frame named from a binary's own
  stack, and step 7's named frames.
- 3441's frame test takes its own stack with `backtrace` (a
  dev-dependency at most) instead of 2598's hook: `shuyi-cli` may never
  depend on `shuyi-app` (`tools/layers.ron`), and its tests may hold no
  `unsafe` (`unsafe_code = "forbid"`).
- `CrashOutput` also has `fatal`, which `CrashReport` gained after the
  ticket was written (ADR-0083).
- `ReplayStep::new(step, command, hash)` was added, so the engine's own
  code builds a step with its new field in one place (`turns`, `ticks`,
  the fake game).
- `crash` reads its report from the working folder, inside a project
  too: `main_with` gives `without_project` the working folder before it
  looks for a project, as for `sfx` and `attach`, and a path typed in a
  shell means that folder.
- The ticket was 36 days old: its Context named `VERSION` 4 at line 52
  and four steps in *The schemas*; fixed to 62, line 84 and five steps.
- 3060, 3071 and 3337 were open: 3060's and 3337's clauses on 2520 now
  say what holds (**From 2520 (done)**); 3071's `object` clause moved to
  3441, which writes the other side.

**Step 7 (manual, on `main` at `03ac186e`, Windows GNU).** hello-grid's
debug build, `SHUYI_PANIC_AT_FRAME=30`, exited with 101 and wrote
`crash-1795755373932.ron`. `cargo shuyi crash` read it: "a panic on
purpose (SHUYI_PANIC_AT_FRAME) at crates\app\src\lib.rs:706:9", thread
main, ended the game, engine 0.1.0, game 0.1.0, windows x86_64 debug,
frame 30, top screen title, no record of play, then std's backtrace and
the log. `cargo shuyi replay --crash` refused it with `SH3019`: the
window build has no `--stage`, and at frame 30 the title was on top, so
there was no stage to record. With a bot try's record (from `playtest
--json`) put into a copy of that report, `replay --crash` ran it in
hello-grid's tool program: 23 steps, "the replay played to its end
without a panic", the rules-only limit, exit 0. The frames named with
`--symbols` are 3441's step 5.

Breaking: none. The new fields are on `#[non_exhaustive]` types
(`ReplayInput`, `ReplayStep`, `ReplayOutput`), `Op` is
`#[non_exhaustive]` with `Op::ALL` a slice, and the new types
(`CrashInput`, `CrashOutput`, `PanicAt`, `CrashReplay`) are
`#[non_exhaustive]` with constructors (ADR-0039).

Behaviour: shuyi-cli: `replay` on a record whose rules panic answers
`ok: false` with the step's `panic` and a limit naming the step, where
the game's tool program used to end with no answer (`SH3005`).

Behaviour: shuyi-cli: `replay::turns` installs a panic hook once per
process, over the hook that was there: a panic inside a step of `turns`
prints nothing and is caught; every other panic goes to the old hook as
before.

Behaviour: shuyi-cli: `replay`'s usage gains a second line (`replay
--crash <report>`), and its wrong-argument error reads "one or two
records, or --crash and one crash report".

Behaviour: shuyi-cli: two new problem codes, `SH3018` (`crash-report`)
and `SH3019` (`no-crash-record`).

Book: none, the book's `replay` (box-pusher, step 9) still holds as it
is; the crash path is `docs/crash-reports.md`'s, which the docs map
names for crash reports.

Game: nothing to adopt, the game's tools call `ReplayInput::new` and
read `steps` and `refused` (`gh search code --repo Zafnok/visions-of-shuyi
ReplayInput`, `ReplayStep`, `ReplayOutput`: one file,
`crates/tools/src/game_tools/tests.rs`, at the pin `96eb097c`), and
none of its records panic. A game that wants `replay --crash` runs it on
a report its `crash_record` filled, with no code change.
