---
id: "2407"
title: "Parked: Phones' browsers play a web build: on-screen controls as data, swipes as buttons, and a web-touch target"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2405"]
nick_input: decision
completed:
---

# 2407 — Phones' browsers play a web build

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). It also waits for Nick's answer below: on 2026-10-20 he left phones' browsers as a target for later (`docs/decisions.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and GameMaker, Defold and Construct 3. All of them but
Bevy let a game show controls on a phone's screen:

- Godot 4.7 has its own `VirtualJoystick` node beside
  `TouchScreenButton` (https://godotengine.org/releases/4.7/);
- Unity's Input System has an On-Screen Stick and an On-Screen Button
  that feed the same actions as a pad;
- Unreal's touch interface is an asset that lists virtual controls;
- Construct 3 and GameMaker games play by touch in a phone's browser.

Bevy reads touches but has no on-screen controls.

What Shuyi has (read 2026-10-20):

- **No touch input.** `RawInputEvent` (`crates/ui/src/game.rs`) has
  keys, text and pad buttons only, and no crate reads a touch or the
  mouse. macroquad 0.4.16 has `touches()` (an id, a phase and a
  position) on native and the web. The vendored
  `crates/cli/web/mq_js_bundle.js` already forwards `touchstart`,
  `touchmove`, `touchend` and `touchcancel` with each touch's
  identifier. Nothing uses them.
- **A phone can't start a game.** `Game::step` ends the "press any key"
  wait (`KeyPrompt::Waiting`) on a key or a pad button only.
- **The web target says nothing of phones.** `targets/web.ron` lists
  the pad's buttons, two sticks, `pointer: true` and `keyboard: true`,
  and no `touch`. `GameControls::fit` (`crates/ui/src/input/fit.rs`)
  passes a target that has a keyboard and no table of its own. So
  `shuyi check --target web` passes a game no phone can play.
- **The profile has room for it.** `Input.touch` and
  `Display.safe_area` exist in `crates/assets/src/target.rs` (3001).
  `android.ron` and `ios.ron` set `touch: true` for on-screen controls.
  `steam-deck.ron` sets it too, with "nothing may need it".
- **The web shell** (`crates/cli/web/index.html`) uses `touchend` only
  to resume audio. Its viewport tag has no `viewport-fit=cover`, and the
  canvas has no `touch-action` rule and no safe-area insets.

What is planned:

- 3015 (parked): Android and iOS apps, with on-screen controls whose
  places the player can move. It is parked for store fees and
  agreements.
- 1121 (parked, real time) leaves out gestures and on-screen controls.
- 2405 (After MVP) reads the mouse and touches for menus and grid maps.
  A tap is a click there, through macroquad's touch-as-mouse. Its
  events carry no touch id, and it leaves several touches at once to
  this ticket.
- 3002 (done) sent on-screen controls to 3015.
- 3052 (After MVP) gives the web shell a download progress bar,
  readable errors and hosting notes; it adds no fullscreen button,
  since the game's Fullscreen setting already reaches the browser. 3053
  (After MVP) tells a running game whether it has touch, from the
  target profile. 3043 (After MVP) suspends a hidden tab.
- 2528 (After MVP) serves the web build on the LAN (`--lan`). It left
  the QR code of that address to this ticket.
- 2408 (parked) leaves a phone's vibration to the tickets this one
  writes.

Why it matters:

- **The game:** nothing changes. Visions of Shuyi ships on PC and the
  Steam Deck. A game shows on-screen controls only if it loads a set of
  them.
- **Other 2D games:** a link to a web build is opened on a phone as
  often as on a computer. A turn game (a puzzle, cards, tactics) plays
  well with a d-pad and two buttons. Today a phone can't get past
  "press any key".
- **The end goal** (2026-10-19: a game "with a web ver" in under an
  hour): until the launch, the agent check measures turn games. Those
  would play from the same link on a phone.
- It costs no store fees and no accounts, unlike 3015.

This is too big for one PR. It asks Nick, then splits, as 3015 does.

## Nick input

**Decision** (the `ask-nick` skill), when unparked:

> Should a phone's web browser be a supported way to play a Shuyi web
> build, before and apart from the parked Android and iOS apps? It
> costs no store fees or accounts. New games made from the templates
> would show on-screen controls on a phone. Visions of Shuyi would get
> them only if it asks.

- **Yes:** the split tickets below are After MVP, not parked.
- **No:** the plan below moves into 3015's split, and this ticket
  closes.

## Scope

**In** (this PR):
- Nick's answer in `docs/decisions.md`.
- On yes: the three tickets below, with their dependencies, and their
  rows in `docs/ROADMAP.md`. 3015 and 1121 point at them.
- On no: the plan below copied into 3015 as its touch-controls part.
- Either way: every open ticket and `docs/ROADMAP.md` line that sends
  work to 2407 points at the ticket that now holds it.
- **Either order with 3015:** if 3015 is done and its split wrote a
  touch-controls ticket from items 1 and 2 below (3015's Scope says
  when), that ticket holds them. On yes, write only the third ticket,
  blocked by that one in place of the first. On no, nothing is left to
  copy, and this ticket closes. If not, the steps below stand as
  written.

**Rules the split tickets keep** (from the 2026-10-20 review):

- **Touches become buttons before the input layer.** The on-screen
  controls act as one more pad, of a new kind `PadKind::Touch`. Their
  presses and releases follow a pad's rules (`Pads`,
  `crates/ui/src/input/pad.rs`), so a button held on a real pad and on
  the screen is one press. Games still read only actions (`AGENTS.md`
  rule 8), help text names the buttons (`HelpKeys`), controls-fit
  (3002) counts them, and replays don't change.
- **`PadKind` gains a variant,** which breaks a game's `match` on it
  with no `_` arm. The first split ticket says so for the game's pin
  bump.
- **The on-screen stick is 4-way,** like a pad's stick ("Sticks are
  4-way buttons", the module doc of `pad.rs`), with the keymap's
  `StickDef` thresholds.
- **A touch that starts on a control is the control's alone.** 2405
  reads touches as the mouse, so the backend sends no pointer event for
  such a touch. A tap on the d-pad never clicks the menu row under it.
- **A painter draws the controls from a plain-data view** (rule 9).
  Never an HTML overlay: it would work on the web only, couldn't be
  tested through the view, and would need JS glue beside macroquad's
  loader.
- **The hit test is pure,** in `shuyi-ui`, which stays `no_std` (3005).
  Only `shuyi-app` reads touches (rule 5). Where the touches meet the
  control set (in `App`, or in `shuyi-app` as pads do today) is the
  first split ticket's choice, in its ADR.
- **Opt-in.** A game with no control set shows none. The templates load
  the engine's default set.
- **Best effort, never an acceptance criterion:** fullscreen and
  orientation lock. iOS Safari has no `screen.orientation.lock`.

**The split.** Each ticket stays near 600 changed lines; if the first
grows past that, its painter goes in a ticket of its own.

1. **24xx: on-screen controls as data.**
   - Kind `TouchControls` and format `touch-controls` (RON) in a new
     `shuyi_assets::touch` (ADR-0007, ADR-0019), with its row in
     `docs/formats.md`: a d-pad, buttons (each a `Button`) and an
     optional stick. Places and sizes are in thousandths of the
     display's safe area, so they are integers. Each control may name a
     picture.
   - An engine default set: a d-pad on the left, `South` and `East` on
     the right, `Start` at the top.
   - `shuyi_ui::input::touch`: touches in, the touch pad's buttons out.
     A touch that starts on the d-pad or the stick steers it while it
     slides. A touch that starts on a button holds it until it lifts.
   - `PadKind::Touch`, named like an Xbox pad in `button_name`, which is
     how the default set labels its buttons. It joins `PadKind::ALL`.
   - The controls show after the first touch, and hide after a key or a
     real pad is used. Any touch ends the key prompt. Either order with
     3053: if it is done, they also show from the start where
     `ctx.platform` says touch and no keyboard (on the web, item 3 makes
     it say so on a phone); if not, the first touch shows them.
   - `DisplayInfo` (`crates/platform/src/platform.rs`) gets the safe
     area's insets (`shuyi_assets::target::Insets`), zero until a
     backend says otherwise.
   - A plain-data view of the controls (each one's place in display
     pixels, and whether it is held), handed to the backend with the
     `Frame`. `shuyi-app` paints it over the whole display, in the bars
     around the console too.
   - The touches: 2405's pointer events carry no id, so this ticket
     reads each touch with its id in `shuyi-app`, from macroquad's
     `touches()`, and keeps a touch on a control out of 2405's pointer.
   - Harness touch scripts, like `Harness::pad`'s, and an ADR.
2. **24xx: swipes as buttons.**
   - `Button::SwipeUp`, `SwipeDown`, `SwipeLeft` and `SwipeRight`: a
     press and a release in one frame, as a crank notch is
     (`Button::CrankBack`, `Button::CrankForward`). `ButtonSet` is a
     `u64` and holds 34 buttons today, so four more fit.
   - Thresholds in the keymap file, as a `SwipeDef` beside `crank`
     (`CrankDef`): a distance in thousandths of the safe area's shorter
     side, as the controls' places are, and a time in milliseconds.
   - Only from a touch that starts on no control. A swipe is no click:
     2405's `clicked` stays `None` for it.
   - Counted by controls-fit when a profile lists them.
   - Last in order: it may wait until a game or an example asks.
3. **30xx: the `web-touch` target.**
   - `targets/web-touch.ron`: no keyboard, `touch: true`, `backend:
     "shuyi-app"`, and text sizes like `android.ron`'s.
   - A new `Input` field (for example `on_screen: bool`) says the game's
     on-screen control set adds its buttons to the profile's.
     `GameControls::fit` counts them. `steam-deck.ron` leaves it off.
   - One package serves `web` and `web-touch`. Either order with 3004:
     if it is done, this ticket makes `shuyi export web` also run the
     `web-touch` check when the game's `shuyi.ron` lists that target; if
     not, it adds that to 3004's scope.
   - The web shell: `viewport-fit=cover`, `touch-action: none` on the
     canvas, no double-tap zoom, and the safe-area insets (CSS
     `env(safe-area-inset-*)`) handed to `DisplayInfo` through a small
     JS plugin of our own, like `gamepad.js`.
   - The same plugin says whether the browser's main pointer is coarse
     (`matchMedia("(pointer: coarse)")`). Either order with 3053, whose
     `shuyi-app` says `web` on wasm32: if it is done, a coarse pointer
     makes `shuyi-app` report `web-touch`'s facts, so a phone shows the
     controls from the start; if not, they show after the first touch.
   - `examples/hello-grid` gets a `web-touch` table in
     `files/keymap.ron` and a `web-touch` target in `shuyi.ron`, and
     `shuyi check --target web-touch` passes on it.
   - ADR-0008 §1's table gets a `web-touch` row.
   - `shuyi web --serve --lan` (2528, so 2528 goes in this ticket's
     `blocked_by`) also prints its LAN address as a QR code in the
     terminal, so a phone's camera opens the game. It is in
     `shuyi-cli` only, so no game build carries it. It uses a QR
     encoder crate under ADR-0002 §8 (for example `qrcodegen`, MIT,
     checked on crates.io on the day), with its license in
     `deny.toml`'s `allow` and `cargo deny check` passing, or a small
     encoder of our own. It is drawn with Unicode half blocks inside a
     quiet zone, readable on a dark or a light terminal.
   - **Sign-off:** Nick plays Hello, Grid on his phone, opened from the
     QR code.

**Out (do not do):**
- any code in this PR;
- a tap on a tile or a menu row (2405);
- the player moving and resizing the controls, the native apps and the
  stores (3015);
- pinch, long press and other gestures;
- the web shell's loading screen and errors (3052), suspending a hidden
  tab (3043), and the running game knowing it has touch (3053);
- rumble on a phone's browser (`navigator.vibrate`, where the browser
  has it): 2408 leaves phones out. If 2408 is done when Nick answers
  yes, the third ticket written here adds it to the web-touch build; if
  not, step 3 writes a parked follow-up blocked by 2408 and that ticket;
- the Steam Deck's touch screen, which stays "nothing may need it".

## Implementation steps

1. Wait for Nick to unpark it, then ask him the question above with the
   `ask-nick` skill.
2. Record his answer in a new entry of `docs/decisions.md`.
3. On yes:
   1. Read 2405 as it landed. Check whether 2528, 2408, 3004, 3052,
      3053 and 3015 are done (3015's clause above).
   2. Write the three tickets with the `write-ticket` skill: the next
      free ids in 24xx (two) and 30xx (one), milestone `After MVP`,
      `status: todo`. The first is blocked by 2405; the second and the
      third by the first; the third also by 2528. Write the either-order
      clauses above with what step 3.1 found. If 2408 isn't done, also
      write the parked rumble follow-up named under **Out**.
   3. Point 3015's "On-screen controls as an input profile" at the first
      ticket; 3015 keeps the player moving the controls and the native
      apps. Point 1121's Out ("gestures (pinch, swipe); on-screen touch
      controls") at the first two; pinch stays out.
   4. Add the tickets' rows to `docs/ROADMAP.md`, in phase 3's table of
      the second comparison after 2405, and take 2407 out of the
      *Parked* section's list and table.
4. On no: copy the plan into 3015's Scope as its touch-controls part,
   and take 2407 out of `docs/ROADMAP.md`'s *Parked* section (3015's row
   holds it now).
5. Either way, find the other lines that send work here
   (`grep -rn 2407 tickets/open docs/ROADMAP.md`: 2019, 2405, 2408,
   2522, 2528, 3043, 3052 and 3053 on 2026-10-21) and point each at the
   ticket that now holds that work.
6. Run `cargo xtask ticket-lint` and the gates.

## Acceptance criteria

- [ ] Nick's decision is in a new entry of `docs/decisions.md`.
- [ ] On yes, the three tickets (the third alone, if 3015's clause
      says so) exist as `After MVP` with
      `status: todo`, with `blocked_by` and either-order clauses that
      name 2405, 2528, 3004, 3052 and 3053 as they stand, and none
      waits on a parked ticket. 3015 and 1121 name them. On no, 3015's scope holds the
      plan.
- [ ] Each split ticket keeps the rules above.
- [ ] `grep -rn 2407 tickets/open docs/ROADMAP.md` finds no line that
      still sends work to 2407.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None in this PR: it changes only tickets and docs, and
  `cargo xtask ticket-lint` checks them.
- Per the split tickets. The first names the hit test's unit and
  property tests, the format's never-a-panic test and Harness touch
  tests. The second names the swipe thresholds' tests. The third names
  the profile's tests, the check on Hello, Grid, and a snapshot of the
  QR code printed for a fixed address.

## Completion notes

*(Filled in by the session that completes the ticket.)*
