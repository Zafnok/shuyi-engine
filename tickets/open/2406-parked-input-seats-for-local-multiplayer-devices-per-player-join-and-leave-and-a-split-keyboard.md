---
id: "2406"
title: "Parked: Input seats for local multiplayer: devices per player, join and leave, a split keyboard, keymaps and Harness scripts per seat, and controls checked per player count"
type: feature
milestone: "Engine: real time"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: blocked
blocked_by: ["2010", "2013", "2412", "2082", "2083", "2085"]
nick_input: none
completed:
---

# 2406 — Input seats for local multiplayer

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (its ticket 0053, which picks the multiplayer modes after the single-player launch, chooses hotseat played over Steam Remote Play Together, or co-op skirmishes, and wants each player's pad to act only for that player; hotseat with one shared pad works today and needs nothing here). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

**From 2412 (done):** the pad ids on the raw events (`PadDown` and
`PadUp` with their pad, `PadRemoved`, `Pads::update` per pad,
`kind_holding` gone, `from_crank`'s pad, and the merge of pads in
`Game`) came before the first release, so this ticket's break isn't one
after it. This ticket starts from its events, and moves its merge in
`Game` (`PadMerge`, `crates/ui/src/game/pads.rs`) from every pad to each
seat's pads. The Scope's bullet *Pad ids on the raw events* says what
2412 left.

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Each of them gives every local player their own
devices:

- Unity's Input System: `PlayerInputManager` joins a player when an
  unpaired device presses a button or a join action, and can split the
  screen among the players
  (`docs.unity3d.com/Packages/com.unity.inputsystem@1.11/manual/PlayerInputManager.html`);
- Unreal 5: `IPlatformInputDeviceMapper` maps each input device to a
  local player;
- Godot: every `InputEvent` carries a `device` id, which a game matches
  to its players;
- Bevy: each gamepad is an entity, and `leafwing-input-manager` gives
  each player an `InputMap`.

Steam Remote Play Together lets friends join a local multiplayer game
online, "as though they were sitting at the same computer"
(partner.steamgames.com/doc/features/remoteplay). A guest's pad reaches
the game as one more local pad. So seats also give online play with no
netcode.

What Shuyi has today:

- `crates/ui/src/input/pad.rs`: `Pads::update` reports each pad's own
  changes with its `PadId` (a `usize`, the platform's own number for a
  pad, stable while it stays plugged in, which gilrs
  (`crates/app/src/pads/native.rs`) and the browser's Gamepad API
  (`crates/app/src/pads/web.rs`) both fill).
- `crates/ui/src/game.rs`: `RawInputEvent::PadDown(Button, PadKind,
  PadId)`, `PadUp(Button, PadId)` and `PadRemoved(PadId)` (2412).
  `Game`'s `PadMerge` (`crates/ui/src/game/pads.rs`) merges every pad
  into one stream: a button held on two pads is one press, released
  when the last pad lets go. `Game` holds one `InputState<G::Action>`,
  which keys and the merged pads feed, and `Game::input()` hands it
  out.
- `crates/platform/src/harness.rs`: `pad`, `hold_pad` and `use_pad`
  drive one pad (one `pad_kind`). `frame-png --pad` and the contract's
  `run --pad` go through `Script::parse_step`, which checks each step
  with `Step::check` (`crates/cli/src/frame_png.rs`, reused by
  `crates/cli/src/contract/run.rs`).
- `docs/platform-backends.md:143` tells every backend "several pads as
  one".
- `crates/ui/src/input/bindings.rs`: `PlayerKeys` saves one profile's
  keys per layout and one pad setup, under `KEYBINDINGS_KEY`
  (`crates/ui/src/ctx.rs`). A keymap's layouts are keyboard layouts the
  player picks between (ADR-0025).
- `crates/ui/src/input/fit.rs`: `controls_fit` and `GameControls` (3002,
  ADR-0025) check one keymap table per target.
  `shuyi_assets::target::Input` (`crates/assets/src/target.rs`) says
  which buttons, axes, pointer, touch and keyboard a target has, not how
  many people can play at once.

What is planned:

- ADR-0003 §2 and 2137: `TickInput` holds input "per player", but no
  ticket says which player a key or a pad belongs to.
- 1121: the pointer and sticks, for one player.
- 1120: world layers with cameras. Split screen comes through each
  layer's `viewport` and `split_viewports` (an edit of 2026-10-20 to
  1120), which 2362 already points at.
