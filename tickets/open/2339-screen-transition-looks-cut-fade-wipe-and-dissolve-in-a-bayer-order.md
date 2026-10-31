---
id: "2339"
title: "Screen transition looks: cut, fade through a colour, wipe and dissolve in a Bayer order, played from the last frame on every painter"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2339 — Screen transition looks

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Unreal's Common UI changes widgets with a transition
animation, which a game can turn off
(<https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Plugins/CommonUI/UCommonAnimatedSwitcher>).
Godot, Bevy and Unity leave a fade between scenes to the game's own
code (a `ColorRect` and an `AnimationPlayer`, a system, a coroutine).
Outside the benchmarks, 2D frameworks such as Phaser and HaxeFlixel
give their cameras a fade.

What Shuyi has today (checked 2026-10-20):

- `screen::Transition` (`crates/ui/src/screen.rs`) is `None`, `Push`,
  `Pop`, `PopWith`, `Replace` or `Quit`. `ScreenStack::apply_in`
  changes the stack at once, and `Game::redraw`
  (`crates/ui/src/game.rs:473`) clears the buffer, draws the new
  stack, then lets each module that draws over the screens draw (2358,
  done). Every change of screen is a cut.
- `GlyphBuffer::fade` (`crates/ui/src/frame/glyph_buffer.rs:720-748`)
  darkens a whole frame toward black, items and backdrop included. The
  cinematic player uses it for one brightness. Nothing fades toward
  another colour: an `Overlay` has no opacity, and a sprite can't be
  tinted toward white.
- The one-bit renderer already fades a sprite's pixels out in a 4 × 4
  Bayer order (`BAYER` and `shows`, `crates/ui/src/frame/onebit.rs:975-983`).
  A colour fade drawn there is a cut at half brightness, since each
  colour is black or white (`ink_of`).
- `cinematic::ease_in_out` (`crates/ui/src/cinematic.rs:160`) is a
  smoothstep in `f32`.
- `Ctx::idle_motion` (the field at `crates/ui/src/ctx.rs:266`, of type
  `IdleMotion` at 163-171) is `Off` from `Ctx::new`, so in every
  Harness, and `Setup::ctx` (`crates/platform/src/app.rs:140`) turns it
  `On` on a real platform. No engine screen reads it yet. The game's
  frame tool turns it on too.
- `frame-png` plays its steps in the Harness (`Harness::keys` runs a
  frame for the press and one for the release, 1/60 s each), and a
  game's `GameTools::frame` reports the top screen's name in
  `Shot::shows` (`crates/cli/src/frame_png.rs:251`).
- [ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)
  §1 says the engine "gives the transition (fades, 0813)" between
  stages. Nothing in the engine does.

Planned: 2340 adds shake, flash and the player's motion settings on top
of this ticket. 2364 (parked) builds a cinematic shot's `enter` on these
looks. 2722 (parked) has a visual novel's own picture transitions.

Why it matters:

