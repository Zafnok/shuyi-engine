---
id: "3004"
title: "shuyi export <target>: one command that checks a game and packages it for a target, starting with Windows, Linux, macOS and the web"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["3001", "1129", "3039"]
nick_input: none
completed: 2026-10-29
---

# 3004 — `shuyi export`

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md).
Each target needs a package:

- a zip per desktop OS;
- a folder for the web;
- a Steam Deck build (3012);
- a `.pdx` for the Playdate (3010).

One operation of the tool contract (ADR-0005), `export`, runs the
target's checks and then its exporter, so an agent or a player gets the
same result from the CLI, MCP and skills. This ticket builds the command
and the exporters that need no new backend. The target tickets add
theirs.

### Packages that run on the players' machines

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Godot exports macOS as one Universal 2 binary that runs
on Apple Silicon and Intel Macs. It also exports Linux for arm64 as well
as x86_64
(<https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_macos.html>).
GitHub's arm64 runners (`ubuntu-24.04-arm`) have been free for public
repositories since 2025-08-07, so a Linux build for Arm costs no money.

What this ticket's plan missed (read 2026-10-20):

- **Build paths in every binary.** `[profile.release]` in the root
  `Cargo.toml` (lines 44 to 47) and in both templates' `Cargo.toml` sets
  `lto = "thin"`, `codegen-units = 1` and `strip = true`. Nothing remaps
  paths. `strip` drops symbols but not the file paths in panic messages,
  so a release binary keeps paths such as
  `C:\Users\<name>\.cargo\registry\src\...`. Every package, the web
  `.wasm` included, would carry the builder's user name.
- **Linux.** A binary built on `ubuntu-latest` needs that runner's glibc
  and doesn't start on older systems. 3012 planned Valve's sniper SDK
  container, but only for the Deck. The binary links `libasound.so.2`
  (quad-snd's `quad-alsa-sys`) and `libudev.so.1` (gilrs-core's
  `libudev-sys`). miniquad 0.4.11 opens `libX11`, `libGL` and `libEGL`
  itself at run time (`src/native/linux_x11/libx11.rs`,
  `src/native/linux_x11/glx.rs`, `src/native/egl.rs`).
- **macOS.** CI's `macos-latest` runner (the `test` job in
  `.github/workflows/ci.yml`) is Apple Silicon, so a plain build runs
  only on Apple Silicon Macs.
- **Windows.** The only check was "a zip whose exe runs" on Nick's
  machine. That machine has `C:\msys64\mingw64\bin` on `PATH`
  (`AGENTS.md`, Environment), and the engine's and the templates'
  `.cargo/config.toml` link with that MinGW `gcc`. An exe that needed a
  MinGW DLL (`libwinpthread-1.dll`, say) would run there and nowhere
  else. Nobody has seen it need one, and nothing proves it doesn't.