- 1144 leaves Remote Play Together out as needing no code. That holds
  only while every pad drives one player (an edit to 1144 says so).
- 2402: the in-house pads crate keeps "a stable id while plugged in".

Turn-based hotseat already works: one shared input, and `TurnRules`
sides (ADR-0006: Nick's first reaction was "hotseat yes"). It stays the
default.

Why it matters:

- **The game first.** Visions of Shuyi needs only hotseat today, which
  shared input serves. Its ticket 0053 decides the multiplayer modes
  after the launch. Hotseat over Remote Play Together, or co-op
  skirmishes, may want each player's pad to act only on their own side.
- **Other 2D games.** Couch versus and co-op (party games, fighting
  games, a two-player snake) need a pad, or half the keyboard, per
  player. With seats, Remote Play Together takes them online with no
  network code.
- **The end goal of 2026-10-19** (snake in under an hour, in few lines):
  a one-player game writes nothing new, since `Shared` is the default. A
  two-player snake sets one policy and reads `FrameInput::seat(n)`.
- **Cheaper now than later.** 2137's `TickInput` and 1121's pointer and
  sticks would otherwise be built on one `InputState`. The edits this
  ticket asks of 2137 and 1121 keep them seat-shaped from the start.

## Nick input

None. Unparking is Nick's call, as for every parked ticket. If he is
asked, the question is a product one: should games made with Shuyi let
two to four people play at once on one machine, each with a pad or half
the keyboard (which is also what Steam Remote Play Together streams
online)?

## Scope

**In:**

- **An ADR** (`write-adr` skill): devices belong to seats.
  - A seat is one local player. A device (the keyboard, half of it, or
    one pad) is in at most one seat.
  - `Shared` is the default and is today's rule: every device drives
    seat 0, and several pads in one seat act as one.
  - It replaces the engine's "several pads as one" rule (`pad.rs`'s
    module docs, from game ADR-0034) with "several pads in one seat as
    one". It says that "per player" in ADR-0003 §2 means per seat, and it
    extends ADR-0025 with the check per player count below.
- **`shuyi_ui::input::seat`** (`crates/ui/src/input/seat.rs`), `no_std`
  as the rest of `shuyi-ui` (3005). New public types follow 2013's ADR:
  - `Seat(u8)`: `Seat::FIRST`, `Seat::new(n) -> Option<Seat>` (`None`
    from `MAX_SEATS` up), `Seat::index()`. `MAX_SEATS` is 4, the same
    constant as `shuyi_assets::target::MAX_LOCAL_PLAYERS` below.
  - `SeatDevice`: `Keyboard` (the whole keyboard, with the mouse and
    touches when there are any), `KeyboardHalf(Layout)` (one layout of
    the keymap's split keyboard, below), `Pad(PadId)`.
  - `SeatPolicy<A>`, plain data, `Shared` by default:
    - `Shared`: every device drives seat 0, exactly as today;
    - `JoinOnPress { join: A, leave: Option<A>, max: u8 }`: an unseated
      device whose press gives the game's `join` action (through the
      keymap that device would use when seated) takes a seat, one device
      per seat; a seated one whose press gives `leave` frees it;
    - `Fixed { max: u8 }`: the game assigns devices itself
      (`Ctx::assign_seat(seat, device)`, `Ctx::free_seat(seat)`), and
      may put several devices in one seat.
  - `SeatError`: a `max` of 0 or above `MAX_SEATS`
    (`Ctx::set_seat_policy`); a seat at or past `max`, or a device
    already in another seat (`Ctx::assign_seat`); either setter under a
    policy it doesn't apply to.
  - `SeatEvent`: `Joined(Seat, SeatDevice)`, `Left(Seat)`,
    `DeviceLost(Seat)`, `DeviceBack(Seat, SeatDevice)`, and
    `Unseated(SeatDevice)`: under `Fixed`, an unseated device pressed
    something, so the game can assign it.
  - `Seats<A>`: the policy and who sits where, a pure state machine. Its
    rules:
    - a join takes the lowest lost seat first (`DeviceBack`), else the
      lowest free seat below `max` (`Joined`); past `max`, nothing;
    - seat numbers never shift: a seat that leaves stays free until the
      next join takes it;
    - the last seated device can't leave (its `leave` does nothing), so
      someone always drives the menus;
    - a removed pad releases what it held. Under `Shared` that is all,
      as today. Under the other policies, a pad that was its seat's only
      device leaves the seat lost (`DeviceLost`), and the seat's held
      input is released; under `Fixed` it waits for the game to assign
      it again;
    - switching from `Shared` to another policy seats the device pressed
      last in seat 0 (`Joined`); switching to `Shared` frees every other
      seat (`Left` for each), and every device drives seat 0 again;
    - an unseated device's other presses are dropped (under `Fixed`,
      each gives one `Unseated`).
  - `Ctx::seats() -> &Seats<A>` and the setters above.
