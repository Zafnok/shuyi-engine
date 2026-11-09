+++
title = "Targets and export"
description = "Each target says what its hardware can do, shuyi check --target holds a game to it, shuyi web builds the browser's page, and shuyi export packages the game for players."
weight = 90

[extra]
part = "chapters"
chapter = 9
+++

Sooner or later you want someone else to play your game. Your friend has a Mac, your
cousin has a Steam Deck with no keyboard, and a stranger only wants to click a link. Each
of those machines can do different things, and the worst time to find that out is after
you've built for it.

Each platform a game can ship on is a **target**, and each target has a **profile**: plain
data that says what its hardware can do. Its display (the sizes it shows, its colors, how
small text may be), its frame rate, its inputs (which buttons, a pointer, a keyboard), its
budgets of time and memory, and the **platform backend** that runs a game there, the part of
the engine that talks to that machine's window, files and sound. A game that needs more than
a target has doesn't export there, and the tools say why before anyone builds for it.

The profiles are the engine's
[`targets/`](https://github.com/Zafnok/shuyi-engine/tree/main/targets) folder: `desktop`
(Windows, Linux and macOS), `web` and `steam-deck` on the macroquad backend, and `playdate`
for its own backend, with phones and the Switch planned.

**Exporting** means making the package a player downloads. The order to work in is: list your
targets, check against them, and export.

## The targets a game ships on

A game lists its targets in its project file, `shuyi.ron`, which is written in RON and read
by the tools. Each has the size of its frame in pixels, how it scales (`integer`, whole
multiples so pixels stay crisp, or `fit`, as big as the window allows) and the height of its
smallest text:

{{ snippet(file="project/shuyi.ron", anchor="targets", language="ron") }}

## Where the game runs

A running game knows its target: `ctx.platform` (`shuyi::ui::PlatformFacts`) holds the
target's id and what its profile says a game may do there. A screen asks it, never a `cfg!`,
Rust's way of asking while compiling, which would make the game a different program on each
machine, with a screen you couldn't test:

- `can_quit()` is false where the player leaves through the system (the web's tab, a console's
  home button): a title offers no Quit there.
- `windowed` is false on a device that is always full screen: the engine's options screen
  leaves out its Fullscreen row.
- `keyboard`, `pointer` and `touch` say what the player has.

The facts are the screens'. A game's rules never read them, so the same commands play the same
game on every target. A test runs a screen as a target with `Harness::as_target`, before the
screen opens:

{{ snippet(file="src/screens.rs", anchor="as_target") }}

{% newcomer(to="rust", topic="looking up by name, `.unwrap()` and `!`") %}
`&profiles["web"].profile` looks up the `web` entry by its name, and lends its
`profile`. `.unwrap()` opens a `Result`, and stops the test on an error. `assert!(!x)` fails
unless `x` is false, as `!` means "not".
{% end %}

## Who the game is

Every target gets the same name, id and version, written once in `shuyi.ron`'s `app`:

{{ snippet(file="project/shuyi.ron", anchor="app", language="ron") }}

- `name` is the name players see, and `id` is **reverse-DNS**, a web address backwards: two or
  more parts between dots, of letters, digits and `-`, each starting with a letter. The player's
  saves are kept under the id (or under `storage`, when the part sets one), so it never changes
  after a release.
- `version` is **semver**, three numbers like `1.4.2` that say how big a change was; without it,
  the crate's version. `build`, a number that only grows, and `publisher` are for the targets
  that ask.
- `cargo shuyi app-info` writes the part into `code` as the game's `APP`, a
  `shuyi::platform::AppInfo` the binary reads; `--check` fails when the file is out of date.
- `icon` is the game's one picture: a square PNG, 512 pixels a side or more. The tools make every
  size a target asks from it, never at run time: `app-info` writes the window's icons, 16, 32 and
  64 pixels, into `window_icons`, and `APP` carries them (`window_icon`), so the window shows it.
  Each size is the mean of the pixels it covers, weighed by how opaque they are, so a transparent
  edge doesn't darken the color beside it, and the same picture gives the same files on every
  computer.
- `icon_sizes` draws window icons by hand, for pixel art that a mean turns to mud: `{ 16:
  "art/icon-16.png", 32: "art/icon-32.png" }`. Each file is a square PNG of its key's side, and
  `app-info` uses its pixels as they are. The sizes it doesn't name still come from `icon`, and so
  do the packages' icons.

## Checking them

```bash
cargo shuyi check --target all
```

checks, for each target the game lists, that:

- its frame at its scale fits the target's display, inside its safe area (the part of a display
  sure to be visible), and its smallest text is as tall as the target asks;
- every screen's required actions bind to distinct inputs the target has
  ([chapter 4](@/learn/book/actions.md)). A target with no keyboard (a Steam Deck) plays with the
  buttons of the keymap file's table named after it, `targets: { "steam-deck": { … } }`; when the
  actions don't fit, the screen changes, since the engine never folds actions onto fewer buttons;
