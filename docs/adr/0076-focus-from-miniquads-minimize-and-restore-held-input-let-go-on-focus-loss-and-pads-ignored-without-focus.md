# ADR-0076: Focus from miniquad's minimize and restore, held input let go on focus loss, and pads ignored without focus

- **Status:** Accepted
- **Date:** 2026-11-04
- **Related tickets:** 3042 (this), 2411, 3093, 3043, 3034, 3037
- **Extends:** ADR-0024 §5

## Context

ADR-0024 §5 named `FocusLost` and `FocusGained` (3003) and said a game
"may pause" on `FocusLost`, but the macroquad backend sent only
`ToggleFullscreen` and `QuitRequested`, so on the desktop and the web the
game's hook never heard a focus event. Two more things went wrong while
the window had no focus:

- **Held input.** A window without focus hears no release. macroquad
  lets go of its keys on a minimize, but `App`'s own input state kept a
  key or button a backend never released, and a repeatable action
  repeated until it was pressed again. 2411 fixed the same thing for a
  suspend: the first frame after `Resume` lets go of everything
  (`InputState::release_all`).
- **Pads.** gilrs and the browser's Gamepad API read the pads whether the
  window has focus or not, so a pad kept driving a game the player had
  left.

miniquad 0.4.11 has no focus event of its own. It reports a focus change
as `window_minimized_event` and `window_restored_event`: `WM_ACTIVATE` on
Windows (and `WM_SIZE` when minimized), the key window on macOS,
`FocusIn` and `FocusOut` on X11, keyboard enter and leave on Wayland, and
the web bundle's `focus`, `blur` and `visibilitychange` listener. Two
quirks: Windows sends the event twice on a minimize, and macOS also sends
`window_minimized_event` when the window is moved, with no restore after.
macroquad 0.4.16 queues both for an input subscriber
(`macroquad::input::utils`), beside the keys and the mouse.

## Decision

### 1. What the events mean

`FocusLost` and `FocusGained` mean the window lost or got the keyboard's
focus. `Suspend` and `Resume` mean the game can't be seen or can't run (a
hidden browser tab, 3043; a device asleep; a phone in the background,
3015). A backend that can't tell a minimized window from an unfocused one
sends only the focus events: miniquad's can't, so a minimized desktop
window loses focus and is not suspended.

### 2. The macroquad backend sends them

`crates/app/src/focus.rs`:

- `WindowFocus` registers a macroquad input subscriber on the first frame
  it reads (so a run that never reads the window, a capture's, queues
  nothing) and replays it each frame into `WindowHeard`, a miniquad
  `EventHandler` in `keys.rs` (the one file that reads the keyboard) that
  keeps, in order, `window_minimized_event`,
  `window_restored_event`, `key_down_event` and
  `mouse_button_down_event` as `Heard::Minimized`, `Restored` and
  `Pressed`, and nothing else;
- `Focus`, pure, turns them into events. It starts focused, as `App`
  does. A minimize while focused gives `FocusLost`; a restore or a press
  while unfocused gives `FocusGained`; anything that changes nothing gives
  nothing. So Windows' second minimize is dropped, and after macOS's move
  the next key or mouse press brings focus back, since only a focused
  window gets one.

`Macroquad::system_events` hands over the focus events before
`ToggleFullscreen` and `QuitRequested`, and in debug builds logs each event
it sends at `info` level (macroquad's `info!`, the browser console on the
web; 3041 moves it with the backend's other lines). A capture run's
platform reads no window events. A winit backend (3034) maps
`WindowEvent::Focused` to the focus events and `WindowEvent::Occluded` to
`Suspend` and `Resume`, where it is reliable.

### 3. `App` lets go of held input on `FocusLost`

`App::on_system_event` calls `Game::forget_held_input` (which calls
`InputState::release_all`) on `FocusLost` once the game runs, before the
game's hook. The next frame reports the actions let go, and a key or
button still down counts once it is pressed again; a `HurryClock` owner
sees `is_held` false and its clock stops by itself. On `Suspend` the same
happens on the first frame after the `Resume` (2411): nothing runs
between the two, so letting go at the suspend too would change nothing.

### 4. Pads are ignored without focus

`PadInput::poll(focused)` (`crates/app/src/pads.rs`) takes the tracker's
state, set by `system_events` earlier in the same frame (on the web's
skipped frames `Macroquad::hold_input` reads the system's events first
for this). Unfocused, it still reads gilrs, so its queue keeps draining,
but hands `RawInputEvent::from_pads` no pads, so every held button and
stick direction goes up and nothing goes down. On the frame focus comes
back it drops that frame's `PadDown`s, so a button held across the change
counts once it is let go and pressed again, as a key does. The rule is a
pure function, `pad_events`, tested without gilrs.

### 5. What a game does while unfocused

Nothing changes by default: the game keeps running with its sound, and
its hook hears both events after `App`'s part. A policy the game sets
(paused, muted) is 3093's.

## Consequences

- The game's hook hears `FocusLost` and `FocusGained` on every desktop
  and the web; one that matches on `SystemEvent` already covers them with
  its `_` arm.
- No key or pad button sticks across an alt-tab, and a pad does nothing
  in a game the player has left.
- On macOS, moving the window reads as a focus loss until the next key or
  mouse press: the pads are ignored meanwhile and the hook hears a
  `FocusLost` it may act on. That is the price of miniquad's events; a
  winit backend (3034) removes it.
- A press that brings focus back (the click on the window) reaches the
  game as any press does; only pad presses of that frame are dropped.
- `PadInput::poll` takes the focus: a change to `shuyi-app`'s public API.

## Alternatives considered

- **Read miniquad's window state each frame** — miniquad 0.4.11 has no
  call that says whether the window has focus; only the events tell.
- **Register the subscriber in `Macroquad::new`** — a capture run never
  replays it, and macroquad would queue every key and mouse event for it
  for the whole run.
- **Let go of held input in `shuyi-app`** — `App` is where every backend's
  events meet; a Playdate or a winit backend gets the rule for nothing.
- **Pause or mute by default in the background** — it would change what
  every game does at its next pin bump; 3093 lets a game choose.
- **Hand the unfocused filter every pad with an empty state** — the same
  releases, but 2406's seats are not built; it does that when they are.
