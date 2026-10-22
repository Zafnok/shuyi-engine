---
id: "2361"
title: "Parked: World layers drawn between the last two steps, in the view only, so tick stages move smoothly on displays faster than 60 Hz"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119", "1120"]
nick_input: none
completed:
---

# 2361 — World layers drawn between the last two steps

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (its ticket 0060 says the army walks around freely, which unparks 1119 and 1120; a free-moving field then judders at the 144 and 165 caps that 2321 offers). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Those engines step their simulation at a fixed rate and
can draw between the last two steps, so motion stays smooth on a display
faster than the step rate:

- Godot has physics interpolation in 2D and 3D, read through
  `Engine.get_physics_interpolation_fraction()`
  (docs.godotengine.org/en/stable/tutorials/physics/interpolation/physics_interpolation_introduction.html);
- Bevy's `Time<Fixed>` gives the overstep, the part of a step left
  over, and the transform interpolation crates build on it
  (docs.rs/bevy/latest/bevy/time/struct.Fixed.html);
- Unity has interpolation on `Rigidbody2D`.

What Shuyi has today:

- `crates/platform/src/tick.rs`: `TickClock` keeps the time not yet
  stepped in a private `owed`, in sixtieths of a nanosecond, so one step
  is `SECOND` of them. `advance` hands out whole steps only. A frame
  with more than `MAX_STEPS` due drops the rest but keeps the part of a
  step it was into.
- `crates/platform/src/app.rs`: `Clock` fills
  `Elapsed { now, dt, ticks }`, and `step` calls
  `game.set_ticks(elapsed.ticks)` (`crates/ui/src/game.rs:241`). A
  screen reads `FrameInput::ticks()` (`crates/ui/src/input/frame.rs:114`).
  Nothing tells a screen how far into the next step a frame is.
- ADR-0003 §2 ends: "Drawing reads the latest state; there is no
  interpolation". ADR-0015 §2 repeats it, and its Alternatives lean on
  it. ADR-0024 §4 says the steps reach screens through `ticks()`.
- `Backdrop::scene_offset(scale)`
  (`crates/ui/src/frame/glyph_buffer/backdrop.rs`) already places a
  panned scene at a fraction of a scene pixel, rounded once to whole
  window pixels. Every painter calls it: `crates/app/src/render.rs`,
  `crates/cli/src/frame_png.rs` and `crates/ui/src/frame/onebit.rs`.
  This ticket does the same for world layers, in integers.
- `frame-png`'s script (`Script` and `Step` in
  `crates/cli/src/frame_png.rs`) reaches every game the same way: each
  game's `GameTools::frame` plays it with `Script::play`. The contract's
  `run` reads the same steps (`parse_args` in
  `crates/cli/src/contract/run.rs`).

What is planned: 1119 (`TickSim`, `TickDriver`, `Fx`, `Arena`, and the
module `shuyi_ui::tick`), 1120 (`WorldLayer`, `Camera`, sprite items in
world pixels), 2321 (frame caps of 30, 60, 120, 144, 165, 240 and Off)
and 2123 (game speed, which changes the units of `owed`). 1147's
snapshot interpolation is a network technique, not this. Nothing plans
drawing between steps.

Why it matters:

- A tick stage steps 60 times a second. On a faster display each step
  is shown for a whole number of frames. At 120 and 240 Hz that is even
  (2 or 4 frames a step), and 60 with VRR needs nothing. At 144 or
  165 Hz, which 2321 offers, a step shows for 2 frames, then 3, in turn.
  A scrolling camera judders: it looks worse than the 60 cap and costs
  more CPU. Nick's rule of 2026-10-19 says the cap "adds no stutter" with
  vsync or VRR; this keeps that true for tick stages.
- The game first: Visions of Shuyi is turn-based, and its screens
  animate by `dt`, which already gains from caps above 60. It needs this
  only if its 0060 adds free movement.
- Other 2D games: platformers and top-down games that scroll (1125,
  1127) show it first. Every benchmark above ships it.
- The end goal of 2026-10-19 (snake in few lines) is untouched: this is
  off by default, and one field turns it on for a layer.

