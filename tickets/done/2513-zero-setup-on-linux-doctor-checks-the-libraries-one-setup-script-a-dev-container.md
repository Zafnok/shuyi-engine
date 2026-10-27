---
id: "2513"
title: "Zero setup on Linux: shuyi doctor checks the C linker and the system libraries, one setup script for containers and cloud agents, a dev container, and CI from a bare Ubuntu"
type: infra
milestone: MVP
model: sonnet-5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-26
---

# 2513 — Zero setup on Linux: doctor checks the libraries, one setup script, a dev container

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker, Defold and Construct 3
([`docs/decisions.md`](../../docs/decisions.md), the last entry). It is
one of the protections Nick put in the MVP, before the first release
(2005).

**What the others have:**

- Bevy lists the system packages to install on each Linux distribution
  ([`docs/linux_dependencies.md`](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md)).
- Unreal's Turnkey checks and installs the SDKs a platform needs.
- Defold's build server compiles native extensions, so a developer
  installs no C toolchain for them. Construct 3 runs in the browser and
  needs nothing installed.
- None of Bevy, Godot, Unity or Unreal ships a dev container or an
  "Open in Codespaces" link. The case here rests on the end goal below,
  not on parity.

**What Shuyi has today** (checked 2026-10-20):

- `shuyi doctor` (`crates/cli/src/contract/doctor.rs`, ticket 1129):
  `checks()` (lines 118–184) looks at `rustc`, `cargo`, rustup's
  targets, MSYS2's GCC on the Windows GNU host and the wasm target,
  through the `Probe` trait (lines 57–64: `output` and `exists`). On
  Linux it checks nothing more, and says so: `doctor()` pushes the limit
  "the system libraries macroquad links on Linux (ALSA, X11, GL) are
  not checked" (lines 194–196).
- What a Linux build needs, read from the crates in `Cargo.lock`:
  - a C linker, `cc`: rustc links every binary and build script with
    it, so in a container without it nothing builds, `shuyi` included;
  - `quad-alsa-sys` 0.3.2 (under `quad-snd`, `crates/app/Cargo.toml`)
    prints `cargo:rustc-link-lib=dylib=asound` in its `build.rs`: the
    link needs `libasound.so`;
  - `libudev-sys` 0.1.4 (under `gilrs`) calls
    `pkg_config::find_library("libudev").unwrap()` in its `build.rs`: it
    needs the `pkg-config` program and libudev's development files;
  - miniquad 0.4.11 opens `libX11.so.6`, `libxkbcommon.so.0` and
    `libXi.so.6` when the window starts (`src/native/linux_x11.rs`,
    lines 650–652, each `try_load()?`) and `libGL.so.1`
    (`src/native/linux_x11/glx.rs`, lines 132–133). Its Linux backend
    defaults to X11 only (`conf.rs`, `LinuxBackend::X11Only`), and the
    engine doesn't change it.
- The package list is pasted by hand:
  `sudo apt-get update && sudo apt-get install -y libasound2-dev
  libx11-dev libxi-dev libgl1-mesa-dev libudev-dev` is in
  `.github/workflows/ci.yml` six times (lines 53, 73, 90, 109, 145,
  255), in `.github/workflows/mutants.yml` four times (lines 99, 124,
  193, 215), and in `templates/blank/.github/workflows/ci.yml` and
  `templates/grid/.github/workflows/ci.yml` (line 29 of each). It
  relies on the GitHub runner already having `cc`, `pkg-config` and
  libxkbcommon. Visions of Shuyi's own `ci.yml` pastes the same line.
- A game made with `shuyi new` runs the engine's tools as `cargo shuyi`,
  which builds the game's `tools` example. Its crate depends on
  `shuyi-app` (`templates/*/crates/game/Cargo.toml`), so in a bare Linux
  container `cargo shuyi doctor` fails to build and never gives its
  advice. Only the engine's own `shuyi` binary avoids the libraries,
  because `shuyi-cli` doesn't depend on `shuyi-app`
  (`crates/cli/Cargo.toml`); it still needs `cc`.
