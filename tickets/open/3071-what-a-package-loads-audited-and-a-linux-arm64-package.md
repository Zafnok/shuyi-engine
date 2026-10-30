---
id: "3071"
title: "What a package loads, audited against a list per target, and a linux-arm64 package"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: ["3004"]
nick_input: none
completed:
---

# 3071 — What a package loads, audited, and `linux-arm64`

## Context

3004 built `shuyi export <target>` ([ADR-0058](../../docs/adr/0058-export-packages-carry-nothing-of-the-build-machine-paths-remapped-and-searched-the-linux-floor-in-the-sniper-sdk-and-a-checked-package-per-target.md),
`crates/cli/src/export.rs`): the `windows`, `linux`, `macos` and `web`
exporters, the path search, the debug file, and CI's `export` job. Its
PR passed 600 changed lines, so two pieces of its scope moved here
(3004's step 8):

- **The library audit.** Today only CI's shell script,
  `tools/export-check/check.sh`, looks at what a program loads, and only
  on the examples: the Windows program's DLLs (no MinGW runtime) and the
  Linux program's newest `GLIBC_` version. A game's own export checks
  nothing of it. An exe that needed `libwinpthread-1.dll` would run on a
  builder with `C:\msys64\mingw64\bin` on `PATH` and nowhere else; a
  Linux program linked against a library the sniper SDK has and a
  player's system lacks would fail before any bug report.
- **`linux-arm64`.** Linux on Arm (a Raspberry Pi 5 with 64-bit
  Raspberry Pi OS, ChromeOS's Linux on an Arm Chromebook) needs no new
  backend. GitHub's `ubuntu-24.04-arm` runners are free for public
  repositories (since 2025-08-07). Valve's sniper SDK image is amd64 only
  (its registry, read 2026-10-29), so this package builds in
  `debian:bullseye`, whose glibc (2.31) is the same floor.

## Nick input

None.

## Scope

**In:**
- **The audit**, in `crates/cli/src/export/libraries.rs`, run by every
  native exporter after its build on the packaged program:
  - It reads the program with the `object` crate and checks each
    library it loads against its target's list in
    `crates/cli/export/libraries.ron`. That file is embedded with
    `include_str!` and read with `ron`. It is the tool's own table, not
    an asset kind, so it isn't registered in `Formats`. The lists are
    data so that 3032 and 3034, which replace quad-snd and macroquad,
    change one file.
  - `windows`: the PE import table (`PeFile::import_table`) names only
    Windows' own DLLs and its API sets (`api-ms-win-*`). No
    `libgcc_s_seh-1.dll`, `libwinpthread-1.dll` or `libstdc++-6.dll`.
  - `linux` and `linux-arm64`: `DT_NEEDED` names only glibc's own
    libraries, `libgcc_s.so.1`, `libasound.so.2` and `libudev.so.1`. The
    version needs (`SectionTable::gnu_verneed`) ask for no `GLIBC_`
    version above the list's floor (2.31).
  - `macos`: each slice of the universal binary (`MachOFatFile`) loads
    only from `/System/Library/` and `/usr/lib/`, and both slices are
    there.
  - A library the package itself carries beside the executable passes.
  - A failure refuses the package with a new problem code (the next free
    `SH30nn`, its row and its page) and names the library and the rule.
  - `export-report.json` gains `loads` (what each program loads) and,
    on Linux, `glibc` (the newest version it asks for). The contract's
    version goes up by one.
  - `object` 0.37 is Apache-2.0 OR MIT, both on `deny.toml`'s allow
    list. Use it with `default-features = false` and only the features
    `std`, `read_core`, `elf`, `macho` and `pe` (no `compression`).
    **Either order with 2520:** if 2520 is done, use the `object` version
    it brought, so `Cargo.lock` holds one; if not, nothing to do (2520
    uses this one).
  - **Either order with 3032:** if 3032 is done, leave `libasound.so.2`
    out of the Linux lists, since ALSA is then opened with `dlopen`; if
    not, nothing to do (3032 removes it).
  - **Either order with 3010:** if 3010 is done, add a `playdate` list
    (expected empty: a `no_std` game links everything statically) and
    run the audit on the ELF in its exporter; if not, nothing to do
    (3010 does both).
  - **Either order with 3012:** if 3012 is done, its `steam-deck`
    exporter runs the audit against the `linux` list too; if not,
    nothing to do (3012 runs it).
  - **Either order with 3034** (parked): if 3034 is done, find the
    ticket it wrote that removes macroquad, and write there a clause
    with 3071 (its PR checks `libraries.ron` against what winit and
    wgpu load), named back here; if not, nothing to do (3034's tickets
    carry that clause).
  - CI's `tools/export-check/check.sh` drops its DLL and `GLIBC_` checks,
    which the audit then makes in every export.
- **`linux-arm64`**, an exporter like `linux`
  (`crates/cli/src/export/desktop.rs`) for `aarch64-unknown-linux-gnu`:
  a `tar.gz`, the `desktop` profile's check, the debug file with
  `--release`, and the run step on an Arm Linux host. CI's `export` job
  gains a `linux-arm64` leg on pushes to `main`, on `ubuntu-24.04-arm` in
  `debian:bullseye` pinned by digest (record the image and how it was
  checked in the job's comment), with the `-dev` packages that
  `scripts/setup-linux.sh` installs, and `tools/export-check/` gets its
  file list. **Either order with 3049:** if 3049 is done, `linux-arm64`
  carries 3049's identity files as `linux` does; if not, nothing to do
  (3049 gives them to every desktop package). **Either order with
  3050:** if 3050 is done, `linux-arm64` gets its `host()`,
  `container()` and `ci()` like the other exporters; if not, nothing to
  do (3050 gives every exporter them). **Either order with 2504:** if
  2504 is done, add a commented `linux-arm64` entry (in
  `debian:bullseye` on `ubuntu-24.04-arm`) that the developer turns on
  to the new game's CI matrix (`.github/workflows/ci.yml` in the
  templates' base part); if not, nothing to do (2504 writes it).
- **Docs.** `docs/targets/desktop.md` gains the `linux-arm64` package:
  what it runs on (marked untested until someone runs it on a
  Raspberry Pi 5 or an Arm Chromebook, with how to report), and how to
  make it by hand in `debian:bullseye`; and the audit's lists, with
  what to do when a game's program loads a library the list lacks.
  [ADR-0058](../../docs/adr/0058-export-packages-carry-nothing-of-the-build-machine-paths-remapped-and-searched-the-linux-floor-in-the-sniper-sdk-and-a-checked-package-per-target.md)
  §8 already says every exporter passes the audit: no new ADR unless the
  design changes.
- The `shuyi-ship` skill (`skills/shuyi-ship/SKILL.md`) names
  `linux-arm64` among the targets, and the audit; `check-skills` passes.

**Out (do not do):**
- Windows on Arm as its own build (`aarch64-pc-windows-gnullvm`); the x64
  exe runs there through Prism. A ticket when a player or a game asks.
- State hashes equal on x86_64 and Arm: 2116.
- The game's identity in the packages (3049) and releases from a tag
  (3050).

## Implementation steps

1. Add `object` to `crates/cli/Cargo.toml` with the features above; run
   `cargo deny check` (no change to `deny.toml`).
2. Write `crates/cli/export/libraries.ron` and
   `crates/cli/src/export/libraries.rs`: the lists, the reading of PE,
   ELF and Mach-O, and the rules; call it from `desktop::pack` and the
   macOS packing after the path search.
3. Add the problem code, its row in `contract::codes::TABLE` and its page
   in `crates/cli/errors/`.
4. Add `loads` and `glibc` to `ExportOutput`, regenerate the schemas,
   raise `contract::VERSION` and add its fingerprint.
5. Add the `linux-arm64` exporter and its CI leg.
6. Update `tools/export-check/check.sh`, `docs/targets/desktop.md`,
   `docs/contract.md`'s `export` row and the skill.

## Acceptance criteria

- [ ] A test feeds the audit the test binary itself
      (`std::env::current_exe()`) on each OS of CI's `test` matrix, and
      the audit names that OS's C library (`kernel32.dll`, `libc.so.6`,
      `/usr/lib/libSystem.B.dylib`).
- [ ] A fixture list without one of those libraries refuses the package
      and names the library. A library inside the package passes.
- [ ] `libraries.ron` reads (test).
- [ ] `shuyi export windows` on an example, on Nick's machine, lists only
      Windows' own DLLs and API sets in its report.
- [ ] CI's `linux` package's report shows no `GLIBC_` version above the
      floor, and it loads nothing outside the list.
- [ ] CI's `linux-arm64` package is built on `ubuntu-24.04-arm` and
      passes the same audit.
- [ ] `cargo deny check` passes with no change to `deny.toml`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the audit's rules over a fixture list; `libraries.ron` reads;
  the report's new fields.
- Integration: the audit over the test binary itself on Windows, Linux
  and macOS (CI's `test` matrix); the `linux-arm64` export in CI with its
  report checked.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