## Nick input

None. The sentence this supersedes is Claude's own technical choice
(ADR-0003 §2); `docs/decisions.md` says nothing about interpolation.

## Scope

**In:**

- **An ADR** (`write-adr` skill). It supersedes the last sentence of
  ADR-0003 §2, amends ADR-0015 §2 (the "no interpolation" line and its
  Alternatives entry) and adds a line to ADR-0024 §4. It says:
  - the simulation stays at 60 steps a second; steps, `TickInput`,
    records and hashes don't change;
  - a world layer may be drawn between the last two steps, in the view
    only. The stage keeps the layers it built after the last two steps,
    and a pure function in `shuyi-ui` blends them. The painters keep no
    state between frames, and the blended layer is plain integer data
    (`Fx` positions), like the layer 1120 builds. Floats stay in the
    backends' draw calls, as ADR-0003 §3 says;
  - it is off by default and chosen per world layer. A smoothed layer is
    drawn up to one step (16.7 ms) behind the newest state;
  - it is never on for a 1-bit display or a backend that shows frames
    at a fixed rate;
  - no extrapolation.

  The status lines of ADR-0003 and ADR-0015, and their rows in
  `docs/adr/README.md`, point at it.
- **The overstep: how far a frame is into the next step.**
  - `shuyi_ui::tick::Overstep`: thousandths of a step, 0 to 999, in a
    `u16`. `Overstep::per_mille(n) -> Option<Overstep>` (`None` above
    999), `Overstep::ZERO`, and `Overstep::lerp(self, from: Fx, to: Fx)
    -> Fx`, also for `FxVec2`: `from + (to - from) * n / 1000` on the
    raw values in `i64`, rounding toward negative infinity as `Fx` does.
    It lives in `shuyi-ui`, which the core doesn't depend on, so no
    `TickSim` can read it.
  - `TickClock::overstep(&self) -> Overstep`: `owed * 1000 / SECOND`. A
    frame that drops time (more than `MAX_STEPS` due) now also forgets
    the part step (`owed = 0`), so the overstep is 0 after it. The doc
    comments of `advance` and the module say so; the existing tests in
    `tick.rs` still pass.
  - **Game speed. Either order with 2123:** 2123 scales the time
    `TickClock` owes by the speed, so once 2123 is done a step is
    `STEP` (`SECOND * 256`) units of `owed`. `overstep` divides `owed`
    by one step in whatever units `TickClock` has when this lands, and
    the dropped-time rule above uses 2123's `max_steps(speed)` if it
    exists. At `GameSpeed::STOPPED` nothing is added, so the overstep
    holds by itself, and slow motion is drawn smoothly. Hit-stop needs
    nothing here (*Holding still*, below). If 2123 is done, add its two
    tests: from an empty clock at `QUARTER`, frames of 1/60 s give an
    overstep of 250, 500, 750 and 0 in turn; at `STOPPED`, 30 frames of
    1/144 s with `BetweenSteps::Clock` draw the same frame. If 2123 isn't
    done, 2123 adds them (its text already says so).
  - `Platform::between_steps(&self, clock: Overstep) -> Option<Overstep>`
    in `crates/platform/src/platform.rs`, with a default body returning
    `None` (draw the newest step). `shuyi-app`'s `Macroquad`
    (`crates/app/src/backend.rs`) returns `Some(clock)`. Backends that
    show frames at a fixed rate (the Playdate, the parked GBA and DS)
    keep the default and need no change.
  - `Elapsed` gains `overstep: Overstep`, the clock's after `advance`.
    `step` turns it into what screens see: `None` when
    `platform.display().color` is `Color::OneBit`, whatever the backend
    says; otherwise `platform.between_steps(elapsed.overstep)`. It hands
    that to a new `Game::set_overstep(Option<Overstep>)`, called beside
    `set_ticks`.
  - `FrameInput::overstep() -> Option<Overstep>` beside `ticks()`, set
    by `with_overstep`. Unlike the ticks, `Game` keeps the last value,
    so an update with no time passing (`Game::act`) sees the same
    overstep as the frame before, and the picture doesn't jump.
  - `HeadlessPlatform::with_between_steps(BetweenSteps)`, a new enum in
    `crates/platform/src/headless.rs`: `Off` (the default, so every
    existing test is unchanged), `Clock` (as `shuyi-app`), or
    `Pinned(Overstep)`. `Harness::between_steps(&mut self, BetweenSteps)
    -> &mut Self` sets it on the Harness's platform.
  - `docs/platform-backends.md`: the new method under *Each method*, and
    a line in *What one frame does*.
