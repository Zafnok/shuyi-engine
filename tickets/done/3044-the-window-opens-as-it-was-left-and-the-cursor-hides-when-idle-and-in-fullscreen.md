---
id: "3044"
title: "The window opens as it was left: its size and mode kept per computer, and a start size in whole multiples of the console"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-16
---

# 3044 — The window opens as it was left

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and GameMaker and Construct 3. The benchmarks give a
game its window and its cursor:

- Godot: `DisplayServer` sets the window's size, position and mode, and
  the mouse mode (hidden, captured, confined)
  (docs.godotengine.org/en/stable/classes/class_displayserver.html). Its
  stretch settings have an integer scale.
- Bevy: `Window` has a resolution, a `WindowPosition`, a mode and
  `CursorOptions::visible`, each with an example
  (github.com/bevyengine/bevy/tree/main/examples/window).
- Unity: a desktop build keeps the last resolution and fullscreen mode
  between launches by default. Unreal's `UGameUserSettings` saves them in
  `GameUserSettings.ini` when the game applies them.
- GameMaker: `window_set_size` and `window_set_cursor(cr_none)`.
  Construct 3: a "letterbox integer scale" fullscreen mode.

No benchmark hides an idle cursor by default: there it is game code,
which every keyboard-and-pad desktop game writes.

Shuyi today (read 2026-10-20):

- `crates/app/src/lib.rs`: `Window { title, width, height, icon }`.
  `window_conf` opens it at that size, resizable, `high_dpi`, and
  windowed. `run_game`'s `basic_window` and hello-grid
  (`examples/hello-grid/src/main.rs`, `src/bin/preview.rs`) work the
  size out by hand: `48 * 8 * 2 + 16`, the console at 2× plus a margin.
  (Re-read 2026-11-16: the one template, `templates/shuyi-default`, runs
  through `run_game` and has no `Window`.)
- `crates/app/src/backend.rs`: `Macroquad::fullscreen` starts `false`,
  and `present` calls `set_fullscreen` whenever a frame's `fullscreen`
  differs. The loading card's and the errors' frames come from
  `Frame::new` (`crates/platform/src/platform.rs`), which says windowed;
  only the game's frames carry the player's setting (`present_game`,
  `crates/platform/src/app.rs`). So a player who left the game
  fullscreen gets the loading card in a window, then a switch.
- `crates/ui/src/settings.rs`: the Fullscreen setting is the only window
  state saved, per profile, in the game's own record. Nothing keeps the
  window's size: a player who makes the window smaller gets the big one
  back at the next launch.
- No code calls `show_mouse`, `set_window_size` or `set_window_position`.
  The system's arrow stays over the picture, in fullscreen too.

What miniquad 0.4.11 allows (the version in `Cargo.lock`, read in the
cargo registry):

- `window::show_mouse`, `set_window_size`, `set_window_position` and
  `set_fullscreen`. On the web `show_mouse` sets the canvas's CSS
  `cursor`. On macOS `set_window_position` only prints "Not implemented
  for macos".
- `window::get_window_position` exists on Windows and Linux only.
- `Conf::window_width`, `window_height` and `fullscreen` are read once,
  when the window opens. Windows (`create_window`) and X11
  (`XCreateWindow`) take the size in device pixels; macOS takes it in
  points.
- No monitor size, work area or list, and no minimum window size.
- Quirks this ticket must handle:
  - Windows (`src/native/windows.rs`): leaving fullscreen
    (`set_fullscreen(false)`) sets the window to the fullscreen
    picture's size at (0, 0), and a window opened with `Conf::fullscreen`
    starts its `WindowsDisplay::fullscreen` flag at `false`. That flag is
    what keeps the screen saver off in fullscreen (`WM_SYSCOMMAND`).
  - macOS (`src/native/macos.rs`): a window opened with
    `Conf::fullscreen` also starts the display's `fullscreen` flag at
    `false`, and `set_fullscreen` toggles only when the flag differs. But
    the window delegate sets the flag once the window has entered or
    left fullscreen (`window_did_enter_fullscreen`,
    `window_did_exit_fullscreen`). So a fullscreen start needs no extra
    call there, and a second `set_fullscreen(true)` sent before the
    delegate has run would take the window out of fullscreen again.
  - X11 (`src/native/linux_x11.rs`): a TODO says going back from
    fullscreen to windowed doesn't work.

