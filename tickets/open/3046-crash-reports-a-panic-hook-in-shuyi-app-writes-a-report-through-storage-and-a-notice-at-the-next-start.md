---
id: "3046"
title: "Crash reports: a panic hook in shuyi-app writes a report through the game's storage, with the game's record of its play, and the game hears of it at the next start"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3040", "2511"]
nick_input: none
completed:
---

# 3046 — Crash reports: a panic hook, and a notice at the next start

## Context

Found by the 2026-10-20 comparison of Shuyi with Bevy 0.20, Godot 4.7,
Unity 6, Unreal Engine 5.8 and the 2D engines GameMaker, Defold and
Construct 3. These keep something when a shipped game fails:

- Unreal: the Crash Reporter Client gathers the call stack and the log
  of a crash and can send them on
  (<https://dev.epicgames.com/documentation/en-us/unreal-engine/crash-reporting-in-unreal-engine>).
- Unity: its Diagnostics collect crashes and exceptions with stack
  traces and logs.
- Godot 4.5 and later: backtraces in release builds, and a game's own
  `Logger` hears errors.
- Defold: `crash.load_previous()` reads the last crash's backtrace at the
  next start, so the game can act on it.
- Bevy: no crash reporter. Its fallback error handler takes the errors
  systems return, not panics.

What Shuyi has today (read 2026-10-21):

- **No panic hook.** Nothing in the workspace calls `set_hook` or
  `take_hook`. The only `catch_unwind` in shipped code is around one
  music decode (`crates/app/src/audio/native_music.rs`, line 97), which
  turns a corrupt file into an error.
- **A release build leaves nothing.** The release profile strips the
  binary (`strip = true`: the root `Cargo.toml`, line 47, and both
  templates' `Cargo.toml`). It doesn't set `panic = "abort"`, so a
  panic unwinds. A Windows release build has no console
  (`windows_subsystem = "windows"` in `examples/hello-grid/src/main.rs`,
  line 6, and in both templates' `crates/game/src/main.rs`). So a panic
  closes the window, and std's message goes to a console that isn't
  there.
- **The web.** miniquad's own hook writes the panic to the browser's
  console (`run` in miniquad 0.4.11's `src/native/wasm.rs`, line 60),
  and the canvas stops. On wasm32 a panic aborts, so nothing runs after
  the hook.
- **What a report is built from exists:**
  - `Storage` (`crates/ui/src/storage.rs`), with `FileStorage` (one
    `<key>.ron` file per key in the OS data folder) and `WebStorage`
    (`localStorage`) in `crates/app/src/storage/`;
  - `Game::top_screen` (`crates/ui/src/game.rs`, line 472);
  - the loop in `shuyi_app::run` (`crates/app/src/lib.rs`, line 148),
    which calls `app.frame` once per frame (line 165);
  - `Setup` and `App::load` (`crates/platform/src/app.rs`, lines 117
    and 475), which hand the game its storage as it loads. `Setup` is
    `#[non_exhaustive]`; `Start` (line 169) is not;
  - a turn game's record: the grid template's and hello-grid's
    `StageRecord` (a seed and the commands,
    `templates/grid/crates/game/src/tools.rs` and
    `examples/hello-grid/src/tools.rs`), which `shuyi replay` re-runs.
    Both `tools.rs` files are behind the `tools` feature, so the shipped
    binary doesn't have `StageRecord` today.
- **`shuyi-platform`'s dependencies** are `shuyi-assets` and `shuyi-ui`
  only (ADR-0024 §1, `crates/platform/Cargo.toml`). `shuyi-ui` has
  `serde` and `ron` from the workspace.
- **Plans:**
  - 3040 (MVP) gives the engine one log: `App` counts its frames
    (`App::frame_number`) and hands the backend its new lines
    (`App::take_new_log`);
  - 2511 (MVP) moves the blank template onto `shuyi_app::run_game`,
    which builds the `AppConfig` from a `BasicGame`. Its Out leaves the
    crash hook in `run_game` to this ticket;
  - 2013 made `AppConfig` `#[non_exhaustive]` with `AppConfig::new`
    (ADR-0039); 2015 replaced CI's `platform-deps` job with
    `cargo xtask check-layers` over `tools/layers.ron`;
  - 3041 writes that log to a file in shipped builds, and sets a panic
    hook that writes the panic's line into it;
  - 2518 and 2519 record sessions at the platform boundary, in debug
    builds;
  - 2520 reads a report and replays its record; 3047 (parked) sends
    reports.

Why it matters:

- **The game first.** Visions of Shuyi is sold on Steam and plays on the
  Steam Deck (game#0903, 3012). Today a player whose game crashes has
  nothing to send its developer. With this, the player's data folder
  holds the message, the place, the versions, the screen, the recent
  log and the record of the play.
- **Other games** on `shuyi-app` get the same by passing their version.
- **The end goal** (2026-10-19, `docs/decisions.md`): an agent given a
  report reproduces the panic with 2520's `replay --crash`, fixes it,
  and checks the fix, without asking the player anything. No benchmark
  does that out of the box.

## Nick input

None. Reports stay on the player's machine. Whether they may be sent
anywhere is 3047's question for Nick.

## Scope

**In:**
- **`shuyi_platform::crash`** (`crates/platform/src/crash.rs`), `no_std`
  with `alloc`. `serde` and `ron` become dependencies of
  `shuyi-platform` from the workspace, as `shuyi-ui` has them, so no
  new crate enters `Cargo.lock`. The ADR below amends ADR-0024 §1 to
  allow them. `tools/layers.ron`: `shuyi-platform`'s `Only` list gains
  `serde` and `ron`, each with a comment saying what for. The
  property tests need `proptest` as a dev-dependency (workspace):
  - `CrashReport`, plain data, `#[non_exhaustive]` (2520 adds fields):
    - `version: u32`, 1;
    - `when: u64`, milliseconds since 1970 from the backend's clock;
    - `message: String`, cut at 4 KiB; `location: Option<String>`
      (`file:line:column`); `thread: Option<String>`;
    - `backtrace: String`, std's text, cut at 32 KiB. A stripped build
      gives `<unknown>` frames; 2520 adds frames a debug file can name;
    - `engine: String` (`env!("CARGO_PKG_VERSION")` of
      `shuyi-platform`; every `shuyi-*` crate has the workspace's),
      `game: String` (the game's version word, empty if the game gave
      none), `target: String` (`std::env::consts::OS` and `ARCH`, and
      `debug` or `release`);
    - `frame: u32`, the frame that was running, counted as
      `App::frame_number` counts (3040); `top_screen: Option<String>`;
    - `log: Vec<String>`, the last 200 lines, each cut at 1 KiB;
    - `record: Option<String>`, the game's record of its play in its
      own record format (below);
    - every field but `version` and `message` is `#[serde(default)]`,
      so 2520 can add fields and older reports still read.
  - keys: a report is stored at `crash-<when>` (`crash_key(when)`), a
    key `is_valid_key` accepts. `report_keys(&dyn Storage)` lists the
    keys that are `crash-` and digits only, newest first, and ignores
    every other key;
  - `CrashReport::to_text()`, and `CrashReport::parse(&str) ->
    Result<Self, String>`, which refuses text over 2 MiB and never
    panics. A report is a record in storage, like the settings and the
    profiles, not an asset kind a game ships, so it is not registered in
    `Formats` (ADR-0019). Its reader still follows ADR-0007 §5: a size
    limit, errors and never a panic;
  - `keep_newest(&mut dyn Storage, n)` deletes all but the newest `n`
    reports.
- **The next start, for every backend:**
  - `Setup` gains `last_crash: Option<LastCrash>` (`when`, `message`,
    `location`, `key`). `Setup` is `#[non_exhaustive]`, so this is no
    break. `LastCrash` is `#[non_exhaustive]` too (3047 adds a field);
  - `App::load` fills it from the storage it hands the game: the newest
    report newer than the one named at the key `crash-offered`, which it
    then sets to that report's key. So each report is offered once;
  - the game decides what its player sees: a line on its title screen,
    a screen of its own, or nothing. The words are the game's
    (`AGENTS.md` rule 4).
- **The game's record:** `Start::crash_record(hook)`, a builder like
  `Start::on_system`, with `type CrashRecord<G> = fn(&Game<G>) ->
  Option<String>`: the game's record of its play as text, in the format
  its tools' `replay` reads. It is stored in a new public field
  `Start::crash_record: Option<CrashRecord<G>>`, which `Start::new`
  sets to `None`. `App::crash_record(&self) -> Option<String>` calls it
  on the running game, and gives `None` while loading or with no hook.
- **The hook in `shuyi-app`** (`crates/app/src/crash.rs`, `pub mod
  crash`):
  - `CrashReports`, `#[non_exhaustive]`, with public fields
    `game_version: &'static str` and `keep: u32`.
    `CrashReports::new(game_version)` keeps 5 reports on native and 1
    on the web, where `localStorage` holds about 5 MB for the whole
    site. `CrashReports::off()` has `keep: 0` and installs nothing.
    `Default` is `CrashReports::new("")`;
  - `AppConfig` gains `crash: CrashReports`. `AppConfig` is
    `#[non_exhaustive]` with `AppConfig::new` (ADR-0039). `crash` is a
    public field that `AppConfig::new` sets to `CrashReports::new("")`,
    and a game changes it by assigning the field (ADR-0039 gives
    `with_*` setters only to types games build in `const` items), so
    the change is additive. `AppConfig::new` is a `const fn` (2013) and
    stays one: dropping `const` is a break the `api` job reports. So
    `CrashReports::new` and `CrashReports::off` are `const fn` too
    (`cfg!(target_arch = "wasm32")` picks 1 or 5), and `AppConfig::new`
    can't call `CrashReports::default()`, a trait method;
  - `crash::install(reports, storage: impl Fn() -> Box<dyn Storage> +
    Send + Sync + 'static)`, which `run` calls before its first frame
    with `|| storage::platform(storage_name)` (making a storage reads
    no file). Tests pass a storage they share with the hook;
  - the hook it installs: `std::panic::take_hook` first. On a panic it
    builds the report, writes it through a new storage, keeps the
    newest `keep`, then calls the hook it took, so std's message in a
    terminal and miniquad's console line on the web stay. If a report
    already sits at `crash-<when>` (two panics in one millisecond),
    `when` goes up by one until the key is free. A write that fails is
    dropped: the hook never panics;
  - what it reads, set by `run` before each `app.frame` through
    `crash::set_frame(frame: u32, top_screen: Option<&'static str>)`:
    the frame about to run (`app.frame_number() + 1`) in an
    `AtomicU32`, and the top screen in a `Mutex<Option<&'static str>>`
    read with `try_lock`, so a panic while it is held still writes a
    report. Statics, not thread-locals, so a panic on the music worker
    thread gets them too. If 3041 is done, the hook reads its sink's
    frame counter (`log_sink::set_frame` holds the frame about to run
    too), not a counter of its own. The thread's name comes from
    `std::thread::current()`;
  - **the log:** `crash::note_log(line: String)` adds a line to a ring of
    the last 200, behind a `Mutex` the hook reads with `try_lock`. `run`
    passes each line of `app.take_new_log()` after each frame (3040) as
    `f{frame} {level} {category}: {text}`, with the level's and the
    category's `name()`. If 3041 is done, its sink feeds the ring
    instead, so macroquad's lines and other crates' are in it too;
  - **a panic caught on purpose is not a crash:** `crash::quiet(f)`
    runs `f` with reports off on this thread (a thread-local flag).
    `native_music.rs`'s `catch_unwind` around `Sound::load` (line 97)
    runs inside it, so a music file that fails to decode writes no
    report;
  - **the record, on native:** the hook keeps the report it wrote and
    its key in a static. `crash::add_record(record: Option<String>)`
    adds the text to that report and writes it again under the same
    key, unless the text is over 1 MiB: then `record` stays `None` and
    the report's `log` gains a line saying the record was too long.
    `run` calls `app.frame` inside `catch_unwind(AssertUnwindSafe(..))`.
    When it catches a panic, the hook has already written the report.
    `run` then:
    1. stops the sound (`Platform::audio().stop_all()`);
    2. calls `app.crash_record()` inside `crash::quiet` and a second
       `catch_unwind`, so a panic there writes no second report;
    3. passes the result to `crash::add_record`;
    4. ends the process with exit code 101, as an uncaught panic does.
       The game's system hook is not called, so a game in a broken
       state never saves over a good save.

    On the web a panic aborts (wasm32's panic strategy), so a web report
    has no record. The docs say so;
  - **a panic on purpose, for tests:** `SHUYI_PANIC_AT_FRAME=<n>`, read
    once by `run` on native in any build, panics at frame `n`, inside
    the same `catch_unwind` as `app.frame`, with the fixed message
    `a panic on purpose (SHUYI_PANIC_AT_FRAME)`. Unset, nothing
    changes;
  - **one hook:** if 3041 is done and installed a hook that writes the
    panic to the log file, this ticket adds the report to that hook
    instead of a second one. If not, 3041 adds its log line to this
    one. Either way, the first line of 3041's log file names the game's
    version from `crash.game_version`: if 3041 is done, add it here; if
    not, 3041 does (its scope says so).
- **The short path (2511):** `run_game` sets `crash` for every game on
  it, in the pure function that makes its `AppConfig` (`basic_config`).
  **From 3048 (done):** `BasicGame` holds the game's
  `shuyi_platform::AppInfo` (`BasicGame::app()`), whose `version` is
  `shuyi.ron`'s `app.version` or the crate's: the report's version is
  `game.app().version`, and `BasicGame` gains no builder. The template's
  `main.rs` (one `run_game` call) doesn't change. If
  2512 is done, `cargo xtask template-lines` still passes.
- **The grid template and hello-grid** (the long form):
  - hello-grid's `config()` sets `crash` to
    `CrashReports::new(APP.version)` (`hello_grid::app_info::APP`, 3048).
    The template has no `config()` since 2511: `run_game` covers it;
  - `StageRecord` moves out of `tools.rs` into the stage screen's module
    (`examples/hello-grid/src/screens/stage.rs`,
    `templates/grid/crates/game/src/stage.rs`), which the shipped binary
    builds. `tools.rs` names it from there with a `pub use`, so
    `tools::StageRecord` still works for the tools and the tests.
    hello-grid's `rules.rs` doc comment (line 76) names the new place;
  - a `pub fn crash_record(game: &Game<Hello>) -> Option<String>`
    beside it (`Game<Data>` in the template):
    the `StageRecord` of the open `StageScreen`
    (`Game::screen::<StageScreen>()`), written with `ron`, `None` when
    no stage is open. `main` adds `.crash_record(crash_record)` after
    `Start::new`;
  - `StageScreen::send` keeps the command it is applying in a public
    `pending: Option<Command>` field until `apply` returns, and the
    record ends with it. So a panic inside the rules is in the record;
  - hello-grid's `StageScreen` keeps the seed `new` drew
    (`seed: Option<u64>`, `None` from `StageScreen::of`, whose stage
    came from elsewhere), as the template's keeps `seed`. With no seed
    its record is `None`;
  - **from 2111 (done):** the record carries `made: Some(crate::made())`
    (the base part's `made()`, `base/crates/game/src/lib.rs`; hello-grid's
    `hello_grid::made()`), as `bots`' records do;
  - the templates are outside `cargo fmt --all`: format them as CI's
    template job checks them.
- **An ADR** (the next free number in `docs/adr/`, the `write-adr`
  skill): one panic hook per backend, writing through the game's
  storage; what a report holds and what it never holds (no settings, no
  saves, nothing typed unless the game's record holds it); reports stay
  on the machine; each is offered to the game once; native catches the
  panic to add the record, the web can't. It amends ADR-0024 §1:
  `shuyi-platform` may use `serde` and `ron`.
- **Docs:**
  - `docs/crash-reports.md`, for a game's developer: what a report
    holds; where it lands (`crash-<when>.ron` in the folder
    `storage::data_dir` gives on each OS, and `localStorage` under the
    storage name on the web); how to word a notice from
    `Setup::last_crash`; how to write `crash_record`; and that a record
    should not hold what a player typed (a name) until Nick answers
    3047. If 3041 is done, it also says that the log files sit beside
    the reports (`storage::log_dir`), and that a player sends both; if
    not, 3041 adds that paragraph (its scope says so);
  - `docs/platform-backends.md`: a *Panics* section that links
    `docs/crash-reports.md`, and a line in the *Checklist for a new
    backend*: "A panic writes a `CrashReport` through the backend's
    storage, or the backend's docs say why it can't."
  - **Either order with 3012** (the Steam Deck): if 3012 is done,
    `docs/targets/steam-deck.md` also says that crash reports
    (`crash-<when>.ron`) land in the data folder it names, and
    `docs/crash-reports.md` links to that page for the Deck; if not,
    nothing to do (3012 adds that line and the link, its scope says
    so).
- **CI's `render` job** (2018, done; `.github/workflows/ci.yml`) also
  runs hello-grid once under `xvfb-run` with `SHUYI_PANIC_AT_FRAME=3`
  and `XDG_DATA_HOME` in a temporary folder, and checks that one
  `crash-*.ron` appeared under it with the fixed message and `frame: 3`,
  and that the exit code is 101. 2018 didn't add this step, so this
  ticket does.
- **Size.** Count the changed lines of code and tests after step 5
  (docs and the ADR aside). If they pass about 650, the grid template's
  part (its `StageRecord` move, `crash_record`, `pending` and test) goes
  to a follow-up ticket (the `write-ticket` skill, After MVP,
  `blocked_by: ["3046"]`), written in this PR from this ticket's
  bullets. hello-grid keeps its part here.
- **For the game:** `AppConfig` gains `crash` (additive), and `Start`
  gains the `crash_record` field and builder. A new public field breaks
  a game that builds `Start` by a struct literal, so the PR's
  `Breaking:` line names it unless `Start` is `#[non_exhaustive]` by
  then. Say so in the PR; the game's PR that moves the pin follows.

**Out (do not do):**
- Stack overflows, aborts, running out of memory and faults in native
  code: they end the process without a panic hook. No minidumps.
- Frames a debug file can name, `shuyi crash` and `replay --crash`:
  2520.
- Sending a report anywhere: 3047 (parked, Nick's decision).
- A notice screen made by the engine: each game words its own.
- A session record (2518, 2519) in the report: only debug builds record
  sessions, and a session can't be cut to its last minutes (2519's
  *What "a ring" means here*). A debug build's report that names the
  session it was recording is 3060 (blocked by this ticket and 2519).
- Reports from the Playdate backend's panic handler (3010) and from the
  GBA and DS backends (parked). The checklist line above says what each
  backend writes.
- The web page showing an error over a stopped canvas: 3052.
- Settings, saves or the keymap in a report.

## Implementation steps

1. Read `run` in `crates/app/src/lib.rs`, `crates/app/src/backend.rs`,
   `crates/app/src/storage/`, `Setup`, `Start` and `App::load` in
   `crates/platform/src/app.rs`, 3040's log, 2511's `BasicGame` and
   `run_game`, and the stage screens and `tools.rs` of hello-grid and the
   grid template, and 2018's `render` job in `.github/workflows/ci.yml`.
   Check whether 2111, 2512, 3012 and 3041 are done.
2. Write `shuyi_platform::crash`: the report, its keys, `parse`,
   `to_text` and `keep_newest`, with their tests. Add `serde`, `ron`
   and `proptest`, and `tools/layers.ron`'s line.
3. Add `Setup::last_crash`, `Start::crash_record` and
   `App::crash_record`, tested through the headless backend.
4. Write `crates/app/src/crash.rs`: `CrashReports`, `install`, the hook,
   the statics, `set_frame`, the log ring, `quiet`, `add_record` and
   `SHUYI_PANIC_AT_FRAME`. Add `AppConfig::crash`, the catch to `run`,
   `quiet` to `native_music.rs`, and the version to `run_game`.
5. Wire hello-grid and the grid template. Count the lines and apply
   **Size** above.
6. Run hello-grid's release build with `SHUYI_PANIC_AT_FRAME=3` on the
   machine at hand, and put the report it wrote in the PR description.
   Add the crash step to the `render` job.
7. Write the ADR and the docs, and run the gates.

## Acceptance criteria

- [ ] With the hook installed over a test storage, a panic inside
      `catch_unwind` stores one `crash-<when>` report whose message,
      location, thread, frame, top screen and log lines are the ones
      set, and the hook that was there before is called once.
- [ ] After that panic, `crash::add_record(Some(text))` leaves one
      report under the same key, holding `text`; with text over 1 MiB,
      `record` is `None` and the last log line says why.
- [ ] A panic inside `crash::quiet` stores nothing.
- [ ] After six panics, five reports remain, the newest; with `keep: 1`,
      one.
- [ ] `CrashReport::parse` of text over 2 MiB, of random bytes as text
      and of every cut of a valid report returns an error or a report,
      and never panics.
- [ ] Through the headless backend, a storage holding two reports gives
      `Setup::last_crash` the newer one at the first load, and `None` at
      a second load on the storage the first game kept.
- [ ] A hello-grid test: a stage with two commands taken and a third
      pending gives a record of the three, which its tools' `replay`
      reads. `tools::StageRecord` still names the record type.
- [ ] hello-grid's release build run with `SHUYI_PANIC_AT_FRAME=3`
      writes a report with the fixed message, `frame: 3` and its top
      screen, with `record: None` (no stage is open at frame 3), and
      exits with 101 (in the PR description, and in 2018's `render`
      job).
- [ ] `run_game`'s config function sets `crash` to the game's version
      (a unit test with `BasicGame`).
- [ ] `shuyi-platform` still builds for `thumbv7em-none-eabihf`
      (`cargo xtask check-no-std`), and `cargo xtask check-layers`
      passes (CI's `layers` job).
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-platform`): keys and their order, and keys that are not
  reports ignored (`crash-offered` among them); `keep_newest`; `parse`
  and `to_text` both ways; old reports without the optional fields
  still read; `last_crash` offered once.
- Property (`shuyi-platform`, `proptest`): `parse` never panics on any
  text; a report written and read back is equal.
- Unit (`shuyi-app`): `run_game`'s config sets `crash` from the
  `BasicGame`'s version; `AppConfig::new` is still a `const fn` (a
  `const fn` in the test calls it).
- Integration (`shuyi-app`, `crates/app/tests/crash_hook.rs`, one
  `#[test]`, since a hook is global to its process): the hook over a
  shared test storage, two panics in one millisecond, the cap, `quiet`,
  `add_record` and the chained hook.
- Integration (hello-grid, and the grid template's `tests/it/main.rs`
  unless **Size** moved it): the record with a pending command, read by
  its tools' `replay`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
