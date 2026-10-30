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

- **The program.** No feature of the game's package is turned on, the
  debug build's live link (`link`) never.
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
- **The debug file.** On Windows and Linux the program is built with its
  line tables and kept as `<game>-<version>-<target>.debug` beside the
  package, never in it. The package's copy is stripped of them with
  `objcopy --strip-debug` (MSYS2's binutils on Windows, the system's on
  Linux; `shuyi doctor` checks it on Windows), which moves no code, so a
  crash's addresses in the shipped program name the same lines in the
  debug file. Keep the debug file with each release.

`export-report.json` beside the package names the debug file, with the
version, the target, the sizes, the check's warnings and the signing.

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
