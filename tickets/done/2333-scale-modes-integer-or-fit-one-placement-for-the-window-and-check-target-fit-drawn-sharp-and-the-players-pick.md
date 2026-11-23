---
id: "2333"
title: "Scale modes: integer or fit, one placement for the window and check --target, fit drawn sharp, and the player's pick"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2511"]
nick_input: none
completed: 2026-11-23
---

# 2333 — Scale modes: integer or fit

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal 5.8 and Construct 3. The benchmarks let a game pick how its
picture fills a window:

- Godot: stretch modes, an aspect policy (`keep`, `expand` and others)
  and, since 4.2, an integer or fractional scale mode
  (docs.godotengine.org/en/stable/tutorials/rendering/multiple_resolutions.html).
- Unity 6: the 2D Pixel Perfect Camera renders at a whole scale into a
  texture, then crops it or stretches it to fill.
- Construct 3: letterbox scale and letterbox integer scale, beside
  scale inner and outer.
- Bevy: `ScalingMode` on a camera's projection. That is closer to the
  cameras of 1120 than to this ticket.

Shuyi today (read 2026-10-21):

- `crates/ui/src/frame/console.rs`, `layout(spec, screen_w: f32,
  screen_h: f32) -> Layout`: the largest whole scale that fits (at
  least 1), centred on whole pixels. Nothing else.
- `crates/app/src/render.rs`, `Renderer::draw`: calls it in physical
  pixels (the window is `high_dpi`) and draws every texture with
  `FilterMode::Nearest`.
- `crates/app/src/backend.rs`, `draw_centred`: the loading card's
  picture at the largest whole scale, with its own copy of the maths,
  in logical pixels. It ignores the DPI factor that `draw` honours.
- `crates/cli/src/project.rs`: `Scale { Integer, Fit }` on each target
  in `shuyi.ron`. Only `check --target` reads it
  (`crates/cli/src/contract/target.rs`: `scale_on` and `Factor`,
  fractions in integers). 3001 defined it, and no renderer work
  followed.
- The templates' `shuyi.ron` (`templates/blank`, `templates/grid`)
  tells every new game that its scale may be `integer` or `fit`.

So `check --target` passes a `fit` game and checks its text at, say,
1.56×, while the window draws it at 1×. No example uses `fit` yet, so
the mismatch is latent. But agents are told to trust `shuyi check
--json`, and it promises something the window doesn't do.

What integer-only costs: an 800 × 512 frame (100 × 32 cells of 8 × 16)
on the Steam Deck's 1280 × 800 screen draws at 1× and covers 40 % of
it. On a 1366 × 768 laptop it covers 39 %. Fit covers 98 % and 88 %. A
browser canvas can be any size, so the web build has the same problem.

Why it matters:

- **The game:** nothing changes. Integer stays the default, and the
  game opts in only through its own `engine-change` process. The Deck
  (3012) is where it would show first.
- **Other 2D games:** every desktop, web and Deck build, on any screen.
- **The contract:** the window and `check --target` share one function,
  so the check can't pass what the window doesn't draw.