So opening at the largest whole multiple that fits the monitor, and
putting a saved position back safely, wait for the winit backend (3034,
parked).

Why it matters:

- **The game** opens a window of 1640 × 1064 device pixels on Windows
  (its console at 2× plus a margin). With a title bar, that is taller
  than a 1920 × 1080 screen above its taskbar. The player can drag it
  smaller, but the next launch opens it big again. After this ticket it
  opens as the player left it, with no windowed flash before fullscreen,
  and the arrow leaves the picture. The game has no ticket for this
  (game#0903, Steam readiness, has no window items), so it is not in the
  MVP.
- **Other 2D games:** every desktop and web build of a keyboard-and-pad
  game.
- **The end goal** (Nick, 2026-10-19: a game in less than an hour): a
  new game's window is one line, `Window::console(..)`, with no pixel
  arithmetic.

## Nick input

None.

## Scope

**In:**
- **The start size in whole multiples** (`crates/app/src/lib.rs`):
  - `Window::console(title: &'static str, spec: ConsoleSpec, scale: u16,
    margin: u16) -> Window`, a `const fn`: `width = cols × cell_w ×
    scale + margin`, `height` likewise, `icon: None`. The margin is there
    because a framebuffer can come out a pixel smaller than asked, which
    would drop the console a whole scale;
  - `Window` is `#[non_exhaustive]` with `Window::new` and
    `Window::with_icon` (`const`, taking `Option<Icon>`, the field's own
    type), as ADR-0039 asks. This ticket adds only `Window::console`,
    built on `Window::new`, and keeps `with_icon`;
  - hello-grid (its game and its dialogue preview) uses it, and
    `shuyi_app::run_game` (2511) builds its window with it (its
    `basic_window`, in `crates/app/src/lib.rs`), since the template has
    no `WINDOW`: `Window::console("…", CONSOLE, 2, 16)`, which is today's
    784 × 784 (`hello_grid::CONSOLE` in hello-grid, the `BasicGame`'s
    console in `run_game`).
- **A record per computer**, owned by the backend, never by the game or
  a profile (`crates/app/src/window.rs`, new):
  - the storage key `window` (`WINDOW_KEY`), beside the computer's
    `profiles` key, written through the native `storage::FileStorage`
    for the game's `storage_name`, not through the storage handed to the
    game. Native only: the web page has no window to size;
  - RON, `(version: 1, width: 1024, height: 768, fullscreen: false)`
    (`WINDOW_VERSION`), in the units `Conf::window_width` takes on that
    system: device pixels on Windows and Linux (`Platform::display`'s
    size), points on macOS (`screen_width()` and `screen_height()`,
    rounded). `width` and `height` are the last windowed size, never the
    fullscreen one;
  - written when the windowed size or the mode has stayed the same for
    1 s (`SAVE_AFTER`, *tunable*) and differs from the last record
    written, and when the game quits (`FrameOutcome::Quit` in `run`). A
    crash loses at most the last second's change;
  - a size under `MIN_WINDOW` (320 × 200) is never recorded: Windows
    reports 1 × 1 for a minimized window (`update_dimensions`);
  - a record that doesn't parse, or has another `version`, is ignored
    with one warning line, and the window opens at the game's `Window`.
- **From 2087 (done):** `window_conf` is private, and no game calls it:
  a game of its own shape (hello-grid, the game) calls
  `shuyi_app::run_app(config, start)` from a plain `main`, and
  `run_app` opens the window with `window_conf(&config.window)` before
  `run` starts. So below, `run_app` finds the game's storage folder
  (`StorageAt::find`, which 3310's `DesktopFolder` may move) and calls
  `window_conf(&config.window, &storage_at)`, then hands both what it
  decided and the `StorageAt` to `run`. `run_game` and
  `run_dialogue_preview` go through `run_app`, hello-grid and the game
  change nothing for it, and the change to `window_conf` is no break.
  `open_window` (the render card's) reads no record.
- **Opening as it was left:**
  - `window_conf(window: &Window, storage_at: &StorageAt) -> (Conf,
    Opened)` reads the record on native, from the folder the game's
    storage is in. The work is a pure `conf_from(window: &Window,
    record: Option<&str>) -> (Conf, Option<String>)` (the warning), so
    tests never touch the real data folder;
  - with a record, the window opens at its size, clamped to `MIN_WINDOW`
    and 16384 a side, and in its mode. Without one, it opens at
    `Window`'s size, windowed, as today;
  - `Conf::fullscreen` comes from the record on Windows, Linux and
    macOS. On Windows, the first frame calls `set_fullscreen(true)` once
    more, so miniquad's `WindowsDisplay::fullscreen` flag agrees and the
    screen saver stays off. On macOS and Linux nothing more is called:
    macOS's window delegate sets its flag itself, and a second call
    there would leave fullscreen;
  - `window_conf` returns what it decided (`Opened`: the mode it opened
    in, the record it read and the warning if any), and `run_app` hands
    it to `run`, which logs the warning and starts `Macroquad` in that
    mode. No `static` is needed since 2087: `run_app` holds both ends.
    On the web, and in a capture run, it opens windowed with no record.
- **No windowed flash:** `Macroquad::fullscreen` starts at the mode the
  window opened in (from `run`, above). `present` applies
  `frame.fullscreen` from `Show::Game` frames only, through a pure
  `wanted_fullscreen(&Frame) -> Option<bool>`, as the headless backend
  already keeps it (`Presented::keep`, `crates/platform/src/headless.rs`).
  The card's and the errors' frames leave the window as it is.
- **Leaving fullscreen on Windows** puts the window back at its windowed
  size and place:
  - on entering fullscreen, the backend reads `get_window_position`;
  - on leaving, after `set_fullscreen(false)`, it calls
    `miniquad::window::set_window_size` with the windowed size (device
    pixels) and `set_window_position` with the place;
  - positions cross miniquad as `u32`. A window on a monitor left of the
    main one has a negative x, which round-trips through `as i32` and
    back, and the pure code keeps it as `i32`;
  - a window that opened fullscreen has no place yet: it is centred in
    the size the fullscreen picture had (`Platform::display`'s size
    while fullscreen);
  - on Linux and macOS leaving fullscreen stays as miniquad does it.
- **The cursor** (a pure `IdleCursor`: hidden after 2 s still and at
  once in fullscreen, shown on a move, a button or focus lost): moved
  to 3314 by the *Size* clause below, with its clauses for 1121, 3106,
  3062 and 3042.
- **The clock stays in the backend:** `run` (`crates/app/src/lib.rs`)
  hands its `now` (`Millis`) to a new `Macroquad::after_frame(now)` after
  `App::frame`, which updates the record. The pure parts
  take `Millis` and are tested with made-up times.
- **Docs:**
  - `docs/platform-backends.md`: a section "The window (desktop)" with
    the record, its key (a game must not use `window` for its own data)
    and the open and leave rules, so the winit backend (3034) does the
    same (3314 adds the cursor rule). The storage section names the key;
  - `Frame::fullscreen` and `Frame::new` say only a game frame's
    `fullscreen` is applied;
  - `crates/app/src/lib.rs`'s module docs: the example uses
    `Window::console`.
- **Dependencies:** `serde` and `ron` become normal dependencies of
  `shuyi-app` (`workspace = true`; `ron` with its `std` feature). Both
  are already in the tree through `shuyi-ui`, under MIT OR Apache-2.0, so
  `deny.toml` doesn't change and `cargo deny` sees no new crate.
- **For the game:** nothing to change: `window_conf` is private since
  2087. The game builds `Window` with `Window::new`, which compiles
  unchanged, and it may move to `Window::console`. Its window opens as
  the player left it from the pin bump on.
- **Either order:**
  - **from 2321 (`Window::vsync`, done):** `Window::new` sets `vsync`
    to its default (`true`), so `Window::console` gets it by building
    on `Window::new`; `conf_from` sets `platform.swap_interval` to
    `Some(i32::from(window.vsync))`, as `window_conf` does now;
  - 2511 (`run_game`) is done: the template has no `WINDOW`, and
    `run_game`'s window function (`basic_window`) uses `Window::console`;
  - **from 3048 (done):** hello-grid's `main.rs` reads who the game is
    from `hello_grid::app_info::APP`: `Window::new(APP.name, …)` and
    `AppConfig::new(WINDOW, APP.storage, …)`. The template has no
    `WINDOW` (its window is `run_game`'s, titled with `game.name()`).
    This ticket writes hello-grid's as
    `Window::console(TITLE, CONSOLE, 2, 16)` (its binaries title it with
    `TITLE`). From 3072 (done): `APP`
    carries the window's icon (`AppInfo::window_icon`, `None` for
    hello-grid, which has no icon) and `run_game` sets it; hello-grid's
    `WINDOW` keeps `.with_icon(APP.window_icon)`;
  - with 3041 (log sinks): if it is done, the warning `run` logs is
    `log::warn!(target: "platform", …)`; if not, macroquad's `warn!`,
    which 3041 moves. `run` logs it, not `window_conf`, because 3041's
    sink is installed by `run`, after `window_conf` has run;
  - with 3012 (the Steam Deck): if it is done, its fullscreen default
    is what `conf_from` gives with no record, and a record's mode never
    takes the Deck out of fullscreen, since its platform facts say not
    windowed. `present` keeps 3012's rule too: full screen whatever the
    frame says. If not, 3012 sets its default through `conf_from` (its
    scope says so);
  - the capture mode (2018, done): `window_conf` reads no record when
    `SHUYI_CAPTURE` (`shuyi_app::capture::PLAN_VAR`) is set, with the
    `capture` feature, and a capture run writes none;
  - with 2333 (scale modes): both touch `backend.rs`; whichever is
    second rebases;
  - 3062 (the Fullscreen setting follows the system's own control) is
    not done: nothing to do here (3062 does it, its clause says so);
- **Size:** if the PR passes about 600 changed lines, the cursor
  (`IdleCursor`, its wiring and its docs) goes to a follow-up ticket
  (the `write-ticket` skill, After MVP, blocked by this one), and the
  `IdleCursor` clauses in 1121 and 3106 then name that ticket.

**Out (do not do):**
- an options row for the window's size (1×, 2×, …): the largest
  multiple that fits needs the monitor's size, which miniquad can't
  give, and dragging the window already gives any size, drawn at a whole
  scale (`console::layout`). 2323 (parked) offers console sizes;
- the monitor: opening at the largest whole multiple that fits its work
  area, keeping a restored window on a connected monitor, choosing a
  monitor, a minimum window size and the maximized state. They need
  monitor facts, so they go to the winit backend (3034, whose scope
  lists them);
- putting the window's place back at the next launch: without the
  monitors, a place saved on a monitor since unplugged opens the window
  off screen (3034);
- leaving fullscreen on Linux (miniquad's X11 TODO; 3034);
- exclusive fullscreen and display modes (3034);
- the cursor: 3314 (split off by the *Size* clause); custom cursor
  images and confining it (the winit backend, 3034); pointer lock
  (3106, parked);
- pointer input for menus and grid maps (2405) and for games (1121);
- a game's default for the Fullscreen setting (a `GameSettings` constant
  like 2321's `DEFAULT_FRAME_CAP`): a follow-up if a game asks;
- the Steam Deck's display (3012), apart from its clause under *Either
  order*;
- syncing the record through Steam Cloud: it stays on the computer (3037
  says so);
- a migration chain for the record (2110): a record of another version
  is ignored.

## Implementation steps

1. Read `crates/app/src/lib.rs`, `backend.rs` and `storage/`,
   `crates/platform/src/platform.rs` (`Frame`), `app.rs`
   (`present_game`) and `headless.rs` (`Presented::keep`), and in the
   cargo registry miniquad 0.4.11's `src/lib.rs` (the `window` module),
   `src/conf.rs`, `src/native/windows.rs` and `src/native/macos.rs`.
2. Write `crates/app/src/window.rs`: the record (`WINDOW_KEY`,
   `WINDOW_VERSION`, `MIN_WINDOW`, `SAVE_AFTER`), `conf_from`, the
   memory that decides when to write, the place on leaving fullscreen
   and `wanted_fullscreen`, all pure, with their tests.
3. Add `Window::console`, and change `window_conf` to return `Opened`.
4. Wire `Macroquad` and `run`: `run` takes `Opened` and logs its
   warning; the starting mode; game frames only for `fullscreen`; the
   first-frame `set_fullscreen(true)` (Windows only); leaving fullscreen
   on Windows; `after_frame`; and the write at quit in `run`.
5. Move hello-grid and `run_game` from `Window::new` to
   `Window::console`.
6. Write the docs. Run hello-grid on Windows and check: resize, quit,
   reopen at that size; go fullscreen, quit, reopen fullscreen with no
   windowed card; leave fullscreen and get the window back where it
   was. Say in
   the PR what was seen, and that macOS and Linux were not checked by
   hand. Run the gates, the web build included.

## Acceptance criteria

- [x] `Window::console("x", ConsoleSpec::text(48, 24), 2, 16)` is
      784 × 784.
- [x] `conf_from` with no record gives the `Window`'s size, windowed;
      with `(version: 1, width: 1000, height: 700, fullscreen: true)` it
      gives 1000 × 700 and fullscreen; with 100 × 50 it gives 320 × 200;
      with 20000 × 20000 it gives 16384 × 16384; with text that doesn't
      parse, or `version: 2`, it gives the `Window`'s size and one
      warning.
- [x] The memory, with made-up times: a size held 999 ms writes nothing,
      held 1000 ms writes once, and the same size again writes nothing;
      a 1 × 1 size never replaces the windowed size; while fullscreen the
      recorded size stays the last windowed one; quitting writes the
      record if it differs from the last written, and nothing if it
      doesn't.
- [x] A record written and read back gives the same size and mode.
- [x] The place on leaving fullscreen: the place read on entering comes
      back, a negative x included (`(-1200i32) as u32` from miniquad
      comes back as -1200); with no place, a 1000 × 700 window in a
      1920 × 1080 picture is centred at (460, 190).
- [x] `wanted_fullscreen` gives `None` for a card and for errors, and
      `Some` of the frame's `fullscreen` for a game frame.
- [x] `Opened`: what `window_conf` builds from `conf_from`'s answer
      keeps the mode, the windowed size, the good record and the
      warning, and an unkept one (the web, a capture run) never writes.
- [x] hello-grid and `run_game` open with `Window::console`.
- [x] The PR says what was seen on Windows for the checks in step 6.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/window.rs`): every case in the criteria above.
- Unit (`crates/app/src/lib.rs`): `Window::console`; the existing
  `an_icon_that_does_not_decode_is_none`
  moves to `conf_from` with no record and still passes.
- Manual: the checks on Windows in step 6, said in the PR.

## Completion notes

**What was done.**
- `crates/app/src/window.rs` (new): the record (`WINDOW_KEY`,
  `WINDOW_VERSION`, `MIN_WINDOW`, `SAVE_AFTER`), `conf_from`, `Opened`,
  `WindowMemory` (when to write), `Place` (where the window goes back to
  on leaving fullscreen) and `wanted_fullscreen`, all pure, with their
  tests. `window/win32.rs` reads the client area on Windows (below).
- `Window::console(title, spec, scale, margin)`, a `const fn` with
  saturating sides; `run_game`'s `basic_window` and hello-grid's game
  and dialogue preview use it.
- `run_app` finds the storage folder, calls `window_conf(&window,
  &storage_at)`, which reads the record beside the game's keys (native,
  outside a capture run) and returns `(Conf, Opened)`, and hands both to
  `run`. `run` logs the warning, `Macroquad` starts in the mode it
  opened in, applies `fullscreen` from game frames only, re-sends
  `set_fullscreen(true)` on the first frame of a Windows fullscreen
  start, puts the window back at its windowed size and place on leaving
  fullscreen on Windows, and writes the record in `after_frame` and as
  the game quits (`save_window`).
- `StorageAt::read_window` and `write_window` (through `FileStorage`,
  in the folder `StorageAt::find` chose). `serde` and `ron` (`std`) are
  normal dependencies of `shuyi-app` now (they were dev-dependencies;
  same versions, already in the tree, so `deny.toml` is unchanged).
- Docs: `docs/platform-backends.md`'s new *The window (desktop)*, the
  storage section names the key, the `present` section says only a
  game frame's `fullscreen` applies; `Frame::fullscreen` and
  `Frame::new`; `run_app`'s and `open_window`'s docs; the crate docs'
  example uses `Window::console`; the book's *Games and screens*
  (`site/content/learn/book/games-and-screens.md`) names
  `Window::console` and the window kept as it was left.

**Deviations.**
- The ticket was 26 days old and was checked against the code first
  (the skill's step 0); the fixes are in its text: there is one
  template now (`templates/shuyi-default`, through `run_game`, no
  `Window`), so the "grid template" steps went; 2087 had made
  `window_conf` private behind `run_app`, so `Opened` is returned by
  `window_conf` and handed to `run` as a value, not kept in a `static`
  (which only crossed `macroquad::main`'s boundary); 3310's
  `DesktopFolder` decides where a desktop keeps the storage, so
  `window_conf` takes the `StorageAt` (found once, in `run_app`, before
  the window opens) instead of the storage's name, and the record goes
  where the game's keys go; hello-grid titles its window with `TITLE`,
  not `APP.name`; nothing changes in the game's `main.rs`.
- **The cursor went to 3314**, by the *Size* clause: without it the PR
  was already past 800 counted lines. 1121, 3106, 3062 and 2405 now
  name 3314 for `IdleCursor`. 3314 also carries a fix this ticket's
  manual check suggested: a mode change can move the mouse's position
  in the window without the mouse moving, so it must not read as a move.
- **The size on Windows is read from the system**, not from miniquad
  (`window/win32.rs`, through `drops::miniquad_window`). The manual
  check found the record shrinking about 2 pixels a launch at 175 %
  (1550, then 1548, then 1547): miniquad's `update_dimensions` divides
  the client area by the scale, cuts it to a whole number and multiplies
  back. `GetClientRect` gives the exact size `Conf::window_width` and
  `set_window_size` take.
- `WindowMemory` holds whether the record is kept (`Opened::kept`), so a
  capture run or the web never writes, rather than a flag on
  `Macroquad` (clippy's `struct_excessive_bools`).

**Manual check on Windows** (Windows 11, 2560 × 1600 at 175 %, after
merging the tip of `main`, 4638655b, into the branch at 36e02830, with
the size fix on top): hello-grid driven from a script through Win32
(`SetWindowPos`, a posted F11 and `WM_CLOSE`), the window's rectangle
sampled every 30 ms from launch.
- Resized to 1550 × 1055 client pixels, quit: the record said 1550 ×
  1055; reopened at that size, and with the fix it stays 1547 × 1050
  over two more launches.
- F11, quit: the record said `fullscreen: true` with the windowed size;
  reopened, the first rectangle sampled was already 2560 × 1600, with no
  windowed one before it (the loading card was full screen).
- F11 out of a fullscreen start: back at 1547 × 1050, centred at (506,
  275); in and out again, the same place. Moved to (300, 120), in and
  out: back at (300, 120).
- Not checked: the screen saver staying off in a fullscreen start, and
  macOS and Linux by hand.

**Follow-ups.** 3314 (the idle cursor), written in this PR.

Game: nothing to adopt, its `Window::new(APP.name, 1640, 1064)` and
`run_app` compile unchanged, and its window opens as left from its next
pin bump (searched `window_conf`, `Window::new`, `"window" storage` and
`WINDOW_KEY` in Zafnok/visions-of-shuyi, pinned at 1ccb1acc: no game key
named `window`). game#0903 has no window items, as the Context says.

No break: `Window::console` is an addition, and `window_conf` and `run`
are private.

Behaviour: shuyi-app: on a desktop a game's window opens at the size and in the mode the player left it in on that computer (a `window` key beside its storage), not at `AppConfig::window`'s size, windowed, after the first launch.

Behaviour: shuyi-app: a loading card's or an error's frame no longer switches the window out of fullscreen; only a game frame's `fullscreen` is applied.

Behaviour: shuyi-app: on Windows, leaving fullscreen puts the window back at its windowed size and place instead of the screen's size at (0, 0).
