# ADR-0050: A setting the platform applies: a request with a serial, confirmed by a report before the frame

- **Status:** Accepted
- **Date:** 2026-10-27
- **Related tickets:** 2321 (this, the frame cap), 2518 (session
  records), 3010 (the Playdate), 3043 (fewer frames in the background),
  2316 and 2325 (the debug readout and profiler)

## Context

Nick asked for a frame cap, 60 by default, that the game's developer
sets, the player changes on the options screen, works with vsync and
VRR, and that a game changes at run time "through a request it can
check, like a promise" (2026-10-19, `docs/decisions.md`).

- **Only the platform can apply it.** The cap is a wait in the backend's
  loop (`shuyi-app`'s `run`), or a device's fixed rate. `shuyi-ui` is
  pure and `no_std`-friendly (ADR-0002, 3005): no clock, no async, no
  callbacks into the platform.
- **The settings already cross the boundary one way.** A setting lives
  in `Settings`, an options row changes it, `App` puts it on the
  `shuyi_platform::Frame` it presents, and the backend applies it
  (Fullscreen). What the platform knows comes back before a frame
  (`Game::set_music_playing`, `Ctx::system_keyboard`). Nothing yet said
  whether a change *took*.
- **A request can be replaced before it applies.** A game may ask twice
  in one frame, or the player may change the cap on the options screen
  while a request is in flight. A boolean "applied" can't tell which
  change the platform applied.
- **Some backends never apply it.** The headless backend runs frames as
  a test drives them; the Playdate runs at its profile's 30.
- **More settings of this kind are coming:** vsync at run time, a
  resolution, a window mode the device must confirm.

## Decision

1. **Every change of such a setting takes the next serial number.** The
   context keeps the serial of the latest change
   (`Ctx::frame_cap_serial`, a `u32` from 0). Any change bumps it,
   wherever it comes from: `Ctx::request_frame_cap`, a row through
   `Ctx::preview_settings` or `change_settings`, Restore defaults, a
   profile switched to with another value (`Ctx::load_settings`). A
   request bumps it even when the value is the one in force, so it is
   confirmed all the same.
2. **The setting goes out with its serial on every frame**:
   `FrameOutput::frame_cap` and `frame_cap_serial`, and the same on the
   `shuyi_platform::Frame` `App` presents (`Frame::frame_cap`,
   `frame_cap_serial`; `Frame::new` gives the default cap, serial 0).
3. **The platform reports what it applied before the next frame**: a
   plain report (`shuyi_ui::frame_cap::Pacing`: the cap, the serial it
   came with, whether vsync was asked for, the measured frames a second)
   through `App::set_pacing`, which the next frame that runs the game
   hands over before the update (`Game::set_pacing`). It goes through
   `App`, never `App::game_mut`, so `App` sees it (a session record
   keeps it, 2518).
4. **A request is a `Copy` handle holding its serial**
   (`CapRequest`), checked synchronously like a promise
   (`Ctx::frame_cap_status`):
   - `Superseded` once a later change has taken the next serial,
     whether or not this one had applied: the value asked for is no
     longer the one in force;
   - else `Applied(report)` once a report carries this serial;
   - else `Pending`. A backend that never reports leaves every request
     here; a device with a fixed rate reports it as the cap applied,
     with the latest serial (3010).
5. **The pacer is pure and lives in the backend** (`shuyi-app`'s
   `pacing.rs`): given the clock's times in seconds, it answers how long
   to wait. It keeps deadlines (after a late frame they start again,
   with no burst), never waits on top of vsync (it measures the frame
   interval without its own waits, and waits nothing when that is
   already within 1 % of the period), leaves the swap's own wait out of
   its wait, and sleeps until 0.5 ms before the due time, then yields.
   On the web it skips the game's frame (`App::present_again` shows the
   last one) and keeps that frame's input for the next that runs.
6. **The next setting of this kind reuses the shape**: a serial on the
   context bumped by every change, the value and serial on `Frame`, a
   field in the backend's report, a request handle and a status. Its
   ticket names this ADR.

## Consequences

- A game knows when its cap took effect, without async or callbacks,
  and a debug readout shows the measured rate (`Ctx::pacing`).
- `shuyi-ui` stays pure: the serial and the report are plain data, and
  only `shuyi-app` reads the clock and sleeps.
- `Superseded` also covers a request that applied and was later
  replaced, which the ticket's wording ("before it was applied") didn't:
  the context keeps two numbers (the latest serial and the last report),
  not a history of every serial.
- A backend has one more duty, written in `docs/platform-backends.md`
  (*The frame cap*): report, or leave requests pending.
- `FrameOutput` became `#[non_exhaustive]` when it gained the two fields
  (ADR-0039 §3), one break for games that never build it.
- The 0.5 ms of yielding before each due time costs a few percent of a
  core at 60 frames a second; it is *tunable* (`SPIN_S`).

## Alternatives considered

- **An `async` request or a callback** — `shuyi-ui` has no executor and
  must build without `std` (3005); a callback into the game from the
  platform would be a second path for state changes beside the frame
  (ADR-0002 §3).
- **A boolean "applied" flag** — can't tell which of two quick changes
  the platform applied, so a game could read the first request as
  applied when the second is in force.
- **The platform comparing values, no serial** — a request for the cap
  already in force would never be confirmed apart from the last one, and
  a change back and forth within a frame would look like nothing.
- **Pacing in `App` (`shuyi-platform`)** — `App` has no clock of its own
  and doesn't own the loop (ADR-0024); the wait is the backend's, and
  a device with a fixed rate has nothing to pace.
- **Sleeping a period after each frame** — drifts with the frame's own
  time and waits on top of vsync, so a 60 cap on a 60 Hz display misses
  vblanks and drops to 30.
