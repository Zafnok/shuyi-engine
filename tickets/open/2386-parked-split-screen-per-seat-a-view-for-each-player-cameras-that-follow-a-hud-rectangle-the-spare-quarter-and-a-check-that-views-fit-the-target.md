---
id: "2386"
title: "Parked: Split screen per seat: a view for each player, cameras that follow, a HUD rectangle, the spare quarter, flashes in one view, and a check that views fit the target"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["1120", "2385", "2406", "2339", "2340", "2018", "2392"]
nick_input: none
completed:
---

# 2386 — Split screen per seat

> **Parked** (Nick, 2026-10-24: "These will be parked since they're not needed for Visions of Shuyi"): local split screen waits until Nick unparks it. Visions of Shuyi is turn-based and plays hotseat on one shared view, so nothing here is for the game. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md)):
"add parked tickets for adding local split screen multiplayer, both
vertical and horizontal, and lets say even 3 or 4 player split screen".

The pieces before this ticket:

- **1120**: world layers with a `Camera` (follow, dead zone, clamp,
  damped follow, a shake that decays) and a `viewport: Option<PxRect>`
  each, measured against its viewport. "Split screen repeats a stage's
  layers once per viewport, each with its own camera", by hand.
- **2385**: `split(frame, layout, n, spec)`, the layouts for two, three
  and four players (side by side or stacked for two), the views in
  console pixels and on whole cells, the spare quarter, the dividers'
  rectangles, and the player's pick per player count
  (`Ctx::split_layout(n)`).
- **2406**: seats. `FrameInput::seat(seat)` is one player's input, and
  `FrameInput::seat_events()` says who joined, left, lost their pad or
  came back.
- **2339** and **2340**: the frame's veil, and a flash drawn on it, over
  the whole frame, with the player's flashing settings.

What is left is the part every split-screen game would otherwise write
itself, and get subtly wrong: which seat has which view, keeping each
player's camera when the screen reflows as someone joins, a place for
each player's HUD, what fills the spare quarter with three players,
dividers, a hit flash in one player's view only, and a check that the
views stay big enough on a small screen (the Playdate's 400 × 240 in
four is 200 × 120 each).

Kinds of game that use it: couch racing, top-down arena and co-op
shooters, a two-player platformer, and a strategy game where two local
players scroll their own maps. A grid game that only needs two boards
uses 2385's `split_cells` with the grid kit and nothing of this.

## Nick input

None. Unparking is Nick's call, as for every parked ticket.

## Scope

**In:**

