---
id: "3099"
title: "Release exports strip their symbols, refuse the debug features, make the crash play-record opt-in and never skip wasm-opt silently"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-09
---

# 3099 — Release exports strip symbols and refuse the debug features

## Context

Research ticket 2917 ([its report](../../docs/research/2917-reverse-engineering-angles-not-yet-covered.md),
section 1) read what a shipped game hands a reverse engineer by the
engine's own doing, and found five things that cost nothing to fix:

- **Windows and Linux packages keep the full symbol table.** The exporter
  builds with `CARGO_PROFILE_RELEASE_STRIP=false` and line tables so a
  `<base>.debug` companion can symbolicate crash reports
  (`crates/cli/src/export.rs:393-396`), then runs `objcopy --strip-debug`
  on the shipped copy (`crates/cli/src/export/desktop.rs:236-241`).
  `--strip-debug` removes debug sections only, so every mangled Rust
  function name ships. macOS keeps `strip = true` and ships none.
- **Export refuses `link` but no other debug feature.** `no_live_link`
  (`export.rs:745-762`) looks for `link` alone. A game whose `shuyi.ron`
  export part lists `debug-tools` (`crates/ui/Cargo.toml:21`) ships the
  debug menu, `SHUYI_PERF` traces and session recording; `capture` and
  `sound-probe` are documented in `crates/app/Cargo.toml` as "never in a
  shipped build" and nothing enforces it.
- **Every release build writes a play record into its crash report**
  (`crates/app/src/lib.rs:528-532`, `crates/app/src/crash.rs:200-209`)
  and honours `SHUYI_PANIC_AT_FRAME` in any build (`crash.rs:26-27`).
  Together they are a state dump on demand: force a panic at frame N and
  read the record.
- **`wasm-opt` is skipped silently when missing** (`crates/cli/src/web.rs:600`),
  and nothing strips the Wasm `name` or `producers` sections explicitly; a
  web export from a machine without Binaryen ships names.
- **The release profile leaves `panic` at unwind**, so panic messages and
  remapped `file:line` strings stay, and `Ctx::log` lines at Debug and
  Info level are compiled into release (`crates/ui/src/ctx.rs:767`).

The report's honest estimate is one to three hours of an analyst's
orientation lost, unmeasured. This ticket is not protection and claims
none ([`decisions.md`](../../docs/decisions.md), 2026-10-31). It removes
leaks the engine adds for free. ADR-0058 (export packages carry nothing of
the build machine) is the precedent; this extends it to symbols and debug
surfaces.

## Nick input

None.

## Scope

**In:**
- The shipped Windows and Linux copy is `objcopy --strip-all`; the
  `<base>.debug` companion is unchanged and still symbolicates.
- `no_live_link` becomes a check of every feature an export never turns
  on: `link`, `debug-tools`, `capture`, `sound-probe`, each with its own
  message naming the feature, and a test per feature.
- The crash report's play record and `SHUYI_PANIC_AT_FRAME` are on in
  debug builds and off in release unless the game opts in (a
  `BasicGame` setting or an `app` part field in `shuyi.ron`; pick one and
  document it). The report keeps its backtrace and log ring.
- `run_wasm_opt` prints a warning naming the missing tool and how to
  install it; `shuyi export web` in release fails unless `wasm-opt` ran or
  the project's export part says `wasm_opt: optional`. The optimized Wasm
  is checked for a `name` custom section and a `producers` section and
  fails with their names if either is present.
- Release exports set `panic = "abort"` through `CARGO_PROFILE_RELEASE_PANIC`
  for desktop and web, unless that breaks the crash report's `catch_unwind`
  path, in which case the completion notes say so and the setting is left
  out.
- `Ctx::log` drops Debug and Info lines at the print site in release
  (`cfg!(debug_assertions)`), keeping Warn and Error; the crash ring keeps
  Warn and Error only in release.
- A test that a release export of `hello-grid` for the host has no Rust
  symbol names (`nm` or `llvm-nm` empty, and `strings` free of a known
  engine function name), no `SHUYI_PERF` string, and no `name` section in
  its Wasm.
- The exposure report of 3081 gains `stripped: true|false` and
  `panic: abort|unwind` under `inputs`, and the contract's version moves.

