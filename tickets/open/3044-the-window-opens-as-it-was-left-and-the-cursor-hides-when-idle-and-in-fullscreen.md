---
id: "3044"
title: "The window opens as it was left: its size and mode kept per computer, a start size in whole multiples of the console, and the cursor hidden when idle and in fullscreen"
type: feature
milestone: After MVP
model: sonnet-5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3044 — The window opens as it was left, and the cursor hides when idle and in fullscreen

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
  windowed. The templates (`templates/blank` and `templates/grid`,
  `crates/game/src/main.rs`) and hello-grid
  (`examples/hello-grid/src/main.rs`) work the size out by hand:
  `48 * 8 * 2 + 16`, the console at 2× plus a margin.
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
  - the grid template and hello-grid use it, and `shuyi_app::run_game`
    (2511) builds its window with it (its `basic_window`, in
    `crates/app/src/lib.rs`), since the blank template has no `WINDOW`:
    `Window::console("…", CONSOLE, 2, 16)`, which is today's 784 × 784
    (`game::CONSOLE` in the grid template, `hello_grid::CONSOLE` in
    hello-grid, the `BasicGame`'s console in `run_game`).
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
- **Opening as it was left:**
  - `window_conf(window: &Window, storage_name: &str) -> Conf` reads the
    record on native. The grid template and hello-grid call
    `window_conf(&WINDOW, STORAGE_NAME)`; `run_game` calls
    `window_conf(&window, storage)` with its `BasicGame`'s storage name.
    The work is a pure `conf_from(window: &Window, record: Option<&str>)
    -> (Conf, Option<String>)` (the warning), so tests never touch the
    real data folder;
  - with a record, the window opens at its size, clamped to `MIN_WINDOW`
    and 16384 a side, and in its mode. Without one, it opens at
    `Window`'s size, windowed, as today;
  - `Conf::fullscreen` comes from the record on Windows, Linux and
    macOS. On Windows, the first frame calls `set_fullscreen(true)` once
    more, so miniquad's `WindowsDisplay::fullscreen` flag agrees and the
    screen saver stays off. On macOS and Linux nothing more is called:
    macOS's window delegate sets its flag itself, and a second call
    there would leave fullscreen;
  - `macroquad::main` calls `window_conf` before the game's `main` runs,
    so `window_conf` keeps what it decided (the mode it opened in, and
    the warning if any) in a private `static`
    (`std::sync::Mutex<Option<Opened>>`). `run` takes it first thing,
    logs the warning, and starts `Macroquad` in that mode. A game that
    builds its own `Conf` leaves it `None`: windowed, as today. On the
    web it is always `None`.
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
- **The cursor:** a pure `IdleCursor` in `window.rs`, on native and the
  web:
  - moving the mouse or pressing a mouse button shows the cursor
    (`mouse_position()` changed since the last frame, or
    `is_mouse_button_pressed` for any button);
  - it hides after 2 s with neither (`CURSOR_IDLE`, *tunable*);
  - it hides at once when the window enters fullscreen, and on the
    first frame of a fullscreen start;
  - `keep_shown: bool` keeps it shown. It is `false` everywhere in this
    ticket; 1121 sets it (its scope says so);
  - `show_mouse` is called only when the answer changes;
  - reading the mouse here is not input: nothing reaches `poll_input` or
    the game, and no action is read (`AGENTS.md` rule 8).
- **The clock stays in the backend:** `run` (`crates/app/src/lib.rs`)
  hands its `now` (`Millis`) to a new `Macroquad::after_frame(now)` after
  `App::frame`, which updates the cursor and the record. The pure parts
  take `Millis` and are tested with made-up times.
- **Docs:**
  - `docs/platform-backends.md`: a section "The window and the cursor
    (desktop)" with the record, its key (a game must not use `window` for
    its own data), the open and leave rules and the cursor rule, so the
    winit backend (3034) does the same. The storage section names the
    key;
  - `Frame::fullscreen` and `Frame::new` say only a game frame's
    `fullscreen` is applied;
  - `crates/app/src/lib.rs`'s module docs: the example uses
    `Window::console` and `window_conf(&WINDOW, STORAGE_NAME)`.
- **Dependencies:** `serde` and `ron` become normal dependencies of
  `shuyi-app` (`workspace = true`; `ron` with its `std` feature). Both
  are already in the tree through `shuyi-ui`, under MIT OR Apache-2.0, so
  `deny.toml` doesn't change and `cargo deny` sees no new crate.
- **For the game:** `window_conf` takes the storage name, a one-line
  change in its `main.rs`. The game builds `Window` with
  `Window::new`, which compiles unchanged, and it may move to
  `Window::console`. The game's PR that moves the pin follows this one,
  and the PR description says so. If 2010 is done, the PR carries the
  `Breaking:` line its job asks for.
