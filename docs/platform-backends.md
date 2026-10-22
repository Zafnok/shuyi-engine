# Writing a platform backend

A **backend** puts a game on a device: a window and a sound card on a
computer, the SDK on a Playdate, a console's own libraries. Everything
else (the screens, the input rules, the frame, the simulation) is the
engine's and is the same everywhere. This page says what a backend must
do. The decisions behind it are
[ADR-0008](adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§2 and
[ADR-0024](adr/0024-the-platform-boundary-app-runs-a-frame-time-in-milliseconds-storage-handed-over-and-the-harness-on-the-headless-backend.md).

| Backend | Crate | Where |
| ------- | ----- | ----- |
| macroquad: Windows, Linux, macOS, the web, the Steam Deck | `shuyi-app` | here, `crates/app` |
| headless: tests, tools, benchmarks | `shuyi_platform::headless` | here, `crates/platform` |
| Playdate | `shuyi-playdate` | here, from ticket 3010 |
| a closed console | its own crate | **a private repository**, never here |

## The rules

- **The backend owns the loop.** macroquad's async loop, the Playdate
  SDK's update callback, a console's main loop: whichever it is, it calls
  [`App::frame`](../crates/platform/src/app.rs) once per frame with the
  time on its clock. The engine never loops and never reads a clock.
- **Only a backend touches a device** (`AGENTS.md` rule 5): windows,
  files, audio devices, controllers, the clock. `shuyi-platform` itself
  touches none, and `cargo xtask check-layers` (CI's `layers` job) keeps
  macroquad, gilrs and audio crates out of it.
- **Nothing of a game in a backend** (`AGENTS.md` rule 4). What a game
  wants of its backend (a card, a voice folder) it says through `Start`
  and the backend's config.
- **Console backends stay private.** The SDKs of Nintendo, Sony and
  Microsoft, and the code written against them, are under non-disclosure
  agreements: a console backend lives in a private repository under that
  console's agreement, depends on the engine's crates like any game, and
  is never merged here, not even behind a feature. The engine's part is
  to keep `Platform` small, stable and documented.
- **Licenses.** A public backend's dependencies follow ADR-0002 §8, like
  the rest of the engine.
- **Small devices are `no_std`.** `shuyi-platform`, `shuyi-ui`,
  `shuyi-core` and `shuyi-assets` build without `std` (their default
  `std` feature off; ticket 3005 gates it), so a backend for a device
  without a standard library can use them.

## The loop

```rust
let mut platform = MyPlatform::new(/* the device's handles */);
let mut app = shuyi_platform::App::new(start); // the game's `Start`
loop {
    let now = Millis(device_clock_ms());
    let outcome = app.frame(&mut platform, now);
    for line in app.take_new_log() {
        device_log(line.level, line.category.name(), &line.text);
    }
    if outcome == FrameOutcome::Quit {
        break; // or keep calling `frame`, which shows the last frame
    }
    wait_for_next_frame();
}
```

`app.take_new_log()` gives each line of the game's log once, after the
frame it was logged in: a `shuyi_ui::log::LogLine` with its frame, its
level (`Error`, `Warn`, `Info`, `Debug`), its category and its English
text. Hand each to the device's own log (`shuyi-app` prints them with
macroquad's macros). `App` keeps the last 500 itself (`app.log()`), for
tests and tools, and `app.set_log_filter` says which levels it keeps
(`Info` and more serious by default).

`App::frame` is synchronous. Anything the device can only do
asynchronously, or only once the game's data exists (decoding its sounds,
reading its stick thresholds), the backend does **between** frames, once
`app.game()` is `Some` (`shuyi-app`'s `run` and `Macroquad::start` do).

A backend whose system says how long the last frame took, rather than
what time it is, calls `App::frame_after(&mut platform, seconds)`
instead. A backend uses one or the other.

## What one frame does

`App` calls the platform in this order:

1. `system_events`: a `Resume` restarts the clock before it is read;
2. the clock: the time since the last frame, and the fixed steps due
   (60 a second, at most 4 a frame);
3. while suspended, `poll_input` (dropped) and nothing else: no update,
   nothing presented;
4. `poll_input`;
5. `text_input`: whether there is a system keyboard, and its answer if a
   screen's ask waits for one;
6. `audio().music_playing(now)`, for the screens that keep time with the
   music;
7. the game's update (`shuyi_ui::Game::frame`);
8. `audio().play(&frame, now)` with that frame's audio, every frame;
9. `text_input().open(&ask)` if a screen asked for the system keyboard;
10. `audio().stop_all()` if the game quit in that update;
11. `present(&frame)`.

The game's first frames are different: frame 1 presents the loading card;
frame 2 calls `display` and `storage` for the game's `Setup`, runs the
game's load (seconds, on a slow device) and presents the game's first
frame; frames after run the game. A failed load presents its errors until
the player quits.

## Each method

### `display(&self) -> DisplayInfo`

The display as it is now, in device pixels, and whether it shows colour
(`Color::Rgba8`) or black and white (`OneBit`): the same
`shuyi_assets::target::Color` a target profile's display names (ADR-0008
§1). `App` hands it to the game's load (`Setup::display`), where a game
may pick its console to fit.

### `present(&mut self, frame: &Frame)`

Show the frame as soon as possible and keep it on the screen until the
next. `frame.show` is one of:

- `Show::Card(&Card)`: the loading card, while the game's data loads.
  Paint it with what the backend has of its own, since the game's font
  isn't loaded: `Card::Text` is a line centred in the backend's font, in
  `ink` on `background`; `Card::Picture` is an image from the game's
  files by path, centred at the largest whole scale that fits.
- `Show::Errors(&str)`: why the game couldn't start. Show the text (the
  backend's font, wrapped). `App` logs it once, at `Error`, so the
  backend needn't.
- `Show::Game { buffer, background, look, .. }`: the game's console, a
  `GlyphBuffer` of glyph cells and sprite items in full colour, on
  `background` (also the letterbox's). `look` holds the game's font atlas
  and image table: make the backend's textures (or 1-bit glyphs) from
  them the first time and keep them. If they can't be made, show why
  instead, as for `Errors`.

`frame.fullscreen` is the player's Fullscreen setting: a windowed backend
switches when it changes; a device that is always full screen ignores it.
`Show`, `Frame` and `DisplayInfo` are `#[non_exhaustive]`: match with a
`_` arm, since later tickets add to them (1-bit frames, ticket 3006; more
than one screen, 3023).

### `poll_input(&mut self, out: &mut Vec<RawInputEvent>)`

Add the input since the last call, in the order it happened:

- keys as `Down(Chord)` and `Up(Key)` (the engine's `Key` names, with
  Shift in the chord), and typed characters as `Text(char)` after the
  key's `Down`;
- controller buttons by position: read every pad's state and give it to
  `shuyi_ui::input::Pads`, then `RawInputEvent::from_pads`, which gives
  releases then presses and holds the rules (sticks as directions, the
  Switch-style swap, several pads as one);
- a crank through `RawInputEvent::from_crank`.

Translate only: which action a key means is the player's keymap's, never
the backend's (`AGENTS.md` rule 8). Keys the system owns (a console's home
button, the Steam button, the Playdate's Menu) never reach the game.

### `system_events(&mut self, out: &mut Vec<SystemEvent>)`

Add what the system said since the last call:

| Event | When | What `App` does |
| ----- | ---- | --------------- |
| `Suspend` | the device sleeps or locks, the home menu opens, the app goes to the background | the game's hook saves; no update and nothing presented until `Resume` |
| `Resume` | the game runs again | the time away is forgotten: the next frame counts none, so no catch-up steps |
| `FocusLost`, `FocusGained` | an overlay or another window takes the player's attention, and gives it back | the game's hook hears it (and may pause) |
| `QuitRequested` | the player closes the game (a window's close button) | the game's hook saves, then `FrameOutcome::Quit` |
| `LowBattery` | the battery is nearly empty | the game's hook may save |
| `ToggleFullscreen` | the window's own fullscreen shortcut (F11, Alt+Enter) | the Fullscreen setting flips |

A device that freezes the process without a word (the Steam Deck asleep)
sends `Resume` itself when it sees a long gap between frames (ticket
3012).

### `storage(&mut self) -> Box<dyn Storage>`

Hand over the game's storage: its saves, settings and key bindings, by
key (`^[a-z0-9_-]{1,64}$`, which a backend may turn straight into a file
name). `App` calls it once, as the game loads, and the game's context
keeps it. It must pass `shuyi_ui::storage::conformance_suite` (the
`storage-conformance` feature), in the backend's tests and, where there is
a debug build on the device, on the device's real files.

### `audio(&mut self) -> &mut dyn Audio`

The sound device:

- `music_playing(now)`: the music cue sounding and the seconds since it
  really started (once its file loaded), or `None`;
- `play(&frame, now)`: one frame's audio. Set the volumes first
  (`music_volume` and `voice_volume` reach what is already playing), then
  play the `SoundRequest`s in order (a sound cue at its manifest volume
  times its own and the sound volume; a voice clip by its file in the
  game's voice folder; stop the voice; the clips coming, to load ahead),
  then apply the `MusicCommand`s (load, start, fade, stop), and check on
  what is still loading. It is called every frame, with nothing to play
  too;
- `stop_all()`: the game quit.

Until ticket 3029 moves the rules of playing (variants, fades, the music
clock) into a player every backend shares, they are in `shuyi-app`'s
`audio` module; `shuyi_platform::Silent` plays nothing, for a backend
with no sound yet.

### `text_input(&mut self) -> Option<&mut dyn TextInput>`

The system's own keyboard for text (a console's, Steam's on-screen one),
if the device has one; `None` where the player types on keys, which come
through `poll_input`. `open(&ask)` shows it with the ask's prompt, text
and length, or is `false` if it can't now; `poll()` gives the answer
(`Typed(text)` or `Cancelled`) once the player closes it.

## The headless backend, as a reference

`shuyi_platform::headless::HeadlessPlatform` is the smallest complete
backend: it plays input and system events queued by a test, keeps the
last frame presented, hands over a memory storage, and stands in for the
music player (`Speaker`). The test Harness (`shuyi_platform::harness`, the
`harness` feature) runs every game's tests on it, through the same
`App::frame` a device runs. Read it before writing a backend, and test a
new backend's translation the same way: a fake of the device's SDK on
one side, the engine's types on the other.

## Checklist for a new backend

- [ ] A crate of its own, depending on `shuyi-platform` (and `shuyi-ui`,
      `shuyi-assets`), with the device's SDK behind it.
- [ ] Each method above, and the loop calling `App::frame`.
- [ ] Its storage passes the conformance suite.
- [ ] Its input tests: the device's buttons to `RawInputEvent`s, releases
      before presses.
- [ ] A target profile (`targets/<id>.ron`, ADR-0008 §1) naming it, and
      the target's device ticket for the numbers (ADR-0008 §6).
- [ ] For a closed console: a private repository, and nothing of it here.