**Out (do not do):**
- String encryption, control-flow obfuscation or any dependency for it
  (2917's report, section 2: nightly-only or worthless).
- Baking a pack for desktop and web exports (3300).
- Removing panic location strings entirely (`panic_immediate_abort` needs
  nightly).
- Any claim of protection in docs or the book.

## Implementation steps

1. In `crates/cli/src/export/desktop.rs::pack`, change `--strip-debug` to
   `--strip-all` for the shipped copy; keep the companion. Confirm
   `objcopy_help()` text still fits. On the GNU host, check that
   `--strip-all` leaves a runnable exe (it does for MinGW; test it).
2. Rename `no_live_link` to `no_debug_features` in `crates/cli/src/export.rs`,
   table-driven over the four names; update the two call sites
   (`export.rs`, `export/playdate.rs`) and the tests.
3. In `crates/app/src/crash.rs` and `lib.rs`, gate `add_record` and the
   `SHUYI_PANIC_AT_FRAME` read behind `DEBUG_TOOLS || opted_in`, where the
   opt-in is a new `BasicGame::crash_record_in_release()` (additive,
   ADR-0039) or a field of the `app` part; document the choice in
   `docs/contract.md` and the book's crash-report passage.
4. In `crates/cli/src/web.rs`, make `run_wasm_opt` return whether it ran;
   the release export path fails when it did not, unless the export part's
   new `wasm_opt: optional`. Add a section check over the final Wasm with
   `wasmparser` (already a dependency? if not, MIT OR Apache-2.0, check
   with `cargo deny`), or `wasm-tools strip --all` if present.
5. Set `CARGO_PROFILE_RELEASE_PANIC=abort` in `Exporter::build` for
   desktop and web; run the crash-report tests; if they need unwind, drop
   this step and say so.
6. In `crates/ui/src/ctx.rs::log` (or `crates/app/src/lib.rs:639-655`),
   skip Debug and Info in release.
7. Add the export test of step "In" to `crates/cli/tests/` next to the
   path-search test from 3004; it runs where `objcopy` and the host target
   exist and is skipped with a message elsewhere.
8. Extend 3081's exposure report and bump the contract version; update
   the docs map entries the `document` skill names.

## Acceptance criteria

- [x] A release Windows or Linux export's shipped program has an empty
      symbol table and the companion still resolves a crash's frames.
- [x] `shuyi export` refuses `debug-tools`, `capture` and `sound-probe`
      with a message naming the feature.
- [x] A release build writes no play record and ignores
      `SHUYI_PANIC_AT_FRAME` unless the game opts in; a debug build keeps
      both.
- [x] A release web export fails, with the install hint, when `wasm-opt`
      is missing and the project did not say `optional`; its Wasm has no
      `name` or `producers` section.
- [x] `strings` of a release export has no Debug or Info log text and no
      `SHUYI_PERF`.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `no_debug_features` over `--features a,debug-tools`,
  `-F game/capture`, `--features=sound-probe` and a clean list.
- Unit: `run_wasm_opt` outcome with the fake `WasmOpt` for ran, failed and
  missing; the section check over a Wasm with and without a `name` section.
- Integration: the release export symbol and string test (step 7); the
  crash-report tests in debug and in release with and without the opt-in.

## Completion notes

Done on 2026-11-09. The choices are [ADR-0094](../../docs/adr/0094-release-exports-ship-no-symbols-no-debug-features-and-no-state-on-demand.md).

- **Symbols.** A release Windows or Linux package ships
  `objcopy --strip-all` of the program; `<base>.debug` keeps it whole.
  Checked on this machine (GNU host, MSYS2's binutils): a release
  `export windows` of Hello, Grid has no symbol table (`nm`: "no
  symbols"), the debug file 10,505 symbols, `.text` at 0x140001000 in
  both, and `addr2line -f -e <base>.debug` names `frame_or_crash`'s
  closure at `crates/app/src/crash.rs:216`. The stripped `.exe` runs.
  macOS and the web build set `CARGO_PROFILE_RELEASE_STRIP=true` over
  the game's profile.
- **Debug features.** `no_live_link` is `no_debug_features`, over
  `DEBUG_FEATURES` (`link`, `debug-tools`, `capture`, `sound-probe`),
  each with its own message; the three call sites and the tests moved.
- **Crash record.** The opt-in is code, not `shuyi.ron`:
  `CrashReports::with_record_in_release()` (a new field,
  `record_in_release`, on the `#[non_exhaustive]` struct) and
  `BasicGame::crash_record_in_release()`. `records_play()` is
  `DEBUG_TOOLS || record_in_release`; only then does `run` add the
  record and read `SHUYI_PANIC_AT_FRAME`. Checked: the release package
  run with `SHUYI_PANIC_AT_FRAME=3` ran 8 seconds and wrote no report.
  CI's crash step runs a debug build, which keeps both.
- **wasm-opt.** `run_wasm_opt` returns `WasmOptRun` (ran, failed,
  missing) and warns with Binaryen's install hint when missing; it now
  runs `-Oz --strip-debug --strip-producers`, and a binary that still
  holds `name` or `producers` is refused, naming them (a section reader
  of our own, `web::custom_sections`, not `wasmparser`). A release
  `export web` fails with `SH1003` unless `export.wasm_opt: optional`
  (`project::WasmOptRule`); `shuyi web --release` only warns. Checked
  here without Binaryen: the export answers `SH1003` with the hint. The
  release build's own `.wasm` already had no custom section at all.
  CI's web leg now installs Binaryen (`apt-get install binaryen`).
- **Logs.** `Ctx::log` drops info and debug lines without debug tools,
  and `run` prints and keeps for a report warnings and errors only. The
  test is `DEBUG_TOOLS`, not `debug_assertions` alone, so a release
  tool program with `debug-tools` keeps its whole log. Deviation found
  on the way: `SHUYI_PERF` was read in every build (to warn that it did
  nothing), so its name was in every program; `perf_file::start` now
  returns first without debug tools, and the warning is gone (the
  `shuyi-perf` skill says so).
- **Exposure report.** `inputs` gains `stripped` and `panic`; the
  contract moves to 42.
- **Deviation: no `panic = "abort"`.** Left out, as the ticket allows:
  on a machine with no sound card quad-snd's threads panic as the game
  starts and the game runs on without sound (ADR-0083); with abort it
  would end. The record after a frame's panic and `crash::quiet` need
  unwinding too.
- **Deviation: the export test.** Not a cargo test in `crates/cli/tests/`
  (a release LTO build of Hello, Grid in every `cargo test`): CI's export
  job already release-exports Hello, Grid on each system, and
  `tools/export-check/check.sh` now holds each package to it: no
  `frame_or_crash` and no `SHUYI_PERF` in `strings`, no symbol table on
  Windows and Linux (and one in the debug file), no `name` or
  `producers` section on the web. Run here on the Windows package: it
  passes.
- **Handed over:** 3085 resolves all four debug features through the
  real feature graph (its "From 3099" note).

Breaking: shuyi-cli: export::no_live_link: renamed `export::no_debug_features`, which refuses `debug-tools`, `capture` and `sound-probe` beside `link`.

```migration
since: 0.1.0
what: The export's feature check is no_debug_features, and refuses every debug feature.
old: shuyi_cli::export::no_live_link(&args)
new: shuyi_cli::export::no_debug_features(&args)
before: |
  shuyi_cli::export::no_live_link(&args)?;
after: |
  shuyi_cli::export::no_debug_features(&args)?;
find: ["no_live_link"]
```

Breaking: shuyi-cli: export::ExportCx: a new field, `wasm_opt`, the `export` part's `WasmOptRule`; code that builds one field by field adds it.

```migration
since: 0.1.0
what: An export's context says whether a release web build needs wasm-opt.
old: ExportCx { beside, .. }
new: ExportCx { beside, wasm_opt, .. }
before: |
  ExportCx { beside: Vec::new(), /* the other fields */ }
after: |
  ExportCx { beside: Vec::new(), wasm_opt: WasmOptRule::Required, /* the other fields */ }
```

Breaking: shuyi-cli: web::Options: a new field, `require_wasm_opt`; code that builds one field by field adds it or ends with `..Options::default()`.

```migration
since: 0.1.0
what: The web build's options say whether a release build fails without wasm-opt.
old: Options { release, features }
new: Options { release, features, require_wasm_opt }
before: |
  let options = Options { release: true, features: Vec::new() };
after: |
  let options = Options { release: true, features: Vec::new(), require_wasm_opt: false };
```

Breaking: shuyi-cli: export::disclosure::BuildInputs: two new fields, `stripped` and `panic` (`disclosure::Panic`); code that builds one field by field adds them.

```migration
since: 0.1.0
what: The exposure report's inputs say whether the program is stripped and what a panic does.
old: BuildInputs { cargo_lock, .. }
new: BuildInputs { cargo_lock, stripped, panic, .. }
before: |
  BuildInputs { cargo_lock: None, /* the other fields */ }
after: |
  BuildInputs { cargo_lock: None, stripped: true, panic: Panic::Unwind, /* the other fields */ }
```