Related: 3012 (the Deck: integer with letterbox), 2323 (parked: whole
glyph scales, and a new buffer for a new console size), 1121 (parked),
2405 (done: the pointer in console pixels) and 2413 (done: `shuyi-app`
reads the mouse through this placement's inverse), 3044
(window sizes), 2321 (a game's default on `GameSettings` and a player
setting: the shape used here). **From 2511 (done):** this extends
`shuyi_cli::BasicTools` (`crates/cli/src/basic.rs`) and the `()`
settings (`impl GameSettings for ()` in `crates/ui/src/settings.rs`).

## Nick input

None. Integer stays the default, so the game gets nothing new unless
it opts in. Whether it offers "fill the screen" to its players is the
game's choice, through its `engine-change` skill.

## Scope

**In:**
- **`shuyi_ui::frame::console`**, in integers only (no floats, still
  `no_std`):
  - `ScaleMode { Integer, Fit }`, `Integer` the default, with the serde
    names `integer` and `fit` (those `shuyi.ron` already uses),
    `ScaleMode::ALL`, and `ScaleMode::key` (the options screen's text
    key of each, as `TextSpeed::key` gives);
  - `Factor { num, den }`: the CLI's type, moved here as it is
    (`whole`, `of`, the private `min`, `shown`), plus `ceil`;
  - `scale(mode, frame: (u32, u32), space: (u32, u32)) -> Factor`: what
    `scale_on` computes today, a frame side of 0 counted as 1.
    `Integer` is the largest whole number that fits, 0 when even 1
    doesn't. `Fit` is the largest fraction that fits, below 1 too,
    **or the largest whole number when that is at least `SNAP` (9 / 10)
    of the fraction** (amended 2026-11-23 for game#1320, Nick's OK: on
    1080p and 4K the whole scale covers about 95 % of an 800 × 512
    frame's height, and stays pixel-perfect);
  - `place(mode, frame, window) -> Placement`, with `Placement { scale:
    Factor, draw_scale: u32, x: i32, y: i32, w: u32, h: u32 }`: where
    the frame goes, in window pixels:
    - `Integer`: `scale` at least 1 (a too-small window crops the edges
      evenly, as today), and `w = frame_w × scale`;
    - `Fit`: `w = frame_w × num / den` and `h = frame_h × num / den`,
      rounded down, so the limiting side equals the window's; when
      `scale` snapped to a whole number, `Integer`'s placement;
    - centred with `div_euclid`, so a leftover odd pixel rounds down
      and a 799-pixel-wide window gives `x = -1`, as today;
    - `draw_scale` is the whole scale the frame is drawn at. For
      `Integer` it is `scale`. For `Fit` it is `scale` rounded up, at
      least 1, and lowered while the frame times it passes
      `MAX_DRAW_PX` (4096) on either side;
  - `layout(spec, f32, f32) -> Layout` and `Layout` keep their
    signatures and fields: the game's `crates/ui/src/console.rs`
    re-exports them and builds `Layout` values in its tests. `layout`
    turns each size into a `u32` (rounded down; not finite or below 0
    is 0) and calls `place(ScaleMode::Integer, …)`. Its tests in
    `console.rs` stay unchanged, which proves integer placement doesn't
    move.
- **`check --target` uses them:** `project::Scale` becomes `pub type
  Scale = ScaleMode`, `scale_on` becomes a one-line call of
  `console::scale` (it unpacks the profile's `Size`), and `target.rs`
  imports `Factor` from `shuyi_ui`. Messages and the tests in
  `crates/cli/src/contract/target/tests.rs` don't change (their `use
  super::*` still finds `Factor` and `Scale`).
- **The game's default and the player's pick:**
  - `GameSettings::DEFAULT_SCALE: ScaleMode`, an associated const that
    defaults to `ScaleMode::Integer`, so existing games compile
    unchanged. It has the shape of 2321's `DEFAULT_FRAME_CAP` (done):
    put it beside `DEFAULT_FRAME_CAP` and `FRAME_CAP_CHOICES` in
    `GameSettings`, and `scale` beside `frame_cap` in `Settings`, the
    toy's `File` and the `()` form's `EngineOnly`;
  - the toy (`crates/ui/src/toy.rs`) sets `DEFAULT_SCALE =
    ScaleMode::Fit`, so a test can tell the game's default from the
    engine's;
  - **moved to 3240 at step 7** (this PR passed 600 lines): the
    player's pick, `Settings::scale`, `OptionRow::scale` and its text,
    and `scale` in the toy's, hello-grid's and the `()` form's saved
    forms, with the clauses about the option rows, the engine's English
    and the save versions that went with them;
- **The frame carries it:** `shuyi_platform::Frame::scale: ScaleMode`
  (`Frame` is `#[non_exhaustive]`; `Frame::new` gives `Integer`).
  `present_game` (`crates/platform/src/app.rs`) sets it from the
  settings, beside `fullscreen`. The loading card's frame (`App::run`,
  `Phase::Loading`) gets `<G::Settings as GameSettings>::DEFAULT_SCALE`,
  since the player's settings aren't read yet. The headless backend
  keeps it in `Presented::Game` beside `fullscreen`, and
  `Harness::scale()` reads it like `Harness::fullscreen()`.
- **`shuyi-app` draws it:**
  - `Macroquad::present` passes `frame.scale` to `draw_game`, which
    calls `Renderer::set_scale(mode)` before `draw`, as it calls
    `set_clear`. The window's size in physical pixels is computed as
    `draw` does today (`(screen_width() * dpi).round()`);
  - when the placement's rectangle is the frame times `draw_scale`
    (`Integer`, or a `Fit` that came out whole), `draw` makes the same
    calls as today, so the pixels are the same;
  - otherwise it draws "sharp bilinear":
    - the frame is drawn at `draw_scale` into a render target
      (`macroquad::texture::render_target`, kept between frames and
      made again when its size changes, its texture set to
      `FilterMode::Linear`), through a `Camera2D` whose
      `render_target` is that target, one unit per target pixel;
    - then the default camera draws the target's texture into the
      placement's rectangle, divided by the DPI factor as today;
    - each logical pixel's edge blends over one window pixel at most,
      and the rest stays sharp (two at most when `MAX_DRAW_PX` lowered
      `draw_scale` below `scale`);
    - macroquad 0.4.16's `Camera2D` turns y the other way for a render
      target (`camera.rs`, `invert_y`: 1 for a target, -1 for the
      screen). Check the picture isn't upside down, and set `flip_y`
      on the draw if it is;
    - a rectangle of 0 pixels draws nothing;
  - on the render-target path, `draw_backdrop` draws on the same
    surface as the frame: its viewport in target pixels (check its rows
    land where the window's do) and its camera's `render_target` set.
    It returns to the target's camera instead of calling
    `set_default_camera()`. The integer path doesn't change;
  - the loading card's picture (`draw_centred`) goes through `place`
    with the frame's mode, in physical pixels. For `Fit` it takes the
    same render-target path, through a small helper outside `Renderer`
    (the card is drawn before the renderer exists);
  - the web build runs the same code.
- **The pointer's inverse.** **From 2413 (done):** the inverse of
  today's placement (`Layout::frame_px` in `console.rs`, called through
  `console_px` in `crates/app/src/pointer.rs`, which places the frame
  with `render::placement`, the function `Renderer::draw` uses) moves
  onto `Placement` as `Placement::frame_px`. For `Fit` it takes the
  fraction: frame x = (window x - x) × den / num, rounded down.
  `pointer.rs` passes the mode the frame was drawn in (the backend
  keeps the last frame's `ConsoleSpec` beside the reader; it keeps the
  mode there too). `Layout::frame_px` stays as a call of the `Integer`
  placement's, as `layout` does, so 2413's tests stay; `Placement`'s
  tests get a `Fit` row.
- **The check compares the game's default with `shuyi.ron`:** moved to
  3242 when this PR passed 1,500 counted lines (`GameTools::scale`, the
  comparison and its limit lines, `Fake`, and the tools of the grid
  template, hello-grid and `BasicTools`).
- **Docs:**
  - an ADR (the `write-adr` skill). A frame may be shown at a
    fractional scale. Logical pixels, glyph scales, the backdrop's
    zoom and camera zooms stay whole; only the backend's last step
    scales by a fraction. One placement function, in integers, serves
    every backend and the check. It says why sharp bilinear (plain
    linear filtering blurs, and stretching makes pixels uneven), why
    4096, and how the default and the player's pick work. It amends
    ADR-0008 §1's `desktop` row ("any, integer scale") and, for the
    engine, game ADR-0012's integer-only rule;
  - the comment in `targets/desktop.ron`, the templates' `shuyi.ron`
    comment (it names `DEFAULT_SCALE`), and in
    `docs/platform-backends.md` the line on the card's picture and a
    line on `frame.scale` beside `frame.fullscreen` (a backend that
    shows its display 1:1 ignores it);
  - `docs/contract.md`, the `check --target` row: with the game's
    tools, its default scale is compared with each target's `scale`.
- **Either order with other tickets:**
  - **3012** (the Deck): if it is done, its placement in `shuyi-app`
    moves onto `place` with the frame's mode, and its test of integer
    with letterbox at 1280 × 800 and 1280 × 720 still passes; if not,
    3012 uses `place`;
  - **2346** (the IME): if it is done, the IME position it computes in
    `shuyi-app` from `layout` goes through `place` with the frame's
    mode; if not, 2346 uses `place` (its clause says so).
- **From 2018 (done):** the capture mode (`shuyi-app`'s `capture`
  feature, `crates/app/src/capture.rs`) compares the window with the
  raster in CI's `render` job. Its plans run at a whole scale
  (`ScaleMode::Integer`), and Hello, Grid's plan
  (`examples/hello-grid/capture/frames.ron`) gains one capture in
  `Fit`. That capture reads the render target `draw` paints into (its
  texture's pixels, before they are stretched) and compares them with
  the raster at `draw_scale`; the stretched window is saved for a look
  only. How a plan asks for `Fit` is this ticket's choice, and a test
  of `Plan::parse` reads it.

**Out (do not do):**
- an aspect policy that grows the console by whole cells when the
  window's shape changes (Godot's `expand`): 2376 (parked). It
  needs 2323's new buffer for a new console size and this ticket's
  `place`, so it comes after both;
- `frame-png` and 2332's raster: they stay at whole scales;
- world layers drawn into the bars: 1120's layers (parked) stay inside
  the frame, and drawing them past it is a ticket when a game asks;
- screen effects (2326);
- window sizes in whole multiples, and a remembered window size (3044);
- a different default per target: the running game knows its target
  (`ctx.platform.target`, from 3053, done), but neither ticket does it:
  it is a follow-up of its own, blocked by this one, written when a game
  needs a different default on one target (3053's Out says the same);
- stretching to fill with uneven pixels;
- other backends: `shuyi-playdate` (3010) shows its display 1:1;
- the game's options screen and saved form (the game's own ticket, if
  it wants the row).

**For the game:** additive. `layout` and `Layout` stay;
`GameSettings::DEFAULT_SCALE` and `GameTools::scale` have defaults;
`Frame` is `#[non_exhaustive]`; `Settings` gains a public field but
already has a private one, so no game builds it field by field.
`Presented::Game` gains `scale` additively (**from 2085 (done):** it is
a `#[non_exhaustive]` variant, built with `Presented::game`, which keeps
its arguments and starts `scale` at `Integer`). The game's PR that moves the pin
follows this one. 2010's `api` job shows the new items. The new public
types follow ADR-0039.

**Estimated size:** about 1,400 lines with the player's pick (code 600,
tests 500, docs 300); about 1,000 without it (written 2026-11-23, after
the code had started: see the completion notes).

**Split:** past 1,000 lines, step 7's player's pick goes to its own
ticket, and the `Fit` capture of 2018's `render` job with it; past
1,500, step 6's comparison of the game's default too.

## Implementation steps

1. Read `console.rs`, `render.rs`, `backend.rs` (`present`,
   `draw_game`, `draw_card`, `draw_centred`),
   `crates/cli/src/contract/target.rs` and its tests,
   `crates/cli/src/contract/check.rs` (`targets`) and `fake.rs`,
   `crates/cli/src/game.rs`, `crates/platform/src/app.rs` (`run`,
   `present_game`), `headless.rs` and `harness.rs`, and `settings.rs`,
   `toy.rs` and `screens/options.rs` in `shuyi-ui`. Check which of
   2110, 2314, 2346, 2351 and 3012 are done (2413 is).
2. Write `ScaleMode`, move `Factor`, write `scale`, `place` and
   `MAX_DRAW_PX`, and put `layout` on `place`, with the tests below.
3. Move `check --target` onto them. Its tests pass unchanged.
4. Add `DEFAULT_SCALE`, `Settings::scale`, `Frame::scale`, the card's
   scale, the headless and `Harness` parts, and the option row, with
   the toy, hello-grid and the `()` form.
5. Draw `Fit` in `shuyi-app`: the render target, the backdrop on it,
   and the card. Move the pointer's inverse onto `Placement`.
6. (Moved to 3242.) Add `GameTools::scale`, the comparison and the limit lines, `Fake`,
   and the tools of the grid template, hello-grid and `BasicTools`.
7. Count the changed lines. If the PR would pass about 600 (fixtures
   and snapshots not counted), move the player's pick to a follow-up
   ticket (the `write-ticket` skill, After MVP): `Settings::scale`,
   `OptionRow::scale`, its text, and `scale` in the toy's, hello-grid's
   and `()`'s saved forms. The frame then carries the game's
   `DEFAULT_SCALE` until that ticket lands, and the PR says so.
8. Run hello-grid (a 384 × 384 frame) at 1280 × 800 and 1920 × 1080 in
   both modes, on the desktop and in the web build, and put
   screenshots in the PR. (At 1366 × 768 it fits exactly 2 in both
   modes, so that window shows no difference.) If step 7 moved the
   row out, switch modes with a local change of hello-grid's
   `DEFAULT_SCALE` that isn't committed.
9. Write the ADR and the doc lines. Run the gates.

## Acceptance criteria

- [x] `place` gives these for an 800 × 512 frame (a table test):

  | Window | `Integer` | `Fit` (`draw_scale`) |
  | ------ | --------- | -------------------- |
  | 1280 × 800 | 1×: 800 × 512 at (240, 144) | 1250 × 800 at (15, 0) (2) |
  | 1366 × 768 | 1×: 800 × 512 at (283, 128) | 1200 × 768 at (83, 0) (2) |
  | 1280 × 720 | 1×: 800 × 512 at (240, 104) | 1125 × 720 at (77, 0) (2) |
  | 1920 × 1080 | 2×: 1600 × 1024 at (160, 28) | snapped: as `Integer` (2) |
  | 2560 × 1440 | 2×: 1600 × 1024 at (480, 208) | 2250 × 1440 at (155, 0) (3) |
  | 3840 × 2160 | 4×: 3200 × 2048 at (320, 56) | snapped: as `Integer` (4) |

  Also: a 384 × 384 frame on 1280 × 800 is 2×, 768 × 768 at (256, 16),
  in both modes (2 is 96 % of 800 / 384: snapped), and on 1920 × 1080 in
  `Fit` 1080 × 1080 at (420, 0) (3). A 1600 × 1000 frame on 3840 × 2600
  in `Fit` (3840 × 2400) has `draw_scale` 2, since 3 would make a
  4800-pixel picture. An 800 × 512 frame on 640 × 400 is 1× at
  (-80, -56), and in `Fit` 625 × 400 at (7, 0) (1). (Amended with the
  snap: the 1920 × 1080 and 3840 × 2160 rows were 1687 × 1080 at
  (116, 0) (3) and 3375 × 2160 at (232, 0) (5), the 384 × 384 case on
  1280 × 800 was 800 × 800 at (240, 0) (3), and the 1600 × 1000 case was
  on 3840 × 2160, where it now snaps.)
- [x] The existing tests of `layout` in `console.rs` pass unchanged.
- [x] The tests in `crates/cli/src/contract/target/tests.rs` pass
      unchanged.
- [x] ~~A check whose tools report `integer` against a `fit` target fails
      on `targets."web".scale`, and the two limit lines~~ (not done:
      moved to 3242 when the PR passed 1,500 lines)
- [x] An `App` test in `crates/platform/src/app/tests.rs`, through a
      platform that records each frame's `scale` (as `RecordLook`
      records the look): the toy's loading card frame carries `Fit`,
      its `DEFAULT_SCALE`.