- **World layers that blend**, on 1120's types in `shuyi_ui::frame`:
  - `Smoothing` on `WorldLayer`: `Off` (the default), `ScreenPixel` (the
    blend placed to the window pixel) and `WorldPixel` (the blend
    floored to whole world pixels with `Fx::floor_px`, for pixel art that
    must stay on its grid).
  - Sprite items gain `key: Option<ItemKey>` (`ItemKey(u64)`, with
    `From<Id<T>>` for 1119's arena ids, packing the index and the
    generation) and `snap: bool` (it teleported this step: draw it where
    it is now). The camera gains `snap: bool` (a cut).
  - `blend(prev: &WorldLayer<P>, now: &WorldLayer<P>, at: Option<Overstep>)
    -> WorldLayer<P>`, pure. It returns `now` with:
    - the camera at `lerp(prev, now)` of its final position (after
      follow, clamp and shake), unless `now`'s camera is snapped;
    - each keyed item at `lerp` of its position in `prev` and in `now`;
    - an item with no key, a key new in `now`, or `snap` set, where it
      is in `now`; a key missing from `now` is gone.

    Everything else (images, sort keys, flips, opacity, animation
    frames) is `now`'s. `None`, or a layer whose `Smoothing` is `Off`,
    gives `now` unchanged. A key that repeats in one layer panics in a
    debug build and draws at `now` in a release build.
  - **The camera first.** Write and test the camera's blend before the
    items': scrolling is where the judder shows, and it needs no keys.
  - **Cuts. Either order with 2362:** 2362's `Camera::cut_to` and
    `RoomMove::Snap` move the camera at once, so on that step they must
    set its `snap`. If 2362 is done, set it there in this ticket. If
    not, add a line to 2362's Scope in this PR saying so.
  - **Tile layers. Either order with 2360:** 2360's `TileLayerItem` is
    placed by the same function as sprite items, so it moves with the
    blended camera. Its `visible` rectangle (from `now`'s camera, with
    its tile of margin) covers the blend while the camera moves less
    than a tile a step. If 2360 is done, the cadence test below includes
    a tile layer. If not, 2360 needs nothing from this ticket.
  - `shuyi_ui::tick::StepViews<P>`: keeps the world layers
    (`Vec<WorldLayer<P>>`) a stage built after each of the last two
    steps. `push(layers)` after each step, `set_overstep(..)` in the
    stage's update (from `FrameInput::overstep()`), and
    `drawn(&self) -> Vec<WorldLayer<P>>` in its draw, which blends the
    layers pair by pair by index; a layer with no partner in `prev` is
    drawn as it is. It doesn't need `TickDriver`, so 2706's
    `CommandDriver` (parked) can use it too.
  - **Holding still.** A stage that isn't updated (under a pause
    overlay, during 2316's pause or between its slow-motion updates)
    keeps the overstep it last read, so the picture holds still.
    Hit-stop needs nothing: 2123's `Freeze` steps build the same layers
    twice, so the blend holds still by itself.
- **Placed once, in window pixels.**
  `WorldLayer::window_px(&self, at: FxVec2, scale: u32) -> (i64, i64)`
  in `shuyi-ui`, built like `Backdrop::scene_offset`: where a world
  point is drawn, in window pixels right of and below the layer's
  top-left, when a console pixel is `scale` window pixels. It uses the
  placement 1120's painters use (camera, parallax, zoom), carried in
  `Fx` all the way to window pixels and rounded once there, halves up.
  - Both painters place a smoothed layer's items with it: `shuyi-app`'s
    renderer, and the software painter (2332's `shuyi_ui::frame::raster`
    if 2332 is done; else `Painter` in `crates/cli/src/frame_png.rs`).
  - A layer with `Off` is placed exactly as 1120 places it, so its
    pixels don't change.
  - The scale is always whole: 2333's `Fit` mode draws the frame at a
    whole `draw_scale` and stretches only the result.
  - The 1-bit painter never gets a smoothed layer and doesn't change.
