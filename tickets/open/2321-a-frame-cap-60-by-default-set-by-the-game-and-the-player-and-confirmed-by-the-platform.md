---
id: "2321"
title: "A frame cap: 60 by default, set by the game and the player, that works with vsync and VRR, and a request the platform confirms"
type: feature
milestone: MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 2321 — A frame cap, 60 by default

## Context

- **Nick's request (2026-10-19, `docs/decisions.md`):** the engine caps
  the frame rate at 60 by default. Too many engines leave it unlocked,
  so even a simple game runs as fast as it can and maxes out a CPU core,
  which feels bad (fans, heat, battery). The game's developer can change
  the cap, turn it off on purpose or raise it. It must work with vsync
  and variable refresh rate (VRR: G-Sync, FreeSync, adaptive sync). The
  options screen offers it, and a game changes it at run time through a
  request it can check, "like a promise", in Rust's own terms.
- **No friction review:** a missing feature, not a request from the
  game.
- **Today** (`crates/app/src/lib.rs`, `run`): the loop calls
  `app.frame(&mut platform, now)` with `now` from
  `miniquad::date::now()` (`App::frame` reads the input, runs the game
  and draws through the backend), then awaits `next_frame()`. Nothing
  limits the rate except the swap interval
  miniquad's default asks the driver for (`window_conf` leaves
  `platform.swap_interval` at `None`, which is only a hint: miniquad's
  `conf.rs` says the driver may ignore it). With vsync off or ignored
  the loop spins.
- **How settings reach the platform today:** a setting lives in
  `Settings` (`crates/ui/src/settings.rs`), an options row changes it
  (`OptionRow::fullscreen` in `crates/ui/src/screens/options.rs`),
  `Game::frame` puts it in `FrameOutput` (`crates/ui/src/game.rs`,
  `fullscreen`), `App` puts it on the `shuyi_platform::Frame` it
  presents (`present_game`), and `shuyi-app` applies it
  (`Macroquad::present`). What the platform knows goes back in before a
  frame (`App` calls `Game::set_music_playing`). The cap uses both
  directions.
- **Layers (ADR-0002, `AGENTS.md` rule 5):** the cap is plain data in
  `shuyi-ui`; only `shuyi-app` reads the clock and sleeps. A turn game's
  state and a tick game's steps (1119) don't depend on the frame rate
  (1119's `TickDriver` keeps a frame's presses until a step runs), so
  nothing deterministic changes.
- **Related:** 3003 (done) moved the run loop into `shuyi-platform`'s
  `App::frame` (`crates/platform/src/app.rs:372`), so the pacer goes in
  the macroquad backend (`crates/app`) and the report goes through
  `App`. 3013 measures frame times and battery on the Steam Deck; a cap
  is what makes its battery numbers good. 2314 (enum option rows):
  either order; if it's done, build the row with it.

## Nick input

**Sign-off:** run the hello-grid example (`cargo run -p hello-grid`),
open Task Manager, and compare its CPU with the cap at 60 and at
Unlimited on its options screen. Say whether the Frame rate row's
wording reads right.

## Scope

**In:**
- **`FrameCap`** in `shuyi-ui` (`crates/ui/src/settings.rs` or a new
  `frame_cap.rs`): `Off` or `Fps(u16)`, with `FrameCap::fps(n) ->
  Result<FrameCap, CapError>` accepting 15 to 1000 (*tunable* consts;
  outside gives `CapError::OutOfRange`). Default `Fps(60)`.
- **The game's default and choices:** two associated consts on
  `GameSettings` with default values, so existing games compile
  unchanged: `const DEFAULT_FRAME_CAP: FrameCap = FrameCap::Fps(60)` and
  `const FRAME_CAP_CHOICES: &'static [FrameCap]` (default 30, 60, 120,
  144, 165, 240, `Off`). `Settings::default()` uses the game's default.
- **The player's setting:** `Settings::frame_cap`. The toy's settings
  file saves it with `#[serde(default)]`, so a save without it loads
  with the game's default.
- **An options row,** `OptionRow::frame_cap()`: steps through the
  game's choices; `Fps(n)` shows as "n FPS" through a text key with a
  placeholder (`options.value.fps`), `Off` as `options.value.unlimited`;
  label `options.row.frame_cap` ("Frame rate"). The toy and hello-grid
  options screens get the row. **Either order with 2351:** if it is
  done, the English of `options.row.frame_cap`, `options.value.fps` and
  `options.value.unlimited` goes only in the engine's
  `crates/ui/lang/en/ui.ron`; if not, in the toy's and hello-grid's
  `en.ron`, and 2351 moves it.