- **Pad ids on the raw events: 2412 (done) did this.** What this ticket
  starts from:
  - `RawInputEvent::PadDown(Button, PadKind, PadId)`,
    `PadUp(Button, PadId)`, and `PadRemoved(PadId)`.
  - `Pads::update` reports each pad's own changes and doesn't merge:
    every pad's releases (in `Button::ALL` order, then by pad id, so a
    poll's presses from several pads stay in one pad's order once
    merged), then each removed pad's `PadRemoved`, then every pad's
    presses.
  - `RawInputEvent::from_crank` and `from_crank_steps` take the `PadId`
    of the pad with the crank; the Harness and a script's steps use pad
    0, and so does the Playdate.
  - `PadMerge` (`crates/ui/src/game/pads.rs`) is `Pads`'s old merge,
    with one rule more: a button let go on one pad and pressed on
    another in one poll stays held. `game/pads/tests.rs` holds its
    tests and the property test against the old merge (`OldMerge`),
    which the property test under `Shared` below can reuse.
  - A session record written before 2412 reads its pad events as pad
    0, at the same `FORMAT_VERSION` (`serde(default)` on the pad).
  - The unfocused pad filter (`pad_events`, `crates/app/src/pads.rs`)
    still hands `from_pads` no pads (ADR-0076 §4), so a window losing
    focus gives each pad's `PadRemoved`. The 3042 bullet below changes
    that.
- **`Game` routes each event to a seat** (`Game::step`):
  - a key `Down` goes to the seat holding `Keyboard`. With halves
    seated, it goes to the seat whose half's layout binds the chord (by
    the player's bindings for that layout). A key both halves bind (the
    player rebound one) and a key neither binds (the game's fixed keys,
    such as Escape, among them) go to the first seat holding a half;
  - a key `Up` goes to the seat its `Down` went to;
  - `PadDown` and `PadUp` go to the pad's seat; `PadRemoved` to the seat
    rules above;
  - `Text` stays one shared stream, as today (the name box);
  - `Game::act` gives its action to the shared stream, as from seat 0;
  - **pads merged per seat:** each seat keeps the buttons each of its
    pads holds. Its `InputState` hears a press when the first of its pads
    presses a button, and a release when the last lets go or is removed.
    This is `Pads`'s merge of today, moved from all pads to each seat's
    pads, so `Shared` behaves exactly as before.
- **A keymap per seat.** Each seat has its own `InputState` (held keys,
  repeats, chord waits and device):
  - a seat holding the whole keyboard, or only pads, uses `Ctx::keymap`,
    as today. Its pad part is the current profile's pad bindings
    (`Ctx::pad_bindings`), one setup for every pad;
  - a seat holding a half uses `Ctx::keymap_for(half)`;
  - `sync_keymap` refreshes every seat's keymap when `Ctx`'s change.
  - **either order with 2404 (parked):** if 2404 is done, every seat's
    `InputState` gets the current profile's `PlayerInput` (toggled
    actions, repeat and chord window) through `set_player_input`, when
    the seat fills and when the settings change, as `sync_keymap` does
    for keymaps. If not, nothing to do (2404 does it).
  Rebinding on the key bindings screen changes them for every seat that
  uses them. `Ctx::device` stays the device pressed last in any seat,
  for shared help text. `Game::input()` stays seat 0's `InputState`, and
  `Game::seat_input(seat) -> Option<&InputState<G::Action>>` gives any
  filled seat's.