- **Snapshots:** a layer with `Off` writes the same lines as before. A
  smoothed layer's line names its `Smoothing`, and its items' and
  camera's lines show the blended positions as `Fx`, with their
  fractions.
- **A script step that pins the overstep.** `Step::Overstep(u16)` in
  `crates/cli/src/frame_png.rs` (serde name `overstep`):
  - `Script::play` handles it by calling
    `h.between_steps(BetweenSteps::Pinned(..))` and then running one
    frame in which no time passes (`h.frame(&[], 0.0)`), so the
    picture shows it at once. Every later frame of the script keeps it.
  - `Step::check` refuses a value above 999, naming the range.
  - `Script::parse_step` reads `--overstep N`. The flag lists in
    `frame_png::parse_args` and in `contract::run::parse_args` gain
    `--overstep`, and `usage` lists it.
  - So every game's `GameTools::frame` and the contract's `run` get it
    with no change to `GameTools`.
  - The step changes the contract's schemas: raise `contract::VERSION`
    and add the line to `crates/cli/schemas/versions.json`, as
    `docs/contract.md` says.
  - **1133** (done). Its MCP tools take a script, so they play the step
    with nothing more. Its scripted client test
    (`crates/cli/tests/it/mcp.rs`) gains one `overstep` step.
- **The pointer. Either order with 1121:** whatever reads a pointer in
  world pixels (1121) must use the camera as drawn, so a click lands
  where the player saw things. If 1121 is done, make it so and test a
  click on a smoothed layer. If not, add a line to 1121's Scope in this
  PR saying so.
- **No API break for the game.** `Platform` gains a method with a
  default body. `FrameInput`, `Game`, `HeadlessPlatform` and `Harness`
  gain methods. `frame_png::Step` gains a variant; no game in
  `templates/` or `examples/` matches on it, and the PR says so. 1120's
  types are parked work the game doesn't use. New public types follow
  2013's ADR (in the MVP).
- **Size.** If the PR passes about 600 changed lines (not counting
  snapshots and schema files), item keys and the items' blend (step 6)
  go to a follow-up ticket, parked like this one and written in this PR.
  This PR then keeps the camera's blend.

**Out (do not do):**

- glyph cells, `Layer::Under` and `Layer::Over` items, backdrops and
  text: they don't step;
- extrapolation (drawing past the newest step);
- turn stages: they animate by `dt` and never read the overstep;
- blending angles (2335), scale, opacity, colours, animation frames
  (1123, 2231, 2359) or sort keys: they show the newest step;
- camera feel: look-ahead, rooms that confine it, blends between cameras
  (2362);
- game speed and hit-stop (2123), beyond the either-order part above;
- debug gizmos over world layers (2317, 2318) that follow blended
  sprites: a follow-up; until then a developer turns smoothing off to
  line them up;
- 3D views and cameras (3103, 3106): a follow-up if 3D is unparked;
- smoothing rollback corrections in networked play (1147), beyond
  `snap`;