- There is no `.devcontainer/`, no Dockerfile (the only planned ones
  are the network servers' images, 1141 and 1146) and no `scripts/`
  folder. The templates' `.cargo/config.toml` sets a linker only for
  `x86_64-pc-windows-gnu`, which Linux ignores: nothing to change
  there.
- The MCP server (1133, done) and its `shuyi mcp install`, local
  only; the book's quick start (2802), which says nothing about Linux;
  the first release (2005), after which `cargo install --locked
  shuyi-cli` gives the `shuyi` binary.

**Why it matters:**

- **The game first.** Visions of Shuyi builds on the same backend. Its
  CI pastes the same apt line, and a cloud session on it meets the
  same missing libraries. Doctor naming them, and one script that
  installs them, serve its sessions too; whether the game calls the
  engine's script is the game's choice.
- **Agents.** The end goal (2026-10-19): an agent asked to make a game
  with Shuyi does it in under an hour. Cloud agents (Claude Code on the
  web, Codex cloud, Copilot's coding agent) start in Linux containers
  that may lack the C linker, `pkg-config` and these libraries. There
  the first build fails with a linker or `pkg-config` error that says
  nothing about Shuyi. Until real time is unparked, the agent check
  measures turn-based games (2026-10-20); they build on the same
  backend, so they hit the same wall. The agent eval's scheduled runs
  (2525) need the same reproducible setup: they run in the bare
  `ubuntu:24.04` container this ticket's script sets up.
- **Other 2D games, and newcomers.** A Linux developer gets one command
  and a doctor that names what is missing. Anyone can open the engine
  in GitHub Codespaces with everything installed, billed to whoever
  opens it, never to Nick.
- **Upkeep.** One package list, tested against every copy, instead of
  twelve pasted lines.

## Nick input

None.

## Scope

**In:**

- **The Linux table**, in a new file
  `crates/cli/src/contract/doctor/linux.rs` (a submodule of `doctor.rs`,
  in the shape of `new.rs` and `new/tests.rs`). One `const` slice of
  rows, `linux::ROWS`; each row is a check's name, how doctor finds it,
  and the package for apt, dnf and pacman. The module and `ROWS` are
  `pub(crate)`, so `new/tests.rs` can read the table. In this order:

  | Check | How doctor finds it | apt | dnf | pacman |
  | ----- | ------------------- | --- | --- | ------ |
  | `cc` | `cc --version` | `build-essential` | `gcc` | `base-devel` |
  | `pkg-config` | `pkg-config --version` | `pkg-config` | `pkgconf-pkg-config` | `pkgconf` |
  | `lib-alsa` | `pkg-config --modversion alsa` | `libasound2-dev` | `alsa-lib-devel` | `alsa-lib` |
  | `lib-udev` | `pkg-config --modversion libudev` | `libudev-dev` | `systemd-devel` | `systemd-libs` |
  | `lib-x11` | the soname `libX11.so.6` | `libx11-dev` | `libX11-devel` | `libx11` |
  | `lib-xi` | the soname `libXi.so.6` | `libxi-dev` | `libXi-devel` | `libxi` |
  | `lib-xkbcommon` | the soname `libxkbcommon.so.0` | `libxkbcommon-dev` | `libxkbcommon-devel` | `libxkbcommon` |
  | `lib-gl` | the soname `libGL.so.1` | `libgl1-mesa-dev` | `mesa-libGL-devel` | `libglvnd` |

  Check each name before writing it: packages.ubuntu.com (24.04),
  packages.fedoraproject.org, archlinux.org/packages, and Bevy's list.
  Correct a name the search disproves, and say so in the PR. The first
  four are needed to build, the last four to open a window; the row's
  doc comment says which, and why (the crate that needs it).
- **Doctor's Linux checks.** On a host whose triple contains `-linux-`,
  `checks()` adds one check per row, after `rustup` (and where
  `msys2-gcc` sits on Windows), with `wasm-target` still last:
  - programs and `pkg-config` modules through `Probe::output`; `found`
    is the first line of what they print (a version);
  - a soname is found in `ldconfig -p`'s output (run once per doctor
    run, not per row), or else as a file in `/usr/lib/x86_64-linux-gnu`,
    `/usr/lib/aarch64-linux-gnu`, `/usr/lib64` or `/usr/lib` through
    `Probe::exists`; `found` is where;
  - a failed row's `help` is one install command for this distro:
    `sudo apt-get install -y <apt>`, `sudo dnf install -y <dnf>` or
    `sudo pacman -S --needed <pacman>`, without `sudo` when `id -u`
    prints `0`, followed by "; in a game made with `shuyi new`, `bash
    scripts/setup-linux.sh` installs them all". The distro comes from
    `ID` and `ID_LIKE` in `/etc/os-release` (quoted or not): `debian`
    or `ubuntu` means apt, `fedora`, `rhel` or `centos` dnf, `arch`
    pacman. Anything else, or no file: the help names all three
    packages, one per manager.
  - `Probe` gains `fn read(&self, path: &Path) -> Option<String>`
    (`ThisMachine`: `fs::read_to_string(path).ok()`). This changes
    `shuyi-cli`'s public API (a new required method of a public trait);
    nothing outside its tests implements `Probe`, so a game builds
    unchanged. The PR description says so.
  - The limit at lines 194–196 goes. When the host is unknown (no
    `rustc`), the existing limit becomes "without rustc the host is
    unknown: MSYS2 and the Linux libraries were not checked".
  - **Doctor stays read-only.** It never runs a package manager or
    `sudo`. The contract says doctor checks, and the script installs.
- **`scripts/setup-linux.sh`**, one Bash script, in the engine's root
  and in every template (`templates/blank/scripts/`,
  `templates/grid/scripts/`; `shuyi new` writes it with the rest).
  The copies are byte-equal. It:
  - starts with `set -euo pipefail` and moves to the folder above its
    own (`cd "$(dirname "$0")/.."`), where `rust-toolchain.toml` is;
  - holds the package lists as three lines in exactly this form, in the
    table's order, so a test can read them:
    `APT="build-essential pkg-config libasound2-dev ..."`, then `DNF="..."`
    and `PACMAN="..."`;
  - runs the package manager as root: directly when `id -u` prints
    `0`, through `sudo` when it doesn't and `sudo` exists, or else stops
    with a message saying to run it as root. Only the package manager
    runs through `sudo`; rustup and the toolchain install as the user
    who ran the script, into their own `$HOME`;
  - finds `apt-get`, `dnf` or `pacman` and installs its list
    (`DEBIAN_FRONTEND=noninteractive`, `apt-get update` first, then
    `apt-get install -y --no-install-recommends`; `dnf install -y`;
    `pacman -S --needed --noconfirm`). With none of the three, it
    prints the three lists and stops with an error;
  - if `rustup` is neither on `PATH` nor in `$HOME/.cargo/bin`, installs
    `curl` and `ca-certificates` with the same manager, then rustup
    from `https://sh.rustup.rs` with `-y --profile minimal
    --default-toolchain none`;
  - installs the toolchain `rust-toolchain.toml` pins, with its
    components (`rustup toolchain install` with no argument, which
    reads the file in rustup 1.28 and later; `rustup show` on an older
    rustup), then `rustup target add wasm32-unknown-unknown` (the
    templates' `rust-toolchain.toml` doesn't list the target). It never
    names a Rust version itself, so 2012's check has no new pin to
    compare;
  - when `GITHUB_PATH` is set, appends `$HOME/.cargo/bin` to it, so a
    later CI step finds `cargo`;
  - is idempotent: a second run installs nothing and succeeds;
  - ends with what it did and the next step ("open a new shell or run
    `. "$HOME/.cargo/env"`, then `cargo shuyi doctor`").
  It is always run as `bash scripts/setup-linux.sh`, never `./`:
  `shuyi new` writes files without the executable bit. The engine's
  `.gitattributes` (`* text=auto eol=lf`) keeps it LF.
- **CI calls the script.** Every "Install macroquad system libraries"
  step becomes `name: Set up Linux`, `run: bash scripts/setup-linux.sh`
  (the `test` job's keeps its `if: matrix.os == 'ubuntu-latest'`): the
  six in `ci.yml`, the four in `mutants.yml`, and the one in each
  template's `ci.yml`. The `rustup show` steps stay where they are.
- **CI's `templates` job starts from a bare Ubuntu.** It runs in
  `container: ubuntu:24.04` (as root, with nothing installed), for both
  templates. It has no `rustup show` step (there is no rustup until the
  script runs), and the job's `env:` entry for `CARGO_TARGET_DIR` goes
  (step 3 sets it):
  1. `actions/checkout` (pinned by SHA as today, `persist-credentials:
     false`). The image has no `git`, so checkout downloads the commit
     through GitHub's API; nothing in the job needs `git`;
  2. `bash scripts/setup-linux.sh`: from nothing;
  3. `echo "CARGO_TARGET_DIR=$GITHUB_WORKSPACE/target" >> "$GITHUB_ENV"`.
     In a container job the `github.workspace` and `runner.temp`
     contexts give the host's paths, so the job uses the
     `$GITHUB_WORKSPACE` and `$RUNNER_TEMP` variables inside `run:`,
     and `cd "$RUNNER_TEMP/demo"` instead of `working-directory`;
  4. `Swatinem/rust-cache` as today. If it doesn't work in the
     container, drop it and say so in the PR;
  5. `cargo run -q -p shuyi-cli --locked -- doctor`, which must pass;
  6. `shuyi new` as today, then `bash "$RUNNER_TEMP/demo/scripts/setup-linux.sh"`:
     the game's own copy, on a set-up machine, installs nothing;
  7. the game's gates as today, then `cargo shuyi doctor` in the game.
  The PR description gives the job's time before and after.
- **`.devcontainer/devcontainer.json`** in the engine's root and in every
  template, plain JSON with no comments:
  `"name"` (`"Shuyi Engine"`; `"{{name}}"` in the templates),
  `"image": "mcr.microsoft.com/devcontainers/base:ubuntu-24.04"` (check
  the tag exists on the registry first), `"postCreateCommand": "bash
  scripts/setup-linux.sh"`, `"remoteEnv"` adding
  `/home/vscode/.cargo/bin` to `PATH` (`"${containerEnv:PATH}:..."`),
  and `"customizations": { "vscode": { "extensions":
  ["rust-lang.rust-analyzer"] } }`. The same Ubuntu as CI's container,
  set up by the same script: no Dockerfile and no second list. The
  image, the packages and rustup are tools on the developer's machine;
  none of them ships in a game, so ADR-0002 §8 (what a game ships) is
  not touched.
- **Text:**
  - `README.md`: an "Open in GitHub Codespaces" link
    (`https://codespaces.new/Zafnok/shuyi-engine?quickstart=1`), saying
    it is billed to whoever opens it; in *Starting a game, and the
    tools on it*, a line: on Linux, `bash scripts/setup-linux.sh` first,
    and the new game has its own copy; and in *Map of the repository*,
    a row for `scripts/setup-linux.sh` and `.devcontainer/`.
  - `AGENTS.md`, *Environment*: one line saying that on Linux, in a dev
    container or in a cloud agent's container, `bash
    scripts/setup-linux.sh` installs the C linker, `pkg-config`, the
    libraries and the pinned Rust, and `shuyi doctor` checks the same
    list.
  - Each template's `AGENTS.md`: a short section before *The tools*,
    "On Linux and in a cloud agent's container": run `bash
    scripts/setup-linux.sh` once before the first build (it is safe to
    run again); `cargo shuyi doctor` builds the game first, so it can't
    report a missing library, and the script installs everything
    doctor checks; a cloud agent's container starts bare, so run the
    script first or name it in the agent's setup step;
    `.devcontainer/devcontainer.json` runs it in a dev container or a
    Codespace.
  - Each template's `README.md`: rows for `scripts/setup-linux.sh` and
    `.devcontainer/` in its *Layout* table.
  - `docs/contract.md`: the `doctor` row adds "on Linux, the C linker,
    `pkg-config` and the system libraries the backend links and opens".
  - `doctor.rs`'s module doc names the Linux checks.
- **Either order with other tickets:**
  - **From 2501 (done):** the script and the dev container go in the
    `base` part of `templates/shuyi-default` (where `templates/blank`
    and `templates/grid` are named above), and the copy tests below run
    over the built-in template's short names, each composed
    (`shuyi_cli::template::built_in()`, `compose`).
  - **2505** (doctor reports `git`): its check and these are
    independent, but CI's bare container has no `git`. If 2505's check
    fails doctor when `git` is missing, `git` becomes a row of the
    table (`git` for apt, dnf and pacman, found by `git --version`), so
    the script installs it and the container's doctor passes. Whichever
    lands second adds the row. With `git` a row, the workflow test
    below still lets a line before `actions/checkout` install `git`
    (the script isn't checked out yet, and 2525's `run` job needs a
    repository), and a line that installs `git` after the script goes,
    since the script installs it.
  - **2509** (done: problem codes, ADR-0028): a check's field is
    `help`, not `fix`, and `doctor::text` prints it as `help:`, so the
    Linux rows' install commands are their `help`. A `doctor` with a
    failed check answers `SH1003` and exits 3, the Linux rows included.
  - **2010** (done: the `api` job, which installs the libraries with
    the `clippy` job's apt line) or any job added first with an apt
    line: this ticket turns that line into the script too; the workflow
    test below finds any left. 2010's job reports `Probe::read` as a
    break (a new required method of a public trait), and the completion
    notes carry a `Breaking:` line for `shuyi-cli` saying a game builds
    unchanged.
  - **2012** (the pinned Rust): the script and the dev container name
    no Rust version, so `check-toolchain` needs nothing new.
  - **2403** (parked: in-house pads, gilrs removed): if it is done and
    `cargo tree -i libudev-sys` is empty, leave the `lib-udev` row out
    of the table and libudev's package out of the script's three
    lines. If not, the row stays, and 2403 drops it when it lands (its
    clause).

**Out (do not do):**
- `doctor --fix`, or doctor running `sudo` or a package manager: the
  script installs;
- registering the MCP server in the dev containers (2531, after this
  ticket);
- the book's quick start (2802 adds the Linux line and the Codespaces
  link);
- installing the engine's `shuyi` binary in the container: it runs
  from source (`cargo run -q -p shuyi-cli --` in the engine, `cargo
  shuyi` in a game, built from the game's own engine version), and the
  release (2005) changes nothing there;
- an in-browser compiler or a hosted playground, a prebuilt image on a
  registry, and anything that costs Nick money;
- the Playdate SDK in the container (its license doesn't allow
  redistributing it);
- macOS's command line tools in doctor, and package managers beyond
  apt, dnf and pacman (the help names the libraries for those);
- per-agent setup files (`.claude/settings.json` hooks, Codex's
  environment settings, `.github/workflows/copilot-setup-steps.yml`):
  the templates' `AGENTS.md` says what to run, and each agent's own
  format is the developer's;
- Dependabot for the dev container's image: its tag
  (`ubuntu-24.04`) moves on its own and it lists no Features, so there
  is nothing to pin (2011 leaves it out for the same reason);
- a fast linker for the templates (2009, which adds its package to the
  table if it needs one);
- what the parked backend work changes (3032 loads ALSA at run time,
  2403 drops libudev, 3034 replaces the window and GL): each changes
  the table and the script when it lands;
- the agent eval's runner (2525).

If the PR grows well past 600 changed lines, not counting the byte-equal
copies, move the dev container and the Codespaces link to a follow-up
ticket (the `write-ticket` skill) and keep the rest. In the same PR, add
that ticket to the `blocked_by` of 2531 (which edits the dev
containers) and 2802 (whose quick start links the Codespace), and point
what they say of 2513's dev container at it.

## Implementation steps

1. Read `crates/cli/src/contract/doctor.rs`, `crates/cli/src/contract/new.rs`
   and `new/tests.rs`, `.github/workflows/ci.yml` and `mutants.yml`, and
   the templates' `ci.yml`, `AGENTS.md` and `README.md`.
2. Check each package name in the table against the distributions'
   package searches.
3. Write `doctor/linux.rs`: the table, the distro from os-release, the
   soname search, the help line and the checks, with their tests. Add
   `Probe::read` and wire the checks into `checks()`; change the limit.
4. Write `scripts/setup-linux.sh` and copy it into each template. Run it
   twice in an `ubuntu:24.04` container if Docker is at hand, and on
   Fedora and Arch containers once each if it is; the PR says what was
   run where.
5. Write the three `.devcontainer/devcontainer.json` files.
6. Write the copy and workflow tests in `new/tests.rs`.
7. Change the CI steps and move the `templates` job into the container.
8. Write the text changes, and run the gates.

## Acceptance criteria

- [x] On a Linux host, `shuyi doctor` lists every check of the table
      in order, and each failed one has a `help` for the distro's
      package manager, without `sudo` as root (unit tests on fakes).
- [x] The "are not checked" limit is gone, and with no `rustc` the limit
      says the Linux libraries were not checked.
- [x] `scripts/setup-linux.sh` exists in the engine and in every
      template, byte-equal, with no `\r`, and its `APT`, `DNF` and
      `PACMAN` lines equal the table's names in order (a test).
- [x] No `apt-get install` line in `.github/workflows/*.yml` or in a
      template's workflows names a package of the table (a test), and
      every Linux job that built with the pasted line calls the script.
- [x] CI's `templates` job passes in `ubuntu:24.04` for both templates:
      the script from nothing, the engine's `shuyi doctor`, `shuyi new`,
      the game's script again, the game's gates and its `cargo shuyi
      doctor`.
- [x] `.devcontainer/devcontainer.json` exists in the engine and in every
      template (`shuyi new` writes it), parses as JSON, and its `image`
      and `postCreateCommand` equal the engine's (a test).
- [x] The README has the Codespaces link, the Linux line and the map
      row; the engine's `AGENTS.md`, the templates' `AGENTS.md` and
      `README.md`, and `docs/contract.md` say what Scope says.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`doctor/linux.rs`, with `doctor.rs`'s `Fake` probe extended
  with files to read; `Fake` and its `with` become `pub(super)` so
  `linux.rs`'s tests can use it):
  - an Ubuntu host with everything: every row passes, in order, with
    versions and paths in `found`;
  - each row missing on Ubuntu: its `help` is exactly `sudo apt-get
    install -y <apt>` and the script hint; with `id -u` printing `0`, no
    `sudo`;
  - Fedora (`ID=fedora`), Arch (`ID=arch`), a derivative (`ID=pop`,
    `ID_LIKE="ubuntu debian"`), an unknown distro and no os-release
    file: dnf, pacman, apt, and all three;
  - a soname found through `ldconfig -p`, and without `ldconfig` in
    `/usr/lib64`;
  - Windows, macOS and an unknown host: no Linux checks;
  - `text()` prints a failed Linux check's `help:` line.
- The existing doctor tests updated: `a_complete_linux_toolchain_passes`
  gets the Linux programs in its fake;
  `without_rust_every_check_says_how_to_install_it` reads the new limit;
  `this_machine_runs_programs_and_finds_files` also reads `Cargo.toml`.
- In `crates/cli/src/contract/new/tests.rs`, through `plan()` for each
  of the built-in template's short names and the engine's root (`CARGO_MANIFEST_DIR/../..`, as
  that file's test at line 166 does):
  - every copy of the script is the engine's, byte for byte, with no
    `\r`, and its three lists equal the table;
  - every `devcontainer.json` parses with `serde_json`, and its `image`
    and `postCreateCommand` equal the engine's;
  - no workflow's `apt-get install` line names a package of the table,
    as a whole word (`libgl1-mesa-dri`, which 2018 installs, is not
    `libgl1-mesa-dev`).
- CI: the `templates` job in the bare container is the integration
  test of the script, the dev container's setup and doctor together.
- Mutation testing runs on the diff in CI: the exact-string asserts on
  the help lines are what kill its mutants.

## Completion notes

**What was done.**

- `crates/cli/src/contract/doctor/linux.rs`: `ROWS`, the table, as
  Scope gives it (every name checked: Launchpad's noble/amd64 binaries,
  Fedora's rawhide metadata and packages search, Arch's package search,
  and Bevy's list; none needed a correction). `Find::Run` for programs
  and `pkg-config` modules, `Find::Soname` for `ldconfig -p` (run once)
  then the four library folders. The distribution comes from `ID`, then
  each word of `ID_LIKE`; a failed row's help is one install command in
  backticks, without `sudo` when `id -u` prints `0`, then the script
  hint; an unknown distribution or no os-release gets all three.
- `doctor.rs`: `Probe::read`, the Linux checks after `rustup` on a
  host with `-linux-`, the old limit gone and the no-`rustc` limit
  naming the Linux libraries. `Fake` gains texts to read (`reading`),
  and it, `with` and `rust` are `pub(super)` for `linux.rs`'s tests.
- `scripts/setup-linux.sh`, byte-equal in `templates/shuyi-default/base/scripts/`.
  It checks what is installed first (`dpkg-query`, `rpm -q`,
  `pacman -Q`) and installs only what is missing, so a second run
  installs nothing and skips `apt-get update` too. It installs rustup
  with `--no-self-update` on `rustup toolchain install`, so it never
  updates the user's rustup.
- `.devcontainer/devcontainer.json` in the root and the template's
  `base` part (`"{{name}}"` there). Image tag `ubuntu-24.04` checked on
  MCR's tag list.
- Tests in `new/tests.rs`: `every_setup_script_is_the_engines_and_installs_doctors_table`,
  `every_dev_container_is_set_up_as_the_engines` (it also compares
  `remoteEnv` and `customizations`) and
  `no_workflow_installs_doctors_packages_by_hand`, over every short name
  of the built-in template (`template.short_names`, composed through
  `plan()`). The template's golden plans (`plans.golden`) are re-blessed
  for the new files.
- CI: the nine "Install macroquad system libraries" steps in `ci.yml`
  (three more than the ticket counted: `modules`, `api` and `coverage`
  came since), the four in `mutants.yml` and the template's
  one are `Set up Linux`. The `templates` job runs in
  `container: ubuntu:24.04` as Scope's steps say.
- Text: README (Codespaces link, Linux line, map row), `AGENTS.md`,
  the template's `AGENTS.md` and `README.md`, `docs/contract.md`, and
  also `SH1003.md` and the `shuyi-new-game` skill (the `document`
  skill: the problem page and the skill that teaches the first setup
  name what doctor checks).

**Run where.** Docker on Nick's machine: the script from nothing and
again in `ubuntu:24.04` (43 s, then 0 s with nothing installed),
`fedora:latest` and `archlinux:latest`, each followed by `cargo run -p
shuyi-cli -- doctor`, which passed every check; in
`mcr.microsoft.com/devcontainers/base:ubuntu-24.04` as its `vscode`
user (the `sudo` path), doctor passing; and in `ubuntu:24.04`, a game
made with `shuyi new --template blank`, whose own script found nothing
to do and whose `cargo shuyi doctor` passed. shellcheck finds nothing.

**Deviations.**

- **pacman runs `-Syu --needed --noconfirm`**, not `-S`: Arch's image
  ships with no package databases, so `-S` found no package, and Arch
  supports only syncing with a full upgrade (`-Sy` alone is a partial
  upgrade). Doctor's help keeps `pacman -S --needed`, right on a synced
  system.
- **curl is installed only when `curl` is missing**: Fedora's image has
  `curl-minimal`, which gives `curl` under another package's name.
- **The `templates` job installs `jq` after the script**
  (`apt-get install -y --no-install-recommends jq`): its `shuyi lint`
  step reads JSON with `jq`, which the bare image lacks. It names no
  package of the table. It also sets `shell: bash` for the job (the
  steps use `<<<`), and the game's script step checks that it printed
  "everything was there already".
- **The PR is past 600 lines, and kept whole.** About 1000 changed lines
  without the byte-equal copy, of which about 340 are tests. The cut
  Scope names (the dev container and the Codespaces link) is under 60
  of them, so it would not bring the PR near 600, and it would cost a
  ticket and two `blocked_by` edits for nothing.
- **`playtest.yml`'s `system-packages` default** still lists the apt
  packages: it is an input of the reusable workflow, run in the
  caller's repository, which may have no script (Visions of Shuyi).
  The workflow test checks that each package of the default is a row's
  apt package, so the two can't drift.

**For the game.** Visions of Shuyi builds unchanged. Its CI may call
the engine's script or keep its apt line; that is the game's choice.

**Follow-ups.** None new. 2531 (MCP in the dev containers) and 2802 (the
book's Linux line and Codespaces link) do what they said; 2531 now names
this ticket's files and test.

Breaking: shuyi-cli: `contract::doctor::Probe::read`, a new required method of `Probe`: nothing; no game implements `Probe` (only `shuyi-cli`'s tests do), so a game builds unchanged
