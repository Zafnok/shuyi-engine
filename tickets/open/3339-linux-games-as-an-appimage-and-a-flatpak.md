---
id: "3339"
title: "Linux games package as an AppImage with a runtime of our own, and a Flatpak manifest is written beside it"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3049", "3071"]
nick_input: none
completed:
---

# 3339 — Linux games as an AppImage, and a Flatpak manifest

## Context

`shuyi export linux` (3004, done) writes one Linux package: a `tar.gz`
of a folder holding the program, its pack, `NOTICES.txt` and what ships
beside it (`pack` in `crates/cli/src/export/desktop.rs`). CI builds it
in Valve's sniper SDK, so glibc 2.31 is its floor (ADR-0058,
`SNIPER` in the same file). A player needs ALSA, libudev, X11 and
OpenGL from their system (`docs/targets/desktop.md`, the `linux` row).

Off Steam, a bare `tar.gz` is the least friendly Linux format. It does
not open on a double-click and does not appear in the desktop's menu.
An AppImage is the usual format for itch.io and DRM-free Linux
releases. Flathub is the main Linux store outside Steam. No ticket
plans either one. 3004 (done, line 359) and 3049 (open, line 265) both
put "Flatpak and AppImage" under Out, and nothing else names them
(`grep -rni 'appimage\|flatpak\|flathub' tickets docs crates`). A
blind-spot audit of 2026-11-23 found this gap (low severity, no
coverage). The game does not ask for it either: `gh search code --repo
Zafnok/visions-of-shuyi 'AppImage'` and `'Flatpak'` return nothing (the
game pins engine `50472bbc`).

What this ticket builds on:

- **3049** (needed first) puts the game's identity in every package.
  The `linux` `tar.gz` gets `share/applications/<id>.desktop` and
  `share/icons/hicolor/<n>x<n>/apps/<id>.png`, and `identity.rs` has
  `Identity` and `icons`. The AppImage and the Flatpak reuse those
  files. They do not write their own.
- **3071** (needed first) adds the `linux-arm64` exporter and an audit
  of what each packaged program loads (`export/libraries.rs`). The
  AppImage's runtime is a second program in the package, so the audit
  checks it too. Both arches get an AppImage.
- `app.id` is reverse-DNS already (`crates/cli/src/app_info.rs`, the
  `id` rule). It becomes the AppImage's desktop id and the Flatpak's
  app id.

**Why the runtime is our own.** An AppImage is an ELF "runtime" with a
SquashFS image of an AppDir appended to it. The usual runtime,
`AppImage/type2-runtime`, statically links libfuse, which is LGPL. It
would go into every AppImage we write. ADR-0002 §8 (hard rule 7) denies
shipped LGPL code. The older runtime loads the system's
`libfuse.so.2` instead, which recent distributions no longer install,
and it is archived. `appimagetool` builds the image with `mksquashfs`
(GPL), which is a build tool and would be allowed, but we don't need it.
So this ticket writes a small extract-and-run runtime in Rust, the same
thing the standard runtime does with `--appimage-extract-and-run`. It
writes the image with `backhand`, a pure-Rust SquashFS library
(MIT OR Apache-2.0). Without FUSE, an AppImage opens on any system that
runs the `tar.gz`.

## Nick input

None.

## Scope