- on a one-bit target, every palette role has a pattern;
- the `app` part keeps its rules, its icon is a square PNG of 512 pixels or more, and the files
  `app-info` writes are up to date. An id under `com.example.` is a warning: two games with it share
  their saves.

Each problem names its target, the setting in `shuyi.ron` and the numbers. `--target <id>` checks
one.

## The web build

The web is the easiest way to hand someone your game: a link, and nothing to install. `cargo shuyi
web` builds the game's binary for the browser (`wasm32-unknown-unknown`, the name of that target to
Rust, which `cargo shuyi doctor` checks rustup has) and writes it with the page it runs in to a
folder. A game made with `shuyi new` has what it needs already. Its `rust-toolchain.toml` lists
that target, so rustup fetches it with the pinned Rust, and its `shuyi.ron` has a `web` part, which
says what to build and where:

{{ snippet(file="project/shuyi.ron", anchor="web", language="ron") }}

Its binary links with `--allow-undefined` on the web, so the page's own scripts (the player's
storage, controllers) are found when it loads. A link argument reaches a binary only from its own
**build script**, a small Rust program Cargo runs before compiling, and the new game has one,
`crates/game/build.rs`:

{{ snippet(file="build.rs", anchor="build") }}

{% newcomer(to="rust", topic="`std::env::var` and a build script's output") %}
`std::env::var(...)` reads an environment variable, which Cargo sets here to the
machine being built for. It gives back a `Result`, and `.as_deref()` lets it be compared with
`Ok("wasm32")` as plain text. `println!` prints a line, and a build script's lines starting
`cargo:` are instructions to Cargo.
{% end %}

Then:

```bash
cargo shuyi web --release
```

writes `dist/web/`: `index.html`, the game's `.wasm` (the compiled game, in the form a browser
runs), the engine's scripts, and `NOTICES.txt`, the licenses of every crate and file the build
carries, with the game's own texts that `shuyi.ron`'s `notices` names. `--release` builds it
optimized, as you would for players. Serve that folder over HTTP and open it in a browser; a page
opened from the disk can't load its `.wasm`. The engine's examples on this site are such folders.

The folder belongs to the build. Each build replaces everything in it, so a file from an older
build can't ship, and keep nothing of your own there: `web.dist` that holds your project or a
repository is refused. A server already running in the folder keeps serving the new build.

## Export

```bash
cargo shuyi export windows --release
```

makes the package players download. It runs the target's check first (`windows`, `linux` and
`macos` all check `desktop`) and builds nothing if the check finds an error. Then it builds the
game for the target and writes into `dist/export/<target>/`:

