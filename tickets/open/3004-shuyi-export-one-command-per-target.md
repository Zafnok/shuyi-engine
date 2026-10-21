---
id: "3004"
title: "shuyi export <target>: one command that checks a game and packages it for a target, starting with Windows, Linux, macOS and the web"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["3001", "1129", "3039"]
nick_input: none
completed:
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
     sizes, the check's warnings, `signing`, and, when 2516 is done, its
     files report's unread and largest files, taken from `shuyi check`,
     not computed again here). `signing` is `"none"` for `windows`,
     `linux`, `linux-arm64` and `web`, and `"ad-hoc"` for `macos`; 3051
     adds the paid kinds. The `version` is `shuyi.ron`'s `app.version`
     if 3048 is done and the part sets one; otherwise it is the built
     package's version, read with `cargo metadata --no-deps
     --format-version 1`.
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
    notarization are parked 3051.
  - **`web`.** It reuses the existing web build (`cargo shuyi web`),
    and the output folder holds `index.html`, the wasm, the JS plugins
    and the `NOTICES.txt` that `shuyi web` writes (3039). It also writes
    a zip of that folder with `index.html` at the zip's root, which
    itch.io and other portals take
    (<https://itch.io/docs/creators/html5>). `export-report.json` gives
    the zip's size, which 3008 compares with the web profile's
    `package_bytes`. 3052's README section "Putting a web build online"
    points at this zip.
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
  the package's triple, the game's `notices.texts`, and the game's
  credits once 2223 lands. No separate tool.
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
    `docker` or `podman`.

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
- **The agent eval.** If 2524 is done, its `Web` check
  (`crates/xtask/src/eval/grade.rs`) runs `shuyi export web` and reads
  that folder, and the turn-based tasks in `evals/tasks/` gain an
  `Export` check for `linux` (the package exists), run in the sniper
  container as CI's `linux` export is, since outside it the audit
  refuses a newer glibc. If 2524 isn't done, 2524's either-order clause
  moves `Web` to `export`.

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
  resource, Info.plist and `.icns`, a `.desktop` file and its icons, web
  icons, the page's title from the name): 3049, which follows this
  ticket;
- the Steam Deck (3012), Playdate (3010) and mobile (3015) exporters;
- the log file of a shipped build (3041). Land 3041 before or with this
  ticket: a Windows release build has no console
  (`windows_subsystem = "windows"`), so the file is the only record a
  player can send;
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

## Acceptance criteria

- [ ] `shuyi export windows` on an example, on Nick's machine (GNU
      toolchain), gives a zip whose exe runs. Its report lists only
      Windows' own DLLs and API sets, so nothing comes from
      `C:\msys64\mingw64\bin`.
- [ ] `shuyi export playdate` on a game that fails `check --target
      playdate` refuses, and prints the check's errors.
- [ ] A missing Rust target gives "run `rustup target add …`", not a
      cargo error dump.
- [ ] Each package has `NOTICES.txt` from 3039's module, and the native
      ones list the SDL_GameControllerDB with its Zlib text (test).
- [ ] CI exports the examples for `windows`, `linux` (in the sniper
      container) and `web` on every PR, and for `macos` and
      `linux-arm64` on pushes to `main`.
- [ ] `shuyi export web` writes a zip with `index.html` at its root, and
      `export-report.json` gives its size.
- [ ] `shuyi export windows --run` on Windows starts the example (by
      hand, in the Completion notes), and a target with no run step
      gives the default answer (test).
- [ ] With `--release`, the `windows` and `linux` packages hold an
      executable with no `.debug_*` section, and `<package>.debug`
      beside them has those sections and the same `.text` address
      (test, reading the files with `objdump -h` or the `object` crate).
- [ ] The `macos` package's `.app` passes `codesign --verify --strict`
      on the macOS runner, and its report says `"ad-hoc"`.
- [ ] Every `windows` and `macos` package carries the player note
      (test).
- [ ] A test feeds the audit the test binary itself
      (`std::env::current_exe()`) on each OS of CI's `test` matrix, and
      the audit names that OS's C library (`kernel32.dll`,
      `libc.so.6`, `/usr/lib/libSystem.B.dylib`).
- [ ] A fixture list without one of those libraries refuses the package
      and names the library. A library inside the package passes.
- [ ] The path search refuses a byte buffer that holds the home folder
      with either slash, and passes one that holds only the remapped
      names.
- [ ] CI's `linux` package is built in the sniper container. Its report
      shows no `GLIBC_` version above the floor, and it loads nothing
      outside the list.
- [ ] CI's `linux-arm64` package is built on `ubuntu-24.04-arm` and
      passes the same audit.
- [ ] The executable in CI's `macos` package has an `arm64` slice and an
      `x86_64` slice.
- [ ] No exported binary, `.wasm` included, contains the runner's home
      folder or the checkout's path (the exporter's own search, in CI).
- [ ] `cargo deny check` passes with no change to `deny.toml`.
- [ ] All gates in the `run-gates` skill pass.

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

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
