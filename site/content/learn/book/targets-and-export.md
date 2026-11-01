+++
title = "Targets and export"
description = "Each target says what its hardware can do, shuyi check --target holds a game to it, shuyi web builds the browser's page, and shuyi export packages the game for players."
weight = 90

[extra]
chapter = 9
+++

Each platform a game can ship on is a **target**, and each target has a
**profile**: plain data that says what its hardware can do. Its display
(the sizes it shows, its colors, how small text may be), its frame rate,
its inputs (which buttons, a pointer, a keyboard), its budgets of time
and memory, and the platform backend that runs a game there. A game
that needs more than a target has doesn't export there, and the tools
say why before anyone builds for it.

The profiles are the engine's
[`targets/`](https://github.com/Zafnok/shuyi-engine/tree/main/targets)
folder: `desktop` (Windows, Linux and macOS), `web` and `steam-deck` on
the macroquad backend, and `playdate` for its own backend, with phones
and the Switch planned.

## The targets a game ships on

A game lists its targets in its project file, `shuyi.ron`, each with the
size of its frame in pixels, how it scales (`integer`, or `fit`) and
the height of its smallest text:

{{ snippet(file="project/shuyi.ron", anchor="targets", language="ron") }}

## Where the game runs

A running game knows its target: `ctx.platform`
(`shuyi::ui::PlatformFacts`) holds the target's id and what its profile
says a game may do there. A screen asks it, never a `cfg!`:

- `can_quit()` is false where the player leaves through the system (the
  web's tab, a console's home button): a title offers no Quit there.
- `windowed` is false on a device that is always full screen: the
  engine's options screen leaves out its Fullscreen row.
- `keyboard`, `pointer` and `touch` say what the player has.

The facts are the screens'. A game's rules never read them, so the same
commands play the same game on every target. A test runs a screen as a
target with `Harness::as_target`, before the screen opens:

{{ snippet(file="src/screens.rs", anchor="as_target") }}

## Who the game is

Every target gets the same name, id and version, written once in
`shuyi.ron`'s `app`:

{{ snippet(file="project/shuyi.ron", anchor="app", language="ron") }}

- `name` is the name players see, and `id` is reverse-DNS: two or more
  parts between dots, of letters, digits and `-`, each starting with a
  letter. The player's saves are kept under the id (or under `storage`,
  when the part sets one), so it never changes after a release.
- `version` is semver; without it, the crate's version. `build`, a
  number that only grows, and `publisher` are for the targets that ask.
- `cargo shuyi app-info` writes the part into `code` as the game's
  `APP`, a `shuyi::platform::AppInfo` the binary reads; `--check` fails
  when the file is out of date.
- `icon` is the game's one picture: a square PNG, 512 pixels a side or
  more. The tools make every size a target asks from it, never at run
  time: `app-info` writes the window's icons, 16, 32 and 64 pixels,
  into `window_icons`, and `APP` carries them (`window_icon`), so the
  window shows it. Each size is the mean of the pixels it covers,
  weighed by how opaque they are, so a transparent edge doesn't darken
  the color beside it, and the same picture gives the same files on
  every computer.

## Checking them

```bash
cargo shuyi check --target all
```

checks, for each target the game lists, that:

- its frame at its scale fits the target's display, inside its safe
  area, and its smallest text is as tall as the target asks;
- every screen's required actions bind to distinct inputs the target
  has ([chapter 3](@/learn/book/actions.md)). A target with no keyboard
  (a Steam Deck) plays with the buttons of the keymap file's table
  named after it, `targets: { "steam-deck": { … } }`; when the actions
  don't fit, the screen changes, since the engine never folds actions
  onto fewer buttons;
- on a one-bit target, every palette role has a pattern;
- the `app` part keeps its rules, its icon is a square PNG of 512
  pixels or more, and the files `app-info` writes are up to date. An id under `com.example.` is a warning: two games with it
  share their saves.

Each problem names its target, the setting in `shuyi.ron` and the
numbers. `--target <id>` checks one.

## The web build

`cargo shuyi web` builds the game's binary for the browser
(`wasm32-unknown-unknown`, which `cargo shuyi doctor` checks rustup
has) and writes it with the page it runs in to a folder. A game that
has a web build says so in `shuyi.ron`:

{{ snippet(file="project/shuyi.ron", anchor="web", language="ron") }}

