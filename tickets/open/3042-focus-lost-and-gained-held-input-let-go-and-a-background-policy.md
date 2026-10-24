---
id: "3042"
title: "Focus on the desktop and the web: shuyi-app sends FocusLost and FocusGained, held keys and pads are let go on focus loss and suspend, and a game may pause or mute in the background"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3042 — Focus lost and gained, held input let go, and a background policy

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6,
Unreal Engine 5.8 and the 2D engines (Defold here). Each tells a game
when its window loses and gets back the player's attention, and most
can stop a game or its pads in the background:

- Godot: `NOTIFICATION_APPLICATION_FOCUS_OUT` and `_FOCUS_IN`, and the
  project setting `input_devices/joypads/ignore_joypad_on_unfocused_application`
  (<https://docs.godotengine.org/en/stable/classes/class_node.html>).
- Unity: `OnApplicationFocus`, `OnApplicationPause` and
  `Application.runInBackground`
  (<https://docs.unity3d.com/ScriptReference/Application-runInBackground.html>).
- Bevy: the `WindowFocused` and `WindowOccluded` events, and
  `WinitSettings`' `unfocused_mode`
  (<https://docs.rs/bevy/latest/bevy/winit/struct.WinitSettings.html>).
- Unreal: `FCoreDelegates::ApplicationWillDeactivateDelegate` and
  `ApplicationHasReactivatedDelegate`.
- Defold: `window.set_listener` with `WINDOW_EVENT_FOCUS_LOST` and
  `WINDOW_EVENT_FOCUS_GAINED` (<https://defold.com/ref/stable/window/>).

What Shuyi has today (checked 2026-10-21):

- **The events exist, and nothing sends them.** 3003 (done) built
  `SystemEvent` with `FocusLost` and `FocusGained`
  (`crates/platform/src/platform.rs:151`). `App::on_system_event`
  (`crates/platform/src/app.rs:401`) only sets `focused` on them
  (line 411), then the game's `SystemHook` hears them, which ADR-0024
  §5 calls the place a game "may pause". The macroquad backend's
  `system_events` (`crates/app/src/backend.rs:272`) sends only
  `ToggleFullscreen` and `QuitRequested`, as 3003's Follow-ups line and
  ADR-0024 §5 say. So on the desktop and the web the hook never hears a
  focus event.
- **miniquad already reports focus, under another name.** miniquad
  0.4.11's `window_minimized_event` and `window_restored_event`
  (`src/event.rs:210-220`) fire on focus changes: `WM_ACTIVATE` on
  Windows (`src/native/windows.rs:763`, and also `WM_SIZE` when
  minimized, lines 439-451), key-window changes on macOS
  (`src/native/macos.rs:446-457`), `FocusIn` and `FocusOut` on X11
  (`src/native/linux_x11.rs:146-151`), keyboard enter and leave on
  Wayland (`src/native/linux_wayland.rs:577-601`), and the web bundle's
  `focus`, `blur` and `visibilitychange` listener calling
  `wasm_exports.focus` (`crates/cli/web/mq_js_bundle.js`;
  `src/native/wasm.rs:334`). Two quirks: Windows sends the event twice
  on a minimize (both messages), and macOS also sends
  `window_minimized_event` when the window is moved
  (`window_did_move`, `src/native/macos.rs:406`), with no restore after.
  macroquad 0.4.16 queues both as `MiniquadInputEvent::WindowMinimized`
  and `WindowRestored` for input subscribers, beside the key and mouse
  events (`macroquad::input::utils::register_input_subscriber` and
  `repeat_all_miniquad_input`, `src/input.rs:231-252`). Nothing in
  `shuyi-app` subscribes.
- **Held keys are let go on the desktop, held pads are not.**
  macroquad's `window_minimized_event` (`src/lib.rs:842-861`) moves
  every held key into `keys_released`, and `keys::poll`
  (`crates/app/src/keys.rs:122`) reads `get_keys_released`, so an
  alt-tab already releases the keyboard. Pads are another matter:
  `pads::native::Source::read` (`crates/app/src/pads/native.rs:51`)
  reads gilrs whether the window has focus or not, so a pad keeps
  driving a game the player has left, and a button held across the
  focus change stays held.
- **A key held through a suspend stays held for good.** While
  suspended, `App::run` (`crates/platform/src/app.rs:429-433`) drains
  the input and drops it, so a release that happens then never reaches
  `InputState`. After `Resume`, `InputState`
  (`crates/ui/src/input/state.rs`) still holds the key, and a
  repeatable action repeats until the key is pressed and let go again.
  The Playdate's lock (3010) and a hidden browser tab (3043) both
  suspend. `InputState` has no way to forget what is held except
  `set_keymap` (line 187), which forgets everything as a side effect
  of new bindings.
- **No policy for the background.** A game that wants to stop while
  the player is elsewhere, or to go quiet, has to write it in its hook.
  2321's Out section asks for a ticket that lowers the frame rate
  while unfocused; that is 3043. Sound held on `Suspend` is 2344's.

Why it matters:

- **The game first** (2026-10-20, `docs/decisions.md`: the engine is
  built for Visions of Shuyi first). The game runs on the desktop and in
  the browser. Today a pad held while the player alt-tabs keeps acting
  on the game, and its hook never hears the focus events the engine
  documents. At its next pin bump it gets the events and the release
  for free, and keeps running in the background as today, since the
  engine's default policy changes nothing.
- **Other 2D games.** A game with a clock of its own (a timed puzzle,
  text typing out, a tick game) may want to stop while the player is in
  another window, and many games mute there. Each would write the same
  hook.
- **The end goal** (2026-10-19, `docs/decisions.md`): an agent's game
  gets the pause and the mute from one constant, not a hook of its own,
  and the Harness proves no key sticks. Until the game launches, that
  game is turn-based (2026-10-20); real-time games come later and use
  the same constant.

Rules it keeps: only `shuyi-app` touches the window and the pads
(`AGENTS.md` rule 5); the pause is "no update", so replays and hashes
don't change; actions, never keys (rule 8).

## Nick input

None. The engine's default keeps today's behaviour (keep running, keep
the sound), so the game gets nothing new until it opts in. Whether
Visions of Shuyi pauses or mutes in the background is the game's
choice, made in its own repository.

## Scope

**In:**
- **`shuyi-app` sends `FocusLost` and `FocusGained`** on Windows,
  macOS, Linux (X11 and Wayland) and the web:
  - `Macroquad::new` registers a macroquad input subscriber
    (`macroquad::input::utils::register_input_subscriber`).
    `Macroquad::system_events` replays it each frame
    (`repeat_all_miniquad_input`) into a small
    `macroquad::miniquad::EventHandler` that records, in order,
    `window_minimized_event`, `window_restored_event`,
    `key_down_event` and `mouse_button_down_event`, and nothing else
    (its required `update` and `draw` do nothing);
  - a pure `Focus` tracker in a new `crates/app/src/focus.rs` turns
    what was recorded into events: a minimize while focused gives
    `FocusLost`; a restore while unfocused gives `FocusGained`; a
    repeat gives nothing (Windows' double event); a key press or a
    mouse button press while unfocused gives `FocusGained`, since only
    a focused window gets them (macOS's move quirk). It starts focused,
    as `App` does;
  - focus events come before `ToggleFullscreen` and `QuitRequested` in
    the frame's list;
  - in debug builds the backend logs each system event it sends, at
    `info` level. **Either order with 3041:** if it is done, with
    `log::info!(target: "platform", …)`; if not, with macroquad's
    `info!` (the browser console on the web), and 3041 moves it with
    the backend's other lines;
  - **Either order with 3044** (the cursor): if it is done, its
    `IdleCursor` shows the cursor on `FocusLost`, so it is never hidden
    over another program; if not, 3044 does that (its scope says so).
  - **Either order with 3037** (parked, Steam): if 3037 is done, feed
    its Steam overlay's open and close to the `Focus` tracker, so the
    pad filter ignores pads while the overlay is open and no focus
    event comes twice; if not, nothing to do: 3037 feeds the tracker
    (its clause says so). Either way `App` forgets held input on the
    overlay's `FocusLost`, as on any other.
- **Pads are ignored while unfocused.** `PadInput::poll`
  (`crates/app/src/pads.rs:37`) takes whether the window has focus (the
  `Focus` tracker's state, set in `system_events` earlier in the same
  frame):
  - unfocused, it still reads gilrs (so its event queue keeps draining)
    and the browser, but hands `RawInputEvent::from_pads` an empty list
    of pads, so every held button and stick direction goes up and
    nothing new goes down (`Pads::update` treats a pad left out as
    removed);
  - on the frame focus comes back, it drops that frame's `PadDown`s, so
    a button held across the change counts only once it is let go and
    pressed again, as a key does.
  The filtering is a pure function in `pads.rs`, tested without gilrs.
- **`App` forgets held input on `FocusLost` and on `Suspend`,** whatever
  the backend did:
  - `InputState::forget_held(&mut self)` in `shuyi-ui`: empties `held`,
    `pending`, `waiting`, `repeat` and `buttons_down` without giving any
    action (a waiting chord button gives nothing, unlike `pad_up`),
    keeping the keymap and the device. `set_keymap` becomes
    `forget_held` plus the new keymap. A key still down is ignored until
    pressed again;
  - `Game::forget_held_input(&mut self)` calls it;
  - `App::on_system_event` calls it on `FocusLost` and `Suspend` once
    the game runs, before the game's hook. So the frame's releases that
    follow (macroquad's keys, the pad filter's `PadUp`s) find nothing
    held. A `HurryClock` owner sees `is_held` false and its clock stops
    by itself.
  - **Either order with 2405:** if 2405 is done,
    `Game::forget_held_input` also forgets a held pointer press (so its
    release gives no click) and the pointer's place
    (`FrameInput::pointer()` is `None` until it moves again), with a
    Harness test. If 2405 is not done, 2405 does it.
  - **Either order with 2406** (parked, input seats): if 2406 is done,
    `forget_held_input` forgets every seat's held input, and the
    unfocused filter hands `from_pads` every connected pad with an empty
    `PadState`, not an empty list, so no pad loses its seat. If 2406 is
    not done, 2406 does it.
- **A background policy the game sets**, in `shuyi-ui`
  (`crates/ui/src/settings.rs`, `no_std`):
  - `Background { pub pause: bool, pub audio: BackgroundAudio }`, and
    `BackgroundAudio::Keep` and `Mute`, both `#[non_exhaustive]` so
    3043's frame cap and a later duck can join; a const
    `Background::KEEP_RUNNING` (no pause, `Keep`) and const builders
    `paused(self)` and `muted(self)`, so a game writes
    `Background::KEEP_RUNNING.paused().muted()`;
  - `GameSettings::BACKGROUND: Background`, an associated const that
    defaults to `Background::KEEP_RUNNING`, so every game compiles and
    behaves as before. (2321 adds its frame-cap consts to the same
    trait.)
- **`App` applies it while unfocused** (not while suspended, which
  already stops everything; sound on `Suspend` is 2344's). `App` keeps
  the policy in a private field set from
  `<G::Settings as GameSettings>::BACKGROUND` when it is made, so its
  own tests (a child module) can set another policy on the toy:
  - **pause:** while the game runs and is unfocused, no update: the
    input is drained and dropped, no fixed steps, the screens see no
    time, and the game's last frame is presented again each frame. The
    loading card and the load go on as before. `FocusGained` restarts
    the clock as `Resume` does, so no catch-up steps. `QuitRequested`
    still quits. `App::is_paused()` says so;
  - **mute:** while unfocused, every frame's `AudioFrame` carries its
    requests as usual with the music, sound and voice volumes at 0, so
    the music keeps its place and a sound started meanwhile is silent
    rather than lost. The music and a voice already playing go quiet at
    once; a sound already playing plays out, since a sound's volume is
    set when it starts (`crates/app/src/audio.rs:206`). While paused and
    muted, `App` still calls `audio().play` each frame with no requests
    and the volumes at 0. The first frame after `FocusGained` sends the
    player's volumes again. **Either order with 2342:** if it is done,
    mute hands the frame `Gains::muted()` instead; if not, 2342
    carries this over;
  - the game's hook still hears `FocusLost` and `FocusGained` after
    `App`'s part.
- **Either order with 2018:** if it is done, a capture run reads no
  focus events (the tracker is skipped, as the keyboard is); if not,
  2018 keeps the capture mode so (its own clause says it).
- **Either order with 2408** (parked, rumble): if 2408 is done, its
  rule already stops the motors on the `FocusLost` this sends, and
  drops cues until `FocusGained`, whatever the `Background` policy; if
  not, nothing to do (2408 needs nothing more from this ticket).
- **Either order with 2738** (the brick-breaker, parked): if 2738 is
  done, `examples/bricks`' system hook pushes its pause menu on
  `FocusLost`, with a Harness test in its `tests/it/`; if not, nothing
  to do (2738 does it when it is written).
- **Either order with 2739** (parked, the real-time grid): if 2739 is
  done, `examples/snake` and the `stage-arcade-grid` part of
  `templates/shuyi-default/` set `GameSettings::BACKGROUND` to
  `Background::KEEP_RUNNING.paused()`, and the snake's system hook
  pushes its pause menu on `FocusLost`; if not, nothing to do (2739
  does it when it is written).
- **An ADR** (the `write-adr` skill, the next free number), extending
  ADR-0024 §5:
  - `FocusLost` and `FocusGained` mean the window lost or got the
    keyboard's focus; `Suspend` and `Resume` mean the game can't be seen
    or can't run (a hidden browser tab, 3043; a device asleep; a phone
    in the background, 3015). A backend that can't tell a minimized
    window from an unfocused one sends only the focus events;
  - `App` forgets held input on `FocusLost` and `Suspend`;
  - the game's `Background` policy, and why the default changes nothing;
  - how the macroquad backend maps miniquad's events, its two quirks,
    and that a winit backend (3034) maps `WindowEvent::Focused` and
    `Occluded` the same way.
  ADR-0024's row in `docs/adr/README.md` says "extended by ADR-NNNN";
  its text stays as it is.
- **`docs/platform-backends.md`:** the `FocusLost` and `FocusGained` row
  of the `system_events` table says what `App` does now (forgets held
  input, applies the game's policy); the `Suspend` row says held input
  is forgotten; *What one frame does* gains the paused branch; a
  paragraph says what the macroquad backend sends and how.
- **What changes for the game:** additive only (`GameSettings::BACKGROUND`
  with a default, `InputState::forget_held`, `Game::forget_held_input`,
  `App::is_paused`, the `Background` types). Its hook now hears
  `FocusLost` and `FocusGained` on the desktop and the web; if it
  matches on `SystemEvent`, its `_` arm already covers them. The game's
  PR that moves the pin follows this one. The new public types follow
  ADR-0039.
- **Size:** if the PR passes about 600 changed lines, the background
  policy (`Background`, `GameSettings::BACKGROUND`, the pause and the
  mute, with their tests) goes to a follow-up ticket (the
  `write-ticket` skill, After MVP, blocked by this one), and 3043, which
  builds on `Background`, then waits for that ticket too. The same PR
  changes 2342's, 2408's and 2739's mentions of 3042 for the background
  policy and the mute to the new ticket's id.

**Out (do not do):**
- the web build's hidden tab and closing page (`Suspend`, `Resume` and
  the save inside the browser's event), fewer frames in the
  background, and the player's Mute in background setting: 3043;
- holding sound on `Suspend`, and looped sounds that follow the mute:
  2344;
- `Suspend` for a minimized desktop window: miniquad 0.4.11 reports a
  minimize as a focus change, so the backend can't tell. A winit
  backend can (3034, parked);
- the Steam Deck's resume after a long gap between frames, and the
  Deck's checks: 3012, which waits for this ticket. This ticket sends
  `FocusLost` and `FocusGained` on every desktop, the Deck's included,
  so 3012's Focus bullet adds nothing Deck-specific;
- Android and iOS (3015, parked);
- a latch or toggle cleared on focus loss: 2404 (parked) builds on
  `forget_held`;
- reading the focus at start: the backend starts focused, as `App`
  does;
- a pause menu that opens on focus loss: a game's hook can push one;
  the engine adds none;
- a background policy for the other templates and examples: they keep
  the default (the real-time grid's is the clause on 2739 above).

## Implementation steps

1. Read `crates/app/src/backend.rs`, `pads.rs`, `pads/native.rs`,
   `keys.rs`, `audio.rs`; `crates/platform/src/app.rs` and its tests
   (`crates/platform/src/app/tests.rs`); `crates/ui/src/input/state.rs`,
   `input/pad.rs` (`Pads::update`), `game.rs` and `settings.rs`;
   ADR-0024 §5 and `docs/platform-backends.md`; macroquad 0.4.16's
   `src/lib.rs:826-861` and `src/input.rs:231-252`. Check whether 2018,
   2342, 2405, 3037, 3041 and 3044 are done.
2. `InputState::forget_held`, `set_keymap` on top of it, and
   `Game::forget_held_input`, with unit tests. If 2405 is done, the
   pointer part too.
3. `Background`, `BackgroundAudio` and `GameSettings::BACKGROUND`, with
   tests of the builders.
4. `App`: the policy field, forget on `FocusLost` and `Suspend`, the
   pause and the mute, `is_paused`, with tests through
   `HeadlessPlatform` in `crates/platform/src/app/tests.rs`.
5. `crates/app/src/focus.rs` and its tests, then the subscriber, the
   recording `EventHandler` and `system_events` in `backend.rs`, and the
   debug log line.
6. The pad filter in `pads.rs`, with tests on fake pad states, and
   `poll_input` handing it the tracker's state.
7. A Harness test in `examples/hello-grid/tests/it/focus.rs`, beside
   `suspend.rs`, with `mod focus;` in `tests/it/main.rs`.
8. Run hello-grid on Windows and in the browser (the `run` skill): hold
   a direction, alt-tab or click outside, come back. Say in the
   Completion notes what the log showed.
9. The ADR and `docs/platform-backends.md`, then the gates.

## Acceptance criteria

- [ ] `Focus` unit tests: one `FocusLost` for two minimizes in a row;
      `FocusGained` on a restore while unfocused, nothing on a restore
      while focused; `FocusGained` on a key press or a mouse button
      press while unfocused, nothing on one while focused.
- [ ] Pad filter tests: unfocused with a button and a stick direction
      held gives their `PadUp`s and no `PadDown`; a press while
      unfocused gives nothing; on regaining focus a button still held
      gives no `PadDown`, and its later release and press give a
      `PadUp` then a `PadDown`.
- [ ] `InputState::forget_held` tests: a held repeatable action stops
      repeating; a waiting chord button gives no action, then or on its
      release; the device and keymap stay; the next press of the same
      key acts.
- [ ] Harness, hello-grid, on the stage: `Right` held on a frame with
      no release, then `system(SystemEvent::FocusLost)`: `is_held` is
      false for its action, and after a second of frames the cursor is
      where the press left it.
- [ ] Harness, hello-grid, on the stage: `Right` held, `Suspend`, its
      release sent while suspended, `Resume`: after a second of frames
      the cursor is where the press left it. (On `main` this fails,
      since the release is dropped and the move repeats; check it does
      first, and say so in the Completion notes.)
- [ ] `App` tests with the policy set to `paused()`: after `FocusLost`,
      frames run no update and no steps and present the last frame;
      after `FocusGained` the first frame counts no time, and a
      10-minute gap steps at most once.
- [ ] `App` tests with `muted()`: while unfocused the `AudioFrame`
      volumes are 0 and its requests are still passed; paused and
      muted, `play` is called with no requests at volume 0; after
      `FocusGained` the player's volumes come back.
- [ ] If 2405 is done: a pointer press held across `FocusLost` gives no
      click on its release, and `FrameInput::pointer()` is `None` until
      the pointer moves (Harness).
- [ ] With the default policy, every existing Harness, snapshot and
      replay test passes unchanged.
- [ ] The ADR is written and ADR-0024's row names it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the `Focus` tracker; the pad filter; `InputState::forget_held`;
  `Background`'s builders.
- Integration: `App` through `HeadlessPlatform` for forget, pause and
  mute; hello-grid's `focus.rs` through the Harness.
- Manual: hello-grid on the desktop and the web, recorded in the
  Completion notes.

## Completion notes

*(Filled in by the session that completes the ticket.)*
