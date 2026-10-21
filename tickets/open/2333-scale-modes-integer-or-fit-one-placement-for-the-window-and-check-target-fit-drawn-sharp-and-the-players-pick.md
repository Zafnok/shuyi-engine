---
id: "2333"
title: "Scale modes: integer or fit, one placement for the window and check --target, fit drawn sharp, and the player's pick"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2511"]
nick_input: none
completed:
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
glyph scales, and a new buffer for a new console size), 1121 (parked)
and 2405 (the pointer, which needs this placement's inverse), 3044
(window sizes), 2321 (a game's default on `GameSettings` and a player
setting: the shape used here). It waits for 2511 (MVP), whose
`BasicTools` and `()` settings it extends.

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
    doesn't. `Fit` is the largest fraction that fits, below 1 too;
  - `place(mode, frame, window) -> Placement`, with `Placement { scale:
    Factor, draw_scale: u32, x: i32, y: i32, w: u32, h: u32 }`: where
    the frame goes, in window pixels:
    - `Integer`: `scale` at least 1 (a too-small window crops the edges
      evenly, as today), and `w = frame_w × scale`;
    - `Fit`: `w = frame_w × num / den` and `h = frame_h × num / den`,
      rounded down, so the limiting side equals the window's;
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
    unchanged. It has the shape of 2321's `DEFAULT_FRAME_CAP`; either
    order with 2321;
  - `Settings::scale: ScaleMode`. `Settings::default()` takes the
    game's default, so "Restore defaults" (`Settings::restored`)
    returns to it;
  - `OptionRow::scale()`: a choice row, label `options.row.scale`
    ("Picture"), values `options.value.sharp` ("Sharp pixels") and
    `options.value.fill` ("Fill the screen"). Either order with 2314:
    if 2314 is done, build it with `OptionRow::choice`; if not, write
    it like `OptionRow::text_speed`, and 2314 moves it;
  - the toy (`crates/ui/src/toy.rs`) sets `DEFAULT_SCALE =
    ScaleMode::Fit`, so a test can tell the game's default from the
    engine's. It saves `scale` in its settings file (`File` is already
    `#[serde(default)]`, so a file without it loads with the default),
    and its `toy/en.ron` gets the row's text for the row's test;
  - hello-grid keeps the default `Integer`. It gets the row on its
    options screen (`option_rows` in `examples/hello-grid/src/screens.rs`,
    after the map look), the text in `files/en.ron`, and `scale` in its
    saved form (`src/settings.rs`, whose `File` is already
    `#[serde(default)]`). The templates don't get the row: a game opts
    in;
  - **Either order with 2351:** if it is done, the English of
    `options.row.scale`, `options.value.sharp` and `options.value.fill`
    goes only in the engine's `crates/ui/lang/en/ui.ron`, and neither
    `en.ron` gets it; if not, as written, and 2351 moves it;
  - 2511's `()` saved form (`impl GameSettings for ()` in
    `crates/ui/src/settings.rs`) keeps `scale` with `serde(default)`,
    as the toy's does. **Either order with 2110:** if it is done, that
    is a new `VERSION` of the `()` form with its fixture, as 2110's
    guard asks; if not, 2110's version 1 has it.
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
- **The pointer's inverse. Either order with 2405:** if 2405 is done,
  its inverse of today's placement (`Layout::frame_px` in `console.rs`,
  called by `crates/app/src/pointer.rs`) moves onto `Placement` as
  `Placement::frame_px`. For `Fit` it takes the fraction: frame x =
  (window x - x) × den / num, rounded down. `pointer.rs` passes the
  mode the frame was drawn in. `Layout::frame_px` stays as a call of
  the `Integer` placement's, as `layout` does, so 2405's tests stay;
  `Placement`'s tests get a `Fit` row. If 2405 is not done, 2405
  writes the inverse on `Placement` when it lands.
- **The check compares the game's default with `shuyi.ron`:**
  - `GameTools::scale(&self) -> Option<ScaleMode>`
    (`crates/cli/src/game.rs`), `None` by default;
  - in `targets` (`crates/cli/src/contract/check.rs`), when the game's
    tools offer it, each asked target whose `scale` differs is an
    error: `TargetCheck::Fits`, setting `targets."<id>".scale`, message
    "`fit` here, but the game draws `integer` by default
    (`GameSettings::DEFAULT_SCALE`)". There is no new `TargetCheck`
    variant, so the contract's schema stays;
  - without it, the answer's limits get a line, as for the controls:
    "the scale isn't compared with the game's: the game's tools don't
    offer it (GameTools::scale)", or, with no tool program, "… it needs
    the game's tools, and shuyi.ron names no game tool program
    (`game`)". It goes after the per-target lines, and the last line
    becomes "only the frame's fit, the text's size, the controls and
    the scale are checked yet";
  - `Fake` (`crates/cli/src/contract/fake.rs`) gets a `scale` field
    for the tests;
  - the grid template's tools (`templates/grid/crates/game/src/tools.rs`)
    and hello-grid's (`examples/hello-grid/src/tools.rs`) return
    `Some(<S as GameSettings>::DEFAULT_SCALE)` for their settings type
    `S` (`Prefs` today). `shuyi_cli::BasicTools` (2511, the blank
    template's tools) returns `Some(<() as GameSettings>::DEFAULT_SCALE)`;
  - the check certifies the game's default. A player who picks the
    other mode chooses what they see.
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

**Out (do not do):**
- an aspect policy that grows the console by whole cells when the
  window's shape changes (Godot's `expand`). It needs 2323's new
  buffer for a new console size, so it is a follow-up after 2323
  (parked), noted in 2323's Out;