and its binary links with `--allow-undefined` on the web, so the page's
own scripts (the player's storage, controllers) are found when it
loads. A link argument reaches a binary only from its own build script,
`crates/game/build.rs`:

{{ snippet(file="build.rs", anchor="build") }}

Then:

```bash
cargo shuyi web --release
```

writes `dist/web/`: `index.html`, the game's `.wasm`, the engine's
scripts, and `NOTICES.txt`, the licenses of every crate and file the
build carries, with the game's own texts that `shuyi.ron`'s `notices`
names. Serve that folder over HTTP and open it in a browser; a page
opened from the disk can't load its `.wasm`. The engine's examples on
this site are such folders.

The folder belongs to the build. Each build replaces everything in it,
so a file from an older build can't ship, and keep nothing of your own
there: `web.dist` that holds your project or a repository is refused. A
server already running in the folder keeps serving the new build.

## Export

```bash
cargo shuyi export windows --release
```

makes the package players download. It runs the target's check first
(`windows`, `linux` and `macos` all check `desktop`) and builds nothing
if the check finds an error. Then it builds the game for the target and
writes into `dist/export/<target>/`:

- `playdate`: a `.pdx` and its zip, for sideloading: the game's library
  built without `std`, its pack baked one-bit, its sounds and music baked to
  ADPCM from what its tools' `GameTools::audio` names. A game adds a `playdate`
  feature and says how it starts with `shuyi_playdate::main!`;
- `windows`: a zip of the `.exe`;
- `linux`: a `tar.gz` that runs on any Linux with glibc 2.31 or newer;
- `macos`: a folder with one `.app` for Apple silicon and Intel Macs,
  signed ad hoc;
- `web`: the web build's folder and a zip of it, `index.html` at its
  root, as itch.io takes it. A file in it that the build didn't make is
  refused by its path.

Each desktop package builds on its own system: Windows on Windows,
Linux on Linux, macOS on a Mac (CI has all three). Every package carries
`NOTICES.txt`. The Windows and macOS ones also carry `HOW-TO-OPEN.txt`:
they aren't signed by a developer, so a player opens them once by hand.

With `--release`, the build carries nothing of the machine it was made
on: its paths are renamed, and a program that still holds one is
refused. On Windows and Linux, `<package>.debug` beside the package
keeps the program's line tables for crash reports, and is never
shipped. `export-report.json` beside it gives the version, the sizes,
the check's warnings and the signing, and `--run` runs the package.

## What a package gives away

A menu that hides a level doesn't hide its file: a player can open a package
and read what is in it. So every export reads its final package back, after
every copy its target makes, and writes down what is there in
`exposure-report.json` beside it: each file with its kind, its source, its size
and its SHA-256, each file of each pack, whether the file is a pack or a program
embeds one, the debug file, the features and what it was built from. The same
input gives the same report.

To hold a package to a list, add a `disclosure` part to `shuyi.ron`:

```ron
disclosure: (
    // Files of the game beside what the exporter writes.
    allowed_files: ["music/title.ogg"],
    // Files a pack may hold.
    allowed_pack_entries: ["maps/first.ron", "palette.ron"],
    // Paths nothing may hold, as a file, in a pack or as a program's text.
    forbidden_paths: ["maps/final-boss.ron"],
    // Texts nothing may hold: put one in each file of what isn't out yet.
    canaries: ["ACT-THREE-CANARY"],
),
```

A path matches whole, with no pattern, and the part is the same for every
target: list each target's names together. What the exporter writes itself (the
program, `NOTICES.txt`, the web page) needs no line. A file or a pack's file the
part doesn't name, a forbidden path, a canary, or a file beside the package that
this export didn't make refuses the export with `SH3012`. It names each file and
where it came from, and leaves the report and no package, and for the web an
empty `web.dist`. The report and the error name a canary by its place in the
list and never by its text, so they can be shared.

This finds what is written plainly: in a file, in what a pack unpacks to, or as
UTF-8 or UTF-16 in a program. It can't find what your code compresses itself, or
say that a rule isn't in the machine code. What a game must not give away, leave
out of the build.

## Size and speed

A target's budgets are the most frame time, heap and start-up it
allows. `cargo shuyi run --perf` measures each frame headless, and the
engine's perf traces say where a frame's time and memory go; the
`shuyi-perf` skill
([chapter 10](@/learn/book/tools-for-agents.md#the-skills)) walks
through them.