- **A request the game can check** (the "promise"). Rust-native and
  synchronous, no async or callbacks in `shuyi-ui` (it must stay
  `no_std`-friendly, 3005):
  - `Ctx::request_frame_cap(cap: FrameCap) -> CapRequest` changes the
    setting (saved like any other) and returns a small `Copy` handle
    holding a serial number. Any change of `frame_cap`, from the
    options row too, takes the next serial.
  - `Ctx::frame_cap_status(req) -> CapStatus`: `Pending` until the
    platform reports it has applied that serial or a later one;
    `Applied(Pacing)` once it has; `Superseded` if a later request
    replaced it before it was applied.
  - `Ctx::pacing() -> Pacing`: the last report, for a screen that shows
    the frame rate (the debug readout, 2316, can use it).
  - `FrameOutput` gains `frame_cap: FrameCap` and `frame_cap_serial:
    u32`; `Game::set_pacing(report: Pacing)` is called by the platform
    before each frame, like `set_music_playing`. `Pacing` holds the
    applied cap, its serial, whether vsync was asked for, and the
    measured frames per second (a rolling average, as a `u16`).
- **Vsync, a developer setting:** `shuyi_app::Window` gains `vsync:
  bool`; `window_conf` sets `platform.swap_interval` to `Some(1)` or
  `Some(0)`. miniquad only reads it when the window opens, so it is not
  a player setting here. If 2013 is done, `Window` is
  `#[non_exhaustive]` with `Window::new`. `new` sets `vsync` to its
  default, and this ticket adds `#[must_use] pub const fn
  with_vsync(self, vsync: bool) -> Window`. The new public types
  (`FrameCap`, `CapError`, `CapRequest`, `CapStatus`, `Pacing`) follow
  2013's ADR. If 2013 isn't done, add the field as written; 2013 then
  sets it in `Window::new` and adds the setter.
- **The pacer, in `shuyi-app`** (a new `pacing.rs`): a pure struct that
  is given times (seconds as `f64`) and answers how long to wait, so it
  is tested without a clock; `run` feeds it `miniquad::date::now()` and
  sleeps. Its rules:
  - **Deadline-based, not "sleep a period after each frame":** the next
    frame is due at `last_due + period`; after a frame that ran late,
    `due = now` (no burst of fast frames to catch up).
  - **Never wait twice (vsync and VRR).** It keeps a rolling measure of
    the frame interval *without* its own waits. When that is already at
    least the period minus a small tolerance (*tunable*, e.g. 3 %),
    vsync, the display or a slow machine is already limiting the
    rate, and the pacer waits 0. This is what stops a 60 cap on a
    60 Hz vsync display from missing a vblank and dropping to 30. On a
    VRR display with vsync on, a cap below the refresh rate makes the
    pacer wait and the display follows the frames; a cap at or above
    it lets vsync limit.
  - **Waits precisely:** sleep (`std::thread::sleep`) until about 1.5 ms
    before the due time (*tunable*), then `std::thread::yield_now` in a
    loop until it. The wait happens after `App::frame` (which draws
    through the backend) and before `next_frame().await`.
  - **`Off`** never waits.
  - **The web:** the browser's `requestAnimationFrame` sets the rate
    and a page can't sleep. When a frame comes before the due time, the
    loop skips `App::frame` and only redraws the last buffer; the `dt`
    the next `App::frame` gives the game is the time since the last one
    that ran, not `get_frame_time()`. A skipped frame still reads the
    input: macroquad 0.4 clears its keys pressed and released at the
    end of every frame (`end_frame`), and since 3003 `App::frame` polls
    input inside the frame it runs (`step` calls
    `Platform::poll_input`, which `Macroquad` answers with `keys::poll`
    and the pads), so skipping it would lose those presses in every
    game. On a skipped frame the backend reads the keyboard, the pads
    and the window's shortcut (`keys::fullscreen_shortcut`), keeps
    those events in order, and hands them to the next frame that runs,
    before that frame's own.
  - It reports `Pacing` each frame through `Game::set_pacing`.
- **Either order with 2518 (session records):** `Pacing` reaches the
  game from the platform, so a session record must hold it. Hand it to
  the game through `App` (for example `App::set_pacing`, applied in
  `App::frame` before the update), never through `App::game_mut`, so
  `App` sees it. If 2518 is done, `FrameRecord` keeps the `Pacing` the
  game was given (`pacing: Option<Pacing>`, written when it changes)
  and `session::Playback` hands it back. If not, 2518 does it (its own
  clause says so).
- **The short path (2511, done):** the engine's
  `impl GameSettings for ()` (`crates/ui/src/settings.rs`, its
  `EngineOnly` record) saves `frame_cap` with `serde(default)`, as the
  toy's file does, and `shuyi_app::run_game`'s window
  (`basic_window` in `crates/app/src/lib.rs`) sets `vsync` to the
  default the templates get.