- **`FrameInput` per seat** (`crates/ui/src/input/frame.rs`):
  - `actions` and `is_held` stay one shared stream: every seat's
    actions, in seat order, and an action is held if any seat holds it.
    Menus, the pause screen and every engine screen work for any player
    with no change;
  - `FrameInput::seat(seat) -> Option<&SeatInput<A>>`: one seat's
    `actions` and `is_held`, `None` for an empty seat. Under `Shared`,
    seat 0 is the whole stream;
  - `FrameInput::seat_events() -> &[SeatEvent]`: this frame's joins,
    leaves, losses, returns and unseated presses, in order;
  - `declared` filters each seat's actions as it filters the shared
    ones, and `is_held` on a seat panics for an undeclared action the
    same way.
- **A split keyboard in the keymap file** (`crates/assets/src/keymap.rs`):
  - an optional `split_keyboard: ("LeftKeys", "RightKeys")` names two of
    the file's layouts that two players can use at once on one keyboard;
    `KeymapDef::split_keyboard: Option<(Layout, Layout)>`;
  - `KeymapDef::from_source` fails, one error each, on a layout the file
    doesn't list, the same layout twice, and a chord that both layouts
    bind (any action, any group);
  - under `JoinOnPress` with a split keyboard, the keyboard is seated
    only by halves: a press of the join action in one half's layout
    joins that half. A switch to `JoinOnPress` with the keyboard pressed
    last seats the half whose layout binds the last chord, else the
    first half. Under `Fixed`, a half is never seated while the whole
    keyboard is, nor the reverse. A file without the field joins the
    keyboard whole;
  - the keymap's baked form changes, so `pack::VERSION` becomes the next
    number, and the pack's equality test covers the field (ADR-0007,
    ADR-0019, ADR-0023);
  - `docs/formats.md` documents the field.