- `frame-png` and 2332's raster: they stay at whole scales;
- world layers drawn into the bars (1120), and screen effects (2326);
- window sizes in whole multiples, and a remembered window size (3044);
- a different default per target: the running game doesn't know its
  target yet (3053);
- stretching to fill with uneven pixels;
- other backends: `shuyi-playdate` (3010) shows its display 1:1;
- the game's options screen and saved form (the game's own ticket, if
  it wants the row).

**For the game:** additive. `layout` and `Layout` stay;
`GameSettings::DEFAULT_SCALE` and `GameTools::scale` have defaults;
`Frame` is `#[non_exhaustive]`; `Settings` gains a public field but
already has a private one, so no game builds it field by field. A test
of the game's that matches or builds `Presented::Game` with every field
listed (no `..`) needs `scale` added. The game's PR that moves the pin
follows this one. If 2010 is done, its API diff shows the new items. If
2013 is done, the new public types follow its rule.

## Implementation steps

1. Read `console.rs`, `render.rs`, `backend.rs` (`present`,
   `draw_game`, `draw_card`, `draw_centred`),
   `crates/cli/src/contract/target.rs` and its tests,
   `crates/cli/src/contract/check.rs` (`targets`) and `fake.rs`,
   `crates/cli/src/game.rs`, `crates/platform/src/app.rs` (`run`,
   `present_game`), `headless.rs` and `harness.rs`, and `settings.rs`,
   `toy.rs` and `screens/options.rs` in `shuyi-ui`. Check which of
   2010, 2013, 2110, 2314, 2321, 2346, 2351, 2405 and 3012 are done.
2. Write `ScaleMode`, move `Factor`, write `scale`, `place` and
   `MAX_DRAW_PX`, and put `layout` on `place`, with the tests below.
3. Move `check --target` onto them. Its tests pass unchanged.
4. Add `DEFAULT_SCALE`, `Settings::scale`, `Frame::scale`, the card's
   scale, the headless and `Harness` parts, and the option row, with
   the toy, hello-grid and the `()` form.
5. Draw `Fit` in `shuyi-app`: the render target, the backdrop on it,
   and the card. Move the pointer's inverse if 2405 is done.
6. Add `GameTools::scale`, the comparison and the limit lines, `Fake`,
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

- [ ] `place` gives these for an 800 × 512 frame (a table test):

  | Window | `Integer` | `Fit` (`draw_scale`) |
  | ------ | --------- | -------------------- |
  | 1280 × 800 | 1×: 800 × 512 at (240, 144) | 1250 × 800 at (15, 0) (2) |
  | 1366 × 768 | 1×: 800 × 512 at (283, 128) | 1200 × 768 at (83, 0) (2) |
  | 1920 × 1080 | 2×: 1600 × 1024 at (160, 28) | 1687 × 1080 at (116, 0) (3) |
  | 2560 × 1440 | 2×: 1600 × 1024 at (480, 208) | 2250 × 1440 at (155, 0) (3) |
  | 3840 × 2160 | 4×: 3200 × 2048 at (320, 56) | 3375 × 2160 at (232, 0) (5) |

  Also: a 384 × 384 frame on 1280 × 800 is 2×, 768 × 768 at (256, 16),
  and in `Fit` 800 × 800 at (240, 0) (3). A 1600 × 1000 frame on
  3840 × 2160 in `Fit` has `draw_scale` 2, since 3 would make a
  4800-pixel picture. An 800 × 512 frame on 640 × 400 is 1× at
  (-80, -56), and in `Fit` 625 × 400 at (7, 0) (1).
- [ ] The existing tests of `layout` in `console.rs` pass unchanged.
- [ ] The tests in `crates/cli/src/contract/target/tests.rs` pass
      unchanged.
- [ ] A check whose tools report `integer` against a `fit` target fails
      on `targets."web".scale`. Tools that report `fit` pass. Tools
      that report nothing, and no tool program, each give their limit
      line.
- [ ] An `App` test in `crates/platform/src/app/tests.rs`, through a
      platform that records each frame's `scale` (as `RecordLook`
      records the look): the toy's loading card frame carries `Fit`,
      its `DEFAULT_SCALE`.
- [ ] A `Harness` test: the toy's presented frame carries
      `ScaleMode::Fit`; after `Settings::scale` is set to `Integer`, it
      carries `Integer`. (If step 7 moved the player's pick out: the
      frame carries the toy's `DEFAULT_SCALE`.)
- [ ] Unless step 7 moved the player's pick out:
      `OptionRow::<Toy>::scale()` steps between the two values and sets
      `Settings::scale`; `Settings::restored()` gives back `Fit`; a toy
      settings file without `scale` loads with `Fit`; hello-grid's
      options test (`tests/it/options.rs`) lists `options.row.scale`
      after `options.row.look`.
- [ ] Every existing frame snapshot is unchanged.
- [ ] The PR has screenshots of hello-grid at 1280 × 800 and
      1920 × 1080 in both modes, on the desktop and in the web build.
      In `Fit` the pixels stay sharp: an edge blends over one window
      pixel at most.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

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
- Integration: the `App` and `Harness` tests above. Either order with
  2018 and 2019: if they are done, their desktop and web comparisons
  add a `Fit` window and compare its whole-scale picture (the render
  target, before smoothing) with the raster; if not, they add it.

## Completion notes

*(Filled in by the session that completes the ticket.)*