- **The game first.** game#0813
  ([`0813-screen-transitions.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0813-screen-transitions.md))
  asks for transitions between its screens: a cut, a fade to black or a
  wipe of cells, which Nick picks there. It wants them over the glyph
  buffer, covering every item, with input ignored while one plays. With
  this ticket the game's choice is one call where its screens change.
  The game also fades in its own code today; moving that is the game's
  choice after a pin bump.
- **Other 2D games.** A fade from the title into play, a dissolve into a
  battle, a wipe between rooms. On the Playdate a fade must dither.
- **The end goal** (2026-10-19, `docs/decisions.md`): an agent asks for
  a fade in one call, and `frame-png` shows the frame half way.

Graphics are a skin ([ADR-0002](../../docs/adr/0002-inherited-rules-layers-determinism-skins-actions-tests-licenses.md)
§4, `AGENTS.md` rule 9). A look changes what is painted, never what
happened: tests of what happened read the stack, the state and the
scene.

## Nick input

None. Which look the game uses stays Nick's choice in game#0813.

## Scope

**In:**
- **`shuyi_ui::frame::transition`** (`crates/ui/src/frame/transition.rs`,
  `no_std`, integers only):
  - `TransitionLook`, plain data (`Copy`, `Eq`):
    - `Cut`, the default;
    - `Fade { ms: u16, through: Rgb }`;
    - `Wipe { ms: u16, edge: Edge, through: Option<Rgb> }`;
    - `Dissolve { ms: u16, through: Option<Rgb> }`;
    - constructors `fade(ms, through)`, `wipe(ms, edge)`, `dissolve(ms)`
      and `through(self, Rgb)`; `name()` (`"cut"`, `"fade"`, `"wipe"`,
      `"dissolve"`) and `ms()`. A look of 0 ms is a cut.
  - `Edge { Left, Right, Top, Bottom }`: the edge the new frame comes in
    from.
  - `progress(elapsed_ms: u32, ms: u16) -> u16`: thousandths, eased in
    and out with `ease_in_out`'s curve in integers. With `p` the linear
    thousandths (`elapsed_ms × 1000 / ms`, at most 1000), it is
    `p² × (3000 − 2p) / 1 000 000` in `u32`. It is 1000 when `ms` is 0
    or the time is up. **Either order with 2109:** if 2109's
    `shuyi_core::ease` is done, use `Ease::Smooth` through its
    `Easing::ease` (1/65536 in and out, scaled from and to
    thousandths); if not, this private formula, and 2136 moves it
    onto `Ease::Smooth`, tested within one thousandth of it (its
    clause on 2339 says so). 2136 is blocked by 2109, so if 2136
    lands first, use `Ease::Smooth` as above.
  - `compose(look, from: &GlyphBuffer, to: &mut GlyphBuffer, permille:
    u16)`, pure: `to` holds the new frame and becomes what shows at that
    progress. Two buffers of different `spec`s compose as a cut.
    - `Fade`: below 500, `to` becomes a copy of `from` under a veil of
      `through` (below) whose alpha rises from 0 to 255; from 500, `to`
      as drawn, under a veil falling from 255 to 0.
    - `Wipe`: the first `permille × cols / 1000` columns from its edge
      show `to` (for `Top` and `Bottom`, the first
      `permille × rows / 1000` rows); the rest are copied from `from`
      with `copy_from` (below).
    - `Dissolve`: a cell shows `to` where its rank in the 4 × 4 Bayer
      order (`BAYER[y & 3][x & 3]`) is below `permille × 16 / 1000`, so
      all of it at 1000 and none at 0. Make `onebit.rs`'s `BAYER`
      `pub(crate)` and use it here; `onebit/reference.rs` keeps its own
      copy, as the reference stays apart from the fast renderer. Copy
      the cells that still show `from` in one pass, cutting `to`'s
      items once against all of them, not with one `copy_from` per
      cell: an 80 × 30 frame would otherwise rebuild the item list up
      to 2400 times a frame.
    - `through` on `Wipe` and `Dissolve`: two halves. Below 500 the
      look runs at `permille × 2` from `from` to a frame of blank cells
      in that colour (`Cell::new(' ', c, c)`, no items, no backdrop);
      from 500 it runs at `(permille − 500) × 2` from that frame to
      `to`.
    - It sets the frame's transition note (below).
- **The frame** (`crates/ui/src/frame/glyph_buffer.rs`):
  - `Veil { color: Rgb, alpha: u8 }`, with `GlyphBuffer::set_veil(Option<Veil>)`
    and `veil()`: one colour over the whole frame, drawn last, over the
    backdrop, the cells and every item, and never cut by cells. Filling
    the whole buffer with a solid cell (how `Game::redraw` clears a
    frame) removes it, as it removes the backdrop. `Game::redraw` also
    sets it to `None` itself after the fill, since a game's blank cell
    may be see-through.
  - `GlyphBuffer::copy_from(&mut self, other: &GlyphBuffer, rect: Rect)`:
    `other`'s cells of `rect`, at the same place, with `other`'s items
    clipped to those cells' pixels (a sprite keeps its `src`, `dest` and
    `base`; its `clip` shrinks). This buffer's items there are cut first
    (`cut_items`). It is `blit` for a part, in place.
  - `TransitionNote { look: &'static str, permille: u16 }`, set with
    `set_transition_note(Option<TransitionNote>)`, read only by the
    snapshot. Painters ignore it. The full solid fill and
    `Game::redraw` remove it as they remove the veil.
  - A see-through cell copied from `from` shows `to`'s backdrop. The
    doc says so: where the backdrop changes with the screen, a `Fade`
    looks right.
- **Snapshots** (`crates/ui/src/frame/snapshot.rs`): a line `--- veil:
  colour alpha ---` (the colour as the legend prints it, a palette name
  or `#rrggbb`) and a line `--- transition: dissolve 375/1000 ---`,
  after the overlays and before the backdrop, each only when present.
  The module doc lists them. No existing snapshot changes.
- **Every painter draws the veil, last:**
  - `shuyi-app` (`Renderer::draw`, `crates/app/src/render.rs:117`): one
    rectangle over the console's place in the veil's colour at
    `alpha / 255`;
  - `frame-png` (`Painter::render`, `crates/cli/src/frame_png.rs:437`):
    every pixel blended with it through `Image::blend`. **Either order
    with 2337:** if done, with `Blend::mix` and `Blend::Alpha`;
  - the one-bit renderer (`onebit.rs` and `onebit/reference.rs`, kept
    equal): a pixel where `shows(alpha, x, y)` takes the veil colour's
    shade (`shades.shape(color)`, as a rectangle's), so a fade dithers
    in the order sprites already fade in. `render_changed` redraws the
    whole screen when the veil changed.
  - **Either order with 2389** (the inks removed): if 2389 is done, the
    renderer's only `Shades` is a `PatternMap`, and the one-bit veil
    tests and their text snapshot use 2389's pattern map helper. If
    not, the veil goes through `Shades` as above, never `ink_of`, and
    2389 moves this ticket's tests that use `InkShades` with the rest.
  - The veil goes in `shuyi_ui::frame::raster` (2332), as the reference
    the backends match.
- **Asking for a look:** `Ctx::transition_look(&mut self, look)`, which
  keeps it in a new private field of `Ctx` until `Game` takes it. A
  screen asks in the update in which it returns its `Transition`, as it
  asks for a sound through `ctx.audio`:

  ```rust
  ctx.transition_look(TransitionLook::fade(400, ctx.palette.engine(EngineColor::Background)));
  Transition::Replace(Box::new(Play::new(ctx)))
  ```

  - Not a new `Transition` variant: a variant wrapping a transition
    would hide `PopWith` from a game's own `match` (the game's flow
    matches `Transition::PopWith(result)`), and changing every variant
    breaks every screen.
  - A look asked for with no change of screen still plays, between the
    last frame and the next: a screen that changes all it shows (a map
    giving way to a battle on the same screen).
- **`Game` plays it** (`crates/ui/src/game.rs`):
  - In `redraw`, a look asked for keeps the screens' last drawing as
    the outgoing frame, and starts at `ctx.clock_s`. Each redraw draws
    the stack, then `compose`s at the look's `progress`. When it ends
    the outgoing frame is dropped.
  - **From 2358 (done, ADR-0040):** `redraw` draws the stack, then each
    module that declared `Hook::DrawOver` draws over it (`draw_over`,
    "over the screens, after them"). `compose` runs right after
    `self.stack.draw`, before the modules' loop, so a module draws over
    a look as it draws over any screen. The outgoing frame is the
    stack's drawing without the modules': with no module that draws,
    `buffer` cloned before the clear; with one, a copy of the buffer
    `Game` takes after `self.stack.draw` each frame. So no module's
    drawing is composed, nor shown twice.
  - **Logic switches at once.** The stack changed when the screen asked,
    and the new top screen updates from the next frame. **While a look
    plays, nothing is pressed:** the screens get frames with time and
    ticks, and no actions, held actions, typing or buttons. Presses are
    dropped, not queued (game#0813: input is ignored while it plays).
    In `Game::step` the events still reach `InputState`, so a key held
    through the look is let go cleanly, but the `FrameInput` handed to
    the stack is empty but for `dt` and `ticks`, and the first-key and
    Debug hooks don't open. `Game::act` does nothing while a look
    plays.
  - A look asked for while one plays replaces it, from the same
    outgoing frame.
  - `Game::transition(&self) -> Option<(TransitionLook, u16)>`: the look
    playing and its progress, for tests and tools.
  - **With idle motion off** (`Ctx::new`, so every Harness), a look
    asked for is dropped: the change is a cut, nothing is dropped from
    input, and tests read end states as today. `IdleMotion`'s doc says
    so: off, nothing moves by time alone.
- **Seeing it:**
  - The toy's frame tool (`ToyTools::frame`, `crates/cli/src/toy.rs`),
    the grid template's (`templates/grid/crates/game/src/tools.rs`, in
    `frame` only, not in `harness`, which its tests use) and the blank
    template's turn idle motion on with `Harness::ctx_mut`, as the
    game's frame tool does, so `frame-png --keys … --wait 0.2` shows a
    look half way. The blank template's tools are
    `shuyi_cli::BasicTools` (2511), so its part is in
    `BasicTools::frame` (`crates/cli/src/basic.rs`). The grid template
    isn't formatted by `cargo fmt --all`; CI's template job checks it.
  - Each of these `frame`s, when `Game::transition()` is still `Some`
    after the script, adds the look and its progress to `Shot::shows`
    (`options, fade 542/1000`). Without it, `frame-png --keys …` alone
    would name the new screen while the picture still shows the old
    one, and an agent couldn't tell why.
  - The toy's lobby (`Lobby::update`, `crates/ui/src/toy.rs`) asks for
    a `Fade` of 400 ms through `EngineColor::Background` when it opens
    the options, and a `Dissolve` of 400 ms when it opens Key bindings.
- **An ADR** (the `write-adr` skill): looks are composed from the last
  frame, per cell; the frame's whole-frame fields (the veil here, 2340's
  shake offset) are look only, never cut, and every painter draws them;
  input is dropped while a look plays; idle motion off means cuts. It
  notes that ADR-0003 §1's promise is now kept.
- **What changes for the game:** additive. `Transition`, `GameHooks`
  and every existing type keep their public shape, and a game that asks
  for no look sees no change. game#0813 can use it once the pin moves;
  the game's PR that moves its pin follows this one. The game's frame
  tool already turns idle motion on, so once its screens ask for looks
  it should add the look to its `shows` as the toy's does.

**Size.** If the PR passes about 600 changed lines (snapshots not
counted), `Wipe`, `Dissolve` and `copy_from` (with their tests and the
lobby's dissolve) go to a follow-up ticket (`write-ticket`), After MVP,
and the PR adds it to the `blocked_by` of 2340 and of 2364 (parked,
whose shots enter with a `Dissolve`), and names it beside 2339 in
1120's either-order clause on transition looks. This PR ships `Cut`
and `Fade` on the veil.

**Out (do not do):**
- an iris (a circle that opens or closes): a follow-up if a game asks;
  `compose`'s rule per cell makes it small;
- slides that move whole frames, and a crossfade of two frames (a mix
  per pixel of two frames needs both in every painter);
- shake, flash, and the player's reduced-motion setting that turns
  wipes and dissolves into fades (2340);
- transitions of a part of a screen: a picture fading in, a visual
  novel's slide (2722);
- a default look for every change, set once: a field on `GameHooks`
  would break games that build it as a struct literal, the game among
  them. A follow-up if a game asks;
- shot enters in cinematics (2364);
- a sheet or an animation of a look's frames over time from
  `frame-png` (2514);
- how 1120's world layers take part (1120 says so);
- GPU screen effects (2326);
- moving the game's own fades onto this (the game's choice, game#0813).

## Implementation steps

1. Read ADR-0002 §4, ADR-0003 §1, game#0813, `glyph_buffer.rs`
   (`blit` and `cut_items`), `game.rs` (`step`, `act`, `redraw` with
   its modules' loop), `onebit.rs`, the two painters and `BasicTools`
   (`crates/cli/src/basic.rs`, from 2511); 2109, 2332 and 2337 if done.
2. Write the ADR.
3. In `glyph_buffer.rs`, add `Veil`, `copy_from` and the note; in
   `snapshot.rs`, the two lines and the module doc. Unit tests beside
   them.
4. Write `crates/ui/src/frame/transition.rs` (and `pub mod transition`
   in `frame.rs`): `TransitionLook`, `Edge`, `progress` and `compose`,
   with unit and property tests in `transition/tests.rs`.
5. Add `Ctx::transition_look` (`ctx.rs`) and the look in `Game`
   (`game.rs`): the outgoing frame, the compose in `redraw`, input
   dropped in `step` and `act`, the idle-motion rule, and
   `Game::transition`. Update `IdleMotion`'s doc.
6. Draw the veil in `shuyi-app` (`render.rs`), `frame-png`
   (`frame_png.rs`) and both one-bit renderers, with `render_changed`'s
   whole-screen redraw.
7. Give the toy's lobby its two looks. Turn idle motion on in the frame
   tools and add the look to `shows`. Add the Harness test beside the
   toy's other Harness tests (`crates/ui/tests/it/`).
8. Add the picture snapshots in `crates/cli/tests/it/toy.rs`, through
   `ToyTools.frame` with the `--lobby` option (a `GameArg`), a
   `Step::Keys` that picks the entry (the options are the first, Key
   bindings the second) and a `Step::Wait(0.2)`. Add
   `insta.workspace = true` to `crates/cli/Cargo.toml`'s
   dev-dependencies, and encode with `shuyi_cli::codec::encode_png` as
   hello-grid does. Add the one-bit veil tests and their text snapshot
   in `crates/ui/src/frame/onebit/tests.rs`.
9. Run the gates.

## Acceptance criteria

- [ ] With idle motion off, every existing snapshot and test passes
      unchanged, and a look asked for leaves no veil, no note and no
      dropped input.
- [ ] `compose` at 0 gives `from`'s cells and items, and at 1000 gives
      `to`'s, for every look. A `Fade` at 500 shows a full veil.
- [ ] Every cell of a composite is the cell at that place of `from`, of
      `to` or of the `through` frame, and every item's visible part lies
      in cells taken from its own frame (property test over random
      frames, looks and progress).
- [ ] The cells showing `to` never decrease as the progress grows, for
      `Wipe` and `Dissolve` (property test).
- [ ] A Harness test with idle motion on: the toy lobby's fade plays
      (`Game::transition()` is `Some` and the snapshot has the
      `transition` line), presses during it change nothing, and once it
      ends the frame's snapshot equals the one drawn with idle motion
      off.
- [ ] A look asked for while one plays restarts from the same outgoing
      frame (unit test in `Game`).
- [ ] `frame-png --lobby` with the options picked and no wait reports
      the look in its summary (`options, fade …/1000`); with `--wait
      0.5` after it, it reports `options` alone.
- [ ] The one-bit renderer and its reference give equal frames with a
      veil at alphas 0, 64, 128 and 255, and alpha 0 draws as no veil.
- [ ] Picture snapshots (`insta::assert_binary_snapshot!`, as
      `examples/hello-grid/tests/it/snapshot.rs` does) of the toy's
      lobby half way through its fade and its dissolve are reviewed and
      committed, and so is an `insta` text snapshot of one-bit pixels
      (`onebit/tests.rs`'s `text`) under a black and a white veil at
      alpha 128.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `progress` at 0, half, the end and past it, and with 0 ms; each
  look's cells at 0, 250, 500, 750 and 1000, with and without
  `through`; two buffers of different `spec`s compose as a cut;
  `copy_from` with a rectangle, a sprite and a sprite with a `base`;
  the veil and the note removed by a full fill and by `Game::redraw`
  with a see-through blank cell; the two snapshot lines; `Game::act`
  and the Debug hook during a look; the look replaced mid-way; the
  toy's `shows` with and without a look playing; a module that draws
  over (2358) keeps its cells and items whole at every progress of a
  `Fade` and a `Dissolve`, and the outgoing frame holds none of them.
- Property: the two composite properties above; `compose` is pure (the
  same inputs give equal buffers).
- Snapshot / integration: the Harness test, which checks that presses
  change nothing by reading the stack (`h.screens()`), not cells
  (`AGENTS.md` rule 9: only tests of a look read cells and pixels); the
  one-bit equality and text snapshot; the picture snapshots.

## Completion notes

*(Filled in by the session that completes the ticket.)*
