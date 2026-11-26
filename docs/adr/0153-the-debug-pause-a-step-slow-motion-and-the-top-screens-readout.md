# ADR-0153: The debug pause: a step, slow motion, and the top screen's readout

- **Status:** Accepted
- **Date:** 2026-11-26
- **Extends:** ADR-0085 (the debug view)
- **Related tickets:** 2316 (this), 2315 (the debug view), 2394 (sounds
  held under a pause), 2358 (modules), 2137 (the tick driver), 2521 (the
  live link), 2123 (game speed, parked), 2367 (overlays that let the
  screen below run, parked), 2375 (the `log` channel), 2104 (animation
  state machines, parked), 2356 (the console), 2357 (rewind)

## Context

ADR-0085 draws what the game's logic sees over the running game. To
read it, a developer stops the game on the frame where something goes
wrong, then moves it on a frame at a time, as Godot's debugger and
Unreal's "pause and step" do. Nick asked for the debug view "in
realtime" (`docs/decisions.md`, 2026-10-19).

The rules that shape it:

- **Determinism** (ADR-0002 §2): a run paused and stepped through must
  end in the state the same inputs give played straight, so a session
  record (2518) of it plays back the same, and 2357 can land paused.
- **No key is hard-coded** (`AGENTS.md` rule 8): pause and step are
  actions a game maps from its keymap file, like Debug.
- **Debug tools only** (`Ctx::debug_tools`): a shipped build pays for
  none of it.

The `input-actions` skill allows a new `UiAction` only when every game's
tools need it, with an ADR: this is that ADR.

## Decision

1. **Two actions.** `UiAction::DebugPause` and `UiAction::DebugStep`
   join `Debug`. `UiAction`'s own `ActionSet` names them, makes them
   neither required nor rebindable, maps them to themselves, and keeps
   them out of `ALL` (ADR-0036 §3). `UiAction::is_debug` names the three.
   A game maps its own actions onto them, keyboard only; one that
   doesn't gets no pause. The sample keymap gives them `F3` and `F4`.
2. **The state is on `DebugView`** (`Ctx::debug_view`): `paused`, a
   pending step (`step()`, taken once by `take_step()`, which counts it
   in `steps()`), and `slow` (1 at normal speed, up to `SLOWEST` = 8),
   with its frame count (`slow_frame_runs()`). None of it is saved.
3. **`Game::frame` paces each frame** (`Pace`), only with debug tools on:
   - The DebugPause and DebugStep keys act the moment they are pressed,
     read from the frame's key presses through the keymap (they are
     keyboard only), before the frame's input is applied, so the frame's
     time is chosen before the input state counts it. Neither action
     ever reaches a screen (nor its held or pressed lists).
   - **A debug tools' screen on top** (`GameHooks::debug_screens`) runs
     as usual, with the frame's input, on a paused game too: the debug
     menu, the "Debug view" screen and 2356's console work while paused.
     The game's clock and the modules stand while it is paused.
   - **Paused:** no screen of the game updates, no module's
     `before_update` runs, `Ctx::clock_s` stands, and the input state
     advances by no time (so a held key's repeat doesn't count). The
     frame's actions are dropped, not kept, but the Debug action still
     opens the debug menu. The stack is drawn and the gizmos built.
   - **A step**, asked while paused, runs one update with this frame's
     input, of `STEP_DT` (1/60 s) and one fixed step (`ticks` 1, so a
     `TickDriver` steps its rules once at any frame time), whatever the
     frame's own time. So a script played as one step a frame, with any
     paused frames between, ends with the script's hash played straight.
   - **Slow motion** (`slow` above 1) updates on the last of each `slow`
     frames, with that frame's own time and steps: `dt` is never
     stretched. The input events of the frames between are kept, in
     order, and applied before the updating frame's own, so a press is
     never lost and repeats keep the slowed pace. Their fixed steps are
     dropped, so a tick game slows too. A pause drops what it kept.
   - `Game::act` takes the two actions the same way, and drops any other
     while the game is paused and no debug tools' screen is on top.
4. **Sounds.** The pause holds the sounds started with a handle under
   the queue's owner of code outside the stack (2394's
   `OUTSIDE_THE_STACK`, which no pop releases), and the game lets them
   go on when it runs again; a step doesn't. Their fades take only the
   time the game's screens played. The music and one-shot cues play on.
5. **The readout** is a channel of the debug view, `readout`, offered by
   the game itself after the screens' channels. While it is on,
   `Game::redraw` draws, as `Note` labels from the frame's top-left cell,
   a row each: `paused` (or `paused, step N`) while paused, then the top
   screen's `Screen::debug_readout(&self, ctx) -> Vec<String>`, a
   defaulted method asked of the top screen only. A later corner channel
   (2375's log) starts on the row after the readout's last.
6. **The "Debug view" screen** lists slow motion after the channels
   (`ViewRow::SlowMotion`; Confirm goes 1x, 1/2x, 1/4x, 1/8x and back to
   1x) and says under its title whether the game runs or is paused, with
   the bound keys of the pause and the step.
7. **Over the live link** (ADR-0047) a client pauses and steps by sending
   the game's actions as `act` steps: nothing new on the wire.

## Consequences

- A paused session record replays paused: the pause keys are in its
  input, and the pacing is a function of the input alone.
- The game's frame counts and its updates now differ in debug builds;
  2357 counts the frames the stack didn't update.
- `UiAction` gains two variants (it is `#[non_exhaustive]`); the sample
  keymap gains two actions, which moves the sample's golden value and
  its pack snapshot (the fixture's data, not a format).
- `DebugViewScreen::menu` holds `ViewRow`s, a break for code that read
  it.
- A debug tools' screen on top of a paused game sees real frame times,
  which a menu needs, while the game's clock stands.
- Game speed (2123, parked) composes with slow motion when it lands: it
  slows by `slow` and keeps a skipped frame's ticks.

## Alternatives considered

- **Read the pause from the frame's actions.** The input state turns
  key presses into actions only once it has counted the frame's time,
  and that time depends on whether the frame steps: the actions would
  come too late, or a paused frame would advance the repeat timers and
  break the stepped run's hash.
- **Stretch `dt` for slow motion.** Fewer updates of longer frames, or
  more of shorter ones, change what the rules see; a turn game's timers
  and a tick game's steps would play differently. One normal update in
  `slow` frames plays the same updates, later.
- **Drop the input of slow motion's skipped frames.** A tap made in slow
  motion would be lost; keeping the events keeps every press.
- **Pause through a screen pushed over the game** (as a pause menu
  does). The game's screen would hear it is covered (a tick stage
  forgets its kept presses), and the debug view's gizmos would be the
  pause screen's.