- **Harness scripts per pad** (`crates/platform/src/harness.rs`):
  - a token of `Harness::pad` and the button of `hold_pad` may start
    with `N:`, the pad's id from 0 to 7 (`MAX_SCRIPT_PADS`, 8: more pads
    than seats, for tests that fill them all).
    `"1:South+East 1:DpadUp South"` presses South and East together on
    pad 1, then DpadUp on pad 1, then South on pad 0. No prefix is pad
    0, as today;
  - `use_pad(kind)` stays pad 0's kind; `plug(pad, kind)` sets another
    pad's;
  - `unplug(pad)`: the pad's releases and its `PadRemoved`, in one frame;
  - `seats() -> Vec<(Seat, SeatDevice)>` and `seat_events()` (every
    frame's, oldest first);
  - `frame-png --pad "1:South"` and the contract's `run --pad` take the
    prefix: `Step::check` checks it up front, so `Script::parse_step`
    and `Script::check` refuse `x:South` and `9:South` before the game
    starts, not with a panic. The usage text in `frame_png.rs` and
    `contract/run.rs` shows it. `Step::Pad`'s doc comment names the
    prefix, which changes the schemas' text: raise `contract::VERSION`,
    regenerate `crates/cli/schemas/` and add the version's line to
    `versions.json` (ADR-0022 §5). The fake session
    (`crates/cli/src/contract/fake.rs`) and whatever else plays a
    `Script` (1133's session `step`) get the prefix with no
    change of their own.
- **Controls checked per player count:**
  - `shuyi_assets::target::Input::local_players: u8`: how many people
    can play at once on one machine there. 1 when the file leaves it
    out (a field default of 1, since `Input`'s derived `Default` would
    give 0). `TargetProfile::from_source` fails on 0 and above
    `MAX_LOCAL_PLAYERS` (4);
  - the engine's profiles in `targets/`: `desktop.ron`, `web.ron`,
    `steam-deck.ron` and `switch.ron` say 4; `playdate.ron`,
    `android.ron` and `ios.ron` leave it out;
  - `GameControls::with_seats(min, max)`: how many seats the game plays
    with, 1 and 1 until set;
  - three new `FitRule`s: `Players`, an error, when `min` is above the
    target's `local_players`; `PlayersCapped`, a warning, when `min`
    fits but `max` is above it (seats past it never fill there);
    `SplitKeyboard`, an error, on a target with a keyboard, when a
    required action of a screen has no key in one of the split
    keyboard's layouts. `FitRule::is_error` is false for
    `PlayersCapped` as for `Undeclared`;
  - pads need no new rule: every pad's seat plays with the target's one
    table, which `controls_fit` already checks;
  - `controls` in `crates/cli/src/contract/target.rs` gives their
    `setting` as `GameControls::with_seats` and `keymap split_keyboard`.
    These are new values in the answer, not a new shape (the version
    goes up once, for `Step::Pad` above).
- **From 2137 (done):** `TickInput` holds one entry per seat, by seat
  number (`TickInput::seats`, at most `MAX_SEATS`), and today
  `TickDriver` fills seat 0 alone, from the shared input, with one latch.
  Make it keep a latch per seat and fill each filled seat's entry from
  `FrameInput::seat(n)`. A seat that joins or leaves changes the entries
  from the next step, so a `TickRecord` replays it.
- **Either order with 1120:** if 1120 is done with its viewports, a test
  scene gives each filled seat the stage's layers in a viewport from
  `split_viewports(frame, n)` for the n filled seats, each camera
  following that seat's dot, redone when one joins or leaves. If 1120
  isn't done, 1120 writes that scene (an edit to 1120 says so).
- **Either order with 1121:** if 1121 is done, a stick's axes go to its
  pad's seat, and the pointer to the seat holding the keyboard (or its
  first half). If not, 1121 does that (an edit to 1121 says so).
- **Rumble per seat, either order with 2408 (parked):** if 2408 is done,
  `RumbleRequest::Play` names a `Seat` (`Seat::FIRST` unless the screen
  says), each seat has its own `RumblePlayer`, `Motors::set` and `stop`
  take the seat, and a seat's cue goes to the last pad pressed in that
  seat. If 2408 isn't done, 2408 does this when it lands.
- **Pointer, focus and records** (2405 and 3042 are After MVP, so they
  are likely done first; 2518 is done). For each one that is done:
  - 2405: its pointer events (`PointerMoved`, `PointerDown`,
    `PointerUp`, `Wheel`) go to the seat holding the keyboard, or its
    first half, and stay in the shared stream for menus;
  - 3042 (done), so this ticket does it: `Game::forget_held_input`
    forgets every seat's held input and each seat's held pad buttons.
    The unfocused pad filter (`pad_events`, `crates/app/src/pads.rs`)
    hands
    `RawInputEvent::from_pads` every connected pad with an empty
    `PadState`, not an empty list, so an alt-tab releases what pads hold
    but no pad loses its seat (an ADR amending ADR-0076 §4 says so);
  - 2518: 2412 gave the recorded `RawInputEvent`s their pad ids and
    `PadRemoved` at the same `FORMAT_VERSION`, an older record's pad
    events reading as pad 0. Seats add nothing to a record's events, so
    nothing more here.
  For each one still open, this PR adds a line to its ticket saying what
  it does for seats.
- **Docs:** `docs/platform-backends.md` (`poll_input`: what seats do with
  the pad ids and removals, where it says `Game` merges every pad), `pad.rs`'s
  module docs, and the `input-actions` skill (a rule: devices belong to
  seats; a stage that has players reads `FrameInput::seat`).

**Out (do not do):**

- a join screen with a slot per seat, and a "reconnect your pad" prompt:
  a game shows its own from `seat_events`; an engine screen is a
  follow-up when a game asks;
- pausing a one-player game when its pad is removed (a console
  certification rule, for when consoles are unparked: 3014, 3016);
- telling several keyboards or mice apart: macroquad and browsers report
  one of each;
- a profile per seat (each seat's own saved keys and settings): every
  seat plays with the current profile's bindings; a follow-up if a game
  asks;
- drawing split screen (1120's viewports, 2385's layouts, and 2386's
  view per seat, which reads this ticket's seats and `seat_events`),
  and camera feel per view (2362);
- online play (1140–1147): Remote Play Together needs nothing beyond
  this;
- our own pad code and its stable ids (2402, parked): gilrs and the
  browser give stable ids today;
- a second physical screen (3023): a surface is not a seat;
- each seat's own hold or toggle and timings: every seat plays with
  the current profile's (2404, the clause above), as with its bindings;
- more than 4 seats;
- turn-based hotseat: it works with `Shared` today.

## Implementation steps

1. Read ADR-0003 §2 and §8, ADR-0025, 2013's ADR, `pad.rs`, `game.rs`,
   `input/state.rs`, `input/frame.rs`, `ctx.rs` (`keymap`, `keymap_for`,
   `pad_bindings`), `harness.rs`, `frame_png.rs`'s `Step` and `Script`,
   `fit.rs`, `target.rs` and 2137's `tick.rs`. Check which of 1120,
   1121, 2405 and 2408 are done.
2. Write the ADR.
3. Move 2412's merge in `Game` (`PadMerge`, `crates/ui/src/game/pads.rs`)
   from every pad to each seat's pads, with its tests
   (`game/pads/tests.rs`).
4. The `seat` module and its state machine, with unit tests.
5. Routing in `Game`, the merge per seat, an `InputState` per seat,
   `Game::seat_input`, `FrameInput::seat` and `seat_events`, and
   `Ctx`'s setters.
6. The keymap's `split_keyboard`, its errors, the pack's version and
   `docs/formats.md`.
7. The Harness's pad ids, `plug`, `unplug`, `seats`, `seat_events`, and
   `--pad N:` with the contract's new version.
8. Measure the diff (snapshots and fixtures aside). If it is past about
   900 changed lines, leave step 9 out: write it as a parked ticket of
   its own (`write-ticket` skill, the next free 24xx id, blocked by this
   one), move its acceptance criteria there, and say so in the
   completion notes.
9. `local_players` in the profiles, `GameControls::with_seats`, the new
   rules and the contract's settings.
10. The either-order parts that are due, the lines this PR adds to the
    open ticket of 2405, the docs and the skill.
11. Run the gates.

## Acceptance criteria

- [ ] `Shared` changes nothing: every existing test passes with only its
      pad events given pad 0, every snapshot and `frame-png` picture is
      unchanged, and the two moved pad tests pass at the level of
      `Game`.
- [ ] A Harness test (`crates/ui/tests/it/seats.rs`, with a small test
      screen that sets `JoinOnPress` and moves one dot per seat): pads 0
      and 1 join as seats 0 and 1, each moves only its own dot, and pad
      2 past a `max` of 2 joins nothing.
- [ ] The same with a split keyboard: each half moves its own dot, and
      a fixed key goes to the first half's seat.
- [ ] `unplug(1)` gives `DeviceLost` for seat 1 and releases its held
      direction; pad 3 pressing the join action then gives `DeviceBack`
      for seat 1 with `Pad(3)`.
- [ ] `leave` frees a seat without moving the others' numbers; the last
      seated device's `leave` does nothing.
- [ ] Under `Fixed`, an unseated pad's press gives one `Unseated` and no
      action; after `Ctx::assign_seat`, its presses reach that seat.
- [ ] `FrameInput::actions` holds every seat's actions in seat order, so
      an engine menu works from any seat (a test opens and moves one
      from seat 1).
- [ ] `frame-png --pad "1:South"` works; `--pad "x:South"` and
      `--pad "9:South"` are errors before the game starts; the schema
      test passes with the new contract version's line.
- [ ] A keymap file whose `split_keyboard` names an unknown layout, the
      same layout twice, or two layouts sharing a chord fails with one
      error each; a baked keymap with the field equals its source.
- [ ] A profile without `local_players` reads as 1; 0 and 5 are errors.
- [ ] `GameControls::with_seats(2, 4)` on `playdate` gives a `Players`
      error; `with_seats(1, 4)` there gives a `PlayersCapped` warning
      and no error; a split keyboard missing a required action in one
      half gives a `SplitKeyboard` error on `desktop`.
- [ ] The PR has a `Breaking:` line naming every break 2010's check
      reports. The pad events were 2412's, and `KeymapDef`,
      `target::Input` and `FitRule` take new fields and variants
      without a break since 2083, 2085 and 2082, so the line is empty
      unless the check finds one.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Seats` (join, lowest free seat, lost then back, leave, the
  last seat staying, `max`, `Fixed` assign, free and `Unseated`, each
  `SeatError`, switching policy both ways, with and without a split
  keyboard); routing a key with halves (bound in one half, bound in
  both, a fixed key, an unbound key, an `Up` after its `Down`);
  the `N:` prefix and its
  bounds; the keymap's errors; `local_players` bounds; each new
  `FitRule`.
- Property: under `Shared`, any sequence of key and pad events from up
  to four pads gives the same actions and held actions as today's merge
  (a copy of the old `Pads::down` union in the test); under
  `JoinOnPress`, no device is ever in two seats, and a seated device
  keeps its seat number until it leaves or is lost.
- Snapshot / integration: the Harness tests above; the pack's equality
  test with `split_keyboard`; the schema test; `check --target` on a
  test game's controls.

## Completion notes

*(Filled in by the session that completes the ticket.)*
