---
id: "2367"
title: "Parked: Overlay screens that take only their declared actions, so the screen below keeps running"
type: feature
milestone: "Engine: screens"
model: reasoning
effort: high
status: blocked
blocked_by: ["2013"]
nick_input: none
completed:
---

# 2367 — Overlay screens that let the screen below keep running

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). It is unparked together with its first consumer: the unlock notice of 3036 or the first tick stage of 1119, whichever Nick unparks first. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engine Defold 1.14. In each of them, "drawn
on top", "gets the input" and "keeps running" are separate settings:

- Godot sends an input event down the tree until a node marks it
  handled (`set_input_as_handled`), and each node's `process_mode` says
  whether it runs while the tree is paused
  (https://docs.godotengine.org/en/stable/tutorials/inputs/inputevent.html).
- Unreal's Enhanced Input stacks mapping contexts by priority, and
  CommonUI routes input to the active layer
  (https://dev.epicgames.com/documentation/en-us/unreal-engine/enhanced-input-in-unreal-engine).
- Unity's Input System turns action maps on one by one, and the time
  scale is a setting of its own.
- Bevy has nothing built in; the `bevy_enhanced_input` crate adds input
  contexts with a priority.
- Defold keeps an input focus stack, in which a component consumes what
  it handles and the rest goes on (https://defold.com/manuals/input/).

Shuyi has one setting for all three. What the code does today:

- `crates/ui/src/screen.rs`: the module docs say the `ScreenStack`
  "routes input to the top screen only" (line 6). `Screen::update` is
  "Called only while this screen is on top" (line 40), and the
  `ScreenStack` docs say "Only the top screen is updated" (line 197).
  `Screen::is_overlay` changes only drawing (`ScreenStack::draw`, line
  360). `ScreenStack::update` (line 290) updates `self.screens.last_mut()`
  alone, and the test `only_the_top_screen_updates`
  (`crates/ui/src/screen/tests.rs`) holds it.
- `crates/ui/src/game.rs`: `Game::step` builds one `FrameInput`, with
  the frame's steps (`with_ticks`), and hands it to `stack.update` (line
  369). Since 3003, fixed steps reach a screen only through
  `FrameInput::ticks()`, and ADR-0024 §4 says they "reach the top
  screen". So any screen pushed over a tick stage stops it and takes all
  of its input.
- What this ticket reuses: `FrameInput::declared`
  (`crates/ui/src/input/frame.rs`, line 74) already splits a frame's
  actions by what a screen declares (`Screen::actions`, ADR-0025 §3),
  and the Harness's guard logs the rest (`ScreenStack::undeclared`,
  ADR-0025 §4). `Screen::covered` and `Screen::uncovered` (ADR-0018)
  are the calls a push and a pop make.
- `controls_fit` (`crates/ui/src/input/fit.rs`, line 173) checks each
  screen alone. The docs of `Screen::actions` say an overlay "is checked
  on its own", and `GameControls::with` lists overlays and popups on
  their own.
- The debug tools avoid the problem by not being screens: 2315's gizmos,
  2316's readout, 2325's profiler channel and 2220's reload errors are
  drawn outside the stack, over a running game.

Why it matters. Visions of Shuyi is turn-based and its menus are modal,
so it needs none of this now, and that is why this is parked. But two
filed tickets build on the wrong assumption:

- 3036's unlock notice is "an overlay screen" shown "for a few
  seconds". Pushed over a tick stage, it stops the game. Over a
  turn-based screen, it eats the player's presses for those seconds.
- 1149's chat panel over a running match, and any toast or notice over a
  stage of 1119, hit the same wall. So do the snake and the platformer of
  the end goal (Nick, 2026-10-19), which wait with real time.

The new trait method has a default that keeps today's behaviour, so a
game that doesn't use it sees no change.

## Nick input

None.

## Scope

**In:**

- **`Passes`**, in `shuyi_ui::screen`:
  - `pub enum Passes { Nothing, Undeclared, Everything }`. It follows
    ADR-0039 on `#[non_exhaustive]`.
  - `Screen::passes(&self) -> Passes`, defaulted to `Passes::Nothing`:
    the screen is modal, as every screen is today.
  - `Undeclared`: the screen takes the actions it declares
    (`Screen::actions`), pressed and held. The other actions go to the
    screen below, which keeps running. A screen that declares nothing
    (`None`) takes every action, so only time goes down.
  - `Everything`: the screen takes no action, and its `actions` is not
    read. It runs (its timer counts) and draws, and every action goes to
    the screen below. This is the shape of a notice or a toast.
  - `passes` is separate from `is_overlay`. Drawing doesn't change.
- **Which screens run.** The running screens are the top screen and each
  one below it, down to and including the first whose `passes` is
  `Nothing`. A stack whose screens all pass runs them all.
  `ScreenStack::update` updates them in a fixed order, top first, so an
  overlay takes its actions before the screen below sees the rest. The
  order is part of determinism, and the ADR says so.
- **What each running screen sees.** A new
  `FrameInput::split(&self, takes: &[A]) -> (Self, Self)`, beside
  `declared`, gives the screen its share and passes the rest down:
  - its share of the actions that reached it, pressed and held;
  - the same `dt()` and `ticks()` as every other running screen. A
    notice's timer adds up `dt()`, and a tick stage under it steps as
    many times as it would alone;
  - the same keys, buttons, typed text and `pad_pressed()` as the
    others. These are not split: only text boxes and the key bindings
    screen read them, and those are modal.
  - **Held state.** When an overlay that takes an action opens while
    that action is held, the screen below sees it let go (`is_held` is
    false) from the overlay's first update on. When the overlay leaves
    with the action still held, the screen below sees it held, with no
    press in `actions`.
- **A transition acts where it was returned.** A screen's transition is
  applied right after its update, at that screen's place in the stack,
  so a notice on top stays on top:
  - `Push`: the new screen goes directly above the screen that returned
    it. For the top screen that is the top, as today. A screen pushed in
    a frame first runs in the next frame, as today.
  - `Pop` and `PopWith`: the screen that returned it leaves. The screens
    above it stay.
  - `Replace`: the screen is swapped in place.
  - `Quit`: as today.
  - A screen that stopped running earlier in the same walk (a modal
    screen was pushed above it, or it left) is not updated in that
    frame. A screen that starts running during the walk (a modal screen
    above it left) first updates in the next frame, as after a pop
    today.
  - Give each entry of the stack a serial number, so the walk knows a
    screen after the stack changes under it.
  - `ScreenStack::apply_in`, which `Game` uses for its hook screens (the
    first-key and debug screens), still applies a transition as if the
    top screen returned it. `ScreenStack::apply` and
    `ScreenStack::push` keep their meaning.
- **From 2358 and 2372 (done; ADR-0040, *Consequences*):** a screen a
  module pushes over a running game on its own is this ticket's. A
  module's hooks get `&mut Ctx<G>` and return no `Transition`
  (`crates/ui/src/module.rs`), so add a request on `Ctx` (for example
  `Ctx::push_over(Box<dyn Screen<G>>)`). `Game::step` pushes each one on
  top of the stack after the stack's update, in the order asked, and it
  first runs in the next frame, as any push does. 3036's unlock notice
  may use the same request.
- **`covered` and `uncovered` follow running.** After each transition,
  in the walk or in `apply_in`, the stack compares the running screens
  before and after:
  - a screen still on the stack that stopped running hears `covered`;
  - a screen that runs again hears `uncovered`. The screen directly
    below a popped one gets what it popped with; the others get `None`.
    The transition each returns is applied at once, at its place;
  - calls go top first. A screen pushed or removed hears nothing, as
    today.

  So each screen hears `covered` and `uncovered` in turn, starting with
  `covered`, and a pass-through push covers nobody. With modal screens
  only, this is exactly ADR-0018's rule.
  - A screen that passes lets the screen below run, so its pop uncovers
    nobody and its value has nowhere to go. `PopWith` from it fails a
    `debug_assert!` that names the screen; in a release build the value
    is dropped.
- **The guard, per screen** (ADR-0025 §4):
  - With the guard on, each running screen that declares its actions is
    handed only those, and asking `is_held` about another panics. For an
    `Everything` screen, that is any action.
  - The actions left after the last running screen are logged with its
    name (`ScreenStack::undeclared`), as today. An action an overlay
    passes on is not logged for it.
  - With the guard off, an `Undeclared` screen still gets only its share:
    the split is behaviour, not a test aid. `is_held` on another action
    returns false there. This amends ADR-0025 §4's "A game's own stack
    never filters" to "never filters a modal screen".
- **Tools:**
  - `ScreenStack::running() -> Vec<&'static str>`,
    `Game::running_screens()` and `Harness::running_screens()`
    (`crates/platform/src/harness.rs`): the running screens' names,
    bottom first, like `ScreenStack::names()` and `Game::screens()`.
  - The contract's `run` and `scene` answers gain `running`
    (`RunOutput` and `SceneOutput` in `crates/cli/src/contract/run.rs`).
    The `Session` trait (`crates/cli/src/contract/session.rs`) gains
    `fn running(&self) -> Vec<String>`, defaulted to the last of
    `screens()` alone; `Headless` answers it from
    `Harness::running_screens()`. Raise `contract::VERSION`
    (`crates/cli/src/contract.rs`) by one, regenerate the schemas and add
    the line to `crates/cli/schemas/versions.json` (ADR-0022 §5). The
    `run` row of `docs/contract.md` names the field.
- **Controls that fit** (`crates/ui/src/input/fit.rs`):
  - `ScreenActions` gains `passes: Passes`, which `ScreenActions::of`
    reads from the screen, and `over: Vec<&'static str>`, set with a
    builder `ScreenActions::over(self, screens: &[&'static str]) ->
    Self`: the screens a passing overlay may run over. Empty means every
    listed screen whose `passes` is `Nothing`.
  - An `Undeclared` overlay is checked on its own, as today, and then
    with each screen it runs over:
    - `FitRule::Taken`, an error: the overlay takes a required action of
      the screen below, which can't reach it while the overlay is up;
    - `FitRule::Shared`, naming both screens in its message: a required
      action of each shares an input.
  - An `Everything` screen takes nothing. `controls_fit` doesn't check
    it, and `GameControls::fit` doesn't warn about it for declaring
    nothing.
  - These reach the contract as `TargetCheck::Controls` problems, as now
    (`controls` in `crates/cli/src/contract/target.rs`), with the
    overlay's name in `screen`. That schema doesn't change.
  - The two types this grows follow 2013's ADR: `FitRule` is a set that
    grows, so it gets `#[non_exhaustive]`. `ScreenActions` is built by
    games through `of` and `declared`, so it gets `#[non_exhaustive]`
    too, with a new `ScreenActions::undeclared(screen)` for the struct
    literal in `crates/cli/src/contract/fake.rs`, which can't compile
    outside `shuyi-ui` then. If ADR-0039 puts either type in another
    kind, do what the ADR says for that kind.
- **Either order with 2316** (the debug pause): if 2316 is done, add a
  Harness test that its pause stops an `Everything` overlay's timer and
  the screen under it alike. If not, 2316 adds that test (an edit to
  2316 says so).
- **From 2344 (done):** a sound's owner is the depth of the screen
  that started it (`AudioQueue::set_owner` and `release`, crate-private,
  in `crates/ui/src/audio/handles.rs`; `ScreenStack::own_top` and
  `release_top` in `screen.rs`). A push between screens moves the
  depths above it, so the owner becomes the entry's serial number:
  change it to the serial number, set it before each running screen's
  `update`, `covered` and `uncovered`, and stop a screen's handles when
  it leaves at its place in the walk, as `apply_in` does.
- **Either order with 2322** (the accessibility tree): if 2322 is done,
  the tree's focus is in the top-most running screen that takes
  actions, so a `Passes::Everything` notice never holds it, and the
  screens a pass-through overlay lets run are in the tree under it,
  with a test; if not, nothing to do (2322 does it).
- **Either order with 2356** (debug commands and their console): if
  2356 is done, its console (`ConsoleScreen`, named `debug_console`)
  lets the game below keep running. Its `passes` is `Undeclared` and
  its `actions` becomes `None`, so it keeps every action while it types
  and only time goes down. Typed text isn't split, so the screen below
  sees it too; a stage reads none. A Harness test opens the console
  over a screen that counts `ticks()`, and the count still rises. If
  not, nothing to do (2356 does it when it lands, as its Out says).
- **Either order with 2405** (hit areas): if 2405 is done,
  `ScreenStack::draw` keeps the hits of the top running screen that
  takes input, not of the top screen: a `Passes::Everything` screen
  records none, and `FrameInput::split` keeps the pointer in both
  halves. A test: a click on a menu's row under a notice chooses that
  row. If not, nothing to do (2405 does it, by its own clause).
- **An ADR** (the `write-adr` skill). It amends ADR-0018 (`covered`
  follows running), ADR-0024 §4 (the steps reach every running screen)
  and ADR-0025 §3 to §5 (an overlay's declaration routes its input, the
  guard per screen, the fit check over pairs). It records:
  - the three variants, the order, where transitions act and the rule
    for `covered` and `uncovered`;
  - the alternatives weighed:
    - a notice layer drawn by `Game`, like 2315's gizmos, for
      display-only notices. It is lighter, but it is a second mechanism
      with its own timers, sounds, skins and tests. A screen already has
      all of them, and the Harness, `shuyi run` and `frame-png` see it
      by name. 2358's `Module::draw_over` is such a layer for a
      module's own drawing. A notice still wants a screen's timers,
      sounds and name, so the choice stands;
    - numeric priorities, like Unreal's mapping contexts. The stack's
      order already gives one;
    - two settings, one for input and one for running, like Godot's
      `process_mode`. They allow pairs no use needs: a screen that takes
      nothing but stops the game is a modal screen whose `actions` is
      `Some(vec![])`;
    - pushing always on top. Then a stage's pause menu goes over a
      notice and stops its timer, and the stage's own pop or replace
      still has to act below the notice.
- **Docs in the code:** the module docs of `screen.rs`, the `ScreenStack`
  docs, and the docs of `Screen::update`, `Screen::covered`,
  `Screen::uncovered`, `Screen::actions`, `ScreenStack::update`,
  `ScreenStack::apply_in`, `Game::frame`, `Game::act` and
  `GameControls::with` say "running screens" where they say "top
  screen" or "on its own". `FrameInput::ticks` says every running screen
  sees the same count. `Game::act` hands its action through the same
  walk.
- **For the game:** a screen that doesn't override `passes` works as
  before. `Passes`, `FitRule::Taken`, `ScreenActions::undeclared` and
  two fields of `ScreenActions` are new. A game that matches `FitRule`
  with no wildcard arm, or builds `ScreenActions` as a struct literal,
  changes at its pin bump; a `tools.rs` that uses `ScreenActions::of`
  (as the templates and `examples/hello-grid` do) needs no change. The
  PR says so, and the game's PR that moves the pin follows it. The
  completion notes carry 2010's `Breaking:` lines
  (`Breaking: shuyi-ui: FitRule: #[non_exhaustive] and a new variant,
  Taken; add a wildcard arm` and `Breaking: shuyi-ui: ScreenActions:
  #[non_exhaustive] with two new fields; build it with of, declared or
  undeclared`).

**Out (do not do):**

- 3036's notice itself: 3036 builds it as an `Everything` screen (an
  edit to 3036 says so);