- a lower step rate (ADR-0015's buckets stay);
- the frame cap itself (2321).

## Implementation steps

1. Read ADR-0003 §2 to §5, ADR-0015 §2, ADR-0024 §4, 1119's `Fx` and
   `shuyi_ui::tick`, 1120's `WorldLayer`, `Camera`, painters and
   snapshot lines, `Backdrop::scene_offset`, and `Script` and `Step` in
   `crates/cli/src/frame_png.rs`.
2. Write the ADR and the status lines of ADR-0003 and ADR-0015.
3. `Overstep`, `TickClock::overstep` and the dropped-time rule, with
   unit tests in `tick.rs`. The 2123 part, if 2123 is done.
4. `Platform::between_steps`, `BetweenSteps` on the headless platform,
   the Harness setter, `Elapsed`, `Game::set_overstep`,
   `FrameInput::overstep`, and `docs/platform-backends.md`.
5. `Smoothing`, the camera's `snap` and blend, `StepViews`, and
   `WorldLayer::window_px` in both painters.
6. Item keys, `snap`, and the items' blend.
7. The snapshot lines, `Step::Overstep` and its flag, and the contract's
   version.
8. A smoothed variant of 1120's test scene, where `frame-png` reaches
   1120's scene (the toy, if 1120 put it there). Its followed sprite
   moves 2 world pixels right each step of a `TickSim` driven by
   `TickDriver`; the camera follows it with no dead zone and no clamp,
   at zoom 1; the stage pushes its layers into `StepViews` after each
   step. If 1120's scene doesn't step, give it a sim of that one sprite.
9. The either-order parts for 2362, 2360, 1121 and 1133, and the tests
   below.
10. Run the gates.

## Acceptance criteria

- [ ] Off by default: every snapshot and `frame-png` picture in the
      repository, 1120's test scene included, is unchanged.
- [ ] Determinism: the smoothed scene played by the Harness for 600
      frames of 1/144 s with `BetweenSteps::Clock` gives the same hash
      list per step, and the same steps per frame, with every layer
      `Off` and with every layer `ScreenPixel`.
- [ ] The judder, measured, in that run at scale 3, from the window
      position of a sprite that doesn't move: with `Off`, it changes
      only on frames that ran a step, 2 or 3 frames apart; with
      `ScreenPixel`, it changes on every frame once two steps exist, and
      the largest and smallest change differ by at most one window pixel.
- [ ] `TickClock::overstep`: a frame of 10 ms gives 600; a frame of
      110 ms (6.6 steps, more than `MAX_STEPS`) gives 0; after `reset`,
      0.
- [ ] If 2123 is done: its two overstep tests pass (`QUARTER` gives 250,
      500, 750 and 0; `STOPPED` draws the same frame 30 times).
- [ ] `blend`: overstep 0 gives `prev`'s positions; `None` gives `now`;
      `snap`, unkeyed and new items are at `now`; an item gone from
      `now` isn't drawn.
- [ ] A headless platform on `DisplayInfo::one_bit()` with
      `BetweenSteps::Clock` hands screens `None`.
- [ ] On the smoothed scene, after some steps, a script ending in
      `Step::Overstep(0)`, `(500)` or `(999)` gives Harness snapshot lines
      with the camera at `prev`'s position, halfway, and 999 thousandths
      of the way to `now`'s. `frame-png --overstep 500` writes a picture
      that differs from `--overstep 0`'s. `--overstep 1000` is an error
      naming the range.
- [ ] With a screen pushed over the smoothed stage, 30 frames of 1/144 s
      with `BetweenSteps::Clock` give the same snapshot each time (and
      during 2316's pause, if 2316 is done).
- [ ] `Smoothing`'s doc says that 120 and 240 Hz are already even, that
      60 with VRR needs nothing, and that a smoothed layer is drawn up to
      one step behind.
- [ ] The contract's schemas test passes with the new `VERSION`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Overstep::per_mille` bounds and `lerp` rounding (moving left and
  up too); `TickClock::overstep`; each `blend` case; `StepViews` pushing
  and holding; `WorldLayer::window_px` rounding halves up, for positive
  and negative positions, at scales 1 to 4; `Step::Overstep` parsing
  and its range check.
- Property: for any two layers and any overstep, each blended keyed item
  lies between its `prev` and `now` positions on each axis, inclusive;
  `blend(prev, now, None) == now`; `blend` changes nothing but
  positions.
- Snapshot / integration: the hash lists on and off; the cadence test;
  the overstep steps and `frame-png --overstep`; `None` on a 1-bit
  display; the pushed screen holding still.
- Manual: if a display faster than 60 Hz is at hand (VRR off, the cap
  at its rate), run 1125's town or 1127's platformer, if done, with
  smoothing on and off, and say in the PR what you saw. If none is at
  hand, say so.

## Completion notes

*(Filled in by the session that completes the ticket.)*
