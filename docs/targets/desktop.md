# The desktop packages

What `shuyi export windows`, `linux` and `macos` make, what each runs
on, and how to make the Linux one by hand. All three run the `desktop`
target profile ([`targets/desktop.ron`](../../targets/desktop.ron)):
there is no profile per system or per processor, and `export` runs
`check --target desktop` before it builds any of them. The operation
itself is in [`contract.md`](../contract.md) (`export`).

Each package builds on its own system: its linker, its libraries and its
tools are that system's. On another one, `export` says where it builds.

| Target | Package | Built with | Runs on |
| ------ | ------- | ---------- | ------- |
| `windows` | `<game>-<version>-windows.zip`: the `.exe`, `NOTICES.txt`, `HOW-TO-OPEN.txt` | `x86_64-pc-windows-gnu`, linked by MSYS2's mingw-w64 GCC | 64-bit Windows 10 and 11 (Rust's support for this target); Windows on Arm runs it through Windows 11's Prism emulation, untested |
| `linux` | `<game>-<version>-linux.tar.gz`: the program and `NOTICES.txt` | `x86_64-unknown-linux-gnu`, in the Steam Linux Runtime 3.0 ("sniper") SDK | 64-bit x86 Linux with glibc 2.31 or newer (Debian 11, Ubuntu 20.04, SteamOS 3 and later), with ALSA, libudev, X11 and OpenGL |
| `macos` | `<game>-<version>-macos/`: `<game>.app`, `NOTICES.txt`, `HOW-TO-OPEN.txt` | `aarch64-apple-darwin` and `x86_64-apple-darwin`, joined by `lipo` into one program | Apple silicon and Intel Macs |

## What each package holds