- **Docs:** the `Window::vsync` and `FrameCap` docs say the default and
  why, and that for VRR a cap a few frames under the display's refresh
  (141 on a 144 Hz display) keeps the display in its variable range.
  They also say that a tick stage (1119) steps 60 times a second, so
  above 60 each step shows for whole frames. That is even at 120 and
  240, but 2 and 3 frames in turn at 144 and 165, until its world
  layers smooth between steps (2361, parked). A cap that is a multiple
  of 60 avoids it.
- **An ADR** (`write-adr` skill) for the request-and-report shape: a
  setting the platform must apply, confirmed through a serial in a
  report before the frame. Later settings of the same kind (vsync at
  run time, a resolution) reuse it.

**Out (do not do):**
- A vsync or VRR option for the player, or switching vsync at run
  time: miniquad sets the swap interval only when the window opens.
  Write a follow-up ticket if the session finds a way.
- Detecting the display's refresh rate or whether VRR is on: miniquad
  doesn't expose either. The pacer's measure covers it.
- Lowering the rate when the window is unfocused or minimized: 3043,
  which waits for this one.
- Fixed simulation steps (1119).
- The Playdate or other backends (they set their own rates; 3010).
- Any game's screens. The game picks up the row and its settings field
  at its next pin bump.

**For the game:** the API is additive. `Settings` has a `pub(crate)`
field, so no game builds it by literal, and `Settings::frame_cap` is
additive. `shuyi_app::Window` gains `vsync`: this is additive if 2013 is
done (a game builds `Window` with `Window::new`); if not, a struct
literal needs the new field. The game's
settings file should save `frame_cap` like the toy's, or the player's
choice isn't kept. The game's PR that moves its pin follows this one.

## Implementation steps

1. `FrameCap`, `CapError`, the `GameSettings` consts and
   `Settings::frame_cap`, with the toy's file saving it.
2. The serial, `CapRequest`, `CapStatus`, `Pacing`,
   `Ctx::request_frame_cap`, `Ctx::frame_cap_status`, `Ctx::pacing`,
   `Game::set_pacing`, and the two `FrameOutput` fields. Bump the serial
   wherever settings change (`change_settings`, `preview_settings` and
   restore defaults) when `frame_cap` differs.
3. `OptionRow::frame_cap()` and its text keys; add the row to the toy's
   and hello-grid's options screens.
4. `crates/app/src/pacing.rs`: the pure pacer and its tests.
5. Wire it into `run`: `Window::vsync`, `window_conf`, the wait before
   `next_frame`, the web skip (keeping the skipped frames' input), and
   `Pacing` through `App` to `Game::set_pacing`.
6. The short path's two parts (2511), and the either-order parts whose
   partner is done (2013, 2314, 2351, 2518).
7. The ADR. Measure CPU with hello-grid at 60 and Unlimited on a desktop
   and put the numbers in the completion notes.

## Acceptance criteria

- [ ] `Settings::default().frame_cap` is `Fps(60)`; a game that sets
      `DEFAULT_FRAME_CAP` gets its own; `FrameCap::fps(14)` and
      `fps(1001)` are `Err(OutOfRange)`, `fps(15)` and `fps(1000)` are
      `Ok`.
- [ ] A toy settings save without `frame_cap` loads with the default;
      one with `Off` loads `Off`.
- [ ] `request_frame_cap` returns `Pending`; after `set_pacing` with its
      serial it's `Applied` with that report; a second request before
      the report makes the first `Superseded` and the second `Applied`
      once its serial is reported.
- [ ] Changing the cap on the options row bumps the serial and
      `FrameOutput` carries the new cap; restore defaults returns it to
      the game's default.
- [ ] Pacer, with a fake clock: at a 60 cap and 2 ms of work it waits
      about 14.7 ms; when unpaced frames already take 16.7 ms (60 Hz
      vsync) it waits 0; at a 30 cap on that display it waits about one
      more vblank's worth; with `Off` it never waits; after a 100 ms
      frame the next due time is not in the past (no catch-up burst).
- [ ] Pacer on the web path: a frame before the due time is skipped and
      the `dt` handed on adds up the skipped time.
- [ ] On the web path, a key pressed and released on skipped frames
      reaches the next frame that runs, once and in order (a unit test
      of the kept events with fake input).
- [ ] Snapshot of the options screen with the Frame rate row, showing
      "60 FPS".
- [ ] The completion notes give hello-grid's CPU at 60 and at Unlimited.
- [ ] All gates in the `run-gates` skill pass, the web build included.

## Tests required

- Unit: `FrameCap`, the settings file, the request statuses, the row's
  stepping, the pacer with a fake clock (cases above), and the input
  kept over skipped web frames.
- Snapshot: the options screen with the row.
- Manual: CPU use at 60 and Unlimited (completion notes).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