- **Either order:**
  - with 2321 (`Window::vsync`): if 2321 is done, `Window::console` sets
    `vsync` to 2321's default through `Window::new`; if not, 2321 sets it
    in `Window::new`, which `Window::console` builds on;
  - 2511 (`run_game`) is done: the blank template has no `WINDOW` or
    `window_conf`, and `run_game`'s window function (`basic_window`)
    uses `Window::console` and the new `window_conf` as above;
  - with 3048 (the game's identity): if it is done, the grid template
    and hello-grid write
    `Window::console(APP.name, CONSOLE, 2, 16).with_icon(APP.window_icon)`
    and call `window_conf(&WINDOW, APP.storage)`; `with_icon` takes
    `Option<Icon>`, so `APP.window_icon` passes straight in. If not,
    3048 makes that change (its scope says so);
  - with 3041 (log sinks): if it is done, the warning `run` logs is
    `log::warn!(target: "platform", …)`; if not, macroquad's `warn!`,
    which 3041 moves. `run` logs it, not `window_conf`, because 3041's
    sink is installed by `run`, after `window_conf` has run;
  - with 3012 (the Steam Deck): if it is done, its fullscreen default
    is what `conf_from` gives with no record, and a record's mode wins
    after. If not, 3012 sets its default through `conf_from` (its
    scope says so);
  - with 2018 (the capture mode): if it is done, `window_conf` reads no
    record when `SHUYI_CAPTURE` is set (with the `capture` feature), and
    a capture run writes none and leaves the cursor alone; if not, 2018
    keeps its capture mode so;
  - with 3042 (focus): if it is done, `IdleCursor` shows the cursor on
    `FocusLost`, so it is never hidden over another program; if not,
    3042 adds that (its scope says so);
  - with 2333 (scale modes): both touch `backend.rs`; whichever is
    second rebases.
- **Size:** if the PR passes about 600 changed lines, the cursor
  (`IdleCursor`, its wiring and its docs) goes to a follow-up ticket
  (the `write-ticket` skill, After MVP, blocked by this one), and the
  `IdleCursor` clauses in 1121 and 3042 then name that ticket.

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
- custom cursor images, confining the cursor, and pointer lock (3106,
  parked, for 3D cameras);
- pointer input for menus and grid maps (2405) and for games (1121).
  2405 needs nothing from here: moving the mouse or clicking shows the
  cursor;
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
   memory that decides when to write, the place on leaving fullscreen,
   `wanted_fullscreen` and `IdleCursor`, all pure, with their tests.
3. Add `Window::console`, change `window_conf`, and add the `Opened`
   static it fills.
4. Wire `Macroquad` and `run`: `run` takes `Opened` and logs its
   warning; the starting mode; game frames only for `fullscreen`; the
   first-frame `set_fullscreen(true)` (Windows only); leaving fullscreen
   on Windows; `after_frame`; and the write at quit in `run`.
5. Move the grid template, hello-grid and `run_game` from
   `Window::new` to `Window::console` and the new
   `window_conf`. `cargo fmt --all` doesn't reach `templates/`, so format
   their files by hand (CI's template job checks them).
6. Write the docs. Run hello-grid on Windows and check: resize, quit,
   reopen at that size; go fullscreen, quit, reopen fullscreen with no
   windowed card; leave fullscreen and get the window back where it was;
   leave the mouse still for 2 s in a window and in fullscreen. Say in
   the PR what was seen, and that macOS and Linux were not checked by
   hand. Run the gates, the web build included.

## Acceptance criteria

- [ ] `Window::console("x", ConsoleSpec::text(48, 24), 2, 16)` is
      784 × 784.
- [ ] `conf_from` with no record gives the `Window`'s size, windowed;
      with `(version: 1, width: 1000, height: 700, fullscreen: true)` it
      gives 1000 × 700 and fullscreen; with 100 × 50 it gives 320 × 200;
      with 20000 × 20000 it gives 16384 × 16384; with text that doesn't
      parse, or `version: 2`, it gives the `Window`'s size and one
      warning.
- [ ] The memory, with made-up times: a size held 999 ms writes nothing,
      held 1000 ms writes once, and the same size again writes nothing;
      a 1 × 1 size never replaces the windowed size; while fullscreen the
      recorded size stays the last windowed one; quitting writes the
      record if it differs from the last written, and nothing if it
      doesn't.
- [ ] A record written and read back gives the same size and mode.
- [ ] `IdleCursor`, with made-up times: shown at a windowed start, still
      shown at 1999 ms, hidden at 2000 ms; shown again on a move or a
      button; hidden at once on entering fullscreen and at a fullscreen
      start; never hidden with `keep_shown`; a change is reported only
      when the answer changes.
- [ ] The place on leaving fullscreen: the place read on entering comes
      back, a negative x included (`(-1200i32) as u32` from miniquad
      comes back as -1200); with no place, a 1000 × 700 window in a
      1920 × 1080 picture is centred at (460, 190).
- [ ] `wanted_fullscreen` gives `None` for a card and for errors, and
      `Some` of the frame's `fullscreen` for a game frame.
- [ ] `Opened`: what `window_conf` stores from `conf_from`'s answer
      (the mode and the warning) is taken once; with nothing stored,
      the take gives `None`. One test touches the `static`.
- [ ] The grid template, hello-grid and `run_game` open with
      `Window::console`, and
      CI's template job passes.
- [ ] The PR says what was seen on Windows for the checks in step 6.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/app/src/window.rs`): every case in the criteria above.
- Unit (`crates/app/src/lib.rs`): `Window::console`; `Opened` filled
  and taken once; the existing `an_icon_that_does_not_decode_is_none`
  moves to `conf_from` with no record and still passes.
- Manual: the checks on Windows in step 6, said in the PR.

## Completion notes

*(Filled in by the session that completes the ticket.)*