**In:**
- **`--package`** on `shuyi export linux` and `linux-arm64`: a
  comma-separated list of `tar-gz` (the default and today's package),
  `appimage` and `flatpak`. It is a new field `packages: Vec<String>`
  on `ExportInput` (`crates/cli/src/contract/export.rs`, already
  `#[non_exhaustive]`; the field is additive). An optional input field
  breaks no client, so the contract's `VERSION` stays and the schemas
  only add their line to `versions.json` (ADR-0150). Any other
  target given `--package` fails with a setup error that names the
  targets that take it. `--recipient` with a package other than
  `tar-gz` fails the same way: a recipient's copy stays a `tar.gz`.
- **The AppImage**, `<base>.AppImage` beside the `tar.gz`
  (`ExportCx::base`, so `hello-grid-0.1.0-linux.AppImage`):
  - its AppDir holds the folder `pack` writes now, under `usr/bin/`
    (the program, its pack and what ships beside it, so the program
    still finds its pack beside itself), plus `usr/share/` from 3049
    (the `.desktop` file and the hicolor icons) and
    `usr/share/metainfo/<id>.metainfo.xml` (below). At the AppDir's root
    are `AppRun` (a symlink to `usr/bin/<bin>`), `<id>.desktop` (3049's
    file plus `X-AppImage-Version=<version>`), `<id>.png` (256 px) and
    `.DirIcon` (the same PNG), and `NOTICES.txt`;
  - the image is SquashFS, zlib-compressed, written by `backhand` with
    every entry at the same fixed time as `archive::tar_gz`
    (1980-01-01), in sorted order, uid and gid 0, and modes 755 for
    programs and 644 for everything else. The same export run twice
    gives the same bytes (ADR-0058);
  - `backhand` is used with `default-features = false` and only
    `gzip`. Its default features pull in xz's C library, and its
    optional `lzo` feature is GPL (`rust-lzo`). Check its license and
    every new crate in the graph with `cargo deny check` and write what
    you found in the PR. It is a `shuyi-cli` dependency only, never a
    game's;
  - a game without an `app` part has no id, so `appimage` fails with
    the message 3049's report uses ("no identity: shuyi.ron has no
    `app`").
- **The runtime**, in `crates/cli/src/export/appimage/runtime.rs`. It is
  one self-contained file with no dependency outside `std` and
  `backhand`:
  - `shuyi-cli` compiles it as a `#[cfg(test)]` module, so its
    functions get unit tests, and it embeds the same file with
    `include_str!`. At export it writes the file and a `Cargo.toml` (a
    const in `appimage.rs`, with an empty `[workspace]`, the same
    exact `backhand` version and features, `panic = "abort"`,
    `opt-level = "z"`, `strip = true`) under the game's `target/` work
    folder. It builds them with `cargo build --release --offline
    --target <triple>`. The game's own `cargo fetch` has already put
    `backhand` in Cargo's cache, because `shuyi-cli` depends on it.
    There is no new workspace crate, and the source stays inside
    `shuyi-cli`'s package, so `cargo package` keeps it;
  - at start it reads `/proc/self/exe` and finds the image at the end
    of the ELF (`e_shoff + e_shnum * e_shentsize`, the AppImage
    spec's rule). It extracts the image once into
    `$XDG_CACHE_HOME/<id>/appimage-<first 16 hex digits of the
    digest>/` (`~/.cache` when unset). It writes a temporary folder
    and renames it into place, so two starts at once never see half a
    folder. It deletes the other `appimage-*` folders under that id
    after it extracts a new one;
  - it `exec`s `usr/bin/<bin>` with the player's arguments. It sets
    `APPIMAGE` (its own path), `APPDIR` (the extracted folder), `OWD`
    (the working directory it started in) and `ARGV0`, as the
    AppImage spec says. The game's exit code is the runtime's, because
    `exec` replaces it;
  - `--appimage-extract` writes `squashfs-root/` in the working
    directory and exits. `--appimage-offset` prints the image's offset.
    Any other argument goes to the game;
  - the exporter patches two things into the built runtime's bytes. The
    first is the magic `AI\x02` at offset 8 (the ELF header's padding,
    which loaders ignore). The second is the image's SHA-256, written
    over a 64-byte placeholder static that begins with a marker string.
    The runtime reads that static through `std::hint::black_box`, so
    the compiler cannot fold it. The hash comes from the CLI's own
    `crates/cli/src/sha256.rs`;
  - 3071's audit checks the runtime as it checks the program: only
    glibc's own libraries, and nothing above the 2.31 floor.
- **The metainfo**, `<id>.metainfo.xml` (AppStream), shared by the
  AppImage and the Flatpak:
  - if the game has `linux/metainfo.xml` beside `shuyi.ron`, that file
    is used as it is. Its `<id>` must equal `app.id`, or the export
    fails, naming both;
  - otherwise the exporter writes a minimal one: `component
    type="desktop-application"`, the id, the name, `<summary>` as
    "<name> by <publisher>", `<developer>` with the publisher,
    `<metadata_license>CC0-1.0</metadata_license>`, `<launchable
    type="desktop-id"><id>.desktop</launchable>`, `<content_rating
    type="oars-1.1"/>` and one `<release version="<version>"/>`, all
    XML-escaped. `export-report.json` then warns that Flathub needs a
    description, a project license, screenshots and a real content
    rating, and that the game should write `linux/metainfo.xml`.
- **The Flatpak manifest**, `<id>.yml` beside the packages, with the
  metainfo and the `tar.gz` it builds from:
  - `id` (Flatpak wants three or more parts. Check its documentation
    and correct this number if it is wrong. A shorter `app.id` fails,
    naming `app.id`); `runtime: org.freedesktop.Platform` and `sdk:
    org.freedesktop.Sdk` at a `runtime-version` held in one const
    (take the current stable branch when worked, and say which in the
    docs); `command: <bin>`;
  - `finish-args` in one const list: `--share=ipc`, `--socket=x11`,
    `--device=dri`, `--socket=pulseaudio` (the runtime's ALSA goes to
    PulseAudio or PipeWire through it) and `--device=all` (controllers
    are read through evdev and libudev). The docs say why each is
    there. There is no network and no home-folder access: saves go to
    `$XDG_DATA_HOME`, which Flatpak maps to `~/.var/app/<id>/data`;
  - one module, `buildsystem: simple`. Its sources are a `type:
    archive` of each Linux `tar.gz` of this version found under the
    export folder (`linux`, and `linux-arm64` from 3071), each with its
    `sha256` and `only-arches` (`x86_64` or `aarch64`). Its
    `build-commands` install the folder into `/app/bin/` and `share/`
    into `/app/share/`, and the metainfo into `/app/share/metainfo/`;
  - the sources are `path:` entries, so `flatpak-builder` builds from
    the export folder. The docs say how to switch to `url:` for a
    release a store fetches.
- `docs/targets/desktop.md`: one row each for the AppImage and the
  Flatpak in the packages table, and a section each. They cover what
  the player does (`chmod +x`, then open it; where its cache folder is),
  why the runtime is our own (the license), the finish args, and how to
  test the manifest locally. The `export` row of `docs/contract.md`,
  and the new field in the export schema.
- An ADR (the `write-adr` skill, the next free number): Linux packages
  beyond the `tar.gz`. It covers the extract-and-run runtime of our
  own and why (LGPL libfuse in the standard runtime), SquashFS through
  `backhand`, and the Flatpak manifest as generated data.

**Out (do not do):**
- submitting to Flathub, its account, and its review: that is the
  game's own release work;
- an AppImage's update information (zsync) and AppImageUpdate, and any
  in-game "newer version" check: the HTTP capability it needs is 2601,
  parked;
- signing an AppImage (GPG) or anything else: 3051, parked;
- a Snap, a `.deb`, an `.rpm` or an install script;
- the Steam Deck and Steam's Linux runtime (3012): Steam uses the
  `tar.gz`;
- Wayland in the finish args: the backend draws through X11 today, so
  the list has `--socket=x11`. The list is one const, so whoever brings
  Wayland changes one line;
- a FUSE mount in the runtime, and bundling ALSA, libudev, X11 or GL in
  the AppImage. It needs what the `tar.gz` needs, on the same floor.

**Estimated size:** about 1,500 lines (code 800, tests 500, docs 200).

**Split:** past 900 lines, split off steps 6–7 (the metainfo, the game's
`linux/metainfo.xml` and the Flatpak manifest) into a new ticket. The
AppImage then ships with the minimal metainfo only.

## Implementation steps

1. Read `crates/cli/src/export.rs`, `export/desktop.rs` (`pack`,
   `archive_folder`), `export/archive.rs`, `contract/export.rs`,
   `tools/export-check/check.sh` and `linux.txt`, CI's `render` job in
   `.github/workflows/ci.yml` and `export.yml`, all as 3049 and 3071
   left them. Read the AppImage specification (type 2: the magic bytes,
   the ELF-size rule, the environment variables).
2. Add `packages` to `ExportInput`, `--package` to the CLI, and the
   setup errors. Regenerate the schemas and add their line to
   `versions.json`.
3. Write `export/appimage/runtime.rs`: find the offset, read the
   digest, extract with `backhand` to a temporary folder, rename,
   prune old folders, handle `--appimage-extract` and
   `--appimage-offset`, set the variables, `exec`. Unit-test the
   offset, the cache path and the argument split as a `#[cfg(test)]`
   module.
4. Write `export/appimage.rs`: build the AppDir from the folder `pack`
   writes and 3049's `share/`, write the SquashFS with `backhand`
   (fixed times, sorted entries), build the runtime as the Scope says,
   patch the magic and the digest, and concatenate. Report the
   `.AppImage` with its size in `export-report.json`.
5. Make 3071's audit read the runtime too.
6. Write the metainfo: the game's `linux/metainfo.xml` with its `<id>`
   check, or the minimal one, and the report's warning.
7. Write `export/flatpak.rs`: the manifest as YAML text (no new YAML
   dependency: it is a few fixed keys, each value quoted and escaped),
   the sources from the Linux `tar.gz` files found, the id rule.
8. CI. In the `render` job (it has Xvfb and Mesa already), export
   hello-grid with `--package tar-gz,appimage,flatpak` without
   `--release`, so it has debug tools. Then:
   - run the AppImage with `SHUYI_PANIC_AT_FRAME=3` under `xvfb-run`
     and expect exit 101, as the `tar.gz` program is checked now;
   - run it with `--appimage-extract` and compare the files with
     `linux.txt`;
   - run `appstreamcli validate --no-net` on the metainfo
     (`appstream`), and `desktop-file-validate` on the AppDir's
     `.desktop` file;
   - build the manifest with `flatpak-builder --user
     --install-deps-from=flathub` (`flatpak` and `flatpak-builder` from
     apt; CI tools, never shipped).

   `check.sh` learns the `.AppImage`: the magic at offset 8, and the
   same no-symbols check on the program inside.
9. The docs and the ADR. Run the gates.

## Acceptance criteria

- [ ] `shuyi export linux --package appimage` writes
      `<base>.AppImage`, and the same export run twice gives the same
      bytes.
- [ ] Its bytes 8–10 are `AI\x02`, and the SquashFS starts at the
      offset the ELF-size rule gives (a test reads both back).
- [ ] Run under Xvfb in CI, the AppImage starts the game: hello-grid
      with `SHUYI_PANIC_AT_FRAME=3` exits 101 through it.
- [ ] A second start reuses the extracted folder (no new extraction,
      checked by the folder's modification time in a test), and an
      AppImage with a different digest replaces the old folder.
- [ ] `--appimage-extract` writes `squashfs-root/` holding the files
      `linux.txt` lists, plus the AppDir's own files.
- [ ] 3071's audit passes on the runtime: it loads only glibc's own
      libraries and asks for no `GLIBC_` version above 2.31.
- [ ] `cargo deny check` passes with `backhand` in the graph, and
      `Cargo.lock` has no `rust-lzo` and no `xz2` or `lzma-sys`.
- [ ] `appstreamcli validate --no-net` reports no error on the minimal
      metainfo, and `desktop-file-validate` passes on the AppDir's
      `.desktop` file.
- [ ] A game's `linux/metainfo.xml` whose `<id>` differs from `app.id`
      fails the export, naming both.
- [ ] `--package flatpak` writes `<id>.yml`, and `flatpak-builder`
      builds it in CI. With both Linux `tar.gz` files present, the
      manifest has two archive sources with `only-arches` `x86_64` and
      `aarch64`.
- [ ] A two-part `app.id` fails `--package flatpak` with a message
      naming `app.id`. With `appimage` alone, the export still works.
- [ ] `--package appimage` on `windows`, and `--package appimage` with
      `--recipient`, each fail with a setup error. `tar-gz` alone gives
      today's package byte for byte.
- [ ] A game without an `app` part fails `--package appimage` with
      3049's "no identity" message.
- [ ] `docs/targets/desktop.md`, `docs/contract.md` and the new ADR say
      what the Scope says.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the runtime's offset from a fixture ELF header, its cache path
  with and without `XDG_CACHE_HOME`, and the split of
  `--appimage-extract`, `--appimage-offset` and the game's arguments;
  the digest placeholder found and patched, and a binary missing it
  refused; the AppDir's file list; the minimal metainfo's text and its
  escaping (a name with `&` and `<`); the manifest's text with one and
  with two arches; the id rule; `--package` parsing and its errors.
- Property: a SquashFS image `backhand` writes from any set of small
  files reads back to the same names, modes and bytes, and two writes
  of one set are byte-equal.
- Snapshot / integration: the manifest and the minimal metainfo for the
  fixture game, as snapshot files; CI's `render` job runs the AppImage
  under Xvfb, extracts it, validates the metainfo and the `.desktop`
  file, and builds the Flatpak.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