- **The program.** Built with the features of `shuyi.ron`'s `export` part
  (below), and none without one; never a debug feature (`link`,
  `debug-tools`, `capture` or the sound gates' probe).
- **`NOTICES.txt`**: every crate compiled in, the engine's data compiled
  in with it (the controller database `gilrs` carries, under Zlib), and
  the game's own texts its `shuyi.ron`'s `notices` names, then each
  license text once.
- **`HOW-TO-OPEN.txt`**, in the Windows and macOS packages: the packages
  aren't signed by a developer, so players open them once by hand. On
  Windows, SmartScreen's More info, then Run anyway. On macOS 15 and
  later, open the app once, then System Settings > Privacy & Security >
  Open Anyway.
- **On macOS**, the `.app` has a minimal `Info.plist` (its program and
  its type) and an ad-hoc signature, made inside out: the program, then
  the bundle. Apple silicon runs only signed code, and a bundle signed as
  a whole is offered Open Anyway instead of being called damaged.

## A release build

`--release` builds what players get:

- **Nothing of the build machine.** The build's paths are renamed: the
  workspace to the game's package name, a crate read by a path
  outside it (the engine, named by `path = "…"`) to its own name,
  Cargo's home to `/cargo`,
  rustup's to `/rustup`, the target folder to `/target`. A panic then
  names `/cargo/registry/…`, not the builder's home folder. The program
  is then searched for the home folder, Cargo's home and the game's
  folders; one found refuses the package (`SH3009`). The renaming goes
  through `CARGO_ENCODED_RUSTFLAGS`, after the flags already in it or in
  `RUSTFLAGS`, so a game's `rustflags` in `.cargo/config.toml` don't
  reach these builds.
- **No symbol table.** The shipped program names none of its functions.
  On Windows and Linux the program is built with its symbols and line
  tables and kept whole as `<game>-<version>-<target>.debug` beside the
  package, never in it. The package's copy is stripped of both with
  `objcopy --strip-all` (MSYS2's binutils on Windows, the system's on
  Linux; `shuyi doctor` checks it on Windows), which moves no code, so a
  crash's addresses in the shipped program name the same functions and
  lines in the debug file (`addr2line -f -e <the .debug file> <address>`).
  Keep the debug file with each release. On macOS the program is built
  stripped (`strip = true`, whatever the game's profile says).
- **No debug surfaces.** Without debug tools a build drops its log's info
  and debug lines, doesn't read `SHUYI_PERF`, and writes no record of play
  into a crash report unless the game asks
  ([crash-reports.md](../crash-reports.md)).

`export-report.json` beside the package names the debug file, with the
version, the target, the sizes, what shipped beside the program, the check's
warnings and the signing.
`exposure-report.json` lists what the package holds and is held to the game's
`disclosure` part (the book's [Targets and
export](../../site/content/learn/book/targets-and-export.md)). A Windows
program carries a fixed time, not the time of its build, so the same input
gives the same report.

## A game's features and the files beside its program

`shuyi.ron`'s `export` part names what a build turns on and what its package
holds besides the program and its notes:

```ron
export: (
    features: ["bought-art"],
    beside: [
        (path: "music"),
        (path: "voice", optional: true),
        (path: "files/licenses", name: "licenses"),
    ],
),
```

- **`package`**, if the program that ships isn't in the `game` part's package
  (a game whose tool program is a package of its own), names it.
- **`features`** go to the build's `--features`, on every desktop target and
  the web, and to `NOTICES.txt`. A debug feature is refused, by name:
  `link` (the live link), `debug-tools` (the debug menu, `SHUYI_PERF`
  traces and session recording), `capture` and the sound gates' probe;
  so is one the build reaches another way, the package's own or the
  engine's (`shuyi-ui/debug-tools`, `shuyi-app/capture` and the rest), by
  a `default` list, a feature that lists it or a dependency, which Cargo
  is asked before anything is built. `--without <feature>` leaves one off
  a build, as a build with no access to a feature's files needs, and `--with <feature>` turns on another the
  package has, for that build only.
- **`targets`** gives one target, by its id, a `package` and `features` of
  its own, each replacing the part's for that target: `targets: {
  "linux": (package: "jumper-linux") }`. The Playdate reads its entry here
  too, and nothing else of the part ([playdate.md](playdate.md)). An id with
  no exporter is refused.
- **`wasm_opt`**, `required` until said, is for the web: a release web export
  fails when Binaryen's `wasm-opt` didn't run (`SH1003` with how to install
  it when it isn't there), and after it runs, when the binary still holds a
  `name` or `producers` section. `wasm_opt: optional` makes a missing one a
  warning, and the binary ships as the build left it (built stripped).
- **`beside`** copies each file or folder under its own name, or `name`. A
  path that isn't there refuses the export, naming it, unless it is
  `optional`; a name the package already holds is refused too.
- **Windows, Linux and the web:** they go next to the program. The web build
  already ships its `web.music` and `web.voice` folders, so a row naming one
  adds nothing there.
- **macOS:** a folder goes in the `.app`'s `Contents/Resources`, before it is
  signed. The backend's `music` and `voice` folders look there as well as next
  to the program, so a game reads them the same way on each system. A file
  goes beside the `.app`, where a player finds it.

The Playdate's package takes none of this: a `.pdx` has its own build and its
own files.

A game with a `disclosure` part names these files in its `allowed_files` too:
the part is the list a package is held to, and `beside` doesn't widen it.

## The Linux package by hand

The Linux package is built inside Valve's sniper SDK image, so the
program asks for no glibc newer than the image's (2.31). Built on a
newer system, it may not start on an older one. With `docker` (or
`podman`, the same arguments), from the game's folder:

```bash
docker run --rm -it -v "$PWD:/game" -w /game registry.gitlab.steamos.cloud/steamrt/sniper/sdk:3.0.20261007.263965
```

then, inside it:

```bash
bash scripts/setup-linux.sh
. "$HOME/.cargo/env"
cargo shuyi export linux --release
```

A game made with `shuyi new` has `scripts/setup-linux.sh`, which
installs Rust and the libraries; the package lands in
`dist/export/linux/` on the host. The engine's CI pins the same image
by its digest (`.github/workflows/ci.yml`, the `export` job).

## Budgets

Its row of ADR-0008 §6's table; *prov.* marks a starting guess. What the
budgets are and how `shuyi bench` checks them is in
[`README.md`](README.md).

| Target | Frame time on device | `frame_instructions` | `peak_heap_bytes` | `package_bytes` | `startup_ms` | Also |
| ------ | -------------------- | -------------------- | ----------------- | --------------- | ------------ | ---- |
| `desktop` | p99 ≤ 16.7 ms | 20 M (prov.) | 512 MB | — | 2 000 | — |

Not yet measured on a device: no ticket covers the desktop yet.

## Hello, Grid's numbers

From CI's `bench` job on 2026-11-05 (ticket 3008's pull request): the
heaviest of Hello, Grid's eight scenarios
([`examples/hello-grid/bench/`](../../examples/hello-grid/bench/)), on an
x86-64 Linux runner. They are proxies, not a device's frame times, and
the budgets above stay as they are until a device is measured.

| Measure | Hello, Grid | Budget |
| ------- | ----------- | ------ |
| Instructions in its heaviest frame | 394,501 (`idle-map-tiles`, the map drawn with its tileset) | 20,000,000 |
| Instructions per frame, mean, its heaviest scenario | 377,524 (`idle-map-tiles`) | — |
| Heap's peak | 158.5 KB (`idle-map-tiles`) | 512 MB |