- **`shuyi_ui::split_screen`** (`crates/ui/src/split_screen.rs`), view
  state, pure and `no_std`. New public types follow 2013's ADR:
  `SplitScreen`, `View`, `Spare` and `SplitCheck` are
  `#[non_exhaustive]` with constructors.
  - `SplitScreen::new(spec: SplitSpec)`: no views until a seat fills.
  - `SplitScreen::update(&mut self, frame: PxRect, seats: &[Seat],
    events: &[SeatEvent], ctx: &Ctx)`, once a frame, before drawing:
    - the filled seats, in seat number order, take the layout's views
      in reading order (seat 0 top-left). With one seat there is one
      view of the whole frame, so a game is the same with one player
      as without split screen;
    - the layout is `ctx.split_layout(n)` (2385) for the n filled
      seats. A change of n, of the player's pick or of the frame's size
      redoes the views (a reflow);
    - a seat keeps its `Camera` across a reflow: its position is kept
      and clamped again to the new viewport's size, so no view jumps
      more than the clamp asks. A seat that joins gets a camera at its
      first follow target, with no damping on that first step;
    - a lost seat (`DeviceLost`) keeps its view, so the game can draw
      "reconnect" there; a seat that leaves gives its view up at once.
  - `SplitScreen::follow(&mut self, seat, target: FxPoint)`, each
    simulation step, the place that seat's camera follows with 1120's
    dead zone and damping. `SplitScreen::camera_mut(seat)` gives the
    rest of 1120's camera (shake, bounds, zoom) per seat.
  - `SplitScreen::views() -> &[View]`: per view its `seat`, its
    `rect: PxRect`, its `cells: Rect` when the spec aligns to cells (the
    view's HUD rectangle, for `GlyphBuffer` writes), and its `camera`.
  - `SplitScreen::layers(&self, build: impl FnMut(&View) ->
    Vec<WorldLayer>) -> Vec<WorldLayer>`: the stage's layers once per
    view, each given that view's viewport and camera, in view order.
    This is the one line a stage writes in place of 1120's repetition
    by hand.
  - **The spare quarter** (`Grid` with three players): `Spare::Blank`
    (the default, the palette's background), `Spare::Overview { bounds:
    FxRect }` (one more camera framing every filled seat's target at
    the largest whole zoom from 1 to 4 that fits them all, clamped to
    `bounds`, drawn from the same `build` with a `View` whose seat is
    `None`), or `Spare::Game` (the game draws into
    `SplitScreen::spare()`, a `PxRect` and cells).
  - **Dividers**: `SplitScreen::dividers()` gives 2385's rectangles,
    and the split screen draws them, between the world layers and the
    cells, in a palette role the game picks (the frame's ink by
    default). On one-bit targets they are the ink, as 2371 draws lines.
    A spec with a divider of 0 draws none.
  - **A flash in one view**: `SplitScreen::flash(seat, Flash)` (2340's
    `Flash`) shows on that seat's view only: the veil (2339) drawn
    clipped to the view's rectangle, by every painter. The player's
    flashing settings and 2340's photosensitivity rules apply to it as
    to a whole-frame flash: a flash in one view counts toward the same
    limits, so four views flashing in turn don't pass what one may.
    1120's shake per camera already moves one view only.
- **Snapshots**: a line `--- split: <layout>, <n> views ---`, then one
  line per view, `view <i>: seat <s> at x,y wxh`, then the spare and
  each divider, after the world layers' lines
  (`crates/ui/src/frame/snapshot.rs`). Each world layer's line already
  names its viewport (1120). A frame with no split screen has no such
  lines, so no existing snapshot changes.
- **Painting cost**: each painter (the raster and `shuyi-app`) skips a
  world item whose bounds lie wholly outside its layer's viewport,
  instead of only clipping it. A test counts the items the raster draws
  for a scene split in four and finds each drawn only in the views that
  see it.
- **The check per target**, in `shuyi check --target <id>`:
  - `SplitPrefs::min_view(w, h)` (on 2385's `SplitPrefs`): the
    smallest view, in console pixels, the game plays in; none by
    default, so a game that sets nothing is never failed;
  - for each player count from 2 to the smaller of the game's
    `GameControls::with_seats` maximum and the target's
    `local_players` (2406), and each layout the game allows for it,
    the check splits the game's frame on that target (its console size
    there, as the check already finds it for text sizes in
    `crates/cli/src/contract/target.rs`) and compares the smallest view
    with `min_view`;
  - a layout whose views are too small is a `SplitTooSmall` warning
    naming the count, the layout and the sizes, and the player's option
    row (2385) hides it on that target. A player count with no layout
    left is a `SplitNoLayout` error;
  - `controls` in the contract's `target` answer lists the layouts kept
    per count, as new values in the answer, not a new shape.
- **Docs and skills**: the frame's page in `docs/` (what the `document`
  skill says), and the `shuyi-stage-field` skill
  (`skills/shuyi-stage-field/SKILL.md`): a section on split screen,
  from seats to views to `layers`, with its worked example extended to
  two players. `check-skills` passes.

**Out (do not do):**

- the layouts' maths and the player's pick (2385);
- a listener per view (2387);
- a join screen with a slot per seat, and a "reconnect your pad"
  prompt: the game draws them in its view from `seat_events` (2406's
  Out says the same);
- a HUD system of its own: a view gives its rectangle in pixels and
  cells, and the game draws there with what the frame has;
- a camera of the camera-feel kind (look-ahead, rooms, blends): 2362's
  work runs per camera, so each view gets it with nothing here;
- an animated reflow (views sliding to their new places), and splits
  that move with the players;
- a different text size or palette per view;
- the example (2774) and the template part (2775).

## Implementation steps

1. Read 1120's camera and world layer as they landed, 2385's `split`
   and `SplitPrefs`, 2406's seats and events, 2339's veil and 2340's
   flash and photosensitivity rules.
2. Write `SplitScreen`: views per seat, the reflow and the cameras that
   survive it, `follow`, `layers`.
3. The spare: blank, overview (the zoom that fits), and the game's.
4. Dividers and the flash in one view, in every painter; culling per
   viewport.
5. The snapshot lines.
6. `min_view`, the check, the hidden option rows and the contract's
   values.
7. Docs and the skill.
8. Run the gates.

## Acceptance criteria

- [ ] A Harness test (`crates/ui/tests/it/split_screen.rs`, with a test
      stage of a world bigger than the frame and one dot per seat
      under `JoinOnPress`): pads 0 to 3 join in turn, the snapshot shows
      one, two, three and four views in the player's layouts, and each
      view's camera follows its own dot.
- [ ] The same test with "Stacked" picked for two players gives a top
      and a bottom view; with "Side by side", a left and a right one.
- [ ] Seat 1 leaving with three seated reflows to two views, and seat
      2's camera is where it was, clamped to its new view (no jump
      beyond the clamp).
- [ ] With three seats in `Grid`, `Spare::Overview` frames all three
      dots at a whole zoom; `Spare::Blank` paints the background.
- [ ] A flash on seat 1 changes only pixels inside seat 1's view in the
      raster, and none with the player's flashing turned off.
- [ ] With one seat, the frame's snapshot and `frame-png` picture equal
      the same stage drawn without `SplitScreen`.
- [ ] `frame-png` pictures of the test stage at two (both layouts),
      three (every three-player layout) and four players match their
      checked-in files, and the window draws the same pixels: the
      four-player scene goes on 2392's render card
      (`shuyi_ui::frame::raster::card`) and the `render` CI job passes.
- [ ] `shuyi check --target playdate` on a fixture game with
      `min_view(240, 160)` and four seats warns `SplitTooSmall` for
      every four-player layout there and fails with `SplitNoLayout`;
      on `desktop` it passes.
- [ ] No existing snapshot changes.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: seats to views in order; reflow on join, leave and a changed
  pick; a camera kept across a reflow; a lost seat keeping its view;
  the overview's zoom; dividers' roles; the flash clipped to its view;
  `SplitTooSmall` and `SplitNoLayout`.
- Property: for any sequence of joins and leaves of up to four seats,
  every filled seat has exactly one view, views follow seat order, and
  views, spare and dividers cover the frame with no overlap.
- Snapshot / integration: the Harness tests above, the `frame-png`
  pictures, the culling count, and `check --target` on the fixture.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