- 1149's chat panel: 1149 picks `Undeclared` if this is done (an edit to
  1149 says so);
- per-player input contexts and seats (2406, parked);
- which screen a pointer click lands on, beyond the hit areas above
  (1121, parked): pointer input goes where it goes today;
- splitting keys, buttons and typed text between screens;
- a screen that passes and answers (pops with a value): a follow-up if a
  game needs one;
- checking chains of two `Undeclared` overlays beyond pairs;
- the debug tools: 2315's gizmos, 2325's profiler and 2220's reload
  errors are drawn outside the stack, not as screens. 2316's readout
  stays the top screen's;
- a module's drawing over the frame: 2358's `Module::draw_over`;
- a notice layer drawn by `Game`: weighed in the ADR, not built.

## Implementation steps

1. Read `crates/ui/src/screen.rs` and its tests, `input/frame.rs`,
   `game.rs`, `input/fit.rs`, `crates/cli/src/contract/run.rs` and
   `session.rs`, ADR-0018, ADR-0024 §4, ADR-0025, ADR-0022 §5, and
   2013's ADR.
2. Write the ADR first, with the rules above.
3. Add `Passes`, `Screen::passes` and `FrameInput::split`, with tests.
4. Give the stack's entries serial numbers. Write the running set, the
   walk, transitions at their place, and the before-and-after rule for
   `covered` and `uncovered`, in the walk and in `apply_in`. Keep every
   existing test in `screen/tests.rs` and `game/tests.rs` passing
   unchanged.
