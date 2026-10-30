# ADR-0057: Export packages carry nothing of the build machine: paths remapped and searched, the Linux floor in the sniper SDK, and a checked package per target

- **Status:** Accepted; extends ADR-0008 (export targets)
- **Date:** 2026-10-29
- **Related tickets:** 3004 (this), 2520 (crash reports from the debug
  file), 3008 (the bench reads the report), 3010, 3012 and 3015 (the
  Playdate, Steam Deck and mobile exporters, which follow this), 3049
  (the game's identity in the packages), 3050 (releases from a tag),
  3051 (signed packages, parked), 3071 (the library audit and
  `linux-arm64`)

## Context

- **ADR-0008** makes targets profiles as data, checked by `check
  --target`, and says each target gets a package. Nothing yet built one:
  a game was shipped by hand.
- **Build paths.** Every `[profile.release]` here and in the templates
  sets `strip = true`, which drops symbols but not the file paths in
  panic messages. A release build kept `C:\Users\<name>\.cargo\registry\…`
  and the checkout's path, so every package, the web `.wasm` too, carried
  the builder's user name.
- **`trim-paths`** is still unstable on the pinned Rust (1.99.1): cargo
  refuses `CARGO_PROFILE_RELEASE_TRIM_PATHS` without `-Z` (checked
  2026-10-29). rustc's `--remap-path-prefix` is stable, and rustc applies
  the last matching flag first.
- **Linux.** A program linked on `ubuntu-latest` needs that runner's
  glibc. Valve's Steam Linux Runtime 3.0 ("sniper") SDK image is Debian
  11 (glibc 2.31) with the build tools and `-dev` packages games need. Its
  registry (`registry.gitlab.steamos.cloud/steamrt/sniper/sdk`) has
  dated tags and an amd64 image only (read 2026-10-29).
- **macOS.** CI's `macos-latest` is Apple silicon; a plain build runs
  only there. Apple silicon runs only signed code, and a bundle not
  signed as a whole can be reported as damaged.
- **Windows.** The GNU toolchain links with MSYS2's GCC, which is on the
  builder's `PATH`: a MinGW DLL the program needed would load there and
  nowhere else.

## Decision

1. **One operation, then an exporter.** `export <target>` (the contract,
   ADR-0005) runs `check --target` with the target's profile and refuses
   on any error (`SH3008`), then the target's `Exporter`
   (`shuyi_cli::export`): `id()`, `profile()`, `triples()`,
   `cargo_args()`, `export()` and `run()`, registered by id. The desktop
   exporters check the `desktop` profile. `export-report.json` beside the
   package is the answer's result.
2. **No feature, ever the live link.** An export builds the game's
   program with no feature; `no_live_link` refuses any build whose
   arguments turn on `link`, and a test reads every exporter's arguments.
3. **Paths remapped and searched.** Every `--release` build, and `shuyi
   web --release`, passes `--remap-path-prefix` through
   `CARGO_ENCODED_RUSTFLAGS`, after the flags already in it or in
   `RUSTFLAGS`, never replacing them: the workspace's root to the
   package's name, `CARGO_HOME` to `/cargo`, `RUSTUP_HOME` to `/rustup`,
   the target folder to `/target`, in that order. A game's `rustflags` in
   `.cargo/config.toml` don't reach these builds. The built program, or
   `.wasm`, is then searched for the home folder, `CARGO_HOME`, the
   project's and the workspace's roots, with either slash and any ASCII
   case; a hit refuses the package and names the path (`SH3009`). When
   `trim-paths` is stable, it replaces the flags and the search stays.
4. **Debug files beside, never inside.** With `--release`, `windows` and
   `linux` build with `strip = false` and `debug = "line-tables-only"`
   (cargo's profile variables), keep that program as `<package>.debug`
   beside the package, and ship a copy stripped with `objcopy
   --strip-debug`, which moves no code (2520 reads it).
5. **Each desktop package builds on its own system.** `windows` is
   `x86_64-pc-windows-gnu` in a zip; `linux` is `x86_64-unknown-linux-gnu`
   in a `tar.gz`, built in CI in the sniper SDK image pinned by tag and
   digest, whose glibc 2.31 is the floor; `macos` is one universal program
   (`lipo` of `aarch64-apple-darwin` and `x86_64-apple-darwin`) in a
   `.app` with a minimal `Info.plist`, signed ad hoc with `codesign
   --force --sign -` on the program, then the bundle (never `--deep`),
   after every file of the bundle is written. `web` is the web build's
   folder and its zip with `index.html` at the root. Every package
   carries `NOTICES.txt`; `windows` and `macos` carry `HOW-TO-OPEN.txt`.
6. **The engine's non-crate files are data.** `notices::FILES` lists
   each with its work, license, text and the builds that carry it, and a
   test keeps it equal to `THIRD_PARTY_ASSETS.md`'s rows under `crates/`.
7. **Checked in CI, per target.** CI's `export` job, one matrix, exports
   every example for `windows`, `linux` and `web` on every PR and for
   `macos` on pushes to `main`, then `tools/export-check/check.sh` checks
   the package's files against a list, the debug file's sections and
   `.text` address, the Windows program's DLLs (no MinGW runtime), the
   Linux program's newest `GLIBC_` version against the floor, and the
   `.app`'s signature and slices.
8. **Every exporter follows this.** The Playdate (3010), Steam Deck
   (3012) and mobile (3015) exporters remap and search their builds and
   refuse a package that fails. The audit of what a program loads, read
   with the `object` crate against a list per target, and the
   `linux-arm64` package come with 3071, and every exporter passes that
   too.

## Consequences

- No package carries the builder's user name or folders, and the
  exporter proves it on each build rather than trusting the flags.
- The Linux package runs on any glibc 2.31 system, and the Steam Deck
  reuses the same container (3012).
- A release export rebuilds everything: the remapping changes the flags,
  so it shares no build with a plain `cargo build --release`.
- A game's own `rustflags` are dropped for exports, as for the web
  build's link argument; the docs say so.
- A desktop package can't be cross-built: Windows on Windows, Linux on
  Linux, macOS on a Mac. CI has all three; a game developer has one.
- Until 3071 lands, only CI's shell checks look at what a program
  loads, on the examples.

## Alternatives considered

- **Nightly's `trim-paths`.** The engine pins stable Rust; it waits for
  stabilization, and the search stays either way.
- **`RUSTFLAGS` instead of the encoded variable.** It splits on spaces,
  which a Windows path holds, and the build script flags differ; the
  encoded variable takes each flag whole.
- **Linux on `ubuntu-latest`, or an older Ubuntu.** Its glibc rises with
  the runner; the Steam Runtime's is the floor Steam players have anyway.
- **A cross toolchain (cargo-zigbuild, osxcross) for every package on
  one machine.** It adds tools to the trusted path and still can't sign
  on macOS; CI runners for each system are free for public repositories.
- **`codesign --deep`.** Apple discourages it: it signs nested code with
  the outer options. Signing inside out is what Apple documents.
- **The `zip` and `tar` crates.** Two dependencies (and theirs) for a
  hundred lines that write fixed, reproducible archives with `flate2`,
  already in the graph.