- `playdate`: a `.pdx` and its zip, for sideloading (installing by hand, with no store): the game's
  library built without `std`, Rust's standard library, its pack baked one-bit, its sounds and music
  baked to ADPCM from what its tools' `GameTools::audio` names. A game adds a `playdate` feature and
  says how it starts with `shuyi_playdate::main!`, where `report = RunReport::new()` makes every run
  say in the console how its start went and how fast it runs ([the Playdate's
  page](https://github.com/Zafnok/shuyi-engine/blob/main/docs/targets/playdate.md#the-run-report));

- `windows`: a zip of the `.exe`;
- `linux`: a `tar.gz` that runs on any Linux with glibc 2.31 or newer;
- `macos`: a folder with one `.app` for Apple silicon and Intel Macs, signed ad hoc;
- `web`: the web build's folder and a zip of it, `index.html` at its root, as itch.io takes it. A
  file in it that the build didn't make is refused by its path.

Each desktop package builds on its own system: Windows on Windows, Linux on Linux, macOS on a Mac
(CI has all three). Every package carries `NOTICES.txt`. The Windows and macOS ones also carry
`HOW-TO-OPEN.txt`: they aren't signed by a developer, so a player opens them once by hand.

With `--release`, the build carries nothing of the machine it was made on: its paths are renamed,
and a program that still holds one is refused. On Windows and Linux, `<package>.debug` beside the
package keeps the program's line tables for crash reports, and is never shipped.
`export-report.json` beside it gives the version, the sizes, the check's warnings and the signing,
and `--run` runs the package.

A game whose build needs a feature, or whose soundtrack ships as loose files, says so in the `export`
part of `shuyi.ron`:

```ron
export: (
    features: ["licensed-art"],
    beside: [(path: "music"), (path: "voice", optional: true)],
),
```

Each desktop and web build turns the features on, and each package holds those files and folders
under their own names (`name` gives another). In a `.app`, a folder goes in `Contents/Resources`,
where the engine's music and voices look for it, and a file goes beside the `.app`. A path that isn't
there refuses the export, unless it is `optional`. `--without <feature>` leaves a feature off one
build, and `--with <feature>` turns on one more that the package has, for a measuring build say.

A game split into packages gives a target its own package and features in `targets`, each replacing
the part's for that target. The Playdate builds the `game` part's package and takes nothing else of
the `export` part, so a game whose Playdate library is a package of its own names it there:

```ron
export: (
    package: "jumper-app",
    targets: {
        "playdate": (package: "jumper-playdate", features: ["demo"]),
    },
),
```

## What a package gives away

A menu that hides a level doesn't hide its file: a player can open a package and read what is in it.
So every export reads its final package back, after every copy its target makes, and writes down what
is there in `exposure-report.json` beside it: each file with its kind, its source, its size and its
SHA-256 (a kind of hash, a fingerprint of the file), each file of each pack, whether the file is a pack
or a program embeds one, the debug file, the features and what it was built from. The same input gives
the same report.

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

A path matches whole, with no pattern, and the part is the same for every target: list each target's
names together. What the exporter writes itself (the program, `NOTICES.txt`, the web page) needs no
line. A file or a pack's file the part doesn't name, a forbidden path, a canary, or a file beside the
package that this export didn't make refuses the export with `SH3012`. It names each file and where it
came from, and leaves the report and no package, and for the web an empty `web.dist`. The report and the
error name a canary by its place in the list and never by its text, so they can be shared.

This finds what is written plainly: in a file, in what a pack unpacks to, or as UTF-8 or UTF-16 in a
program. It can't find what your code compresses itself, or say that a rule isn't in the machine code.
What a game must not give away, leave out of the build.

## Size and speed

A target's budgets are the most frame time, heap (the memory a program asks for as it runs) and start-up
it allows. `cargo shuyi run --perf` measures each frame headless, and the engine's perf traces say where
a frame's time and memory go; the `shuyi-perf` skill
([chapter 10](@/learn/book/tools-for-agents.md#the-skills)) walks through them.

A frame's time changes from one run to the next, so it can't be a test. `cargo shuyi bench` measures
what doesn't: it plays your benchmark scenarios, files named `<name>.bench.ron` with the start, the
steps and the frames to measure, and counts each frame's instructions (with Valgrind, on Linux), the
heap's peak and the package's size. It compares each with the budgets of every target your `shuyi.ron`
lists, and fails naming the scenario, the frame and the target when one is over:

```ron
(
    format_version: 1,
    about: "the stage's map, idle",
    start: [(flag: "--stage")],
    frames: 60,
)
```

Write one for each of your game's heaviest moments: its biggest battle, its fullest menu, its longest
dialogue.

## When a player's game crashes

If the game panics, the engine writes a crash report beside its saves: `crash-<when>.ron` in its
data folder on a computer, and in the browser's storage on the web. It holds the panic's message and
place, the versions, the frame, the screen on top and the log's last lines, never the player's
settings or saves, and it stays on the player's machine. The player sends it to you; the game's
version in it is `shuyi.ron`'s `app.version`.

At the next start the game's load hears of it once, in `Setup::last_crash`: whether the player sees
anything, and in what words, is up to the game. A panic the game survived, like a sound library's on a
machine with no sound card, keeps its report but isn't offered as a crash. A game that gives `Start::crash_record` a function
writing its record of play (the record its `replay` reads) gets that in the report too, on a computer,
so a crash can be replayed to the panic; a game on the short path gives it to `BasicGame::crash_record`.
A game made with `shuyi new --template grid` already does: its report holds the open stage's record,
the command its rules were applying included. To try it, run the game with `SHUYI_PANIC_AT_FRAME=3`: it
panics at its third frame on purpose and exits with code 101.

## When it goes wrong

The point of `check --target` is that these show up before you build anything. These are its
real answers.

**A frame too big for the display.** A game of 384 × 384 pixels, listed for the Playdate, whose
display is 400 × 240. The answer names the target, the setting and the numbers:

```text
check --target playdate: 2 error(s), 1 warning(s) (targets/playdate.ron)
  error: playdate: targets."playdate".frame: frame 384 × 384 at scale 1 is larger than the display 400 × 240
  error: playdate: keymap targets."playdate": the keymap file has no buttons for playdate, which has no keyboard: give it a table in `targets` ("playdate")
```

The second error there is the other common one: a target with no keyboard needs its own table of
buttons in the keymap file.

**A target the game doesn't list.**

```text
check: error[SH3003]: shuyi.ron's `targets` has no "gameboy" (it lists desktop, web): a game lists the targets it ships on
  help: name a target the message lists, or fix the target profiles' files
  see: shuyi explain SH3003
```

**The placeholder id.** It's a warning, so nothing stops you, but fix it before you hand out a build:

```text
  warning: desktop: app.id: "com.example.box-pusher" is a placeholder: two games with this id share their saves on one computer and on one web origin; set your own before you hand out a build, and before the first release, since the save folder follows it
```

**Passing isn't everything.** A check also says what it didn't look at, under `limits`. Read those
lines: a display that takes any size, for instance, says its frame fits whatever you gave it.

```text
limits:
  - desktop's display takes any size: its frame fits, and its text is checked at scale 1
```

**A web page that stays blank.** If you opened `index.html` by double-clicking it, that's the rule
above: the page must be served over HTTP. If `cargo shuyi web` itself fails to build, run `cargo shuyi
doctor`, which checks that rustup has the `wasm32-unknown-unknown` target.