5. Make the guard and the undeclared log work per screen.
6. Do the 2344 part (either order, above).
7. Add `running()`, `running_screens()`, `Session::running`, the
   contract field, the version bump, the schemas and the
   `docs/contract.md` row.
8. Add `passes` and `over` to `ScreenActions`, `ScreenActions::undeclared`,
   `FitRule::Taken`, the two `#[non_exhaustive]` attributes, the pair
   checks in `controls_fit` and the skip in `GameControls::fit`, with
   tests in `fit/tests.rs`. Update the struct literals in
   `fit/tests.rs`, and use `ScreenActions::undeclared("title")` in
   `crates/cli/src/contract/fake.rs`.
9. Update the docs in the code.
10. Run the gates, including the `no_std` build of `shuyi-ui`.

Size: expect about 1,000 changed lines. If the diff runs well past 600,
step 8 (the controls check of overlays: `passes` and `over`,
`FitRule::Taken`, `ScreenActions::undeclared` and the two
`#[non_exhaustive]`) goes to a follow-up ticket written in this PR.

## Acceptance criteria

- [ ] Every existing test in `crates/ui/src/screen/tests.rs` and
      `crates/ui/src/game/tests.rs` passes unchanged, including
      `only_the_top_screen_updates` and
      `covered_on_push_uncovered_on_pop_neither_on_replace`.
