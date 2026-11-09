# ADR-0094: Release exports ship no symbols, no debug features and no state on demand

- **Status:** Accepted
- **Date:** 2026-11-09
- **Related tickets:** 3099, 2917, 3085, 3303

## Context

Research ticket 2917 ([its report](../research/2917-reverse-engineering-angles-not-yet-covered.md),
section 1) read what a shipped game hands a reverse engineer by the engine's
own doing:

- Windows and Linux exports ran `objcopy --strip-debug`, which keeps the
  symbol table, so every mangled function name shipped. macOS and the web
  used the release profile's `strip = true`, unless a game's profile said
  otherwise.
- `export` refused the `link` feature only; `debug-tools`, `capture` and
  `sound-probe` were "never in a shipped build" by comment alone.
- Every build wrote the game's record of play into a crash report and read
  `SHUYI_PANIC_AT_FRAME`: together, a state dump on demand.
- `wasm-opt` was skipped silently when missing.
- `SHUYI_PERF` was read, and named in a warning, in every build; `Ctx::log`
  kept its info and debug lines in release.

None of this is protection, and nothing here claims any
([`decisions.md`](../decisions.md), 2026-10-31). It removes what the engine
adds for free. [ADR-0058](0058-export-packages-carry-nothing-of-the-build-machine-paths-remapped-and-searched-the-linux-floor-in-the-sniper-sdk-and-a-checked-package-per-target.md)
(nothing of the build machine) is the precedent;
[ADR-0079](0079-crash-reports-one-panic-hook-per-backend-writing-through-the-games-storage-offered-once-and-the-games-record-added-on-native.md)
made the crash report, with its record in every build.

## Decision

1. **No symbol table.** A release Windows or Linux export ships
   `objcopy --strip-all` of the program; `<package>.debug` keeps it whole,
   symbols and line tables, and still names a crash's functions and lines
   (`.text` is at the same address in both). A release build without that
   companion (macOS) and the web build set `CARGO_PROFILE_RELEASE_STRIP=true`
   over the game's profile.
2. **No debug feature.** `export::DEBUG_FEATURES` lists `link`,
   `debug-tools`, `capture` and `sound-probe`, each with what it is;
   `export::no_debug_features` (was `no_live_link`) refuses any of them in a
   build's arguments, of the package or a dependency (`game/capture`), by
   name, with `SH1002`. The live link is also refused as Cargo resolves the
   build ([ADR-0096](0096-an-export-asks-cargo-what-each-build-turns-on-and-refuses-the-live-link-before-anything-is-built.md),
   whose `live_link::refuse` runs this check first); the other three, as
   resolved, are ticket 3303's.
3. **No state on demand.** `shuyi_app::crash::CrashReports` gains
   `record_in_release` (`with_record_in_release()`), and `records_play()` is
   `shuyi_ui::DEBUG_TOOLS || record_in_release`. Only then does `run` add the
   game's record to a report and read `SHUYI_PANIC_AT_FRAME`. On the short
   path, `BasicGame::crash_record_in_release()` sets it. A report keeps its
   backtrace and log either way. The opt-in is code, not a `shuyi.ron` field:
   the record itself is code (`Start::crash_record`), and a project file read
   by the tools would need its way into the program.
4. **What only a developer reads stays in debug.** Without debug tools,
   `Ctx::log` drops info and debug lines where they are written (so a literal
   the compiler sees is never used goes too), `run` prints and keeps for a
   report only warnings and errors, and `perf_file::start` returns before
   reading `SHUYI_PERF`, so the name isn't in the program. The filter is
   `DEBUG_TOOLS`, not `debug_assertions` alone, so a release tool program
   built with `debug-tools` keeps its whole log.
5. **`wasm-opt` is never skipped silently.** `wasm-opt -Oz --strip-debug
   --strip-producers` runs on a release web build, and the binary is then
   refused if it still holds a `name` or `producers` custom section
   (`web::custom_sections`, a reader of our own: a dozen lines, no
   dependency). Missing, it is a warning naming Binaryen and how to install
   it; a release `export web` fails there instead (`SH1003`) unless
   `shuyi.ron`'s `export` part says `wasm_opt: optional`
   (`project::WasmOptRule`).
6. **The exposure report says it.** `BuildInputs` gains `stripped` (from
   `Exporter::strips_symbols`: a desktop release build, a web release build
   whose binary has no `name` section; `false` where not said, the Playdate)
   and `panic` (`unwind`, or `abort` on the Playdate). The contract's version
   moves to 42.
7. **`panic` stays `unwind`.** A release export doesn't set
   `CARGO_PROFILE_RELEASE_PANIC=abort`.

## Consequences

- A shipped Windows or Linux program has no symbol table, and nothing of
  `frame_or_crash` or `SHUYI_PERF` in its strings; CI's export job checks this
  on every PR (`tools/export-check/check.sh`), and on the web the sections.
- A game that ships a debug feature by mistake is told so before it builds.
- A player's report from a release build holds no record unless the game
  asked, so replaying a release crash needs that opt-in.
- `SHUYI_PERF` set on a release build without debug tools now does nothing,
  without the warning it used to print.
- Exporting the web in release needs Binaryen on the machine; CI's web leg
  installs it.
- A game's info and debug lines are gone from a release build's console and
  reports: warnings and errors stay.
- Panic messages and their `file:line` stay in the program: removing them
  needs `panic_immediate_abort`, a nightly feature.

## Alternatives considered

- **`panic = "abort"` for desktop and web exports** — lost: a panic on
  another thread would end the game. On a Linux machine with no sound card,
  quad-snd's own threads panic as the game starts, and the game runs on
  without sound ([ADR-0083](0083-a-crash-report-says-whether-the-panic-ended-the-game-and-only-those-are-offered.md));
  with abort it would never start. A game's record of play could no longer be
  added after its frame's panic, and `crash::quiet`'s caught panics would end
  it too. It would also save only the unwinding tables, not the messages.
- **The opt-in as an `app` field of `shuyi.ron`** — lost: the program reads
  nothing of `shuyi.ron` at run time; the field would need generating into
  the game's code, as `app_info.rs` is, for one boolean a game's code sets as
  easily.
- **`wasmparser` for the section check** — lost: a new dependency for a walk
  over a handful of section headers.
- **A cargo test that release-exports Hello, Grid** — lost: a release build
  with LTO in every `cargo test` run, minutes on a developer's machine. CI's
  export job already builds each release package on its own system; its check
  script holds the package to this ADR there.
- **String encryption or control-flow obfuscation** — out: nightly-only or
  worthless (2917's report, section 2).
