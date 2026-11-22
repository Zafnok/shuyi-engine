---
id: "3041"
title: "Log sinks in shipped builds: a log file with the last three runs kept, the panic message in it, the browser console on the web, and a SHUYI_LOG filter"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["3040"]
nick_input: none
completed: 2026-11-21
---

# 3041 — Log sinks in shipped builds: a log file and the browser console

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. A game shipped with three of them leaves a log a player
can send:

- Godot writes `user://logs/godot.log` by default and keeps the last
  five runs (`godotengine.org/releases/4.5` adds a game's own `Logger`);
- Unity writes `Player.log` for every build
  (`docs.unity3d.com/Manual/LogFiles.html`);
- Unreal writes a log per run under the project's `Saved/Logs`, with
  categories and verbosity
  (`dev.epicgames.com/documentation/en-us/unreal-engine/logging-in-unreal-engine`);
- Bevy logs to the console through `tracing`, and its `LogPlugin` takes
  a custom layer for a file
  (`docs.rs/bevy/latest/bevy/log/struct.LogPlugin.html`).

What Shuyi has with 3040 done (ADR-0030), lines as of 2026-10-24:

- **Lines with a level and a category.** `App` keeps the engine's lines
  and hands the new ones to the backend (`App::take_new_log`, 3040), and
  `shuyi-app` prints them with macroquad's macros (`print_line`,
  `crates/app/src/lib.rs:355-370`), as it prints its own (`lib.rs:262`,
  `backend.rs:47, 147, 192`, `render.rs:87, 268, 297`,
  `pads/native.rs:43`, `storage/mod.rs:31, 59, 62`). Under those
  macros, miniquad 0.4.11 (with its `log-impl` feature, which macroquad
  0.4.16 always turns on) writes with `eprintln!` on the desktop and
  with `console.debug`, `info`, `warn` and `error` on the web
  (miniquad's `src/log.rs`).
- **No console in a release build on Windows.**
  `examples/hello-grid/src/main.rs:6`,
  `templates/blank/crates/game/src/main.rs:6` and
  `templates/grid/crates/game/src/main.rs:6` set
  `windows_subsystem = "windows"` when `debug_assertions` is off, so a
  release build is a GUI program with no stderr. Every line, and the
  message of a panic, goes nowhere. The release profile also sets
  `strip = true` (the root `Cargo.toml`).
- **No panic hook of ours.** No code in this repository sets one. On the
  desktop, std's default hook writes the message to stderr, which a
  Windows release build doesn't have. On the web, miniquad sets its own
  hook (`src/native/wasm.rs:60`, before the game's code runs). It writes
  the `Debug` form of the panic's info with `console.log`: the file and
  line, but not the message, since the payload prints as `Any { .. }`.
  Then the wasm stops with a `RuntimeError`.
- **Lines nobody reads.** gilrs (controllers) writes to the `log` crate,
  and no logger is installed.

Why it matters. Visions of Shuyi is getting ready for Steam
(game#0903) as a Windows release build. The first player whose game
misbehaves has nothing to send, and its developer nothing to read. The
same holds for every game `shuyi export` (3004, done) packages.

## Nick input

None.

## Scope

**In:**
- **The `log` crate in `shuyi-app`**, as 3040's ADR says: `log = "0.4"`
  with its default features (none). Its license is MIT OR Apache-2.0,
  both in `deny.toml`'s `allow`, and it is already in `Cargo.lock` at
  0.4.34 through gilrs, so no second version comes in.
  - **macroquad's `log-rs` feature** is turned on in
    `crates/app/Cargo.toml` (`macroquad = { workspace = true, features =
    ["log-rs"] }`). Then `macroquad::logging` is the `log` crate
    (macroquad's `src/lib.rs:139-146`), so macroquad's own messages (8
    calls in 0.4.16, `src/quad_gl.rs` and `src/window.rs`) reach the
    sinks. Cargo's feature unification turns it on for every game that
    depends on `shuyi-app`. Two things change for a game, and the PR
    says both for the game's pin bump:
    - its own macroquad `warn!` and `error!` reach the sinks too;
    - macroquad's prelude re-exports the `log` crate's items
      (`pub use crate::logging::*`, `src/prelude.rs:25`): `Level`,
      `Record`, `Log`, `log!` and the rest. A name that another glob
      import also brings in becomes ambiguous where it is used.

    miniquad's own messages are plain `eprintln!` and `println!` calls
    (about 20 in 0.4.11, such as `src/graphics/gl.rs:1732`), which no
    logger sees: they stay on stderr, and the docs say so.
  - **Every line `shuyi-app` writes** uses `log::…!(target: "<name>",
    …)` with 3040's `LogCategory` names: `lib.rs:262` and
    `backend.rs:47` (`audio`); `backend.rs:192` and `render.rs:87, 268,
    297` (`assets`); `pads/native.rs:43` (`input`);
    `storage/mod.rs:31, 59, 62` (`storage`); the broken renderer's line
    3040 moved to `backend.rs:147` (`platform`). Also every other
    macroquad log call in `crates/app/src` when this ticket is worked
    (`rg 'warn!|error!|info!|debug!' crates/app/src`): 2019, 2519, 2521,
    2522 and 3044 may have added some, 3042 added the debug line of each
    system event in `backend.rs` (`Macroquad::system_events`), and 3045 added
    `log_warning` in `storage/native.rs` (`storage`), the warning of a
    read that falls back to a save's backup. They name the `log`
    crate, not macroquad's prelude, so they outlive macroquad (3034).
    **From 2019 (done):** the capture mode's lines
    (`crates/app/src/capture.rs`, and the `render_card` example's) are
    macroquad `info!` and `error!` calls starting `shuyi-capture: `; they
    move with the others, under the target `platform`. CI's
    `tools/web-check/check.py` finds them anywhere in a console line and
    fails on any console error, so a sink's prefix changes nothing there.
  - **App's lines:** `print_line` (`lib.rs:355-370`) hands each line of
    `app.take_new_log()` to `log` with its category's name as the target
    and its level mapped (`LogLevel::Error` to `log::Level::Error`, and
    so on to `Debug`), in place of macroquad's macros.
- **The sink**, a public module `shuyi_app::log_sink`
  (`crates/app/src/log_sink.rs`):
  - `install(storage_name: &str, dir: Option<PathBuf>, filter:
    Option<&str>) -> LogFilter` builds a `log::Log`, leaks it into a
    `&'static` and calls `log::set_logger`. `run` calls it first thing,
    with `storage::log_dir(config.storage_name)` and `SHUYI_LOG` on the
    desktop, and `None` and `None` on the web. It returns the filter in
    use, which `run` hands to `App::set_log_filter` (3040), so App's ring
    keeps the same lines. If a logger was installed before (a game's),
    `set_logger` fails: that logger stays, and one line on stderr says
    so.
  - **The frame:** a static `AtomicU32` that `log_sink::set_frame(n)`
    sets. The loop sets it to `app.frame_number() + 1` before each
    `app.frame`, the frame about to run, as 3046 reads it. So a line the
    backend logs during frame n, App's lines handed on after frame n,
    and a panic in frame n all carry n. Lines before the first frame
    carry 0.
  - **The line:** `{seconds:>9.3} f{frame} {LEVEL:<5} {target}: {text}`.
    The seconds count from `install`, read with `pacing::seconds()`
    (a backend may read the clock; it is an `Instant` on native, which
    never jumps as `miniquad::date::now()` can). `pacing::seconds()`
    is the loop's one clock (the pacer, `App::frame`'s time and the perf
    clock; 3067, done): nothing more to do.
  - **The filter.** `LogFilter::parse(&str) -> Result<LogFilter, String>`
    in `shuyi_ui::log` (pure, `no_std`), over 3040's `LogFilter`:
    - it reads `info`, `warn,audio=debug` and `gilrs=warn`: items split
      by commas, spaces trimmed. Levels are `error`, `warn`, `info` and
      `debug`. Anything else is refused with a reason;
    - a level alone sets the engine's categories. `name=level` sets one
      category, or, for a name that is no category, every other `log`
      target that starts with it (`gilrs` covers `gilrs_core::…` too);
    - `LogFilter` gains what that needs: a level for other targets
      (default `Warn`), a list of `(prefix, level)`, and
      `allows_target(&str, LogLevel)`, which uses the category's level
      when the target is a category's name. `allows(category, level)`
      stays as 3040 made it. `log`'s `Trace` counts as `Debug`;
    - with no filter (unset or empty): `Info` and up for the engine's
      categories, `Warn` and up for other targets. A bad value is
      ignored, with one `Warn` line, target `platform`, that says why;
    - the sink checks it in `enabled`, and `log::set_max_level` gets its
      most verbose level.
  - **Where the filter comes from:** the `SHUYI_LOG` environment
    variable on the desktop. Nothing on the web.
- **On the desktop** (not `wasm32`):
  - **The file**, `<data dir>/logs/latest.log`. The data dir is
    `data_dir(AppConfig::storage_name)` (`crates/app/src/storage/native.rs:47`,
    `crates/app/src/lib.rs:176`). `FileStorage::list` names only `.ron`
    files at the top of that folder, so `logs/` never shows up as a save.
    A public `shuyi_app::storage::log_dir(name) -> Option<PathBuf>`
    (`data_dir(name)` joined with `logs`) names the folder, for `run`,
    the docs (`docs/targets/desktop.md` among them) and 3046.
    - **Rotation at start:** `previous-2.log` becomes `previous-3.log`,
      `previous-1.log` becomes `previous-2.log`, `latest.log` becomes
      `previous-1.log`, and the old `previous-3.log` is deleted. The last
      three runs stay beside this one.
    - **The first line:** the engine's version (`shuyi-app`'s
      `CARGO_PKG_VERSION`, which every `shuyi-*` crate shares), the OS
      and architecture (`std::env::consts`), debug or release, and the
      game's storage name. **From 3046 (done):** the game's version,
      `AppConfig::crash.game_version`, goes in the line too.
    - **A cap:** 2 MiB per run (`LOG_FILE_CAP`). Past it, one line says
      the log was cut, and after that only `Error` lines go in (a panic's
      line is one), up to 256 KiB more. A loop of warnings never fills a
      disk.
    - **Each line is written whole**, with one `write_all` to a file
      opened to append and no buffer, so a crash loses nothing written
      before it. Lines are few.
    - **If the folder can't be made or the file opened** (or `log_dir`
      gives `None`), the file sink is off, one line goes to stderr, and
      the game runs on.
  - **stderr** in debug builds, and in release builds when `SHUYI_LOG` is
    set (a developer at a terminal). The sink writes with `writeln!` on
    `std::io::stderr()` and ignores the result, never with `eprintln!`,
    which panics when the write fails. A Windows release build has no
    stderr, and the write goes nowhere.
- **On the web** (`wasm32`): the browser's console, by level, through
  miniquad's macros (`miniquad::error!` to `miniquad::debug!`, reached
  as `macroquad::miniquad`). They call `console.error` to
  `console.debug` through the functions in the vendored
  `crates/cli/web/mq_js_bundle.js` (miniquad's `js/gl.js`). No new JS.
  The level's macro comes from a pure function, so it is tested on the
  desktop. miniquad panics on a NUL in a message (`src/log.rs`,
  `CString::new(…).unwrap_or_else(|_| panic!())`), so the sink replaces
  each `\0` with `U+FFFD` first.
- **The panic hook,** set by `install` with `std::panic::set_hook`, once
  per process (a `std::sync::Once`), chaining the hook that was there
  (`std::panic::take_hook`):
  - it writes one line with `log::error!(target: "panic", …)`: the
    message (the payload as `&str` or `String`, else `Box<dyn Any>`), its
    `file:line`, and the thread's name (`std::thread::current().name()`,
    `unnamed` without one). So it lands in `latest.log` on the desktop
    and in the console on the web, or in a game's own logger if one was
    installed first;
  - a panic inside the sink never deadlocks: the sink sets a
    thread-local flag while it holds its lock, and a line logged while
    that flag is set on the same thread is dropped. Another thread
    waits for the lock as usual. A poisoned lock is used anyway
    (`PoisonError::into_inner`), never unwrapped;
  - a panic that `catch_unwind` catches still runs the hook, such as the
    music decoder's on a corrupt file (`audio/native_music.rs:97`), so
    it is logged too. That stays: the line is true, and 3046's
    `crash::quiet` keeps it out of crash reports;
  - then the earlier hook runs, as before. On the web that is
    miniquad's, so its `console.log` line still follows ours.
  - **From 3046 (done):** `shuyi_app::crash::install` sets one hook
    already, once per process, chaining the one before it. Add the log
    line to that hook rather than setting a second; feed its log ring
    (`crash::note_log`) from the sink in place of `run`'s lines; have
    its hook read `set_frame`'s counter in place of its own `FRAME`
    static (`crash::set_frame`); and add to `docs/crash-reports.md`
    that the log files sit beside the reports (`storage::log_dir`) and
    that a player sends both. One hook, chained.
- **Docs:** a *Logs* section in `docs/platform-backends.md`: what `App`
  hands over, how `shuyi-app` sinks it, where the file is on Windows,
  macOS and Linux, `SHUYI_LOG`'s syntax, that miniquad's own messages
  stay on stderr, that a device's backend (3010) sends lines to the
  device's own log, and that a log holds file paths, which may hold the
  player's account name, so a game asks before it sends one anywhere
  (sending is parked 3047).
  **Either order with 3012** (the Steam Deck): if 3012 is done,
  `docs/targets/steam-deck.md` also says that the log files land in
  the `logs` folder (`storage::log_dir`) of the data folder it names;
  if not, nothing to do (3012 adds that line, its scope says so).
  **From 3004 (done):** `docs/targets/desktop.md` (what the `windows`,
  `linux` and `macos` packages run on, and how to make them) has no
  section on the log file. This ticket adds one: where
  `shuyi_app::storage::log_dir` puts `latest.log` and the previous
  runs' files on Windows, macOS and Linux for an exported game, so a
  developer can tell a player what to send, with a link to
  `docs/platform-backends.md`'s *Logs*.
- **Size:** if the PR passes about 600 changed lines, `SHUYI_LOG` and
  `LogFilter::parse`, with their tests and criteria, go to a follow-up
  ticket (the `write-ticket` skill, After MVP, blocked by this one), and
  the sink keeps the default levels until it lands.

**Out (do not do):**
- a debug-menu row for the filter, and a filter in the web page's
  address (follow-ups if a developer asks);
- crash reports, symbols and sending anything anywhere (3046, 2520, and
  parked 3047);
- timing spans (2325, parked);
- the Playdate's sink (3010) and its opt-in run report (3090);
- a log file on the web (`localStorage` is for saves);
- sending the web build's console lines to a terminal (2528);
- miniquad's own `eprintln!` messages (they leave with macroquad, 3034);
- logs of the `shuyi` tool itself.

## Implementation steps

1. Read 3040's ADR and `shuyi_ui::log`; `crates/app/src/lib.rs`,
   `backend.rs` and `storage/`; macroquad 0.4.16's `src/lib.rs:139-146`
   (its `logging` module) and `src/prelude.rs`; miniquad 0.4.11's
   `src/log.rs` and `src/native/wasm.rs:60`, and 3046's
   `crates/app/src/crash.rs`. Check whether 3012 is done.
2. `LogFilter::parse`, the other-target levels and `allows_target` in
   `shuyi-ui`, with their unit tests.
3. `log_sink.rs`: the line, the filter check, the rotation, the cap, the
   first line and the web's level mapping, unit-tested on a temporary
   folder (`tempfile`, already a dev-dependency). Keep the file sink a
   plain struct the tests build without `install`, since the logger and
   the hook are global.
4. `install`, `set_frame` and the panic hook; `storage::log_dir`. Call
   `install` first thing in `run`, `App::set_log_filter` after
   `App::new`, and `set_frame` before each `app.frame`. Move the call
   sites to `log::…!`, and turn on `log-rs`.
5. The integration test `crates/app/tests/log_sink.rs`: its own binary,
   so its own process, with one `#[test]`, since the logger and the hook
   are global. It sets a hook that flips an `AtomicBool`, installs into
   temporary folder A, installs again into folder B, logs a `Warn` line,
   then panics on a spawned thread with a known message and joins it.
6. The docs. Run the gates.
7. The web build: `cargo shuyi web` in `examples/hello-grid`, served by
   any static file server. In the browser's console, set
   `localStorage["shuyi-hello-grid/settings"] = "not ron"` and reload:
   the `storage` warning shows with `console.warn`. Add a `panic!` to
   hello-grid for one build, never committed, and see its message with
   `console.error`. (In a browser pane that runs no frames while hidden,
   call `wasm_exports.frame()` from the console.)
8. Build hello-grid in release on Windows (`cargo build --release -p
   hello-grid`), write `not ron` into
   `%APPDATA%\shuyi-hello-grid\settings.ron`, run it, close it, and
   paste `latest.log` into the completion notes.

## Acceptance criteria

- [x] On a temporary folder, a start with 0, 1, 3 and 4 earlier logs
      leaves `latest.log` and at most `previous-1.log` to
      `previous-3.log`, oldest last (test).
- [x] Past the cap, a `Warn` line is not written and an `Error` line is,
      up to the second limit (test).
- [x] ~~`LogFilter::parse` takes `info`, `warn,audio=debug` and
      `gilrs=warn`, and refuses `loud` and `audio=loud` with a reason
      (test).~~ (not done: moved to 3324 by the **Size** rule)
- [x] With no filter, a `Debug` line with target `gilrs` is not allowed
      and an `Info` line with target `audio` is (test).
- [x] A line formatted after `set_frame(4)` reads `f4`, with its level
      and target in place (test).
- [x] The web's level mapping gives each level's macro, and a line with
      a NUL comes out without one (test, run on the desktop).
- [x] In `crates/app/tests/log_sink.rs`: folder A's `latest.log` has the
      `Warn` line, and a `panic` line with the message, the test file's
      `file:line` and the thread's name; folder B's has neither; the
      earlier hook ran.
- [x] Every call `rg -n 'warn!|error!|info!|debug!' crates/app/src`
      finds is a `log::` call, or a `miniquad::` call in the web sink
      (the output in the PR description).
- [x] `cargo deny check` passes, and `Cargo.lock` has one version of
      `log`.
- [x] The PR says, for the game's pin bump, that `log-rs` is on for
      every game and what that changes (its macroquad lines reach the
      sinks; macroquad's prelude re-exports the `log` crate).
- [x] In a browser, hello-grid's web build shows the settings warning in
      the console with its category, and a panic's message (the
      completion notes say how it was seen).
- [x] A Windows release build of hello-grid writes `latest.log` under
      `%APPDATA%\shuyi-hello-grid\logs` with the settings warning (the
      completion notes have the file).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `LogFilter::parse` and `allows_target`; the line's format and
  frame; `enabled` per target; the rotation; the cap; the first line;
  each level's console macro on the web and the NUL replacement (pure
  functions).
- Integration: `crates/app/tests/log_sink.rs` (installing twice, a
  `Warn` line, a panic on a thread, the earlier hook).
- Manual: the browser's console and the Windows release build, in the
  completion notes.

## Completion notes

What was done:

- `shuyi_app::log_sink` (`crates/app/src/log_sink.rs`): a `log::Log` that
  `run` installs first thing, inside the window, so on the web the hook
  chains miniquad's. Its line is `{seconds:>9.3} f{frame} {LEVEL:<5}
  {target}: {text}`; it goes to `latest.log` on a desktop (rotation of
  three earlier runs, the first line, the 2 MiB cap and 256 KiB of errors
  after it, one unbuffered `write_all` a line), to stderr in a debug build
  (`writeln!` on `io::stderr()`, result ignored), to the console by level
  on the web (a NUL replaced first), and to the crash report's ring
  (`crash::note_log`, in place of `run`'s own lines). A thread-local flag
  drops a line logged from inside the sink; a poisoned lock is used
  anyway.
- One panic hook: `crash::hook_once` (set once a process, chaining the one
  before) logs the panic's line (`target: "panic"`: the message, else
  `Box<dyn Any>`, its `file:line` and the thread's name), then writes the
  report unless `crash::quiet`, then calls the earlier hook. Both
  `crash::install` and `log_sink::install` call it, so the line is logged
  with crash reports off too. `crash::set_frame` stores into
  `log_sink::set_frame`'s counter; crash's own `FRAME` is gone.
- `shuyi_ui::log::LogFilter` gains the other targets' level (`Warn` by
  default, `with_others`), `allows_target` and `most_detailed`.
- `storage::log_dir(name, desktop)`, and `StorageAt::log_dir` for `run`.
- `log = "0.4"` and macroquad's `log-rs` in `crates/app/Cargo.toml`
  (`Cargo.lock` keeps the one `log`, 0.4.34). Every log call in
  `crates/app/src` is a `log::…!(target: …)` call, the prefix a target now
  says (`audio: `, `storage: `) dropped from the text. `rg -n
  'warn!|error!|info!|debug!' crates/app/src`, less the `log::` calls,
  finds only the web sink's four:

  ```text
  crates/app/src\log_sink.rs:318:            miniquad::error!("{}", text);
  crates/app/src\log_sink.rs:321:            miniquad::warn!("{}", text);
  crates/app/src\log_sink.rs:324:            miniquad::info!("{}", text);
  crates/app/src\log_sink.rs:327:            miniquad::debug!("{}", text);
  ```

- The render card installs the sink (no file), so its `shuyi-capture: `
  lines still reach the web check's console. The sound probe's web line
  calls `miniquad::info!` itself: it installs no logger, and macroquad's
  `logging` is the `log` crate now.
- Docs: *Logs* in `docs/platform-backends.md` (and its *Panics*), *The
  log file* in `docs/targets/desktop.md`, the logs beside the reports in
  `docs/crash-reports.md`, and the book's *When a player's game crashes*.

Deviations:

- **The Size rule.** The PR measured 1,361 counted lines with `SHUYI_LOG`;
  as the ticket says, `SHUYI_LOG`, `LogFilter::parse` (with the prefixes
  of other targets) and their tests and criterion went to 3324, and the
  sink keeps the default levels. `install` takes no filter yet and
  returns nothing; stderr is a debug build's alone. Without them the PR
  is 1,226 counted lines; past 1,000, the clean split would have been
  the call sites' move to `log::` (about 120 lines) from the sink.
- **ADR-0094 §4 keeps holding.** A build without debug tools passes
  warnings and errors alone to every sink (`kept_in_this_build`, moved
  from `lib.rs` into `log_sink.rs`), as `run` printed before; the
  ticket's `Info` default applies to builds with debug tools, and to a
  build with the capture mode (the `capture` feature, never shipped),
  whose `shuyi-capture: ` lines are `Info` and read by CI's web check
  (`KEEPS_EVERY_LEVEL`: CI's `web-render` found it).
- **`log_dir` takes the game's `DesktopFolder`** (`log_dir(name,
  desktop)`): 3310 (done) let a game keep its storage in `My Games`, and
  the logs sit beside the saves and reports. `run` uses the folder
  `StorageAt::find` settled on (`StorageAt::log_dir`).
- **The storage move's lines are kept until the sink is up.**
  `StorageAt::find` runs before the window, so `storage/desktop.rs`
  keeps its move's lines and `run` logs them after `install`
  (`storage::take_move_lines`). Installing first instead would open
  `logs/latest.log` in the new folder before the move, whose `create_new`
  copy of an old `logs/latest.log` would then fail it.
- **`install` takes the game's version** (`AppConfig::crash.game_version`)
  for the first line, as 3046's clause asks.
- **Without a data folder** the missing file is a `platform` warning
  through the sink rather than a raw stderr line, so it reaches the
  console and a report too.
- **The settings key is `p1-settings`** now (profiles), not `settings`:
  the manual checks broke `p1-settings.ron` and
  `localStorage["shuyi-hello-grid/p1-settings"]`.
- **3012 is open:** nothing to do for `docs/targets/steam-deck.md`; 3012's
  clause is rewritten as **From 3041 (done)**: it adds the line.

Manual evidence, at `50472bbc` (the tip of `main`, which the branch is
on):

- **Windows release build** (`cargo build --release -p hello-grid`), with
  `not ron` in `%APPDATA%\shuyi-hello-grid\p1-settings.ron`, run and
  closed. `logs\latest.log`:

  ```text
      0.000 f0 INFO  platform: Shuyi Engine 0.1.0, windows x86_64 release, storage shuyi-hello-grid, game version 0.1.0
      0.021 f2 WARN  storage: settings: unreadable: 1:1-1:4: Expected struct `Settings` but found `not`, using the defaults
  ```

  A second run moved it to `previous-1.log` and wrote a new `latest.log`.
  The folder was put back as it was afterwards.
- **The web build** (`cargo shuyi web` in `examples/hello-grid`, served by
  `python -m http.server`, frames driven with `wasm_exports.frame()` in a
  hidden browser pane). With `localStorage["shuyi-hello-grid/p1-settings"]
  = "not ron"`, the console had `[warn]     0.140 f2 WARN  storage:
  settings: unreadable: …, using the defaults`. With a `panic!` in
  hello-grid's load for one build (not committed): `[error]     0.079 f2
  ERROR panic: a panic on purpose for 3041's check at
  examples\hello-grid\src\lib.rs:660 on the thread main`, then miniquad's
  `[log] PanicHookInfo { payload: Any { .. }, … }` and the `RuntimeError`.

Tickets written:

- 3324: `SHUYI_LOG` and `LogFilter::parse` (the Size rule).
- No ticket: a debug-menu row for the filter and a filter in the web
  page's address, until a developer asks (the ticket's **Out**).

For the game's pin bump: macroquad's `log-rs` feature is on for every
game that depends on `shuyi-app` (Cargo unifies features). The game's own
macroquad `warn!` and `error!` lines now reach the engine's sinks (its
log file, the console and its crash reports) with the game's module path
as their target, `warn` and more serious by default. Macroquad's prelude
re-exports the `log` crate's items, so a name another glob import brings
in too becomes ambiguous where it is used. `gh search code --repo
Zafnok/visions-of-shuyi 'macroquad::prelude::*'` finds nothing; the game
has one `macroquad::logging::warn!` (`crates/app/src/main.rs:115`, a
debug-only folder read), which compiles as before.

Breaking: shuyi-app: macroquad's `log-rs` feature: `macroquad::prelude::*` brings the `log` crate's items (`Level`, `Log`, `Record`, `log!`, …), so a name another glob import brings too is ambiguous where it is used

```migration
since: 0.1.0
what: macroquad's prelude re-exports the `log` crate's items, so a game that glob-imports it beside another module with a `Level` or `Record` names the one it means.
old: "use macroquad::prelude::*; use my_game::stats::*;"
new: "use macroquad::prelude::*; use my_game::stats::{self, Level};"
before: |
  use macroquad::prelude::*;
  use my_game::stats::*;
  let level: Level = Level::new(3);
after: |
  use macroquad::prelude::*;
  use my_game::stats::{self, Level};
  let level: Level = Level::new(3);
find: ["use my_game::stats::*;"]
```

Behaviour: shuyi-app: a desktop build writes `logs/latest.log` (and keeps `previous-1.log` to `previous-3.log`) in the game's storage folder
Behaviour: shuyi-app: the log's lines on stderr and in the browser's console read `{seconds} f{frame} {LEVEL} {target}: {text}` in place of `{category}: {text}`; App's lines take their category's name as the target
Behaviour: shuyi-app: a game's macroquad log lines (and any `log` line) reach the engine's sinks once `run_app` installs its logger, `warn` and more serious for a target that names no category; one logged before `run_app` goes nowhere, and a game that installs a `log` logger before `run_app` keeps it
Behaviour: shuyi-app: a panic is logged as an `Error` line of the target `panic` (message, `file:line`, thread) whether crash reports are on or not, and a panic the engine catches on purpose (a music file that won't decode) is logged too
Behaviour: shuyi-app: a crash report's log lines are every line the sink passes (the backend's and libraries' too, by their target), not App's alone
Behaviour: shuyi-app: the backend's `audio: ` and `storage: ` prefixes are now the lines' targets; miniquad's own messages stay plain stderr lines

Game: game#0903 adopts it with its next pin bump (the Steam build's log file)