- [ ] Harness: an `Everything` notice over a test screen that counts
      `ticks()` and Confirm presses. Over 120 frames of
      `2.0 * FRAME_DT` each (2 steps a frame), all while the notice
      shows, the count rises by 240; a Confirm press reaches the screen
      below; the notice pops itself once its `dt()` adds up to its
      seconds; the screen below hears neither `covered` nor `uncovered`.
- [ ] An `Undeclared` overlay that takes Cancel: Cancel reaches the
      overlay and not the screen below; Confirm reaches the screen
      below. With the guard on, nothing is logged as undeclared.
- [ ] Held: with Confirm held as an overlay taking Confirm opens, the
      screen below reads `is_held(Confirm)` as false while the overlay
      is up, and as true with no press after it leaves.
- [ ] From the screen under a notice: a push lands between it and the
      notice, a pop leaves the notice on top over the screen below
      (which hears `uncovered`), and a replace tells nobody.
- [ ] A modal push from an overlay covers it and the screen it let run,
      top first. Its `PopWith` uncovers both: the value goes to the
      overlay, `None` to the other.
- [ ] Screens update top first (a log test); a screen covered earlier in
      the walk is not updated in that frame, and one uncovered during
      the walk first updates in the next.
- [ ] `PopWith` from a passing screen panics in a debug build.
- [ ] A module's `before_update` pushes a `Passes::Everything` notice
      through `Ctx`; it shows from the next frame, and the screen below
      keeps running and getting the player's actions.
- [ ] `controls_fit` reports `Taken` and the shared input across an
      overlay and the screen below; `over` narrows the pairs; an
      `Everything` screen is neither checked nor warned about.
- [ ] `FitRule` and `ScreenActions` are `#[non_exhaustive]` (or as
      2013's ADR says for their kind).
- [ ] A sound started by the screen under a notice
      stays that screen's after a push lands between them, and stops
      when that screen leaves.
- [ ] `shuyi run` and `shuyi scene` answers list `running`; the contract
      version is raised and the schemas are regenerated.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `FrameInput::split` keeps `dt`, `ticks`, keys, buttons and text
  in both halves and splits actions and held ones; each variant of
  `Passes`; transitions from below an overlay; the guard per screen;
  each new fit problem and `over`; `Session::running`'s default.
- Property: for random stacks of screens with random `passes` and random
  transitions, each screen's calls alternate `covered` and `uncovered`,
  starting with `covered`; the screens updated in a frame are the
  running ones at its start, minus those stopped during it; the same
  script run twice gives the same log.
- Snapshot / integration: the Harness case above; a `run` answer with
  `running`; the regenerated schemas.

## Completion notes

*(Filled in by the session that completes the ticket.)*