- [x] A `Harness` test: the toy's presented frame carries
      `ScaleMode::Fit`; after `Settings::scale` is set to `Integer`, it
      carries `Integer`. (If step 7 moved the player's pick out: the
      frame carries the toy's `DEFAULT_SCALE`.)
- [x] ~~Unless step 7 moved the player's pick out: the option row, the
      restored default, the saved form and hello-grid's options test~~
      (not done: step 7 moved them to 3240)
- [x] Every existing frame snapshot is unchanged.
- [x] ~~Screenshots of hello-grid at 1280 × 800 and 1920 × 1080 in
      both modes, on the desktop and in the web build~~ (not done: with
      the snap, 1280 × 800 is 2× in both modes; the PR shows 1280 × 720
      on the desktop in both modes, upright and sharp; the web's filled
      frame is 3241's capture in `web-render`)
- [x] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Factor` (moved with its tests, plus `ceil`); `scale` in both
  modes, including 0 for `Integer` when 1 doesn't fit; the `place`
  table above; `DEFAULT_SCALE` and `Settings::default()` (the toy's
  `Fit`); the option row; the toy's saved form with and without
  `scale`; the check's comparison and its two limit lines (the tests
  of `targets` in `check.rs` that list the limits gain them).
- Property (proptest, frames 1 to 2000 pixels a side, windows 0 to
  8000):
  - `Fit`'s rectangle lies inside the window and equals it on at least
    one axis;
  - when `scale` gives `Integer` 1 or more, `Fit`'s rectangle is at
    least as big as `Integer`'s;
  - in `Fit`, `draw_scale` is at least 1, and the frame times
    `draw_scale` stays within `MAX_DRAW_PX` unless `draw_scale` is 1;
  - `place` gives the same answer twice.
- Integration: the `App` and `Harness` tests above, and the `Fit`
  capture in 2018's `render` CI job (the part from 2018 above). **From
  2019 (done):** CI's `web-render` job runs the same plan,
  `examples/hello-grid/capture/frames.ron`, in headless Chromium, so the
  `Fit` capture added to it is compared in WebGL too, read from the
  render target.

## Completion notes

What was done:

- `shuyi_ui::frame::console` (`console/place.rs`, in whole numbers,
  `no_std`): `ScaleMode { Integer, Fit }` with `ALL`, `Factor` (moved
  from the CLI, with `floor`, `ceil`, `is_whole`), `scale`, `place`,
  `Placement` (with `is_whole` and `frame_px`), `SNAP` and
  `MAX_DRAW_PX`; `frame_of(spec)`. `layout` is `place(Integer, …)` and
  `Layout::frame_px` is `Placement::frame_px` at a whole scale: their
  tests in `console.rs` pass unchanged. Table, case and property tests
  in `console/place/tests.rs`.
- `check --target`: `project::Scale` is `ScaleMode`, `scale_on` is
  `console::scale`, and `target.rs` re-exports `Factor`; the tests in
  `target/tests.rs` pass unchanged.
- `GameSettings::DEFAULT_SCALE` (default `Integer`; the toy's `Fit`),
  `Frame::scale` (set by `present_game` and on the loading card's frame),
  `Presented::Game`'s `scale` and `Harness::scale`, with an `App` test
  (the card's and the game's frames carry the toy's `Fit`) and a
  `Harness` test.
- `shuyi-app`: `Renderer::set_scale`; `draw` draws a whole placement as
  before and a fraction through `Stretch` (a render target kept between
  frames, filtered linearly, the backdrop's viewport and camera on it);
  the loading card's picture through `place` and the same `Stretch`;
  the pointer maps through `Placement::frame_px` with the mode the last
  frame was drawn in (`Macroquad` keeps it beside the spec), with a test
  of a filled frame at 1280 × 720.
- ADR-0139; the ADR-0008 status and index rows; `targets/desktop.ron`'s
  and the template `shuyi.ron`'s comments; `docs/platform-backends.md`'s
  `frame.scale` line; `docs/contract.md`'s `check --target` row; the
  roadmap's rows for 3240, 3241 and 3242.

Tried by hand on this branch (9f3b74e5 and after, on main fd927adb), on
Windows 11: hello-grid in a 1280 × 720 window through the capture mode,
with an uncommitted change to `Fit` and a full-window read. The 384 × 384
frame filled the height, 720 × 720 at (280, 0), drawn at 2× and
stretched, upright, the title and both looks of the stage. With
`Integer` in the same window, the three captures matched the raster
exactly (0 pixels over): the whole-scale path draws as before. The web
build wasn't tried: 3241 puts a filled frame in the `web-render` job.

Deviations:

- **The snap** (Nick's OK, 2026-11-23, for game#1320): `Fit` keeps the
  whole scale when it is at least 9 / 10 of the fraction, in `scale`
  itself so the check agrees. The acceptance table's 1920 × 1080 and
  3840 × 2160 rows, the 384 × 384 case on 1280 × 800 and the
  1600 × 1000 case changed with it (the ticket says what they were); a
  1280 × 720 row was added. The property "Fit equals the window on one
  axis" holds when it doesn't snap, and a snapped `Fit` equals
  `Integer`'s placement; the `MAX_DRAW_PX` property holds on the
  stretched path, the only one that draws into a texture.
- **Split at step 7** (past 1,000 lines): the player's pick
  (`Settings::scale`, `OptionRow::scale`, `ScaleMode::key`, its text and
  the saved forms) is ticket 3240; the frame carries the game's
  `DEFAULT_SCALE` until then. The `Fit` capture of the `render` and
  `web-render` jobs is ticket 3241. 2314's, 2351's and 2110's
  either-order clauses now name 3240.
- **Split past 1,500 lines** (1,530 measured with it): step 6, the
  check's comparison of each target's `scale` with the game's default
  (`GameTools::scale`, the limit lines, `Fake`, the tools of
  `BasicTools`, hello-grid and the template), is ticket 3242. Until it
  lands nothing checks that `shuyi.ron` and `DEFAULT_SCALE` agree
  (ADR-0139 says so).
- Unticked acceptance boxes: the check's comparison (3242); the player's
  pick (3240); the screenshots, which are of hello-grid at 1280 × 720
  on the desktop in both modes, not at 1280 × 800 and 1920 × 1080 or on
  the web: with the snap, a 384 × 384 frame on 1280 × 800 is 2× in both
  modes, and the capture mode reads a window of the size the plan's
  machine gives (3241 adds a filled capture on both).
- The ticket had no size estimate (it was on
  `tools/unestimated-tickets.txt`): written after the code had started,
  and the id taken off the list.
- `Fit` for a window of 0 pixels places the frame at 1×, as `Integer`
  does (the fraction 0 is whole), instead of a rectangle of 0.

Breaking: shuyi-cli: `project::Scale`: an alias of `shuyi_ui::frame::console::ScaleMode`, no longer an enum of its own; its variants and serde names are the same.

```migration
since: 0.1.0
what: shuyi.ron's scale is the engine's ScaleMode.
old: shuyi_cli::project::Scale
new: shuyi_ui::frame::console::ScaleMode
before: |
  use shuyi_cli::project::Scale;
  let s = Scale::Fit;
after: |
  use shuyi_ui::frame::console::ScaleMode;
  let s = ScaleMode::Fit;
find: ["project::Scale"]
```

Breaking: shuyi-cli: `contract::target::Factor`: a re-export of `shuyi_ui::frame::console::Factor`, with the same fields, `whole`, `of` and `shown`.

```migration
since: 0.1.0
what: Factor lives in shuyi-ui; the CLI re-exports it.
old: shuyi_cli::contract::target::Factor
new: shuyi_ui::frame::console::Factor
before: |
  use shuyi_cli::contract::target::Factor;
after: |
  use shuyi_ui::frame::console::Factor;
find: ["target::Factor"]
```

Behaviour: shuyi-cli: `check --target` checks a `fit` target's text at the scale the window draws: the whole scale when it is at least 9 / 10 of the fraction.

Book: none, a game opts in through `GameSettings::DEFAULT_SCALE`, documented on the item, in ADR-0139 and in `docs/platform-backends.md`; the book's chapter follows the player's pick (3240).

Searched the game (pin 50472bbc): a `grep` of its crates and `shuyi.ron`
for `Scale::`, `target::Factor`, `scale_on` and `Presented::Game {`
found none of the engine's; its targets all say `scale: integer`, so
the snap changes nothing it checks.

Measured: 1,371 counted lines (`cargo xtask pr-size`), after the two splits.

Game: game#1320 moves the pin and sets `DEFAULT_SCALE = Fit`.