Why it matters: Visions of Shuyi's packages from these exporters would
otherwise carry Nick's user name. So would its Deck package, since 3012
builds on the `linux` exporter. The Linux package would also fail on
older systems before any bug report came in. Every other game gets the
same packages. The end goal (2026-10-19, `docs/decisions.md`) asks for
many ports. Linux on Arm (a Raspberry Pi 5, an Arm Chromebook's Linux)
is the cheapest one, since it needs no new backend.

## Nick input

None.

## Scope

**In:**
- **`shuyi export <target> [--out <dir>] [--release]`:**
  1. it runs `shuyi check --target <target>` (3001 and 3002) and
     refuses on any error; the `windows`, `linux`, `linux-arm64` and
     `macos` exporters check the `desktop` profile
     (`targets/desktop.ron`), since there is no profile per desktop
     system or architecture;
  2. it builds with the target's Rust triple and the backend's crate;
  3. it collects the assets, packed if the game asks for it (2204,
     optional);
  4. it writes the package and a `export-report.json` (target, version,
     sizes, the check's warnings and `signing`). `signing` is `"none"`
     for `windows`, `linux`, `linux-arm64` and `web`, and `"ad-hoc"` for
     `macos`; 3051 adds the paid kinds. The `version` is the built
     package's version, read with `cargo metadata --no-deps
     --format-version 1`. **Either order with 3048:** if 3048 is done,
     it is `shuyi.ron`'s `app.version` when the part sets one, else the
     package's version; if not, 3048 makes the report read it (its scope
     says so). **Either order with 2516:** if 2516 is done, the report
     also holds its files report's `unread` and `largest`, taken from a
     plain `shuyi check` run (`check --target` makes no files report),
     not computed again here; if not, 2516 adds them (its clause says
     so). **Either order with 3017:** if 3017 is done, the report also
     lists the check's `left_out` (the optional parts the target can't
     run); if not, nothing to do (3017 adds it).
- **Symbols kept per release** (for 2520): with `--release`, the
  `windows` and `linux` exporters (and `linux-arm64`) build with
  `CARGO_PROFILE_RELEASE_STRIP=false` and
  `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, keep that executable
  beside the package as `<package>.debug` (never inside the package),
  and package a copy stripped with `objcopy --strip-debug` (binutils:
  MSYS2's mingw-w64 on Windows GNU, the build container's on Linux;
  `doctor` says how to install it if missing). Stripping after the link
  moves no code, so a crash report's offsets (2520) name the right
  lines. `export-report.json` names the debug file with the game's
  version and the target, and CI uploads it as an artifact beside the
  package.
- **The `Exporter` trait** in `shuyi-cli`: `id()`, `triples()`,
  `export(&ExportCx) -> Result<ExportReport, ExportError>`. Exporters
  are registered by target id. A missing tool (a missing Rust target,
  the Playdate SDK) gives an error that says how to install it.
- **Exporters in this ticket:**
  - **`windows`.** `x86_64-pc-windows-gnu`, then a zip with the exe and
    `NOTICES.txt`. MSVC isn't needed.
  - **`linux`.** `x86_64-unknown-linux-gnu`, then a `tar.gz`. It is
    built in CI, not on Nick's machine, inside Valve's Steam Linux
    Runtime 3.0 ("sniper") SDK container. Valve publishes it as
    `registry.gitlab.steamos.cloud/steamrt/sniper/sdk`: check it, pin it
    by digest, and record the image and how it was checked. The binary
    then needs no glibc newer than the container's (Debian 11's, 2.31).
    3012 reuses this job for the Deck.
  - **`linux-arm64`.** The same for `aarch64-unknown-linux-gnu`, built
    natively on GitHub's `ubuntu-24.04-arm` runner. Use the sniper SDK
    image if Valve publishes it for arm64. Otherwise use
    `debian:bullseye` with the `-dev` packages that ci.yml's "Install
    macroquad system libraries" step lists, so both Linux packages have
    the same glibc floor.
  - **`macos`.** It builds in CI only, on macOS runners, as one
    universal binary. It builds `aarch64-apple-darwin` and
    `x86_64-apple-darwin` (rustup still ships its std) and joins them
    with `lipo -create`. The output is a `.app` folder with an ad-hoc
    signature: `codesign --force --sign -` on each piece of nested code,
    deepest first, then on the bundle (not `--deep`), after `lipo`
    (which drops signatures) and after every file inside the bundle is
    written. Apple silicon runs only signed code, and a bundle that
    isn't signed as a whole can be reported to a player as damaged
    instead of offering Open Anyway. A Developer ID signature and
    notarization are parked 3051. The bundle holds a minimal
    `Contents/Info.plist` (`CFBundleExecutable`, `CFBundlePackageType`
    `APPL`), written before the signature, since `codesign` and Finder
    find the executable through it. 3049 replaces it with one from the
    game's identity.
  - **`web`.** It reuses the existing web build (`cargo shuyi web`),
    and the output folder holds `index.html`, the wasm, the JS plugins
    and the `NOTICES.txt` that `shuyi web` writes (3039). It also writes
    a zip of that folder with `index.html` at the zip's root, which
    itch.io and other portals take
    (<https://itch.io/docs/creators/html5>). `export-report.json` gives
    the zip's size, which 3008 compares with the web profile's
    `package_bytes`. **Either order with 3052:** if 3052 is done, its
    README section "Putting a web build online"
    (`crates/cli/web/README.md`) names this zip in its itch.io point;
    if not, 3052 names it (its clause says so).
    It needs the game's `web` part, as `cargo shuyi web` does, and
    refuses a game without one with that command's error. The
    templates have no `web` part or web `build.rs` yet (2524's Out
    leaves them to a ticket of its own), so CI exports the examples,
    and a game made by `shuyi new` exports for `web` once it adds them.
  - **Either order with 2407 (parked):** one `web` package also serves
    the `web-touch` target 2407 plans for phones' browsers. If 2407 is
    done and `targets/web-touch.ron` exists, `shuyi export web` also
    runs the `web-touch` check when the game's `shuyi.ron` lists that
    target. If not, nothing to do: the ticket that adds `web-touch`
    makes `shuyi export web` run it.
- **The bench's package size.** **Either order with 3008:** if 3008 is
  done, `shuyi bench` takes each package's size from
  `export-report.json` (for `web`, the zip's) in place of the asset
  bytes it measures until then, with a test; if not, nothing to do
  (3008 reads the report).
- **`--run`.** `shuyi export <target> --run` runs the package after
  writing it. The `Exporter` trait also has `run(&RunCx) -> Result<(),
  ExportError>`, whose default answers "no run step for <target> yet"
  (exit 1).
  - In this ticket the host's own desktop target (`windows` on Windows,
    `linux` on Linux, `macos` on macOS) starts the program from the
    folder the package was made from and prints its output until it
    exits. Every other target gives the default answer.
  - `web`: either order with 2528. If it is done, `web`'s run step
    serves the exported folder through 2528's server
    (`crates/cli/src/web/serve.rs`) on 127.0.0.1, with no dev script and
    no watching, and prints the URL. If not, `web` keeps the default and
    2528 adds it.
  - The Playdate and the Steam Deck add theirs (3010, 3011, 3013).
- **A note for players.** Every `windows` and `macos` package carries
  `HOW-TO-OPEN.txt`, a few lines in English: on macOS 15 and later, open
  the app once, then System Settings > Privacy & Security > Open Anyway;
  on Windows, if SmartScreen stops it, More info > Run anyway. 3051
  drops or rewords it for signed packages.
- **License notices.** Every package carries `NOTICES.txt`, written by
  3039's `notices` module (`crates/cli/src/notices.rs`): the crates for
  the package's triple and the game's works (its `notices.texts`), from
  one function every exporter calls. No separate tool.
  - **Either order with 2223:** if 2223 is done, the game's works also
    hold its credits (2223's `credits::works(project)`), and the
    fixture game of the `web` and `linux` export tests has a `credits`
    part, whose section "Works the game credits" its `NOTICES.txt`
    shows; if not, nothing to do here (2223 adds both).
  - Native packages also need the engine's files that reach a game and
    aren't crates. Today that is the SDL_GameControllerDB that `gilrs`
    compiles in (Zlib, `crates/app/SDL_GameControllerDB-LICENSE.txt`),
    which the crate walk misses because it sits in a subfolder of
    `gilrs`'s package.
  - Make that list data: each file's work, license, license text and
    the builds that carry it (`web`, `native`), kept equal to
    `THIRD_PARTY_ASSETS.md` by a test, with 3039's table of the web
    shell's scripts moved into it.
- **No live link in an export.** An export never turns on a feature
  named `link` (2521's debug-only listener), whatever the game's
  `web.features` list. A unit test checks the cargo arguments of every
  exporter.
- **Either order with 2717** (RTS passes on rayon threads): if 2717 is
  done, `export` refuses a game that turns on `shuyi-rts`'s `parallel`
  feature for a target whose profile isn't `desktop` or `steam-deck`
  (of this ticket's targets, only `web`), and names the feature; a unit
  test checks it. If not, nothing to do (2717 adds the rule when it
  lands).
- **Nothing of the build machine in a package.** One function remaps
  the build's paths. Every `--release` build the exporters make goes
  through it, and so does `shuyi web --release` (`web::run`,
  `crates/cli/src/web.rs`, line 146). It remaps `CARGO_HOME` (the
  registry, and the git checkouts where the engine's sources live when a
  game depends on it by revision), `RUSTUP_HOME`, the target folder
  (`web::target_dir`, line 113) and the project root to short fixed
  names: `/cargo`, `/rustup`, `/target` and the package's name.
  - First check whether Cargo's `trim-paths` profile setting is stable
    on the pinned toolchain (`rust-toolchain.toml`, 1.98.1). If it is,
    use it (`CARGO_PROFILE_RELEASE_TRIM_PATHS`).
  - If it isn't, pass `--remap-path-prefix=<from>=<to>` through
    `CARGO_ENCODED_RUSTFLAGS`. Add it to the flags already in
    `CARGO_ENCODED_RUSTFLAGS` or `RUSTFLAGS` (CI's `-D warnings`), and
    never replace them. As with the wasm link argument
    (`crates/cli/web/README.md`), a game's `rustflags` in its
    `.cargo/config.toml` then don't reach these builds, and the docs say
    so.
  - After the build, the exporter searches the binary (or `.wasm`) for
    the home folder (`HOME` or `USERPROFILE`), `CARGO_HOME` and the
    project root, with either slash. If one is there, it refuses the
    package and names the path.
- **What a package loads, checked.** After the build, the exporter reads
  the executable with the `object` crate. It checks each library the
  executable loads against its target's list in
  `crates/cli/export/libraries.ron`. That file is embedded with
  `include_str!` and read with `ron`. It is the tool's own table, not an
  asset kind, so it isn't registered in `Formats`. The lists are data so
  that 3032 and 3034, which replace quad-snd and macroquad, change one
  file.
  - `windows`: the PE import table (`PeFile::import_table`) names only
    Windows' own DLLs and its API sets (`api-ms-win-*`). No
    `libgcc_s_seh-1.dll`, `libwinpthread-1.dll` or `libstdc++-6.dll`.
  - `linux` and `linux-arm64`: `DT_NEEDED` names only glibc's own
    libraries, `libgcc_s.so.1`, `libasound.so.2` and `libudev.so.1`. The
    version needs (`SectionTable::gnu_verneed`) ask for no `GLIBC_`
    version above the list's floor.
    **Either order with 3032:** if 3032 is done, leave out
    `libasound.so.2`, since ALSA is then opened with `dlopen`; if not,
    nothing to do (3032 removes it).
  - `macos`: each slice of the universal binary (`MachOFatFile`) loads
    only from `/System/Library/` and `/usr/lib/`, and both slices are
    there.
  - A library the package itself carries beside the executable passes.
  - A failure refuses the package and names the library and the rule.
    `export-report.json` lists what each binary loads, and its glibc
    floor on Linux.
  - `object` 0.37 is Apache-2.0 OR MIT, both on `deny.toml`'s allow
    list. Use it with `default-features = false` and only the features
    `std`, `read_core`, `elf`, `macho` and `pe` (no `compression`).
    Either order with 2520: if 2520 is done, use the `object` version it
    brought, so `Cargo.lock` holds one.
- **Docs.** `docs/targets/desktop.md` covers:
  - what each package runs on: the glibc floor, Apple Silicon and Intel
    Macs, and the oldest Windows Rust's `x86_64-pc-windows-gnu`
    supports;
  - that Windows on Arm runs the x64 exe through Windows 11's Prism
    emulation, untested;
  - Linux on Arm (a Raspberry Pi 5 with 64-bit Raspberry Pi OS,
    ChromeOS's Linux on an Arm Chromebook) with the `linux-arm64`
    package. It is marked untested until someone runs it, with how to
    report;
  - how to make the Linux packages by hand in the container with
    `docker` or `podman`;
  - where a package's log file is on each system (3041's
    `shuyi_app::storage::log_dir`), so a developer can tell a player
    what to send.

  These packages all run the `desktop` profile.
- **An ADR** (the `write-adr` skill, the next free number) extending
  ADR-0008. Every exporter's package passes the path search and the
  library audit against its target's list; 3010, 3012 and 3015 follow
  it too. The Linux floor is the sniper container's glibc.
- **Either order with 2520:** if it is done, the by-hand recipe in
  `docs/crash-reports.md` gains the same path remapping (or points at
  `shuyi export --release`). It also says that `shuyi crash` names files
  by the remapped names. If 2520 isn't done, 2520 adds it (its scope
  says so).
- **CI.** Export every example for `windows`, `linux` (in the sniper
  container) and `web` on every PR. Export them for `macos` (on
  `macos-latest`) and `linux-arm64` (on `ubuntu-24.04-arm`) on pushes to
  `main`. Each export runs the path search and the library audit, and
  each `macos` export runs `codesign --verify --strict --verbose=2` on
  the `.app`. Upload the packages as artifacts for 7 days. Write the
  export steps per exporter (runner, toolchain setup) as one job over a
  matrix, so 3050 can move them into a reusable
  `.github/workflows/export.yml`.
- **Either order with 2739** (snake, parked): if 2739 is done,
  `examples/snake` is one of the examples CI exports, with nothing of
  its own; if not, nothing to do (2739 has `shuyi export web` build it
  when it lands).
- **The agent eval.** If 2524 is done, its `Web` check
  (`crates/xtask/src/eval/grade.rs`) runs `shuyi export web` and reads
  that folder, and the turn-based tasks in `evals/tasks/` gain an
  `Export` check for `linux` (the package exists), run in the sniper
  container as CI's `linux` export is, since outside it the audit
  refuses a newer glibc. If 2524 isn't done, nothing to do: 2524's
  either-order clause moves `Web` to `export` and adds the `linux`
  `Export` check.
- **From 1134 (done):** its `shuyi-ship` skill
  (`skills/shuyi-ship/SKILL.md`) gains a section on `shuyi export
  <target>`: the targets, `--release` and the `.debug` file beside the
  package, `--run`, `export-report.json`, the `NOTICES.txt` in every
  package and the player note; `check-skills` passes.

**Out (do not do):**
- a Developer ID signature, notarization and signing Windows exes:
  parked 3051 (they cost money);
- store uploads (Steam is the game's 0903), and releases from a tag:
  GitHub Releases, checksums, attestations and itch.io (3050). A PR's
  export artifacts are not attested;
- uploading the web zip to itch.io or any portal (3050), portal SDKs and
  ads;
- pre-compressed `.br` and `.gz` copies of the web files: hosts compress
  what they serve, and a pre-compressed file needs server headers;
- the web page's loading screen, errors and hosting notes (3052);
- the game's identity in the packages (the exe's icon and version
  resource, an Info.plist with the game's id, name and version, and
  `.icns`, a `.desktop` file and its icons, web icons, the page's title
  from the name): 3049, which follows this ticket;
- the Steam Deck (3012), Playdate (3010) and mobile (3015) exporters;
- the log file of a shipped build (3041), which comes before this ticket
  in `docs/ROADMAP.md`'s order (step 2): a Windows release build has no
  console (`windows_subsystem = "windows"`), so the file is the only
  record a player can send;
- a macOS `.dSYM` and Windows PDB files (the GNU toolchain writes
  DWARF): follow-ups when a game ships there;
- Windows on Arm as its own build (`aarch64-pc-windows-gnullvm`). It
  needs an llvm-mingw linker that MSYS2's `mingw64` packages don't give,
  and the x64 exe runs there through Windows 11's Prism emulation. Write
  a ticket when a player or a game asks;
- AppImage and Flatpak, 32-bit builds, 32-bit Arm and RISC-V;
- a `targets/*.ron` profile per system or architecture: these packages
  run the `desktop` profile;
- state hashes equal on x86_64 and Arm: 2116;
- checking Linux's system libraries on a developer's machine: 2513
  (`shuyi doctor`, `crates/cli/src/contract/doctor.rs` line 193 lists
  them as unchecked today).

## Implementation steps

1. Add the `export` operation to the contract, with its JSON shape.
2. Check whether Cargo's `trim-paths` is stable on the pinned toolchain.
   Then write the path remapping and the path search, and use them in
   `web::run` for `--release`.
3. Write the `Exporter` trait (with `run`) and the five exporters
   (`windows`, `linux`, `linux-arm64`, `macos`, `web`), with the debug
   file, the web zip, the macOS ad-hoc signature and the player note.
4. Add the library audit and `crates/cli/export/libraries.ron`.
5. Call 3039's `notices` module for every package, and add the data
   list of the engine's non-crate files.
6. Add the CI jobs: the sniper container for `linux`, `ubuntu-24.04-arm`
   for `linux-arm64`, and `macos-latest` with `lipo` for `macos`.
7. Write `docs/targets/desktop.md` and the ADR.
8. If the PR would pass about 600 changed lines, keep the rest here
   (with the container and the universal app). File step 4 and
   `linux-arm64` as one follow-up ticket with the `write-ticket` skill,
   After MVP, blocked by 3004, and say so in the completion notes.
   That ticket names 3049 and 3050: if they are done, it gives
   `linux-arm64` 3049's identity files and 3050's `host()`,
   `container()` and `ci()`; if not, they do (their scopes say so).

## Acceptance criteria

- [x] `shuyi export windows` on an example, on Nick's machine (GNU
      toolchain), gives a zip whose exe runs. Its report lists only
      Windows' own DLLs and API sets, so nothing comes from
      `C:\msys64\mingw64\bin`. (The exe imports only those, by
      `objdump -p`, and CI's `check.sh` checks it; the report's list is
      3071's audit.)
- [x] `shuyi export playdate` on a game that fails `check --target
      playdate` refuses, and prints the check's errors.
- [x] A missing Rust target gives "run `rustup target add …`", not a
      cargo error dump.
- [x] Each package has `NOTICES.txt` from 3039's module, and the native
      ones list the SDL_GameControllerDB with its Zlib text (test).
- [x] CI exports the examples for `windows`, `linux` (in the sniper
      container) and `web` on every PR, and for `macos` and
      `linux-arm64` on pushes to `main`. (`linux-arm64`: 3071.)
- [x] `shuyi export web` writes a zip with `index.html` at its root, and
      `export-report.json` gives its size.
- [x] `shuyi export windows --run` on Windows starts the example (by
      hand, in the Completion notes), and a target with no run step
      gives the default answer (test).
- [x] With `--release`, the `windows` and `linux` packages hold an
      executable with no `.debug_*` section, and `<package>.debug`
      beside them has those sections and the same `.text` address
      (test, reading the files with `objdump -h` or the `object` crate).
      (CI's `check.sh` with `objdump -h`; by hand on Windows.)
- [x] ~~The `macos` package's `.app` passes `codesign --verify --strict`
      on the macOS runner, and its report says `"ad-hoc"`.~~ (not done: CI's `macos`
      leg checks both, on pushes to `main`: not run before the merge.)
- [x] Every `windows` and `macos` package carries the player note
      (test). (The Windows package's test; the macOS one's file list in
      CI.)
- [x] ~~A test feeds the audit the test binary itself
      (`std::env::current_exe()`) on each OS of CI's `test` matrix, and
      the audit names that OS's C library (`kernel32.dll`,
      `libc.so.6`, `/usr/lib/libSystem.B.dylib`).~~ (not done: 3071.)
- [x] ~~A fixture list without one of those libraries refuses the package
      and names the library. A library inside the package passes.~~ (not done: 3071.)
- [x] The path search refuses a byte buffer that holds the home folder
      with either slash, and passes one that holds only the remapped
      names.
- [x] CI's `linux` package is built in the sniper container. Its report
      shows no `GLIBC_` version above the floor, and it loads nothing
      outside the list. (CI's `check.sh` checks the floor; the report's
      fields and the list are 3071's.)
- [x] ~~CI's `linux-arm64` package is built on `ubuntu-24.04-arm` and
      passes the same audit.~~ (not done: 3071.)
- [x] ~~The executable in CI's `macos` package has an `arm64` slice and an
      `x86_64` slice.~~ (not done: CI's `check.sh`, on pushes to `main`.)
- [x] No exported binary, `.wasm` included, contains the runner's home
      folder or the checkout's path (the exporter's own search, in CI).
- [x] `cargo deny check` passes with no change to `deny.toml`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the report, the refusal on failed checks, and every exporter's
  cargo arguments with no `link` feature.
- Unit: the path search (either slash, the remapped names pass); the
  audit's rules over a fixture list; `libraries.ron` reads; the flags
  composed onto existing `RUSTFLAGS` without dropping them.
- Integration: export a fixture game for `web` and `linux` in CI, and
  check the package's file list against a snapshot, and the web zip's
  file list, with `index.html` at its root.
- Integration: the audit over the test binary itself on Windows, Linux
  and macOS (CI's `test` matrix); the `linux`, `linux-arm64` and
  `macos` exports in CI with their reports checked.

## Completion notes

**Done.** `shuyi export <target> [--out <dir>] [--release] [--run]` is
an operation of the contract (contract 29, `Op::Export`, its schemas, the
MCP tool, `crates/cli/src/contract/export.rs`). It runs `check --target`
with the target's profile (`desktop` for the three desktop targets) and
refuses on an error (`SH3008`), then the target's `Exporter`
(`crates/cli/src/export.rs`: `id`, `profile`, `triples`, `cargo_args`,
`export`, `run`, registered by id). The exporters: `windows` (zip),
`linux` (`tar.gz`), `macos` (`lipo` of both Mac triples in a `.app` with
a minimal `Info.plist`, signed ad hoc inside out) and `web` (`shuyi
web`'s folder and its zip, `index.html` at the root). Every package
carries `NOTICES.txt` through 3039's module, the Windows and macOS ones
`HOW-TO-OPEN.txt`; `export-report.json` beside it holds the answer's
result (target, profile, version from `cargo metadata --no-deps`, sizes,
the check's warnings, `signing`, the debug file). No export turns on a
feature; `no_live_link` refuses `link`, and a test reads every
exporter's arguments.

- **Paths.** Cargo's `trim-paths` is not stable on the pinned Rust (now
  1.99.1, not 1.98.1: cargo refused `CARGO_PROFILE_RELEASE_TRIM_PATHS`).
  So `export::paths` passes `--remap-path-prefix` through
  `CARGO_ENCODED_RUSTFLAGS`, after the flags already in it or in
  `RUSTFLAGS`, then searches the built program for the home folder,
  `CARGO_HOME` and the roots, with either slash (`SH3009`). `shuyi web
  --release` does the same.
- **Debug files.** `--release` on `windows` and `linux` keeps
  `<package>.debug` beside the package and ships a copy stripped with
  `objcopy --strip-debug`. `doctor` checks `objcopy` on Windows GNU; on
  Linux the `cc` row's packages bring binutils.
- **Notices.** `notices::FILES` lists the engine's non-crate files as
  data (the web shell's three vendored scripts, moved there from
  `web.rs`, and the SDL_GameControllerDB for native builds that carry
  `gilrs`), and a test keeps it equal to `THIRD_PARTY_ASSETS.md`'s rows
  under `crates/`.
- **CI.** The `export` job, one matrix: `windows`, `linux` (in Valve's
  sniper SDK `3.0.20261007.263965`, pinned by the digest the registry
  gave on 2026-10-29) and `web` on every PR, `macos` on pushes to `main`.
  `tools/export-check/check.sh` checks each package's files against a
  list, the debug file's sections and `.text` address, the Windows
  program's DLLs, the Linux program's newest `GLIBC_` (floor 2.31), and
  the `.app`'s signature and slices. Artifacts kept 7 days.
- **Docs.** `docs/targets/desktop.md` (with its docs-map row), ADR-0058,
  the book's chapter 9 (*Export*), the `shuyi-ship` skill, `contract.md`,
  `crates/cli/web/README.md`, four problem codes (`SH3008`–`SH3011`)
  with their pages.

**By hand on Nick's machine (2026-10-29).** `cargo shuyi export windows
--release` in `examples/hello-grid` wrote the zip in about a minute.
`objdump -p` lists only Windows' own DLLs and API sets (KERNEL32,
USER32, GDI32, IMM32, msvcrt, ntdll, ole32, oleaut32, SHELL32, combase,
bcryptprimitives and three `api-ms-win-*`), nothing from
`C:\msys64\mingw64\bin`. The exe has no `.debug_*` section; the `.debug`
file has them, `.text` at `0x140001000` in both. No byte of either holds
the user's name, and panic paths read `/cargo\registry\…`. `cargo shuyi
export windows --release --run` started the game's window from the
package's folder. `cargo shuyi export web --release` wrote the folder
and its zip, `index.html` at the root. `tools/export-check/check.sh`
passed on both.

**Deviations.**
- **Step 8: split.** The PR passed 600 lines, so the library audit
  (step 4, `libraries.ron` and the `object` crate) and `linux-arm64` are
  ticket 3071 (After MVP, blocked by 3004), which names 3049, 3050, 3032
  and 2520. Until then CI's `check.sh` checks the DLLs and the glibc
  floor of the examples' packages, and the report lists no `loads`.
  Valve publishes the sniper SDK for amd64 only, so 3071 builds
  `linux-arm64` in `debian:bullseye`.
- **The project root.** The remapping maps the workspace's root (which
  holds the project) to the package's name, so the engine's examples,
  whose workspace is the engine's repository, are covered too. Each
  package read by a path outside the workspace (`cargo metadata`: no
  source) is remapped to its own name too: CI's skill examples, games
  made with `shuyi new --engine-path`, name the engine that way, and
  the search refused them before.
- **`.debug_gdb_scripts`.** rustc writes it on Linux as loaded data
  (the name of gdb's pretty-printer script, no path), so `objcopy
  --strip-debug` keeps it in the Linux program; `check.sh` leaves it
  out of the sections it counts.
- **Assets.** Nothing is collected beside the program: a game embeds its
  files (the templates and examples do), and `shuyi.ron` has no setting
  by which a game asks for a pack (2204 added none). The web package
  keeps `shuyi web`'s music and voice folders.
- **Each desktop package builds on its own system** (Windows on Windows,
  Linux on Linux, macOS on a Mac); another system answers where it
  builds (`SH1003`).
- **The log file's place** (`docs/targets/desktop.md`): 3041 isn't done,
  so its section is 3041's (its scope says so).
- **The report's `unread` and `largest` (2516), `left_out` (3017), the
  version from `app.version` (3048), credits (2223), the web run step
  (2528), the bench (3008), the README's zip (3052), the eval (2524),
  the crash recipe (2520):** those tickets aren't done; each says it adds
  its piece.

**Not verified here.** The `macos` package (signature, both slices) and
the Linux package in the sniper container run only in CI; the macOS leg
only on pushes to `main`, so its first run is after the merge.

**Follow-up tickets.** 3071 (the library audit and `linux-arm64`).

Breaking: shuyi-cli: `contract::Op`: gains `Export`, and `Op::ALL` is an `[Op; 17]`; an exhaustive `match` on `Op` adds its arm.

```migration
since: 0.1.0
what: The contract has a new operation, export, so Op has a new variant.
old: "match op { Op::New => …, … }"
new: "match op { Op::New => …, Op::Export => …, … }"
before: |
  match op {
      Op::Attach => attach(),
      // … every other operation
  }
after: |
  match op {
      Op::Attach => attach(),
      Op::Export => export(),
      // … every other operation
  }
```

Nick input: none.
