---
id: "3049"
title: "The game's identity in every package: the exe's icon and version resource on the GNU toolchain, Info.plist and .icns in the .app, a .desktop file and hicolor icons, web icons, and the web title from the name"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3004", "3048", "3072"]
nick_input: none
completed:
---

# 3049 — The game's identity in every package

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, with GameMaker and Defold. Their exports write the
game's name, version and icon into each package:

- Godot's Windows export sets the exe's icon and its version fields
  (company, product, file and product version) from the export preset
  (<https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_windows.html>);
- Unity and Defold write a macOS bundle's Info.plist and icons from
  their project settings, and Defold takes icons per platform from
  `game.project`;
- Bevy leaves it to each game: a build script with a Windows resource
  crate, and a hand-made bundle.

What Shuyi has (read on 2026-10-21):

- **From 3048 (done):** the identity is `shuyi.ron`'s `app` part
  (`AppPart` in `crates/cli/src/project.rs`: `name`, `id`, `version`,
  `build`, `publisher`, `storage` and `code`), checked by
  `app_info::problems` in `crates/cli/src/app_info.rs`, whose
  `check_version` parses with the `semver` crate (already the CLI's).
  `ExportCx::version` is already `app.version` when the part sets one,
  else the package's.
- **From 3072 (done):** `AppPart` has `icon` (one square PNG of 512 px
  or more) and `window_icons`. `app_info::icon(part, root)` reads and
  checks the icon (an `app_info::Icon { side, rgba }`, or the error
  `check --target` reports as `app.icon`), and
  `app_info::downscale(w, h, rgba, size)` makes a `size` × `size`
  picture in integers (ADR-0061). `app-info` writes the window's 16, 32
  and 64 px icons with them; every package's size uses the same two.
- 3004 (done) added `shuyi export` and four exporters
  (`crates/cli/src/export.rs`, `export/desktop.rs`, `export/web.rs`):
  `windows` (a zip built for `x86_64-pc-windows-gnu`), `linux` (a
  `tar.gz`, built in CI in Valve's sniper SDK container), `macos` (a
  universal `.app` with a minimal Info.plist from `desktop::info_plist`
  and an ad-hoc signature, built in CI on macOS runners) and `web` (the
  web build's folder and its zip). 3071 adds `linux-arm64`. None puts
  an icon or version in the exe, a bundle id, a `.desktop` file or a
  favicon.
- The web shell (`crates/cli/web/index.html`) has a `<title>%TITLE%</title>`
  and no icon. `package` in `crates/cli/src/web.rs` fills it from
  `web.title` (`Web::title`, a `String` that is empty when unset), the
  name's second copy beside the window's title. `crates/cli/web/README.md`
  says so.
- `codec::encode_png(width, height, rgba)` and `codec::decode_png(bytes)`
  are in `crates/cli/src/codec.rs`. 2565 removes `decode_png` for
  `shuyi_assets::bitmap::convert::png_pixels` (the clause with 2565
  below).
- `shuyi_app::window_conf` (`crates/app/src/lib.rs`) sets `high_dpi:
  true`.
- `shuyi doctor` (`crates/cli/src/contract/doctor.rs`, `checks`) checks
  MSYS2's GCC on a `windows-gnu` host (`msys2-gcc`), and nothing else
  that a Windows resource needs. `docs/contract.md`'s `doctor` row says
  what it checks.

Why it matters:

- **Visions of Shuyi:** its desktop and web packages look finished: an
  exe with its icon, name and version in Explorer, a `.app` with a bundle
  id (signing, 3051, needs one), a page with its icon in the tab.
- **Other 2D games:** the same for every game that ships, with nothing
  written per OS.
- **The end goal** (Nick, 2026-10-19, `docs/decisions.md`): an agent
  ports a game to many targets from one file, in few lines.

## Nick input

None.

## Scope

**In:**
- **Shared,** as `identity.rs` beside 3004's exporters:
  - `Identity { name, id, version, build, publisher, icon:
    Option<PathBuf> }`, from 3048's part and the project's root. The
    `version` is the part's, or else the game's package version from
    `cargo metadata --no-deps --format-version 1`: the same value as
    3004's report, which `metadata` in `crates/cli/src/export.rs` reads
    into `ExportCx::version`: use it;
  - `Identity::numbers() -> Result<[u16; 4], String>`: major, minor and
    patch from `semver::Version::parse` (as `app_info::check_version`
    parses it), then `build`. A number over 65535
    is an error naming `app.version` or `app.build`;
  - `icons(identity, sizes) -> Result<Vec<(u32, Vec<u8>)>, String>`: a
    PNG at each size no larger than the source, through `decode_png`,
    3072's `downscale` and `encode_png`. Empty without an icon. `shuyi
    web` doesn't run `check --target`, so a bad icon is an error here
    too, naming its path;
  - `ico(pictures) -> Vec<u8>`: the 6-byte header (0, type 1, the
    count), one 16-byte entry per picture (width and height, 0 for
    256; 0 colours; 0; planes 1; 32 bits; the PNG's length; its offset),
    then the PNG data in the same order;
  - a project without an `app` part exports as it does today, and the
    report says "no identity: shuyi.ron has no `app`".
- **`windows`:**
  - `app.ico` (16, 24, 32, 48, 64 and 256 px) and `app.rc`, written in
    the export's work folder under `target/`, never in the zip;
  - `app.rc` is UTF-8 and holds `#pragma code_page(65001)`,
    `1 ICON "app.ico"` and a `VERSIONINFO`:
    - `FILEVERSION` and `PRODUCTVERSION` as `major,minor,patch,build`
      (`Identity::numbers`);
    - a `StringFileInfo` block `"040904B0"` with `CompanyName` (the
      publisher, left out when empty), `FileDescription` and
      `ProductName` (the name), `FileVersion` and `ProductVersion` (the
      version string), `InternalName` (the binary) and
      `OriginalFilename` (`<bin>.exe`);
    - a `VarFileInfo` block with `Translation` `0x409, 1200`;
    - a `"` inside a string is written `""`;
  - a number over 65535 refuses the export with `numbers`' error;
  - compiled to a COFF object by binutils' `windres`
    (`x86_64-w64-mingw32-windres` if it is found, else `windres`; MSYS2's
    `mingw-w64-x86_64-gcc` brings its binutils), run in the work folder
    as `windres --codepage=65001 -O coff -i app.rc -o app.o`. Check on
    the day that `Lune Café` shows right in Explorer. If it doesn't,
    write each non-ASCII UTF-16 unit of a string as `\xHHHH` in an
    `L"..."` string, and say which worked in the completion notes;
  - linked into the exe by building with `cargo rustc -p <package> --bin
    <bin> --target x86_64-pc-windows-gnu [--release] -- -C
    link-arg=<work folder>/app.o`, so only the game's binary gets it and
    the game gains no build script. The object is passed as a file, not
    in an archive, so the linker keeps it;
  - `windres` is a build tool like the linker, never shipped, so
    `deny.toml` doesn't change. A missing `windres` says how to install
    it: the MSYS2 step on a Windows host (`pacman -S
    mingw-w64-x86_64-gcc`), apt's `binutils-mingw-w64-x86-64` elsewhere;
  - `shuyi doctor` checks `windres` on a `windows-gnu` host, after
    `msys2-gcc`, as a check named `windres` that runs `windres
    --version` through the `Probe`. The module's docs and
    `docs/contract.md`'s `doctor` row name it.
- **`macos`** (built in CI on macOS, as 3004 does):
  - `Contents/Info.plist`, written as XML text by our code (`&`, `<` and
    `>` escaped): `CFBundleIdentifier` (the id), `CFBundleName` and
    `CFBundleDisplayName` (the name), `CFBundleShortVersionString`
    (`major.minor.patch`), `CFBundleVersion` (the build),
    `CFBundleExecutable` (the binary), `CFBundleIconFile` (`AppIcon`,
    only with an icon), `CFBundlePackageType` (`APPL`),
    `CFBundleInfoDictionaryVersion` (`6.0`), `NSHighResolutionCapable`
    (true, since `window_conf` sets `high_dpi`) and
    `LSApplicationCategoryType` (`public.app-category.games`). It
    replaces the Info.plist 3004 writes;
  - `Contents/Resources/AppIcon.icns`, written by our code: `icns` and
    the file's length (big-endian), then per entry its type, its length
    with the 8-byte entry header, and a PNG (`ic11` 32 px, `ic12` 64,
    `ic07` 128, `ic08` 256, `ic13` 256, `ic09` 512, `ic14` 512, `ic10`
    1024). A source under 1024 px leaves out `ic10`, and the report
    warns;
  - both are written before 3004's ad-hoc signature (`codesign --force
    --sign -`, nested code first, then the bundle), since the signature
    covers them;
  - the `.app` folder is named after the name, with `/` and `:`
    replaced by `-`.
- **`linux`** (**Either order with 3071:** if 3071 is done, its
  `linux-arm64` package gets the same; if not, nothing to do, 3071
  gives it these files when it lands):
  - in each `tar.gz`: `share/applications/<id>.desktop` (`[Desktop
    Entry]`, `Type=Application`, `Name`, `Exec` the binary's name,
    `Icon` the id, `Categories=Game;`, `Terminal=false`), and
    `share/icons/hicolor/<n>x<n>/apps/<id>.png` for 16, 32, 48, 64, 128,
    256 and 512;
  - `docs/targets/desktop.md` (3004's page on the desktop packages)
    says how a player installs them: `share/` into
    `~/.local/share`, and the binary on `PATH` or `Exec` set to its full
    path. A player who runs the binary directly needs neither.
- **`web`,** in `package` (`crates/cli/src/web.rs`), so `shuyi web` and
  `shuyi export web` both get it. `web::run` and `package` take
  `Option<&AppPart>` beside `rules`; `lib.rs`'s `web` passes
  `project.config.app.as_ref()`:
  - `favicon.ico` (16, 32 and 48 px) and `apple-touch-icon.png` (180 px)
    beside the page, and a `%ICONS%` placeholder in `index.html`'s
    `<head>` that becomes `<link rel="icon" href="favicon.ico">` and
    `<link rel="apple-touch-icon" href="apple-touch-icon.png">`, or
    nothing without an icon;
  - **the title from the name:** `Web::title` becomes `Option<String>`.
    Without it the page takes `app.name`; with neither, `web` fails with
    "shuyi.ron has no `web.title` or `app.name`". The title is written
    with `&`, `<`, `>` and `"` escaped. hello-grid's `web.title` goes,
    since its `app.name` (`"Hello, Grid"`, 3048) is the same. The name's second copy
    ends;
  - `crates/cli/web/README.md` and `web.rs`'s module docs say where the
    title comes from and what `%ICONS%` becomes.
- **CI** (3004's `export` job in `.github/workflows/ci.yml`, which
  exports `examples/hello-grid` and checks its file lists in
  `tools/export-check/`; 3004 made no fixture game, so this ticket adds
  one), on that fixture game given an `app` part
  (`name: "Lune Café"`, a publisher, `version: "1.2.3"`, `build: 4`) and
  an icon. The icon is a 1024 px PNG of a few flat squares, drawn by a
  test helper with `encode_png` and checked in; a test checks the file
  equals the helper's output, so it stays code, not art:
  - an `#[ignore]`d test in `crates/cli/tests/it/` reads the exe that
    `SHUYI_EXPORTED_EXE` names, and finds by byte search `.rsrc` in its
    section table and, in UTF-16LE, `VS_VERSION_INFO`, the product name
    and the publisher. The job runs it with `--ignored` after `shuyi
    export windows`;
  - the job's `macos` leg (`macos-latest`, on pushes to `main`) exports the
    fixture for `macos`. `plutil -lint` passes on the Info.plist, and
    `iconutil -c iconset` reads the `.icns`;
  - on the Linux runner, `desktop-file-validate` passes on the
    `.desktop` file (apt's `desktop-file-utils`, a CI tool, never
    shipped);
  - the web folder has both icons, and `index.html` links them.
- **Either order:**
  - **3052** (the web shell) changes the same `index.html`: whichever
    lands second keeps the others' parts.
  - **Either order with 2565:** if 2565 is done, `icons` decodes the
    source with `png_pixels` instead of `decode_png`; if not, use
    `decode_png` and 2565 moves the call onto `png_pixels`.
  - **From 3066 (done):** `index.html`'s `<head>` has
    `<link rel="license" href="NOTICES.txt">` after the `<title>`; keep
    it, and put it after the icons' links.
  - **From 3039 (done):** `NOTICES.txt`'s heading takes the game's
    title from `web.title` (`notices::write`'s `title`, from
    `web::package`). This ticket makes the heading take the same title
    the page does (the `web.title`, else `app.name`).
  - **3050** (releases from a tag): if it is done, its release packages
    come from these exporters, so they carry the identity with no change
    there. If not, nothing.
- **For the game:** a `shuyi.ron` with `web.title` reads as before.
  `Web::title`'s type changes, for code that builds a `Web` by hand.
  Nothing else changes until the game adds an `app` part. Say so in the
  PR.
- **Size:** if the PR passes about 600 changed lines, the web part (the
  icons, `%ICONS%`, the title from `app.name` and its escaping, the
  README, and the `NOTICES.txt` heading) goes to a follow-up ticket (the
  `write-ticket` skill, After MVP, blocked by this one), and
  hello-grid's `web.title` stays until that ticket.

**Out (do not do):**
- signing and notarising (3051, parked: they cost money);
- Playdate's `pdxinfo`, launcher icon and card: 3010, which reads the
  `app` part;
- the `app` part and its checks (3048, done), the icon and the window's
  icons (3072);
- the Steam Deck (3012), since Steam shows its own art; store and
  library art;
- mobile manifests (3015, parked);
- a web app manifest, an install script, Flatpak and AppImage;
- the web shell's progress bar and error panel (3052), and its notices
  (3039);
- an icon for hello-grid: an icon is art, and the engine's examples ship
  without one until one is drawn from the engine's own data.

## Implementation steps

1. Read 3004's exporters and CI's `export` job as merged,
   `crates/cli/src/app_info.rs` and `AppPart` as 3048 and 3072 left them, `crates/cli/src/web.rs`,
   `web/index.html` and `web/README.md`, `contract/doctor.rs`, and the
   `doctor` and `export` rows of `docs/contract.md`.
2. Write `identity.rs`: `Identity`, `numbers`, `icons` and `ico`, with
   unit tests.
3. Windows: the `.rc` text, `windres`, the `cargo rustc` build and the
   doctor check. Export the fixture with the GNU toolchain and look at
   the exe's properties in Explorer.
4. macOS: Info.plist and the `.icns`. Linux: the `.desktop` file and the
   icons.
5. The web: `%ICONS%`, the icons, the title and its escaping, the
   README.
6. The fixture's `app` part and icon, the CI steps, and the docs. Run
   the gates.

## Acceptance criteria

- [ ] `shuyi export windows` on the fixture, with the GNU toolchain,
      gives an exe whose Explorer Details tab shows the name as its file
      description and product name, the version, and the icon (a
      screenshot in the PR), and CI's byte test passes, publisher
      included.
- [ ] A version number or build over 65535 refuses `windows`, naming
      `app.version` or `app.build`.
- [ ] On CI's macOS runner the `.app`'s Info.plist passes `plutil -lint`
      and has the id as `CFBundleIdentifier`, and `iconutil` reads its
      `.icns`.
- [ ] On CI's Linux runner `desktop-file-validate` passes on the
      `.desktop` file, and the icons are at their hicolor paths.
- [ ] `shuyi web` on the fixture writes `favicon.ico` and
      `apple-touch-icon.png` and links them. Without an icon it writes
      neither, and no `<link>`. Without `web.title` the page's title is
      `app.name`, escaped; with neither, `web` fails with the message
      above.
- [ ] A project without `app` exports as before, with the report's line.
- [ ] `shuyi doctor` on a `windows-gnu` host without `windres` fails the
      `windres` check and says how to install it (with the fake probe).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `numbers` and the 65535 rule; `icons` (sizes above the source
  left out, none without an icon, a bad file's error); the `.ico` writer
  (header, entries, offsets, a 256 px entry's 0); the `.rc` text (with
  and without a publisher, with `Lune Café`, with a `"` in the name);
  the Info.plist text and its escaping, with and without an icon; the
  `.icns` writer (the file's length, the entry types, which entries a
  512 px source gives); the `.desktop` text; `%ICONS%` with and without
  an icon; the title's fallback and escaping; the doctor's `windres`
  check.
- Integration: each exporter on the fixture, with 3004's file lists
  (`tools/export-check/<target>.txt`) updated; `windres` and `cargo rustc` through a fake runner,
  for their arguments; the fixture icon equals the helper's; the CI
  checks above on real packages.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
